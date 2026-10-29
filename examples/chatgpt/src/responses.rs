//! A minimal streaming client for the OpenAI Responses API.
//!
//! A ChatGPT sign-in reaches the API through the ChatGPT backend that Codex
//! uses; an `OPENAI_API_KEY` reaches `api.openai.com/v1/responses` directly.
//! Both speak the same request body and server-sent events.

use std::{
    io::{BufRead as _, BufReader},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Context as _, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::ORIGINATOR;

const CHATGPT_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const API_URL: &str = "https://api.openai.com/v1/responses";
const CHATGPT_MODELS_URL: &str = "https://chatgpt.com/backend-api/codex/models";
/// The ChatGPT backend hides models whose minimum client version is newer
/// than the caller's. This client speaks the plain Responses API, so it
/// claims a version new enough to see every listed model.
const CLIENT_VERSION: &str = "99.0.0";

/// A model the account can use, as `/codex/models` describes it.
#[derive(Clone, Debug)]
pub struct Model {
    pub slug: String,
    pub name: String,
    pub efforts: Vec<String>,
    pub default_effort: Option<String>,
    pub context_window: Option<u64>,
}

/// The models offered to this account, in the backend's order. An API key
/// has no such list; the caller supplies its own.
pub fn list_models(backend: &Backend) -> Result<Vec<Model>> {
    let Backend::ChatGpt {
        access_token,
        account_id,
    } = backend
    else {
        return Ok(Vec::new());
    };

    #[derive(Deserialize)]
    struct Effort {
        effort: String,
    }
    #[derive(Deserialize)]
    struct Entry {
        slug: String,
        display_name: Option<String>,
        visibility: Option<String>,
        priority: Option<i64>,
        context_window: Option<u64>,
        default_reasoning_level: Option<String>,
        #[serde(default)]
        supported_reasoning_levels: Vec<Effort>,
    }
    #[derive(Deserialize)]
    struct Models {
        models: Vec<Entry>,
    }

    let mut request = reqwest::blocking::Client::new()
        .get(CHATGPT_MODELS_URL)
        .query(&[("client_version", CLIENT_VERSION)])
        .bearer_auth(access_token)
        .header("originator", ORIGINATOR);
    if let Some(account_id) = account_id {
        request = request.header("chatgpt-account-id", account_id);
    }
    let mut entries: Vec<Entry> = request
        .send()?
        .error_for_status()?
        .json::<Models>()?
        .models
        .into_iter()
        .filter(|entry| entry.visibility.as_deref() != Some("hide"))
        .collect();
    entries.sort_by_key(|entry| entry.priority.unwrap_or(i64::MAX));

    Ok(entries
        .into_iter()
        .map(|entry| Model {
            name: entry.display_name.unwrap_or_else(|| entry.slug.clone()),
            slug: entry.slug,
            efforts: entry
                .supported_reasoning_levels
                .into_iter()
                .map(|level| level.effort)
                .collect(),
            default_effort: entry.default_reasoning_level,
            context_window: entry.context_window,
        })
        .collect())
}

/// Where requests go and how they authenticate.
#[derive(Clone)]
pub enum Backend {
    ChatGpt {
        access_token: String,
        account_id: Option<String>,
    },
    ApiKey(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

/// One earlier turn sent back as context; the requests are stateless
/// (`store: false`), so the whole conversation goes with each request.
pub struct Turn {
    pub role: Role,
    pub text: String,
    /// Images the user attached, as `(mime type, bytes)`.
    pub images: Vec<(String, Vec<u8>)>,
}

pub struct Request {
    pub model: String,
    pub effort: Option<String>,
    pub instructions: String,
    pub turns: Vec<Turn>,
}

/// Token counts from `response.usage`.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub input_tokens_details: InputDetails,
    #[serde(default)]
    pub output_tokens_details: OutputDetails,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct InputDetails {
    #[serde(default)]
    pub cached_tokens: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct OutputDetails {
    #[serde(default)]
    pub reasoning_tokens: u64,
}

#[derive(Debug)]
pub enum StreamEvent {
    /// A piece of the reasoning summary.
    Reasoning(String),
    /// A piece of the answer.
    Text(String),
    /// A web search the model ran, with its query when known.
    WebSearch {
        id: String,
        query: Option<String>,
        done: bool,
    },
    /// A generated image so far, as PNG bytes; `done` once it is final.
    Image {
        id: String,
        bytes: Vec<u8>,
        done: bool,
    },
    /// Image generation started; the picture is not available yet.
    ImageStarted {
        id: String,
    },
    /// A web page the answer cites.
    Citation {
        title: String,
        url: String,
    },
    Completed(Usage),
    Failed(String),
}

impl Request {
    fn body(&self) -> Value {
        let input: Vec<Value> = self
            .turns
            .iter()
            .map(|turn| match turn.role {
                Role::User => {
                    let mut content = vec![json!({ "type": "input_text", "text": turn.text })];
                    content.extend(turn.images.iter().map(|(mime, bytes)| {
                        json!({
                            "type": "input_image",
                            "image_url": format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
                        })
                    }));
                    json!({ "type": "message", "role": "user", "content": content })
                }
                Role::Assistant => json!({
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": turn.text }],
                }),
            })
            .collect();

        let mut body = json!({
            "model": self.model,
            "instructions": self.instructions,
            "input": input,
            "stream": true,
            "store": false,
            "tools": [
                { "type": "web_search" },
                { "type": "image_generation", "partial_images": 2 },
            ],
        });
        if let Some(effort) = &self.effort {
            body["reasoning"] = json!({ "effort": effort, "summary": "auto" });
        }
        body
    }
}

/// Send `request` and report events until the response ends, fails or
/// `cancel` is set. Blocks; call it from a background thread.
pub fn stream(
    backend: &Backend,
    request: &Request,
    cancel: &Arc<AtomicBool>,
    mut emit: impl FnMut(StreamEvent),
) -> Result<()> {
    let client = reqwest::blocking::Client::builder().timeout(None).build()?;
    let builder = match backend {
        Backend::ChatGpt {
            access_token,
            account_id,
        } => {
            let mut builder = client
                .post(CHATGPT_URL)
                .bearer_auth(access_token)
                .header("OpenAI-Beta", "responses=experimental")
                .header("originator", ORIGINATOR);
            if let Some(account_id) = account_id {
                builder = builder.header("chatgpt-account-id", account_id);
            }
            builder
        }
        Backend::ApiKey(key) => client.post(API_URL).bearer_auth(key),
    };

    let response = builder
        .header("Accept", "text/event-stream")
        .json(&request.body())
        .send()
        .context("couldn't reach OpenAI")?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().unwrap_or_default();
        bail!("{status}: {}", error_message(&text).unwrap_or(text));
    }

    let mut data = String::new();
    for line in BufReader::new(response).lines() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let line = line?;
        if let Some(chunk) = line.strip_prefix("data:") {
            data.push_str(chunk.trim_start());
            continue;
        }
        if !line.is_empty() || data.is_empty() {
            continue;
        }
        let event: Value = match serde_json::from_str(&std::mem::take(&mut data)) {
            Ok(event) => event,
            Err(_) => continue,
        };
        if let Some(event) = parse_event(&event) {
            let finished = matches!(event, StreamEvent::Completed(_) | StreamEvent::Failed(_));
            emit(event);
            if finished {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn parse_event(event: &Value) -> Option<StreamEvent> {
    let text = |key: &str| event[key].as_str().map(str::to_string);
    match event["type"].as_str()? {
        "response.output_text.delta" => Some(StreamEvent::Text(text("delta")?)),
        "response.reasoning_summary_text.delta" => Some(StreamEvent::Reasoning(text("delta")?)),
        // Separate summary parts the way ChatGPT does, as paragraphs.
        "response.reasoning_summary_part.added" if event["summary_index"].as_u64() > Some(0) => {
            Some(StreamEvent::Reasoning("\n\n".into()))
        }
        "response.output_item.added" | "response.output_item.done"
            if event["item"]["type"] == "web_search_call" =>
        {
            Some(StreamEvent::WebSearch {
                id: event["item"]["id"].as_str()?.to_string(),
                query: event["item"]["action"]["query"]
                    .as_str()
                    .map(str::to_string),
                done: event["type"] == "response.output_item.done",
            })
        }
        "response.output_text.annotation.added"
            if event["annotation"]["type"] == "url_citation" =>
        {
            let url = event["annotation"]["url"].as_str()?.to_string();
            Some(StreamEvent::Citation {
                title: event["annotation"]["title"]
                    .as_str()
                    .unwrap_or(&url)
                    .to_string(),
                url,
            })
        }
        "response.output_item.added" if event["item"]["type"] == "image_generation_call" => {
            Some(StreamEvent::ImageStarted {
                id: event["item"]["id"].as_str()?.to_string(),
            })
        }
        "response.image_generation_call.partial_image" => Some(StreamEvent::Image {
            id: text("item_id")?,
            bytes: STANDARD.decode(event["partial_image_b64"].as_str()?).ok()?,
            done: false,
        }),
        "response.output_item.done" if event["item"]["type"] == "image_generation_call" => {
            Some(StreamEvent::Image {
                id: event["item"]["id"].as_str()?.to_string(),
                bytes: STANDARD.decode(event["item"]["result"].as_str()?).ok()?,
                done: true,
            })
        }
        "response.completed" => Some(StreamEvent::Completed(
            serde_json::from_value(event["response"]["usage"].clone()).unwrap_or_default(),
        )),
        "response.failed" | "response.incomplete" => Some(StreamEvent::Failed(
            event["response"]["error"]["message"]
                .as_str()
                .or(event["response"]["incomplete_details"]["reason"].as_str())
                .unwrap_or("The response failed.")
                .to_string(),
        )),
        "error" => Some(StreamEvent::Failed(
            text("message").unwrap_or_else(|| "The response failed.".into()),
        )),
        _ => None,
    }
}

fn error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value["error"]["message"]
        .as_str()
        .or(value["detail"].as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stream_events() {
        let text = parse_event(&json!({ "type": "response.output_text.delta", "delta": "Hi" }));
        assert!(matches!(text, Some(StreamEvent::Text(t)) if t == "Hi"));

        let part = parse_event(&json!({
            "type": "response.reasoning_summary_part.added", "summary_index": 1
        }));
        assert!(matches!(part, Some(StreamEvent::Reasoning(t)) if t == "\n\n"));
        assert!(
            parse_event(
                &json!({ "type": "response.reasoning_summary_part.added", "summary_index": 0 })
            )
            .is_none()
        );

        let completed = parse_event(&json!({
            "type": "response.completed",
            "response": { "usage": {
                "input_tokens": 10, "output_tokens": 5, "total_tokens": 15,
                "input_tokens_details": { "cached_tokens": 4 },
                "output_tokens_details": { "reasoning_tokens": 2 },
            }},
        }));
        let Some(StreamEvent::Completed(usage)) = completed else {
            panic!("expected completion");
        };
        assert_eq!(usage.total_tokens, 15);
        assert_eq!(usage.input_tokens_details.cached_tokens, 4);
        assert_eq!(usage.output_tokens_details.reasoning_tokens, 2);

        let search = parse_event(&json!({
            "type": "response.output_item.done",
            "item": { "type": "web_search_call", "id": "ws_1", "action": { "query": "gpui" } },
        }));
        assert!(matches!(
            search,
            Some(StreamEvent::WebSearch { done: true, .. })
        ));

        let citation = parse_event(&json!({
            "type": "response.output_text.annotation.added",
            "annotation": { "type": "url_citation", "url": "https://apnews.com/a", "title": "AP" },
        }));
        assert!(matches!(citation, Some(StreamEvent::Citation { title, .. }) if title == "AP"));
    }

    #[test]
    fn request_body_is_stateless_with_reasoning() {
        let body = Request {
            model: "gpt-5".into(),
            effort: Some("low".into()),
            instructions: "Be brief.".into(),
            turns: vec![Turn {
                role: Role::User,
                text: "Hi".into(),
                images: vec![("image/png".into(), vec![1, 2, 3])],
            }],
        }
        .body();
        assert_eq!(body["store"], false);
        assert_eq!(
            body["input"][0]["content"][1]["image_url"],
            "data:image/png;base64,AQID"
        );
        assert_eq!(body["stream"], true);
        assert_eq!(body["reasoning"]["effort"], "low");
        assert_eq!(body["input"][0]["content"][0]["type"], "input_text");
    }
}

//! The app window: the history sidebar, the open conversation or project,
//! and the composer.

mod project;
mod sidebar;
mod transcript;

use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _, TitleBar,
    ai::{ContextUsage, ModelOption, ModelSelector, PromptInput, PromptStatus, Source},
    attachment::{Attachment, AttachmentMedia},
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{InputEvent, InputState, TextareaState},
    message_scroller::{MessageScroller, MessageScrollerState},
    sidebar::SidebarToggleButton,
    text::TextViewState,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::{
    auth::{Credentials, Login},
    responses::{self, Backend, Model, Request, Role, StreamEvent, Turn, Usage},
    store::{self, Conversation, Project, Store, StoredMessage, StoredRole, StoredSource},
};

/// The transcript and composer share one readable column, in rems.
const READING_WIDTH: f32 = 48.;
const DEFAULT_INSTRUCTIONS: &str = "You are ChatGPT, a helpful assistant running in a desktop app. \
    Answer concisely and format with Markdown. You can search the web and generate images.";

/// How the app reaches the Responses API.
enum Session {
    SignedOut { error: Option<SharedString> },
    SigningIn,
    ChatGpt(Credentials),
    ApiKey(String),
}

/// What the main area shows.
#[derive(Clone, PartialEq, Eq)]
enum Page {
    /// The open conversation, or a new one when it has no messages.
    Chat,
    /// A project's overview, by project id.
    Project(String),
}

#[derive(Clone)]
struct WebSearch {
    id: SharedString,
    query: Option<SharedString>,
    done: bool,
}

/// An image in the transcript: attached by the user or generated.
#[derive(Clone)]
struct ChatImage {
    /// The generation call id, or the stored file name.
    key: SharedString,
    image: Option<Arc<Image>>,
    file: Option<String>,
    done: bool,
}

#[derive(Clone)]
struct ChatMessage {
    id: usize,
    role: Role,
    /// The user's text, or the answer received so far.
    text: String,
    markdown: Option<Entity<TextViewState>>,
    reasoning: SharedString,
    reasoning_open: bool,
    thought_for: Option<Duration>,
    searches: Vec<WebSearch>,
    sources: Vec<Source>,
    sources_open: bool,
    images: Vec<ChatImage>,
    error: Option<SharedString>,
    streaming: bool,
}

/// An image waiting in the composer.
struct PendingImage {
    id: usize,
    image: Arc<Image>,
}

pub struct ChatView {
    session: Session,
    store: Option<Store>,
    projects: Vec<Project>,
    conversations: Vec<Conversation>,
    /// The open conversation's id; `None` until its first message.
    current: Option<String>,
    /// The project a new conversation joins.
    new_chat_project: Option<String>,
    page: Page,
    messages: Vec<ChatMessage>,
    scroller: Entity<MessageScrollerState>,
    prompt: Entity<TextareaState>,
    search: Entity<InputState>,
    attachments: Vec<PendingImage>,
    sidebar_collapsed: bool,
    models: Vec<Model>,
    model: SharedString,
    effort: SharedString,
    usage: Option<Usage>,
    next_id: usize,
    cancel: Option<Arc<AtomicBool>>,
    _stream: Option<Task<()>>,
    _login: Option<Task<()>>,
    _models: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl ChatView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 8)
                .submit_on_enter(true)
                .placeholder("Message ChatGPT…")
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search chats"));
        let scroller = cx.new(|cx| MessageScrollerState::new(0, cx));
        let _subscriptions = vec![
            cx.observe(&scroller, |_, _, cx| cx.notify()),
            cx.subscribe_in(
                &prompt,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { shift: false, .. } = event {
                        this.send(window, cx);
                    }
                },
            ),
            cx.observe(&prompt, |_, _, cx| cx.notify()),
            cx.observe(&search, |_, _, cx| cx.notify()),
        ];

        let session = match (Credentials::load(), std::env::var("OPENAI_API_KEY")) {
            (Some(credentials), _) => Session::ChatGpt(credentials),
            (None, Ok(key)) if !key.is_empty() => Session::ApiKey(key),
            _ => Session::SignedOut { error: None },
        };
        let store = Store::open().ok();
        let projects = store.as_ref().map(Store::projects).unwrap_or_default();
        let conversations = store.as_ref().map(Store::conversations).unwrap_or_default();

        let mut this = Self {
            session,
            store,
            projects,
            conversations,
            current: None,
            new_chat_project: None,
            page: Page::Chat,
            messages: Vec::new(),
            scroller,
            prompt,
            search,
            attachments: Vec::new(),
            sidebar_collapsed: false,
            models: Vec::new(),
            model: SharedString::default(),
            effort: "medium".into(),
            usage: None,
            next_id: 0,
            cancel: None,
            _stream: None,
            _login: None,
            _models: None,
            _subscriptions,
        };
        this.load_models(cx);
        this
    }

    fn backend(&self) -> Option<Backend> {
        match &self.session {
            Session::ChatGpt(credentials) => Some(Backend::ChatGpt {
                access_token: credentials.access_token().to_string(),
                account_id: credentials.account_id().map(str::to_string),
            }),
            Session::ApiKey(key) => Some(Backend::ApiKey(key.clone())),
            _ => None,
        }
    }

    /// Ask the backend which models this account may use. `CHATGPT_MODELS`
    /// (comma-separated) overrides the list, and supplies it for API keys.
    fn load_models(&mut self, cx: &mut Context<Self>) {
        let Some(backend) = self.backend() else {
            return;
        };
        let configured: Vec<Model> = std::env::var("CHATGPT_MODELS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|slug| !slug.is_empty())
            .map(|slug| Model {
                slug: slug.to_string(),
                name: slug.to_string(),
                efforts: ["low", "medium", "high"].map(String::from).to_vec(),
                default_effort: Some("medium".into()),
                context_window: None,
            })
            .collect();
        if !configured.is_empty() || matches!(backend, Backend::ApiKey(_)) {
            let models = if configured.is_empty() {
                vec![Model {
                    slug: "gpt-5".into(),
                    name: "gpt-5".into(),
                    efforts: ["minimal", "low", "medium", "high"]
                        .map(String::from)
                        .to_vec(),
                    default_effort: Some("medium".into()),
                    context_window: Some(400_000),
                }]
            } else {
                configured
            };
            self.set_models(models, cx);
            return;
        }

        let (tx, rx) = smol::channel::bounded(1);
        std::thread::spawn(move || _ = tx.send_blocking(responses::list_models(&backend)));
        self._models = Some(cx.spawn(async move |this, cx| {
            let Ok(result) = rx.recv().await else {
                return;
            };
            _ = this.update(cx, |this, cx| match result {
                Ok(models) => this.set_models(models, cx),
                Err(err) => {
                    this.session = Session::SignedOut {
                        error: Some(format!("Couldn’t load models: {err:#}").into()),
                    };
                    cx.notify();
                }
            });
        }));
    }

    fn set_models(&mut self, models: Vec<Model>, cx: &mut Context<Self>) {
        self.models = models;
        if !self
            .models
            .iter()
            .any(|model| model.slug == self.model.as_ref())
            && let Some(first) = self.models.first()
        {
            self.model = first.slug.clone().into();
        }
        self.select_model(self.model.clone(), cx);
    }

    fn selected_model(&self) -> Option<&Model> {
        self.models
            .iter()
            .find(|model| model.slug == self.model.as_ref())
    }

    fn select_model(&mut self, slug: SharedString, cx: &mut Context<Self>) {
        self.model = slug;
        if let Some(model) = self.selected_model()
            && !model
                .efforts
                .iter()
                .any(|effort| effort == self.effort.as_ref())
        {
            self.effort = model
                .default_effort
                .clone()
                .or_else(|| model.efforts.first().cloned())
                .unwrap_or_default()
                .into();
        }
        cx.notify();
    }

    fn status(&self) -> PromptStatus {
        match self.messages.last() {
            Some(message) if message.streaming && message.text.is_empty() => {
                PromptStatus::Submitted
            }
            Some(message) if message.streaming => PromptStatus::Streaming,
            Some(message) if message.error.is_some() => PromptStatus::Error,
            _ => PromptStatus::Ready,
        }
    }

    fn is_busy(&self) -> bool {
        matches!(
            self.status(),
            PromptStatus::Submitted | PromptStatus::Streaming
        )
    }

    fn sign_in(&mut self, cx: &mut Context<Self>) {
        self.session = Session::SigningIn;
        cx.notify();

        let (url_tx, url_rx) = smol::channel::bounded::<String>(1);
        let (done_tx, done_rx) = smol::channel::bounded(1);
        std::thread::spawn(move || {
            let result = Login::start().and_then(|login| {
                _ = url_tx.send_blocking(login.url().to_string());
                login.finish()
            });
            _ = done_tx.send_blocking(result);
        });

        self._login = Some(cx.spawn(async move |this, cx| {
            if let Ok(url) = url_rx.recv().await {
                _ = cx.update(|cx| cx.open_url(&url));
            }
            let Ok(result) = done_rx.recv().await else {
                return;
            };
            _ = this.update(cx, |this, cx| {
                this.session = match result {
                    Ok(credentials) => Session::ChatGpt(credentials),
                    Err(err) => Session::SignedOut {
                        error: Some(format!("Couldn’t sign in: {err:#}").into()),
                    },
                };
                this.load_models(cx);
                cx.notify();
            });
        }));
    }

    fn sign_out(&mut self, cx: &mut Context<Self>) {
        self.stop(cx);
        Credentials::forget();
        self.session = Session::SignedOut { error: None };
        cx.notify();
    }

    /// Start an empty conversation, inside `project` when given.
    fn new_chat(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        self.stop(cx);
        self.current = None;
        self.new_chat_project = project;
        self.page = Page::Chat;
        self.messages.clear();
        self.usage = None;
        self.scroller.update(cx, |state, cx| state.reset(0, cx));
        cx.notify();
    }

    fn open_conversation(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(conversation) = self.conversations.iter().find(|c| c.id == id).cloned() else {
            return;
        };
        self.stop(cx);
        self.page = Page::Chat;
        self.current = Some(conversation.id.clone());
        self.new_chat_project = conversation.project_id.clone();
        self.usage = None;
        self.messages = conversation
            .messages
            .iter()
            .map(|stored| self.restore_message(stored, cx))
            .collect();
        let count = self.messages.len();
        self.scroller.update(cx, |state, cx| {
            state.reset(count, cx);
            state.scroll_to_end(cx);
        });
        cx.notify();
    }

    fn restore_message(&mut self, stored: &StoredMessage, cx: &mut Context<Self>) -> ChatMessage {
        let role = match stored.role {
            StoredRole::User => Role::User,
            StoredRole::Assistant => Role::Assistant,
        };
        let id = self.next_id;
        self.next_id += 1;
        ChatMessage {
            id,
            role,
            text: stored.text.clone(),
            markdown: (role == Role::Assistant)
                .then(|| cx.new(|cx| TextViewState::markdown(&stored.text, cx))),
            reasoning: stored.reasoning.clone().into(),
            reasoning_open: false,
            thought_for: stored.thought_for_ms.map(Duration::from_millis),
            searches: stored
                .searches
                .iter()
                .enumerate()
                .map(|(ix, query)| WebSearch {
                    id: ix.to_string().into(),
                    query: Some(query.clone().into()),
                    done: true,
                })
                .collect(),
            sources: stored
                .sources
                .iter()
                .map(|source| Source::new(source.title.clone(), source.url.clone()))
                .collect(),
            sources_open: false,
            images: stored
                .images
                .iter()
                .map(|file| ChatImage {
                    key: file.clone().into(),
                    image: self.read_image(file),
                    file: Some(file.clone()),
                    done: true,
                })
                .collect(),
            error: stored.error.clone().map(Into::into),
            streaming: false,
        }
    }

    fn read_image(&self, file: &str) -> Option<Arc<Image>> {
        let path = self.store.as_ref()?.image_path(file);
        let format = image_format(&path)?;
        Some(Arc::new(Image::from_bytes(
            format,
            std::fs::read(path).ok()?,
        )))
    }

    /// Write the open conversation back to the history.
    fn save_current(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.current.clone() else {
            return;
        };
        let messages: Vec<StoredMessage> = self
            .messages
            .iter()
            .map(|message| StoredMessage {
                role: match message.role {
                    Role::User => StoredRole::User,
                    Role::Assistant => StoredRole::Assistant,
                },
                text: message.text.clone(),
                reasoning: message.reasoning.to_string(),
                thought_for_ms: message.thought_for.map(|d| d.as_millis() as u64),
                searches: message
                    .searches
                    .iter()
                    .filter_map(|search| search.query.as_ref().map(ToString::to_string))
                    .collect(),
                sources: message
                    .sources
                    .iter()
                    .map(|source| StoredSource {
                        title: source.title().to_string(),
                        url: source.url().to_string(),
                    })
                    .collect(),
                images: message
                    .images
                    .iter()
                    .filter_map(|i| i.file.clone())
                    .collect(),
                error: message.error.as_ref().map(ToString::to_string),
            })
            .collect();

        let Some(ix) = self.conversations.iter().position(|c| c.id == id) else {
            return;
        };
        let mut conversation = self.conversations.remove(ix);
        conversation.messages = messages;
        conversation.updated_at = store::now();
        if let Some(store) = &self.store {
            _ = store.save_conversation(&conversation);
        }
        // Most recent first, as the sidebar lists them.
        self.conversations.insert(0, conversation);
        cx.notify();
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.prompt.read(cx).value();
        self.send_text(text.to_string(), window, cx);
    }

    fn send_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if text.trim().is_empty() && self.attachments.is_empty() || self.is_busy() {
            return;
        }
        if self.backend().is_none() {
            return;
        }

        // The first message creates the conversation in the history.
        if self.current.is_none() {
            let mut conversation = Conversation::new(self.new_chat_project.clone());
            conversation.title = if text.trim().is_empty() {
                "Image".into()
            } else {
                store::title_from(&text)
            };
            self.current = Some(conversation.id.clone());
            self.conversations.insert(0, conversation);
        }
        self.page = Page::Chat;

        let images: Vec<ChatImage> = std::mem::take(&mut self.attachments)
            .into_iter()
            .map(|pending| ChatImage {
                key: pending.id.to_string().into(),
                file: self.store.as_ref().and_then(|store| {
                    store
                        .save_image(&pending.image.bytes, pending.image.format.extension())
                        .ok()
                }),
                image: Some(pending.image),
                done: true,
            })
            .collect();

        self.prompt
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.push_message(Role::User, text, images, cx);
        self.start_response(cx);
    }

    /// Send the conversation so far and stream the reply into a new message.
    fn start_response(&mut self, cx: &mut Context<Self>) {
        let Some(backend) = self.backend() else {
            return;
        };
        let credentials = match &self.session {
            Session::ChatGpt(credentials) => Some(credentials.clone()),
            _ => None,
        };
        let turns: Vec<Turn> = self
            .messages
            .iter()
            .filter(|message| message.error.is_none())
            .filter(|message| !message.text.is_empty() || !message.images.is_empty())
            .map(|message| Turn {
                role: message.role,
                text: message.text.clone(),
                images: match message.role {
                    Role::User => message
                        .images
                        .iter()
                        .filter_map(|image| image.image.as_ref())
                        .map(|image| (image.format.mime_type().to_string(), image.bytes.clone()))
                        .collect(),
                    Role::Assistant => Vec::new(),
                },
            })
            .collect();
        self.push_message(Role::Assistant, String::new(), Vec::new(), cx);

        let project_instructions = self
            .current_project()
            .map(|project| project.instructions.clone())
            .filter(|instructions| !instructions.trim().is_empty());
        let mut instructions =
            std::env::var("CHATGPT_INSTRUCTIONS").unwrap_or_else(|_| DEFAULT_INSTRUCTIONS.into());
        if let Some(project_instructions) = project_instructions {
            instructions.push_str("\n\nProject instructions:\n");
            instructions.push_str(&project_instructions);
        }
        let request = Request {
            model: self.model.to_string(),
            effort: Some(self.effort.to_string()),
            instructions,
            turns,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());

        // `None` refreshed credentials mean the session did not change.
        let (tx, rx) = smol::channel::unbounded::<(StreamEvent, Option<Credentials>)>();
        std::thread::spawn(move || {
            let mut backend = backend;
            if let Some(credentials) = credentials.filter(Credentials::is_expiring) {
                match credentials.refresh() {
                    Ok(credentials) => {
                        backend = Backend::ChatGpt {
                            access_token: credentials.access_token().to_string(),
                            account_id: credentials.account_id().map(str::to_string),
                        };
                        _ = tx.send_blocking((StreamEvent::Text(String::new()), Some(credentials)));
                    }
                    Err(err) => {
                        _ = tx.send_blocking((StreamEvent::Failed(format!("{err:#}")), None));
                        return;
                    }
                }
            }
            let result = responses::stream(&backend, &request, &cancel, |event| {
                _ = tx.send_blocking((event, None));
            });
            if let Err(err) = result {
                _ = tx.send_blocking((StreamEvent::Failed(format!("{err:#}")), None));
            }
        });

        let started = Instant::now();
        self._stream = Some(cx.spawn(async move |this, cx| {
            while let Ok((event, credentials)) = rx.recv().await {
                let finished = matches!(event, StreamEvent::Completed(_) | StreamEvent::Failed(_));
                let alive = this.update(cx, |this, cx| {
                    if let Some(credentials) = credentials {
                        this.session = Session::ChatGpt(credentials);
                    }
                    this.apply(event, started, cx);
                });
                if alive.is_err() || finished {
                    break;
                }
            }
            _ = this.update(cx, |this, cx| this.finish_streaming(started, cx));
        }));
        self.save_current(cx);
    }

    /// Replace the last reply with a new one.
    fn regenerate(&mut self, cx: &mut Context<Self>) {
        if self.is_busy() || self.messages.last().map(|m| m.role) != Some(Role::Assistant) {
            return;
        }
        self.messages.pop();
        let count = self.messages.len();
        self.scroller.update(cx, |state, cx| state.reset(count, cx));
        self.start_response(cx);
    }

    fn push_message(
        &mut self,
        role: Role,
        text: String,
        images: Vec<ChatImage>,
        cx: &mut Context<Self>,
    ) {
        let markdown =
            (role == Role::Assistant).then(|| cx.new(|cx| TextViewState::markdown("", cx)));
        self.messages.push(ChatMessage {
            id: self.next_id,
            role,
            text,
            markdown,
            reasoning: SharedString::default(),
            reasoning_open: true,
            thought_for: None,
            searches: Vec::new(),
            sources: Vec::new(),
            sources_open: false,
            images,
            error: None,
            streaming: role == Role::Assistant,
        });
        self.next_id += 1;
        self.scroller.update(cx, |state, cx| {
            state.append(1, cx);
        });
    }

    fn apply(&mut self, event: StreamEvent, started: Instant, cx: &mut Context<Self>) {
        let Some(message) = self.messages.last_mut() else {
            return;
        };
        match event {
            StreamEvent::Reasoning(delta) => {
                message.reasoning = format!("{}{delta}", message.reasoning).into();
            }
            StreamEvent::Text(delta) => {
                if delta.is_empty() {
                    return;
                }
                // The answer has started: close the thinking and time it.
                if message.text.is_empty() {
                    message.reasoning_open = false;
                    message.thought_for = Some(started.elapsed());
                }
                message.text.push_str(&delta);
                if let Some(markdown) = &message.markdown {
                    markdown.update(cx, |state, cx| state.push_str(&delta, cx));
                }
            }
            StreamEvent::WebSearch { id, query, done } => {
                match message.searches.iter_mut().find(|search| search.id == id) {
                    Some(search) => {
                        search.done |= done;
                        if query.is_some() {
                            search.query = query.map(Into::into);
                        }
                    }
                    None => message.searches.push(WebSearch {
                        id: id.into(),
                        query: query.map(Into::into),
                        done,
                    }),
                }
            }
            StreamEvent::ImageStarted { id } => {
                if !message.images.iter().any(|image| image.key == id) {
                    message.images.push(ChatImage {
                        key: id.into(),
                        image: None,
                        file: None,
                        done: false,
                    });
                }
                message.reasoning_open = false;
            }
            StreamEvent::Image { id, bytes, done } => {
                let file = done
                    .then(|| self.store.as_ref()?.save_image(&bytes, "png").ok())
                    .flatten();
                let image = Some(Arc::new(Image::from_bytes(ImageFormat::Png, bytes)));
                match message.images.iter_mut().find(|image| image.key == id) {
                    Some(existing) => {
                        existing.image = image;
                        existing.done = done;
                        existing.file = file.or(existing.file.take());
                    }
                    None => message.images.push(ChatImage {
                        key: id.into(),
                        image,
                        file,
                        done,
                    }),
                }
            }
            StreamEvent::Citation { title, url } => {
                // The answer cites the same page once per sentence; list it once.
                if !message
                    .sources
                    .iter()
                    .any(|source| source.url().as_ref() == url)
                {
                    message.sources.push(Source::new(title, url));
                }
            }
            StreamEvent::Completed(usage) => self.usage = Some(usage),
            StreamEvent::Failed(error) => message.error = Some(error.into()),
        }
        self.remeasure_last(cx);
        cx.notify();
    }

    fn remeasure_last(&mut self, cx: &mut Context<Self>) {
        let Some(last) = self.messages.len().checked_sub(1) else {
            return;
        };
        self.scroller.update(cx, |state, cx| {
            _ = state.remeasure_items(last..last + 1, cx)
        });
    }

    fn finish_streaming(&mut self, started: Instant, cx: &mut Context<Self>) {
        if let Some(message) = self.messages.last_mut() {
            message.streaming = false;
            message.thought_for.get_or_insert_with(|| started.elapsed());
            if message.text.is_empty() && message.error.is_none() {
                message.reasoning_open = false;
            }
        }
        self.cancel = None;
        self.save_current(cx);
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        let was_streaming = self._stream.take().is_some();
        if let Some(message) = self.messages.last_mut() {
            message.streaming = false;
        }
        if was_streaming {
            self.save_current(cx);
        }
        cx.notify();
    }

    fn set_message_flag(
        &mut self,
        id: usize,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut ChatMessage),
    ) {
        if let Some(ix) = self.messages.iter().position(|message| message.id == id) {
            update(&mut self.messages[ix]);
            self.scroller
                .update(cx, |state, cx| _ = state.remeasure_items(ix..ix + 1, cx));
            cx.notify();
        }
    }

    fn attach_images(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            _ = this.update(cx, |this, cx| this.add_image_paths(&paths, cx));
        })
        .detach();
    }

    fn add_image_paths(&mut self, paths: &[std::path::PathBuf], cx: &mut Context<Self>) {
        for path in paths {
            let (Some(format), Ok(bytes)) = (image_format(path), std::fs::read(path)) else {
                continue;
            };
            self.attachments.push(PendingImage {
                id: self.next_id,
                image: Arc::new(Image::from_bytes(format, bytes)),
            });
            self.next_id += 1;
        }
        cx.notify();
    }

    fn save_image_as(&mut self, image: Arc<Image>, cx: &mut Context<Self>) {
        let directory = dirs::download_dir().unwrap_or_else(std::env::temp_dir);
        let path = cx.prompt_for_new_path(&directory, Some("image.png"));
        cx.spawn(async move |_, _| {
            if let Ok(Ok(Some(path))) = path.await {
                _ = std::fs::write(path, &image.bytes);
            }
        })
        .detach();
    }

    fn current_project(&self) -> Option<&Project> {
        let id = match &self.page {
            Page::Project(id) => Some(id),
            Page::Chat => self.new_chat_project.as_ref(),
        }?;
        self.projects.iter().find(|project| &project.id == id)
    }

    fn current_title(&self) -> SharedString {
        if let Page::Project(_) = self.page {
            return self
                .current_project()
                .map(|project| project.name.clone())
                .unwrap_or_default()
                .into();
        }
        self.current
            .as_ref()
            .and_then(|id| self.conversations.iter().find(|c| &c.id == id))
            .map(|conversation| conversation.title.clone().into())
            .unwrap_or_else(|| "New chat".into())
    }

    fn render_signed_out(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (signing_in, error) = match &self.session {
            Session::SigningIn => (true, None),
            Session::SignedOut { error } => (false, error.clone()),
            _ => (false, None),
        };

        v_flex()
            .flex_1()
            .w_full()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .child(div().text_xl().font_semibold().child("ChatGPT"))
            .child(
                div()
                    .max_w_96()
                    .text_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if signing_in {
                        "Finish signing in in your browser."
                    } else {
                        "Sign in with your ChatGPT account, or set OPENAI_API_KEY to use an API key."
                    }),
            )
            .child(
                Button::new("sign-in")
                    .primary()
                    .label("Sign in with ChatGPT…")
                    .loading(signing_in)
                    .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))),
            )
            .when_some(error, |this, error| {
                this.child(
                    div()
                        .max_w_96()
                        .text_center()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error),
                )
            })
    }

    fn render_title_bar(&self, signed_in: bool, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new().child(
            h_flex()
                .w_full()
                .min_w_0()
                .pr_2()
                .gap_2()
                .when(signed_in, |this| {
                    this.child(
                        SidebarToggleButton::new()
                            .collapsed(self.sidebar_collapsed)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sidebar_collapsed = !this.sidebar_collapsed;
                                cx.notify();
                            })),
                    )
                })
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_sm()
                        .font_medium()
                        .child(if signed_in {
                            self.current_title()
                        } else {
                            "ChatGPT".into()
                        }),
                ),
        )
    }

    fn render_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.status();
        let usage = self.usage.unwrap_or_default();
        let selected = self.selected_model();
        let context_window = selected.and_then(|model| model.context_window);
        let efforts: Vec<ModelOption> = selected
            .map(|model| {
                model
                    .efforts
                    .iter()
                    .map(|effort| ModelOption::new(effort.clone(), capitalize(effort)))
                    .collect()
            })
            .unwrap_or_default();

        PromptInput::new("prompt", &self.prompt)
            .status(status)
            .when(!self.attachments.is_empty(), |this| {
                self.attachments.iter().fold(this, |this, pending| {
                    let id = pending.id;
                    this.attachment(
                        Attachment::new()
                            .id(("attachment", id))
                            .axis(Axis::Vertical)
                            .media(AttachmentMedia::new().src(pending.image.clone()))
                            .on_remove(cx.listener(move |this, _, _, cx| {
                                this.attachments.retain(|pending| pending.id != id);
                                cx.notify();
                            })),
                    )
                })
            })
            .tool(
                Button::new("attach")
                    .ghost()
                    .small()
                    .icon(IconName::Paperclip)
                    .accessibility_label("Attach images")
                    .tooltip("Attach images…")
                    .disabled(self.is_busy())
                    .on_click(cx.listener(|this, _, _, cx| this.attach_images(cx))),
            )
            .tool(
                ModelSelector::new("model")
                    .models(
                        self.models
                            .iter()
                            .map(|model| ModelOption::new(model.slug.clone(), model.name.clone())),
                    )
                    .efforts(efforts)
                    .model(self.model.clone())
                    .effort(self.effort.clone())
                    .disabled(self.is_busy())
                    .on_model_change(cx.listener(|this, id: &SharedString, _, cx| {
                        this.select_model(id.clone(), cx)
                    }))
                    .on_effort_change(cx.listener(|this, id: &SharedString, _, cx| {
                        this.effort = id.clone();
                        cx.notify();
                    })),
            )
            .when_some(
                context_window.filter(|_| self.usage.is_some()),
                |this, window| {
                    this.tool(
                        ContextUsage::new("context", usage.total_tokens, window)
                            .input(usage.input_tokens)
                            .output(usage.output_tokens)
                            .reasoning(usage.output_tokens_details.reasoning_tokens)
                            .cached(usage.input_tokens_details.cached_tokens),
                    )
                },
            )
            .on_submit(cx.listener(|this, text: &SharedString, window, cx| {
                this.send_text(text.to_string(), window, cx)
            }))
            .on_stop({
                let view = cx.entity().downgrade();
                move |_, cx| _ = view.update(cx, |this, cx| this.stop(cx))
            })
    }

    fn render_main(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let messages = self.messages.clone();
        let view = cx.entity().downgrade();

        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.add_image_paths(paths.paths(), cx)
            }))
            .child(div().flex_1().min_h_0().w_full().map(|this| {
                if let Page::Project(id) = &self.page {
                    this.child(self.render_project_page(id, cx))
                } else if self.messages.is_empty() {
                    this.child(self.render_empty(cx))
                } else {
                    this.child(
                        MessageScroller::new(
                            "transcript",
                            self.scroller.clone(),
                            move |ix, _, cx| match messages.get(ix).cloned() {
                                Some(message) => div()
                                    .w_full()
                                    .max_w(rems(READING_WIDTH))
                                    .child(Self::render_row(message, view.clone(), cx))
                                    .into_any_element(),
                                None => div().into_any_element(),
                            },
                        )
                        .with_list_style(StyleRefinement::default().px_6().py_6())
                        .with_row_style(StyleRefinement::default().flex().flex_col().items_center())
                        .with_bottom_fade(cx.theme().background),
                    )
                }
            }))
            .child(
                h_flex().w_full().justify_center().px_6().pb_4().child(
                    div()
                        .w_full()
                        .max_w(rems(READING_WIDTH))
                        .child(self.render_prompt(cx)),
                ),
            )
    }
}

impl Render for ChatView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let signed_in = matches!(self.session, Session::ChatGpt(_) | Session::ApiKey(_));
        let root = v_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(self.render_title_bar(signed_in, cx));

        if !signed_in {
            return root.child(self.render_signed_out(cx));
        }

        root.child(
            h_flex()
                .items_stretch()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(self.render_sidebar(cx))
                .child(self.render_main(cx)),
        )
    }
}

fn image_format(path: &Path) -> Option<ImageFormat> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::Webp),
        "gif" => Some(ImageFormat::Gif),
        _ => None,
    }
}

fn capitalize(value: &str) -> String {
    match value {
        "xhigh" => "Extra high".into(),
        _ => {
            let mut chars = value.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        }
    }
}

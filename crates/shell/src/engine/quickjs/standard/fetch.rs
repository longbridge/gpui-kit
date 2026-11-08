use crate::engine::quickjs::js_error;
use std::{io::Read as _, time::Duration};

use rquickjs::{
    ArrayBuffer, Ctx, FromJs, Function, IntoJs, Object, Promise, Result, TypedArray, Value,
    function::{Func, Opt},
};

use super::super::{host, scheduler};

const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
const TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals().set("fetch", Func::from(fetch))
}

fn fetch<'js>(ctx: Ctx<'js>, url: String, options: Opt<FetchOptions>) -> Result<Promise<'js>> {
    let initial = reqwest::Url::parse(&url)
        .map_err(|error| js_error::type_error(&ctx, &format!("invalid fetch URL: {error}")))?;
    let options = options.0.unwrap_or_default();
    let capabilities = host::capabilities();
    authorize_request(&capabilities, &options.method, &initial)
        .map_err(|error| js_error::type_error(&ctx, &error))?;

    #[cfg(test)]
    let injected_client =
        crate::scope::current_runtime().and_then(|runtime| runtime.test_http_client());
    #[cfg(not(test))]
    let injected_client = None;

    scheduler::blocking(&ctx, "fetch(url, options)", move || {
        request(capabilities, initial, options, injected_client)
    })
}

/// The intentionally small, data-only request surface. It keeps HTTP traffic
/// capability-gated while allowing OAuth form exchanges and authenticated reads.
struct FetchOptions {
    method: reqwest::Method,
    headers: reqwest::header::HeaderMap,
    body: Vec<u8>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            method: reqwest::Method::GET,
            headers: reqwest::header::HeaderMap::new(),
            body: Vec::new(),
        }
    }
}

impl<'js> FromJs<'js> for FetchOptions {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.is_null() || value.is_undefined() {
            return Ok(Self::default());
        }
        let Some(object) = value.into_object() else {
            return Err(js_error::type_error(
                ctx,
                "fetch(url, options) expects an object with method, headers and/or body",
            ));
        };
        for key in object.keys::<String>() {
            let key = key?;
            if !matches!(key.as_str(), "method" | "headers" | "body") {
                return Err(js_error::type_error(
                    ctx,
                    &format!(
                        "unknown option `{key}` for fetch(url, options); expected method, headers or body"
                    ),
                ));
            }
        }

        let method = match object.get::<_, Option<String>>("method")? {
            None => reqwest::Method::GET,
            Some(method) => parse_method(ctx, &method)?,
        };
        let headers = parse_headers(ctx, object.get("headers")?)?;
        let body = parse_body(ctx, object.get("body")?)?;
        Ok(Self {
            method,
            headers,
            body,
        })
    }
}

/// Any method the capability policy is willing to grant.
///
/// This is a token check rather than a list. What may be sent, to which host,
/// on which path, is `Capabilities::may_request`'s decision and it already
/// takes the method -- a second list here would be a second policy to keep in
/// step with the first, and the way that goes wrong is a grant that cannot be
/// exercised. So a method that is a method is parsed, and refusing it stays
/// where refusing belongs.
///
/// What is refused here is a string that is not an HTTP method at all: an
/// empty field, a space, a quote. `Method::from_bytes` is exactly that judge.
/// Known methods are upper-cased on the way through, as `fetch` does.
fn parse_method(ctx: &Ctx<'_>, method: &str) -> Result<reqwest::Method> {
    reqwest::Method::from_bytes(method.to_ascii_uppercase().as_bytes()).map_err(|_| {
        js_error::type_error(
            ctx,
            &format!("fetch(url, options).method `{method}` is not an HTTP method"),
        )
    })
}

fn parse_headers<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<reqwest::header::HeaderMap> {
    if value.is_null() || value.is_undefined() {
        return Ok(reqwest::header::HeaderMap::new());
    }
    let Some(object) = value.into_object() else {
        return Err(js_error::type_error(
            ctx,
            "fetch(url, options).headers expects a plain object of string values",
        ));
    };

    let mut headers = reqwest::header::HeaderMap::new();
    for entry in object.props::<String, Value>() {
        let (name, value) = entry?;
        if prohibited_header(&name) {
            return Err(js_error::type_error(
                ctx,
                &format!("fetch(url, options).headers may not set `{name}`"),
            ));
        }
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            js_error::type_error(
                ctx,
                "fetch(url, options).headers has an invalid header name",
            )
        })?;
        let value = String::from_js(ctx, value).map_err(|_| {
            js_error::type_error(
                ctx,
                "fetch(url, options).headers expects string header values",
            )
        })?;
        let value = reqwest::header::HeaderValue::from_str(&value).map_err(|_| {
            js_error::type_error(
                ctx,
                "fetch(url, options).headers has an invalid header value",
            )
        })?;
        headers.append(name, value);
    }
    Ok(headers)
}

fn prohibited_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "host"
            | "content-length"
            | "connection"
            | "expect"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn parse_body<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Vec<u8>> {
    if value.is_null() || value.is_undefined() {
        return Ok(Vec::new());
    }
    if value.is_string() {
        return limited_body(ctx, String::from_js(ctx, value)?.into_bytes());
    }
    let bytes = TypedArray::<u8>::from_js(ctx, value).map_err(|_| {
        js_error::type_error(
            ctx,
            "fetch(url, options).body expects a string or Uint8Array",
        )
    })?;
    // SAFETY: finish copying before error creation or other JavaScript can run.
    let bytes = unsafe { bytes.as_bytes().map(<[u8]>::to_vec) }.ok_or_else(|| {
        js_error::type_error(
            ctx,
            "fetch(url, options).body received a detached Uint8Array",
        )
    })?;
    limited_body(ctx, bytes)
}

fn limited_body<'js>(ctx: &Ctx<'js>, body: Vec<u8>) -> Result<Vec<u8>> {
    if body.len() as u64 > MAX_BODY_BYTES {
        return Err(js_error::range_error(
            ctx,
            &format!("fetch request body exceeded the {MAX_BODY_BYTES} byte limit"),
        ));
    }
    Ok(body)
}

fn authorize_request(
    capabilities: &crate::Capabilities,
    method: &reqwest::Method,
    url: &reqwest::Url,
) -> std::result::Result<(), String> {
    let host_name = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if capabilities.may_request(
        url.scheme(),
        &host_name,
        url.port(),
        method.as_str(),
        url.path(),
    ) {
        Ok(())
    } else {
        Err(format!(
            "HTTP request {} {} is not granted; add it to capabilities.network.hosts or capabilities.network.http",
            method, url
        ))
    }
}

fn request(
    capabilities: crate::Capabilities,
    mut url: reqwest::Url,
    options: FetchOptions,
    injected_client: Option<reqwest::blocking::Client>,
) -> std::result::Result<FetchResponse, String> {
    let client = match injected_client {
        Some(client) => client,
        None => client_builder()
            .build()
            .map_err(|error| format!("creating HTTP client failed: {error}"))?,
    };

    let mut method = options.method;
    let mut headers = options.headers;
    let mut body = options.body;
    for _ in 0..=MAX_REDIRECTS {
        let response = client
            .request(method.clone(), url.clone())
            .headers(headers.clone())
            .body(body.clone())
            .send()
            .map_err(|error| format!("fetching {url} failed: {error}"))?;
        if follows_location(response.status()) {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .ok_or_else(|| format!("redirect from {url} has no Location header"))?
                .to_str()
                .map_err(|error| format!("redirect from {url} has an invalid Location: {error}"))?;
            let next = url
                .join(location)
                .map_err(|error| format!("redirect from {url} is invalid: {error}"))?;
            rewrite_redirect_request(response.status(), &mut method, &mut headers, &mut body);
            authorize_redirect(&capabilities, &method, &url, &next, &headers)?;
            url = next;
            continue;
        }

        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let mut bytes = Vec::new();
        response
            .take(MAX_BODY_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("reading response from {final_url} failed: {error}"))?;
        if bytes.len() as u64 > MAX_BODY_BYTES {
            return Err(format!(
                "response body from {final_url} exceeded the {MAX_BODY_BYTES} byte limit"
            ));
        }
        return Ok(FetchResponse {
            status,
            url: final_url,
            body: bytes,
        });
    }
    Err(format!("fetch exceeded the {MAX_REDIRECTS} redirect limit"))
}

fn follows_location(status: reqwest::StatusCode) -> bool {
    matches!(
        status,
        reqwest::StatusCode::MOVED_PERMANENTLY
            | reqwest::StatusCode::FOUND
            | reqwest::StatusCode::SEE_OTHER
            | reqwest::StatusCode::TEMPORARY_REDIRECT
            | reqwest::StatusCode::PERMANENT_REDIRECT
    )
}

fn rewrite_redirect_request(
    status: reqwest::StatusCode,
    method: &mut reqwest::Method,
    headers: &mut reqwest::header::HeaderMap,
    body: &mut Vec<u8>,
) {
    let becomes_get = ((status == reqwest::StatusCode::MOVED_PERMANENTLY
        || status == reqwest::StatusCode::FOUND)
        && *method == reqwest::Method::POST)
        || (status == reqwest::StatusCode::SEE_OTHER && *method != reqwest::Method::HEAD);
    if becomes_get {
        *method = reqwest::Method::GET;
        body.clear();
        headers.remove(reqwest::header::CONTENT_LENGTH);
        headers.remove(reqwest::header::CONTENT_TYPE);
    }
}

/// The default `User-Agent` sits on the client rather than in the request's
/// headers: a header the script sets still replaces it, and it does not count
/// as a caller-supplied header that `authorize_redirect` keeps on its origin.
fn client_builder() -> reqwest::blocking::ClientBuilder {
    reqwest::blocking::Client::builder()
        .user_agent(super::USER_AGENT)
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
}

#[cfg(test)]
pub(super) fn direct_test_client() -> std::result::Result<reqwest::blocking::Client, reqwest::Error>
{
    client_builder().no_proxy().build()
}

fn authorize_redirect(
    capabilities: &crate::Capabilities,
    method: &reqwest::Method,
    current: &reqwest::Url,
    next: &reqwest::Url,
    headers: &reqwest::header::HeaderMap,
) -> std::result::Result<(), String> {
    authorize_request(capabilities, method, next)
        .map_err(|error| format!("redirect target refused: {error}"))?;
    if current.scheme() == "https" && next.scheme() != "https" {
        return Err(format!(
            "redirect from {current} to {next} refused because it is an HTTPS downgrade"
        ));
    }
    if *method != reqwest::Method::GET && !same_origin(current, next) {
        return Err(format!(
            "cross-origin redirect from {current} to {next} refused because it would replay a {method} request"
        ));
    }
    if headers.contains_key(reqwest::header::AUTHORIZATION) && !same_origin(current, next) {
        return Err(format!(
            "cross-origin redirect from {current} to {next} refused because the request carries Authorization"
        ));
    }
    if !headers.is_empty() && !same_origin(current, next) {
        return Err(format!(
            "cross-origin redirect from {current} to {next} refused because caller-supplied request headers would be replayed"
        ));
    }
    Ok(())
}

/// An origin includes the scheme and effective port as well as the host. A
/// bearer credential may follow a same-origin redirect, never a cross-origin
/// one, even when both hosts are individually capability-granted.
fn same_origin(left: &reqwest::Url, right: &reqwest::Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str().is_some_and(|host| {
            right
                .host_str()
                .is_some_and(|other| host.eq_ignore_ascii_case(other))
        })
        && left.port_or_known_default() == right.port_or_known_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_location_redirect_statuses_are_followed() {
        for status in [
            reqwest::StatusCode::MOVED_PERMANENTLY,
            reqwest::StatusCode::FOUND,
            reqwest::StatusCode::SEE_OTHER,
            reqwest::StatusCode::TEMPORARY_REDIRECT,
            reqwest::StatusCode::PERMANENT_REDIRECT,
        ] {
            assert!(follows_location(status), "status {status}");
        }
        for status in [
            reqwest::StatusCode::MULTIPLE_CHOICES,
            reqwest::StatusCode::NOT_MODIFIED,
            reqwest::StatusCode::USE_PROXY,
        ] {
            assert!(!follows_location(status), "status {status}");
        }
    }

    #[test]
    fn redirect_statuses_apply_fetch_method_and_body_rewrites() {
        for (status, initial, expected) in [
            (
                reqwest::StatusCode::MOVED_PERMANENTLY,
                reqwest::Method::POST,
                reqwest::Method::GET,
            ),
            (
                reqwest::StatusCode::FOUND,
                reqwest::Method::POST,
                reqwest::Method::GET,
            ),
            (
                reqwest::StatusCode::SEE_OTHER,
                reqwest::Method::PUT,
                reqwest::Method::GET,
            ),
            (
                reqwest::StatusCode::TEMPORARY_REDIRECT,
                reqwest::Method::POST,
                reqwest::Method::POST,
            ),
            (
                reqwest::StatusCode::PERMANENT_REDIRECT,
                reqwest::Method::POST,
                reqwest::Method::POST,
            ),
        ] {
            let mut method = initial;
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            );
            let mut body = b"side effect".to_vec();

            rewrite_redirect_request(status, &mut method, &mut headers, &mut body);

            assert_eq!(method, expected, "status {status}");
            if method == reqwest::Method::GET {
                assert!(body.is_empty());
                assert!(!headers.contains_key(reqwest::header::CONTENT_TYPE));
            } else {
                assert!(!body.is_empty());
                assert!(headers.contains_key(reqwest::header::CONTENT_TYPE));
            }
        }
    }

    #[test]
    fn authorization_never_follows_a_cross_origin_redirect() {
        let capabilities = crate::Capabilities::new().network_hosts([
            "api.example.test".to_owned(),
            "login.example.test".to_owned(),
        ]);
        let current = reqwest::Url::parse("https://api.example.test/v1/quote").expect("URL");
        let next = reqwest::Url::parse("https://login.example.test/continue").expect("URL");
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_static("Bearer secret"),
        );

        let error = authorize_redirect(
            &capabilities,
            &reqwest::Method::GET,
            &current,
            &next,
            &headers,
        )
        .expect_err("authorization must not cross origins");
        assert!(error.contains("cross-origin redirect"), "{error}");
    }

    #[test]
    fn https_redirects_never_downgrade_to_plain_http() {
        let capabilities =
            crate::Capabilities::new().network_hosts(["api.example.test".to_owned()]);
        let current = reqwest::Url::parse("https://api.example.test/start").expect("URL");
        let next = reqwest::Url::parse("http://api.example.test/continue").expect("URL");

        let error = authorize_redirect(
            &capabilities,
            &reqwest::Method::GET,
            &current,
            &next,
            &reqwest::header::HeaderMap::new(),
        )
        .expect_err("an HTTPS request must never be redirected onto plaintext HTTP");
        assert!(error.contains("HTTPS downgrade"), "{error}");
    }

    #[test]
    fn a_post_is_never_replayed_across_origins() {
        let capabilities = crate::Capabilities::new().network_hosts([
            "api.example.test".to_owned(),
            "login.example.test".to_owned(),
        ]);
        let current = reqwest::Url::parse("https://api.example.test/token").expect("URL");
        let next = reqwest::Url::parse("https://login.example.test/token").expect("URL");

        let error = authorize_redirect(
            &capabilities,
            &reqwest::Method::POST,
            &current,
            &next,
            &reqwest::header::HeaderMap::new(),
        )
        .expect_err("a credential-bearing POST body must stay on its origin");
        assert!(error.contains("POST"), "{error}");
        assert!(error.contains("cross-origin"), "{error}");
    }

    #[test]
    fn caller_supplied_headers_never_follow_a_cross_origin_redirect() {
        let capabilities = crate::Capabilities::new()
            .network_hosts(["api.example.test".to_owned(), "cdn.example.test".to_owned()]);
        let current = reqwest::Url::parse("https://api.example.test/data").expect("URL");
        let next = reqwest::Url::parse("https://cdn.example.test/data").expect("URL");
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::HeaderName::from_static("x-api-key"),
            reqwest::header::HeaderValue::from_static("secret"),
        );

        let error = authorize_redirect(
            &capabilities,
            &reqwest::Method::GET,
            &current,
            &next,
            &headers,
        )
        .expect_err("caller-supplied headers belong to the original origin");
        assert!(error.contains("request headers"), "{error}");
    }

    #[test]
    fn an_http_rule_cannot_be_bypassed_with_an_unlisted_method_or_path() {
        let capabilities =
            crate::Capabilities::new().http_requests([crate::HttpRequestGrant::new(
                "api.example.test",
                ["GET"],
                ["/v1/read/profile"],
                std::iter::empty::<&str>(),
            )]);
        let allowed =
            reqwest::Url::parse("https://api.example.test/v1/read/profile").expect("allowed URL");
        let denied =
            reqwest::Url::parse("https://api.example.test/v1/write/item").expect("denied URL");

        assert!(authorize_request(&capabilities, &reqwest::Method::GET, &allowed).is_ok());
        assert!(authorize_request(&capabilities, &reqwest::Method::POST, &allowed).is_err());
        assert!(authorize_request(&capabilities, &reqwest::Method::POST, &denied).is_err());
    }

    #[test]
    fn an_http_rule_is_bound_to_scheme_effective_port_and_path_segments() {
        let capabilities =
            crate::Capabilities::new().http_requests([crate::HttpRequestGrant::new(
                "api.example.test",
                ["GET"],
                std::iter::empty::<&str>(),
                ["/v1/account"],
            )]);
        let https =
            reqwest::Url::parse("https://api.example.test/v1/account/profile").expect("HTTPS URL");
        let plaintext =
            reqwest::Url::parse("http://api.example.test/v1/account/profile").expect("HTTP URL");
        let alternate_port =
            reqwest::Url::parse("https://api.example.test:8443/v1/account/profile")
                .expect("alternate-port URL");
        let adjacent = reqwest::Url::parse("https://api.example.test/v1/accounts-delete")
            .expect("adjacent path URL");

        assert!(authorize_request(&capabilities, &reqwest::Method::GET, &https).is_ok());
        assert!(authorize_request(&capabilities, &reqwest::Method::GET, &plaintext).is_err());
        assert!(authorize_request(&capabilities, &reqwest::Method::GET, &alternate_port).is_err());
        assert!(authorize_request(&capabilities, &reqwest::Method::GET, &adjacent).is_err());
    }

    #[test]
    fn a_redirect_is_checked_against_http_method_and_path_rules() {
        let capabilities =
            crate::Capabilities::new().http_requests([crate::HttpRequestGrant::new(
                "api.example.test",
                ["GET"],
                ["/allowed"],
                std::iter::empty::<&str>(),
            )]);
        let current = reqwest::Url::parse("https://api.example.test/allowed").expect("URL");
        let next = reqwest::Url::parse("https://api.example.test/admin").expect("URL");

        let error = authorize_redirect(
            &capabilities,
            &reqwest::Method::GET,
            &current,
            &next,
            &reqwest::header::HeaderMap::new(),
        )
        .expect_err("redirect path needs its own grant");
        assert!(error.contains("HTTP request"), "{error}");
    }

    #[test]
    fn origin_comparison_includes_scheme_host_and_effective_port() {
        let origin = reqwest::Url::parse("https://api.example.test:443/v1/quote").expect("URL");
        for (candidate, expected) in [
            ("https://api.example.test/next", true),
            ("http://api.example.test/next", false),
            ("https://other.example.test/next", false),
            ("https://api.example.test:8443/next", false),
        ] {
            assert_eq!(
                same_origin(&origin, &reqwest::Url::parse(candidate).expect("URL")),
                expected,
                "{candidate}"
            );
        }
    }
}

struct FetchResponse {
    status: u16,
    url: String,
    body: Vec<u8>,
}

impl<'js> IntoJs<'js> for FetchResponse {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let response = Object::new(ctx.clone())?;
        response.set("status", self.status)?;
        response.set("ok", (200..300).contains(&self.status))?;
        response.set("url", self.url)?;
        let body = std::rc::Rc::new(self.body);
        let text = String::from_utf8_lossy(&body).into_owned();
        let text_method = text.clone();
        let buffer_body = body.clone();
        response.set(
            "arrayBuffer",
            Func::from(move |ctx: Ctx<'js>| -> Result<Promise<'js>> {
                let (promise, resolve, _) = Promise::new(&ctx)?;
                let buffer = ArrayBuffer::new_copy(ctx.clone(), buffer_body.as_slice())?;
                resolve.call::<_, ()>((buffer,))?;
                Ok(promise)
            }),
        )?;
        response.set(
            "bytes",
            Func::from(move |ctx: Ctx<'js>| -> Result<Promise<'js>> {
                let (promise, resolve, _) = Promise::new(&ctx)?;
                let bytes = TypedArray::<u8>::new_copy(ctx.clone(), body.as_slice())?;
                resolve.call::<_, ()>((bytes,))?;
                Ok(promise)
            }),
        )?;
        response.set(
            "text",
            Func::from(move |ctx: Ctx<'js>| -> Result<Promise<'js>> {
                let (promise, resolve, _) = Promise::new(&ctx)?;
                resolve.call::<_, ()>((text_method.clone(),))?;
                Ok(promise)
            }),
        )?;
        response.set(
            "json",
            Func::from(move |ctx: Ctx<'js>| -> Result<Promise<'js>> {
                let (promise, resolve, reject) = Promise::new(&ctx)?;
                let json: Object = ctx.globals().get("JSON")?;
                let parse: Function = json.get("parse")?;
                match parse.call::<_, Value>((text.clone(),)) {
                    Ok(value) => resolve.call::<_, ()>((value,))?,
                    Err(rquickjs::Error::Exception) => {
                        reject.call::<_, ()>((ctx.catch(),))?;
                    }
                    Err(error) => return Err(error),
                }
                Ok(promise)
            }),
        )?;
        Ok(response.into_value())
    }
}

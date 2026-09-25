use super::*;
use crate::{HttpRequestGrant, ScriptView, ShellRuntime};
use gpui::{
    Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext,
    VisualTestContext, div,
    http_client::{AsyncBody, FakeHttpClient, Method, RedirectPolicy, Response},
};
use std::{ops::Deref as _, sync::Mutex};

// A two-pixel PNG generated in memory, not an external fixture or URL.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0xf4, 0x22, 0x7f,
    0x8a, 0x00, 0x00, 0x00, 0x11, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x14, 0x32, 0x09, 0xfb,
    0xcf, 0xc0, 0xc0, 0xc0, 0x00, 0x00, 0x09, 0x0d, 0x01, 0x9d, 0xf7, 0x66, 0xcd, 0x15, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

type Requests = Arc<Mutex<Vec<String>>>;

fn get_grant(paths: &[&str]) -> Capabilities {
    Capabilities::new().http_requests([HttpRequestGrant::new(
        "images.example",
        ["GET"],
        paths.iter().copied(),
        [] as [&str; 0],
    )])
}

fn recording_client(
    reply: impl Fn(&str) -> Response<AsyncBody> + Send + Sync + 'static,
) -> (Arc<dyn HttpClient>, Requests) {
    let requests = Requests::default();
    let recorded = requests.clone();
    let client = FakeHttpClient::create(move |request| {
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.extensions().get::<RedirectPolicy>(),
            Some(&RedirectPolicy::NoFollow),
            "the transport must not follow redirects before authorization",
        );
        let url = request.uri().to_string();
        recorded.lock().unwrap().push(url.clone());
        let response = reply(&url);
        async move { Ok(response) }
    });
    (client, requests)
}

fn png_response() -> Response<AsyncBody> {
    Response::builder()
        .status(200)
        .header("content-type", "image/png")
        .body(PNG.to_vec().into())
        .unwrap()
}

fn fetch(
    client: Arc<dyn HttpClient>,
    capabilities: Capabilities,
) -> Result<Vec<u8>, ImageCacheError> {
    smol::block_on(request_image(
        client,
        capabilities,
        reqwest::Url::parse("https://images.example/image.png").unwrap(),
    ))
}

fn redirect(status: u16, target: &str) -> Response<AsyncBody> {
    Response::builder()
        .status(status)
        .header("location", target)
        .body(().into())
        .unwrap()
}

#[test]
fn document_image_urls_obey_get_grants() {
    let exact = get_grant(&["/image.png"]);
    for (url, allowed) in [
        ("https://images.example/image.png", true),
        (
            "https://images.example:443/image.png?size=small#preview",
            true,
        ),
        ("https://images.example/other.png", false),
        ("https://other.example/image.png", false),
        ("https://images.example:8443/image.png", false),
        ("http://images.example/image.png", false),
        ("data:image/png;base64,AAAA", false),
        ("file:///image.png", false),
        ("custom:image.png", false),
        ("//images.example/image.png", false),
        ("/image.png", false),
        ("image.png", false),
        ("https://", false),
        ("https://user:password@images.example/image.png", false),
    ] {
        assert_eq!(image_url(&exact, url).is_ok(), allowed, "{url}");
        assert!(image_url(&Capabilities::new(), url).is_err(), "{url}");
        if let Ok(parsed) = reqwest::Url::parse(url) {
            let (client, requests) = recording_client(|_| png_response());
            assert_eq!(
                smol::block_on(request_image(client, exact.clone(), parsed)).is_ok(),
                allowed,
                "{url}",
            );
            assert_eq!(
                requests.lock().unwrap().len(),
                usize::from(allowed),
                "{url}"
            );
        }
    }
    let post_only = Capabilities::new().http_requests([HttpRequestGrant::new(
        "images.example",
        ["POST"],
        ["/image.png"],
        [] as [&str; 0],
    )]);
    assert!(image_url(&post_only, "https://images.example/image.png").is_err());
    let (client, requests) = recording_client(|_| png_response());
    assert!(fetch(client, post_only).is_err());
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn document_image_redirects_reauthorize_each_hop() {
    for target in [
        "https://other.example/image.png",
        "https://images.example/not-granted.png",
        "https://images.example:8443/image.png",
        "file:///image.png",
        "data:image/png;base64,AAAA",
        "custom:image.png",
        "https://user:password@images.example/image.png",
    ] {
        let (client, requests) = recording_client(move |_| redirect(302, target));
        let result = fetch(client, get_grant(&["/image.png"]));
        assert!(result.is_err(), "{target}");
        assert_eq!(requests.lock().unwrap().len(), 1, "{target}");
    }
    // Even a legacy grant to both protocols must not allow HTTPS downgrade.
    let (client, requests) = recording_client(|_| redirect(302, "http://images.example/image.png"));
    let result = fetch(
        client,
        Capabilities::new().network_hosts(["images.example".to_owned()]),
    );
    assert!(result.is_err());
    assert_eq!(requests.lock().unwrap().len(), 1);
}

#[gpui::test]
fn document_image_redirects_preserve_authorized_get(cx: &mut TestAppContext) {
    for status in [301, 302, 303, 307, 308] {
        let (client, requests) = recording_client(move |url| {
            if url.ends_with("/image.png") {
                redirect(status, "/final.png")
            } else {
                png_response()
            }
        });
        let bytes = fetch(client, get_grant(&["/image.png", "/final.png"])).unwrap();
        assert_eq!(bytes, PNG);
        let image = cx
            .update(|cx| decode_image(bytes, cx.svg_renderer()))
            .unwrap();
        assert_eq!(image.size(0), gpui::size(2.into(), 1.into()));
        assert_eq!(&image.as_bytes(0).unwrap()[..4], &[0x56, 0x34, 0x12, 0xff]);
        assert_eq!(
            *requests.lock().unwrap(),
            [
                "https://images.example/image.png",
                "https://images.example/final.png",
            ],
        );
    }
}

#[test]
fn document_image_requests_bound_bodies_and_redirects() {
    for (status, body, allowed) in [
        (200, vec![0; MAX_IMAGE_BYTES as usize], true),
        (200, vec![0; MAX_IMAGE_BYTES as usize + 1], false),
        (404, Vec::new(), false),
        (302, Vec::new(), false), // Missing Location must not trigger a fallback.
    ] {
        let (client, requests) = recording_client(move |_| {
            Response::builder()
                .status(status)
                .body(body.clone().into())
                .unwrap()
        });
        let result = fetch(client, get_grant(&["/image.png"]));
        assert_eq!(result.is_ok(), allowed);
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
    let (client, requests) = recording_client(|_| redirect(302, "/image.png"));
    assert!(fetch(client, get_grant(&["/image.png"])).is_err());
    assert_eq!(requests.lock().unwrap().len(), MAX_REDIRECTS + 1);
}

struct Documents(Vec<Entity<ScriptView>>);

impl Render for Documents {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().children(self.0.iter().cloned())
    }
}

fn source(format: &str, document: &str, scrollable: bool) -> String {
    let document = serde_json::to_string(document).unwrap();
    format!(
        r#"
import {{ View }} from "gpui-kit";
import {{ TextView }} from "gpui-base";
export default class Document extends View {{
  render() {{ return TextView.{format}("document", {document}).scrollable({scrollable}); }}
}}
"#,
    )
}

fn mount_documents(
    cx: &mut TestAppContext,
    source: &str,
    capabilities: Vec<Capabilities>,
) -> VisualTestContext {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("document-images.js", source)
        .expect("source");
    let window = cx.add_window(move |window, cx| {
        let views = capabilities
            .into_iter()
            .enumerate()
            .map(|(ix, capabilities)| {
                let policy = Rc::new(
                    Policy::new()
                        .with_application(format!("document-{ix}"))
                        .with_capabilities(capabilities),
                );
                runtime
                    .instantiate_view_with_policy(&view_type, policy, window, cx)
                    .expect("view")
            })
            .collect();
        Documents(views)
    });
    VisualTestContext::from_window(*window.deref(), cx)
}

fn draw_documents(cx: &mut VisualTestContext) {
    for _ in 0..4 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
    }
}

#[gpui::test]
fn text_view_document_images_require_policy(cx: &mut TestAppContext) {
    for (format, document) in [
        ("markdown", "![image](https://images.example/image.png)"),
        (
            "markdown",
            "Before ![image](https://images.example/image.png) after",
        ),
        (
            "markdown",
            "| Image |\n| --- |\n| ![image](https://images.example/image.png) |",
        ),
        ("html", r#"<img src="https://images.example/image.png">"#),
        (
            "html",
            r#"<p>Before <img width="2" height="1" src="https://images.example/image.png"> after</p>"#,
        ),
        (
            "html",
            r#"<table><tr><td><img src="https://images.example/image.png"></td></tr></table>"#,
        ),
    ] {
        for scrollable in [false, true] {
            for allowed in [false, true] {
                let mut app = cx.new_app();
                let (client, requests) = recording_client(|_| png_response());
                app.update(|cx| cx.set_http_client(client));
                let capabilities = if allowed {
                    get_grant(&["/image.png"])
                } else {
                    Capabilities::new()
                };
                let mut context = mount_documents(
                    &mut app,
                    &source(format, document, scrollable),
                    vec![capabilities],
                );
                draw_documents(&mut context);
                assert_eq!(
                    requests.lock().unwrap().len(),
                    usize::from(allowed),
                    "{format}, scrollable={scrollable}, allowed={allowed}: {document}",
                );
                // Repainting must use this document's cache, not refetch or fall back.
                draw_documents(&mut context);
                assert_eq!(requests.lock().unwrap().len(), usize::from(allowed));
                app.quit();
            }
        }
    }
}

#[gpui::test]
fn text_view_data_images_cannot_bypass_policy(cx: &mut TestAppContext) {
    // This valid image would otherwise be decoded directly by Base, bypassing
    // ImageCache::load entirely. Check the asset cache as well as the transport.
    let data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAAEUlEQVR4nGMUMgn7z8DAwAAACQ0BnfdmzRUAAAAASUVORK5CYII=";
    for (format, document) in [
        ("markdown", format!("Before ![image]({data}) after")),
        ("html", format!(r#"<img src="{data}">"#)),
    ] {
        let mut app = cx.new_app();
        let (client, requests) = recording_client(|_| png_response());
        app.update(|cx| cx.set_http_client(client));
        let mut context = mount_documents(
            &mut app,
            &source(format, &document, false),
            vec![Capabilities::new()],
        );
        draw_documents(&mut context);
        assert!(requests.lock().unwrap().is_empty());
        let image = Arc::new(gpui::Image::from_bytes(
            gpui::ImageFormat::Png,
            PNG.to_vec(),
        ));
        assert!(!context.update(|_, cx| gpui::ImageSource::Image(image).is_asset_cached(cx)));
        app.quit();
    }
}

#[gpui::test]
fn text_view_document_images_keep_policies_separate(cx: &mut TestAppContext) {
    let (client, requests) = recording_client(|url| {
        if url.ends_with("/image.png") {
            redirect(302, "/final.png")
        } else {
            png_response()
        }
    });
    cx.update(|cx| cx.set_http_client(client));
    let mut context = mount_documents(
        cx,
        &source(
            "markdown",
            "![image](https://images.example/image.png)",
            false,
        ),
        vec![
            get_grant(&["/image.png", "/final.png"]),
            get_grant(&["/image.png"]),
            Capabilities::new(),
        ],
    );
    draw_documents(&mut context);
    let mut actual = requests.lock().unwrap().clone();
    actual.sort();
    assert_eq!(
        actual,
        [
            "https://images.example/final.png",
            "https://images.example/image.png",
            "https://images.example/image.png",
        ],
        "only the first document may follow the redirect; none may borrow another cache",
    );
}

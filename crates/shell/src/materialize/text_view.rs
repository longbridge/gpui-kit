//! Document images use the describing script's grant, not the host's ambient
//! image loader. Scope the cache across every phase: InlineFlow also creates
//! image elements during prepaint, and table sizing can measure them in layout.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io::Cursor,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use gpui::{
    App, Context, Element, ElementId, Entity, EntityId, GlobalElementId, ImageCache,
    ImageCacheError, InspectorElementId, IntoElement, LayoutId, RenderImage, Resource,
    SharedString, SharedUri, SvgRenderer, Task, Window, http_client::HttpClient,
};
use gpui_base::TextView;
use image::AnimationDecoder as _;
use smol::io::AsyncReadExt as _;

use crate::{Capabilities, capability::is_openable_url, policy::Policy};

const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
const IMAGE_TIMEOUT: Duration = Duration::from_secs(30);

type ImageResult = Result<Arc<RenderImage>, ImageCacheError>;

/// A transparent wrapper: keep TextView's identity, layout and interactions.
/// No script callback can replace this cache or reach the default image loader.
pub(crate) struct PolicyTextView {
    view: TextView,
    policy: Rc<Policy>,
    cache_id: SharedString,
}

impl PolicyTextView {
    pub(super) fn new(view: TextView, policy: Rc<Policy>) -> Self {
        // A cache's authority never changes. Including the retained policy identity
        // also separates old and replacement snapshots with the same element id.
        let cache_id = format!(
            "{}/shell-images/{:p}",
            view.id().expect("TextView has an element id"),
            Rc::as_ptr(&policy),
        )
        .into();
        Self {
            view: view.image_source(|uri| uri.clone().into()),
            policy,
            cache_id,
        }
    }
}

impl IntoElement for PolicyTextView {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for PolicyTextView {
    type RequestLayoutState = (
        <TextView as Element>::RequestLayoutState,
        Entity<DocumentImages>,
    );
    type PrepaintState = <TextView as Element>::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.view.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.view.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let policy = self.policy.clone();
        let images = window.use_keyed_state(self.cache_id.clone(), cx, move |_, cx| {
            DocumentImages::new(policy, cx)
        });
        let (layout, state) = window.with_image_cache(Some(images.clone().into()), |window| {
            self.view.request_layout(id, inspector_id, window, cx)
        });
        (layout, (state, images))
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        window.with_image_cache(Some(state.1.clone().into()), |window| {
            self.view
                .prepaint(id, inspector_id, bounds, &mut state.0, window, cx)
        })
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        state: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_image_cache(Some(state.1.clone().into()), |window| {
            self.view
                .paint(id, inspector_id, bounds, &mut state.0, prepaint, window, cx);
        });
    }
}

/// Each keyed TextView owns a separate cache. A URL loaded by a broader grant
/// must not satisfy the same URL under a different grant after a redirect.
pub(crate) struct DocumentImages {
    policy: Rc<Policy>,
    entries: HashMap<SharedUri, ImageEntry>,
}

impl DocumentImages {
    fn new(policy: Rc<Policy>, cx: &mut Context<Self>) -> Self {
        cx.on_release(|images: &mut Self, cx| images.clear(cx))
            .detach();
        Self {
            policy,
            entries: HashMap::new(),
        }
    }

    fn clear(&mut self, cx: &mut App) {
        for entry in self.entries.values() {
            entry.drop_image(cx);
        }
        // Dropping entries cancels their pending tasks as well.
        self.entries.clear();
    }
}

impl ImageCache for DocumentImages {
    fn load(
        &mut self,
        resource: &Resource,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<ImageResult> {
        let Resource::Uri(uri) = resource else {
            return Some(Err(denied_image()));
        };
        // Authorize before consulting any cache, including a failed entry.
        let mut url = match image_url(self.policy.capabilities(), uri.as_ref()) {
            Ok(url) => url,
            Err(error) => return Some(Err(error)),
        };
        url.set_fragment(None);
        let key: SharedUri = url.as_str().to_owned().into();
        let entry = self
            .entries
            .entry(key)
            .or_insert_with(|| ImageEntry::new(url, self.policy.capabilities().clone(), cx));
        let mut state = entry.state.borrow_mut();
        if let Some(result) = &state.result {
            Some(result.clone())
        } else {
            state.views.insert(window.current_view());
            None
        }
    }
}

struct ImageEntry {
    state: Rc<RefCell<ImageLoad>>,
    _task: Task<()>,
}

#[derive(Default)]
struct ImageLoad {
    result: Option<ImageResult>,
    views: HashSet<EntityId>,
}

impl ImageEntry {
    fn new(url: reqwest::Url, capabilities: Capabilities, cx: &mut App) -> Self {
        let state = Rc::new(RefCell::new(ImageLoad::default()));
        let client = cx.http_client();
        let renderer = cx.svg_renderer();
        let timeout = cx.background_executor().timer(IMAGE_TIMEOUT);
        let task = cx.background_executor().spawn(async move {
            smol::future::race(
                async move {
                    let bytes = request_image(client, capabilities, url).await?;
                    decode_image(bytes, renderer)
                },
                async move {
                    timeout.await;
                    Err(ImageCacheError::Asset(
                        "TextView image request timed out".into(),
                    ))
                },
            )
            .await
        });
        let weak = Rc::downgrade(&state);
        let task = cx.spawn(async move |cx| {
            let result = task.await;
            let Some(state) = weak.upgrade() else {
                return;
            };
            let views = {
                let mut state = state.borrow_mut();
                state.result = Some(result);
                std::mem::take(&mut state.views)
            };
            cx.update(|cx| {
                for view in views {
                    cx.notify(view);
                }
            });
        });
        Self { state, _task: task }
    }

    fn drop_image(&self, cx: &mut App) {
        if let Some(Ok(image)) = &self.state.borrow().result {
            cx.drop_image(image.clone(), None);
        }
    }
}

fn denied_image() -> ImageCacheError {
    ImageCacheError::Asset(
        "TextView images require an absolute HTTP(S) URL and a capabilities.network GET grant"
            .into(),
    )
}

fn image_url(capabilities: &Capabilities, value: &str) -> Result<reqwest::Url, ImageCacheError> {
    if !is_openable_url(value) {
        return Err(denied_image());
    }
    let url = reqwest::Url::parse(value).map_err(|_| denied_image())?;
    // Do not turn document-supplied userinfo into implicit HTTP credentials.
    if !url.username().is_empty() || url.password().is_some() {
        return Err(denied_image());
    }
    if !capabilities.may_request(
        url.scheme(),
        url.host_str().unwrap_or_default(),
        url.port(),
        "GET",
        url.path(),
    ) {
        return Err(denied_image());
    }
    Ok(url)
}

async fn request_image(
    client: Arc<dyn HttpClient>,
    capabilities: Capabilities,
    mut url: reqwest::Url,
) -> Result<Vec<u8>, ImageCacheError> {
    for redirects in 0..=MAX_REDIRECTS {
        url = image_url(&capabilities, url.as_str())?;
        url.set_fragment(None);
        // The host transport must never follow a redirect on our behalf.
        let mut response = client.get(url.as_str(), ().into(), false).await?;
        if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            if redirects == MAX_REDIRECTS {
                return Err(ImageCacheError::Asset(
                    "Too many TextView image redirects".into(),
                ));
            }
            let next = response
                .headers()
                .get("location")
                .and_then(|location| location.to_str().ok())
                .and_then(|location| url.join(location).ok())
                .ok_or_else(|| ImageCacheError::Asset("Invalid TextView image redirect".into()))?;
            if url.scheme() == "https" && next.scheme() == "http" {
                return Err(ImageCacheError::Asset(
                    "TextView image HTTPS downgrade refused".into(),
                ));
            }
            // The next iteration re-authorizes scheme, host, port, GET and path.
            url = next;
            continue;
        }
        if !response.status().is_success() {
            return Err(ImageCacheError::Asset(
                format!("TextView image request returned {}", response.status()).into(),
            ));
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .take(MAX_IMAGE_BYTES + 1)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() as u64 > MAX_IMAGE_BYTES {
            return Err(ImageCacheError::Asset(
                "TextView image exceeds the 8 MiB limit".into(),
            ));
        }
        return Ok(bytes);
    }
    unreachable!("the final redirect is rejected above")
}

/// Match GPUI's resource decoding without handing it the document URL again.
/// Otherwise decoding would silently start a second, ungated HTTP request.
fn decode_image(bytes: Vec<u8>, renderer: SvgRenderer) -> ImageResult {
    let Ok(format) = image::guess_format(&bytes) else {
        return renderer
            .render_single_frame(&bytes, 1.0)
            .map_err(Into::into);
    };
    let frames = match format {
        image::ImageFormat::Gif => animation_frames(
            image::codecs::gif::GifDecoder::new(Cursor::new(&bytes))?.into_frames(),
        )?,
        image::ImageFormat::WebP => {
            let mut decoder = image::codecs::webp::WebPDecoder::new(Cursor::new(&bytes))?;
            if decoder.has_animation() {
                let _ = decoder.set_background_color(image::Rgba([0, 0, 0, 0]));
                animation_frames(decoder.into_frames())?
            } else {
                static_frame(decoder)?
            }
        }
        _ => static_frame(
            image::ImageReader::with_format(Cursor::new(&bytes), format).into_decoder()?,
        )?,
    };
    Ok(Arc::new(RenderImage::new(frames)))
}

fn static_frame(
    mut decoder: impl image::ImageDecoder,
) -> Result<smallvec::SmallVec<[image::Frame; 1]>, ImageCacheError> {
    let orientation = decoder.orientation()?;
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    let mut data = image.into_rgba8();
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Ok(smallvec::smallvec![image::Frame::new(data)])
}

fn animation_frames(
    frames: image::Frames<'_>,
) -> Result<smallvec::SmallVec<[image::Frame; 1]>, ImageCacheError> {
    let mut decoded = smallvec::SmallVec::new();
    for frame in frames {
        match frame {
            Ok(mut frame) => {
                for pixel in frame.buffer_mut().chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }
                decoded.push(frame);
            }
            Err(error) => tracing::debug!(%error, "Skipping an invalid TextView image frame"),
        }
    }
    if decoded.is_empty() {
        return Err(ImageCacheError::Asset(
            "TextView image has no decodable frames".into(),
        ));
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests;

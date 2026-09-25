//! Fetch document images with the creating script's policy. GPUI owns asynchronous
//! loading, caching and completion notifications; the document owns their lifetime.

use std::{collections::HashSet, io::Cursor, rc::Rc, sync::Arc, time::Duration};

use gpui::{
    App, Asset, Bounds, Element, ElementId, GlobalElementId, ImageCacheError, ImageSource,
    InspectorElementId, IntoElement, LayoutId, Pixels, RenderImage, SharedString, SharedUri,
    SvgRenderer, WeakEntity, Window, http_client::HttpClient,
};
use gpui_base::TextView;
use image::AnimationDecoder as _;
use smol::io::AsyncReadExt as _;

use crate::{Capabilities, capability::is_openable_url, policy::Policy};

const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
const IMAGE_TIMEOUT: Duration = Duration::from_secs(30);

type ImageResult = Result<Arc<RenderImage>, ImageCacheError>;

pub(super) fn with_policy(view: TextView, policy: Rc<Policy>) -> impl IntoElement {
    PolicyTextView { view, policy }
}

/// CLI check constructs elements without drawing. Only layout may initialize
/// the keyed image owner; GPUI's Asset still owns all loading and notifications.
struct PolicyTextView {
    view: TextView,
    policy: Rc<Policy>,
}

impl IntoElement for PolicyTextView {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for PolicyTextView {
    type RequestLayoutState = <TextView as Element>::RequestLayoutState;
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
        self.view = image_sources(self.view.clone(), self.policy.clone(), window, cx);
        self.view.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.view
            .prepaint(id, inspector_id, bounds, state, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.view
            .paint(id, inspector_id, bounds, state, prepaint, window, cx);
    }
}

fn image_sources(
    view: TextView,
    policy: Rc<Policy>,
    window: &mut Window,
    cx: &mut App,
) -> TextView {
    let key = SharedString::from(format!(
        "{}/shell-images/{:p}",
        view.id().expect("TextView has an element id"),
        Rc::as_ptr(&policy),
    ));
    let images = window.use_keyed_state(key, cx, |_, cx| {
        let owner = cx.weak_entity();
        cx.on_release(move |images: &mut DocumentImages, cx| {
            for uri in &images.used {
                let source = (owner.clone(), uri.clone());
                if let Some(Ok(image)) = cx.fetch_asset::<DocumentImage>(&source) {
                    cx.drop_image(image, None);
                }
                cx.remove_asset::<DocumentImage>(&source);
            }
        })
        .detach();
        DocumentImages {
            policy,
            used: HashSet::new(),
        }
    });
    view.image_source(move |uri| {
        let images = images.clone();
        let uri = uri.clone();
        ImageSource::Custom(Arc::new(move |window, cx| {
            images.update(cx, |images, _| {
                images.used.insert(uri.clone());
            });
            window.use_asset::<DocumentImage>(&(images.downgrade(), uri.clone()), cx)
        }))
    })
}

struct DocumentImages {
    // Retaining the immutable policy also keeps its identity from being reused.
    policy: Rc<Policy>,
    used: HashSet<SharedUri>,
}

struct DocumentImage;

impl Asset for DocumentImage {
    // The owner scopes both successful and failed loads to this view and policy.
    // A weak reference lets releasing the view cancel pending loads.
    type Source = (WeakEntity<DocumentImages>, SharedUri);
    type Output = ImageResult;

    fn load(
        (owner, uri): Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = ImageResult> + Send + 'static {
        let capabilities = owner
            .upgrade()
            .map(|images| images.read(cx).policy.capabilities().clone());
        let client = cx.http_client();
        let renderer = cx.svg_renderer();
        let timeout = cx.background_executor().timer(IMAGE_TIMEOUT);
        async move {
            let capabilities = capabilities.ok_or_else(denied_image)?;
            let url = image_url(&capabilities, uri.as_ref())?;
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

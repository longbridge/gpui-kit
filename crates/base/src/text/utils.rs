use std::sync::Arc;

use data_url::DataUrl;
use gpui::{Image, ImageFormat};

const NUMBERED_PREFIXES_1: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERED_PREFIXES_2: &str = "abcdefghijklmnopqrstuvwxyz";

const BULLETS: [&str; 5] = ["•", "◦", "▪", "‣", "⁃"];

/// Returns an ordered-list item's ordinal, defaulting omitted starts to one.
pub(super) fn ordered_list_ordinal(start: Option<u32>, ix: usize) -> u32 {
    start
        .unwrap_or(1)
        .saturating_add(u32::try_from(ix).unwrap_or(u32::MAX))
}

/// Returns the prefix for a list item.
pub(super) fn list_item_prefix(
    ix: usize,
    start: Option<u32>,
    ordered: bool,
    depth: usize,
) -> String {
    if ordered {
        let ordinal = ordered_list_ordinal(start, ix);
        if depth == 0 || start == Some(0) {
            return format!("{ordinal}. ");
        }

        let alpha_ix = ordinal.saturating_sub(1) as usize;
        if depth == 1 {
            return format!(
                "{}. ",
                NUMBERED_PREFIXES_1
                    .chars()
                    .nth(alpha_ix % NUMBERED_PREFIXES_1.len())
                    .unwrap()
            );
        } else {
            return format!(
                "{}. ",
                NUMBERED_PREFIXES_2
                    .chars()
                    .nth(alpha_ix % NUMBERED_PREFIXES_2.len())
                    .unwrap()
            );
        }
    } else {
        let depth = depth.min(BULLETS.len() - 1);
        let bullet = BULLETS[depth];
        return format!("{} ", bullet);
    }
}

/// Decodes a `data:` URL whose mime type names an image format GPUI can
/// decode. Anything else (another scheme, a non-image body, malformed
/// base64) yields `None`, and the caller keeps the URL URI-backed so GPUI's
/// own loader reports the failure.
pub(super) fn data_url_image(url: &str) -> Option<Arc<Image>> {
    let data_url = DataUrl::process(url).ok()?;
    let mime = data_url.mime_type();
    let format = ImageFormat::from_mime_type(&format!("{}/{}", mime.type_, mime.subtype))?;
    let (bytes, _fragment) = data_url.decode_to_vec().ok()?;
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

#[cfg(test)]
mod tests {
    use gpui::ImageFormat;

    use crate::text::utils::{data_url_image, list_item_prefix};

    #[test]
    fn test_data_url_image() {
        fn image(url: &str) -> (ImageFormat, Vec<u8>) {
            let image = data_url_image(url)
                .unwrap_or_else(|| panic!("expected an embedded image for {url:?}"));
            (image.format(), image.bytes().to_vec())
        }

        assert_eq!(
            image("data:image/png;base64,iVBORw0KGgo="),
            (ImageFormat::Png, b"\x89PNG\r\n\x1a\n".to_vec())
        );
        // Legacy mime aliases and the percent-encoded (non-base64) body form.
        assert_eq!(
            image("data:image/jpg;base64,/9j/4A=="),
            (ImageFormat::Jpeg, b"\xff\xd8\xff\xe0".to_vec())
        );
        assert_eq!(
            image("data:image/svg+xml,%3Csvg%3E%3C/svg%3E"),
            (ImageFormat::Svg, b"<svg></svg>".to_vec())
        );

        assert!(data_url_image("https://example.com/logo.png").is_none());
        assert!(data_url_image("data:text/plain;base64,aGVsbG8=").is_none());
        assert!(data_url_image("data:image/png;base64,not*base64").is_none());
        assert!(data_url_image("data:image/png").is_none());
    }

    #[test]
    fn test_list_item_prefix() {
        assert_eq!(list_item_prefix(0, Some(1), true, 0), "1. ");
        assert_eq!(list_item_prefix(1, Some(1), true, 0), "2. ");
        assert_eq!(list_item_prefix(2, Some(1), true, 0), "3. ");
        assert_eq!(list_item_prefix(10, Some(1), true, 0), "11. ");
        assert_eq!(list_item_prefix(0, Some(3), true, 0), "3. ");
        assert_eq!(list_item_prefix(1, Some(3), true, 0), "4. ");
        assert_eq!(list_item_prefix(0, Some(1), true, 1), "A. ");
        assert_eq!(list_item_prefix(1, Some(1), true, 1), "B. ");
        assert_eq!(list_item_prefix(0, Some(4), true, 1), "D. ");
        assert_eq!(list_item_prefix(1, Some(4), true, 1), "E. ");
        assert_eq!(list_item_prefix(0, Some(1), true, 2), "a. ");
        assert_eq!(list_item_prefix(1, Some(1), true, 2), "b. ");
        assert_eq!(list_item_prefix(6, Some(1), true, 2), "g. ");
        assert_eq!(list_item_prefix(0, Some(0), true, 1), "0. ");
        assert_eq!(list_item_prefix(1, Some(0), true, 1), "1. ");
        assert_eq!(list_item_prefix(0, None, false, 0), "• ");
        assert_eq!(list_item_prefix(0, None, false, 1), "◦ ");
        assert_eq!(list_item_prefix(0, None, false, 2), "▪ ");
        assert_eq!(list_item_prefix(0, None, false, 3), "‣ ");
        assert_eq!(list_item_prefix(0, None, false, 4), "⁃ ");
    }
}

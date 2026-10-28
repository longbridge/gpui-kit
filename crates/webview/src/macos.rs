//! AppKit calls that do not depend on window composition.

/// Give the page an appearance of its own; `None` inherits the window's again.
pub(crate) fn set_theme(webview: &wry::WebView, theme: wry::Theme) {
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization as _, NSAppearanceNameAqua,
        NSAppearanceNameDarkAqua,
    };
    use wry::WebViewExtMacOS as _;

    let name = match theme {
        wry::Theme::Dark => Some(unsafe { NSAppearanceNameDarkAqua }),
        wry::Theme::Light => Some(unsafe { NSAppearanceNameAqua }),
        _ => None,
    };
    let appearance = name.and_then(NSAppearance::appearanceNamed);
    webview.webview().setAppearance(appearance.as_deref());
}

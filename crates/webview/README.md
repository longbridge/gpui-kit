# GPUI WebView

A webview supports for GPUI, based on [Wry](https://github.com/tauri-apps/wry).

This crate was previously published as `gpui-wry`. Replace the dependency with `gpui-webview` and `gpui_wry::` paths with `gpui_webview::`.

This still a experimental with limited features, please file issues for any bugs or missing features.

Supported platforms:

| Platform | Engine | GPUI overlays above the page |
| --- | --- | --- |
| macOS | WKWebView | Supported |
| Windows | WebView2 | Not yet; the WebView covers overlays |
| Linux (X11 / XWayland) | WebKitGTK | Supported |
| Linux (Wayland) | — | Not supported; run on XWayland |

- This crate is built on GPUI Fast: enable the `gpui-fast` feature of `gpui-kit` in the application. The default gpui-pre backend is not supported. The crate's own `gpui-fast` feature has no effect and is kept only so existing manifests still resolve.
- On macOS, `WebView::new()` registers the native view with window composition so GPUI overlays can render above it. Composition failures are logged and fall back to the plain native attachment.
- On Linux, `WebView::build()` builds the native view in a window composition surface, with the same fallback.
- Use `WebView::set_bounds()` and `WebView::set_visible()` to keep the managed container and Wry child synchronized. Raw Wry handles bypass this coordination.
- Use `WebView::build()` to attach a webview to a GPUI window with the platform setup Wry needs.
- On Linux, the webview uses WebKitGTK and requires the application to run on X11 (or XWayland in a Wayland session). Start it with `gpui_kit::platform::linux(WindowingModes::X11)`; `WebView::build()` returns an error on a Wayland window. Wayland is not supported.

On Windows, use the webview in a separate window or a Popup layer when overlapping GPUI content is required.

## Run Example

In the root of the repository, run:

```
cargo run -p webview
```

## License

Apache-2.0

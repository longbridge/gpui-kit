# Wry for GPUI

A webview supports for GPUI, based on [Wry](https://github.com/tauri-apps/wry).

This still a experimental with limited features, please file issues for any bugs or missing features.

- With the default backend, native WebViews render above GPUI content.
- On macOS, enabling `gpui-fast` makes `WebView::new()` register the native view with window composition so GPUI overlays can render above it. Composition failures are logged and fall back to the default native attachment.
- Use `WebView::set_bounds()` and `WebView::set_visible()` to keep the managed container and Wry child synchronized. Raw Wry handles bypass this coordination.
- Only supports macOS and Windows currently.

With the default backend or on Windows, use the webview in a separate window or a Popup layer when overlapping GPUI content is required.

## Run Example

In the root of the repository, run:

```
cargo run -p webview
```

## License

Apache-2.0

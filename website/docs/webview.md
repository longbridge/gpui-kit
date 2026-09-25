---
title: WebView
description: Embed a native Wry WebView in a GPUI Kit window, with the current platform and overlay limitations.
order: -2.32
---

# WebView

[`gpui-wry`](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) is GPUI Kit's **experimental** integration with [Wry](https://github.com/tauri-apps/wry). Use it when a screen needs browser behavior; [TextView HTML](/component/text-view#html) renders document content but is not a browser. To open a URL in the user's default external browser, use [`cx.open_url`](./context#open-a-url-in-the-default-browser). The integration currently supports macOS and Windows. The Linux path in the repository's example is unfinished.

## Run the example

From the repository root:

```sh
cargo run -p webview
```

The [complete example](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/src/main.rs) creates a Wry child view from GPUI's native window handle, wraps it in an `Entity<WebView>`, and places that Entity in a normal GPUI layout. For an application that uses the same integration, add `gpui-wry`, `wry` (the `lb-wry` package used by the repository), and `raw-window-handle` alongside `gpui-kit`; check the example's `Cargo.toml` for matching versions.

```rust
use gpui_kit::*;
use gpui_wry::WebView;

let webview = cx.new(|cx| {
    use raw_window_handle::HasWindowHandle;

    let handle = window.window_handle().expect("No window handle");
    let native = wry::WebViewBuilder::new()
        .build_as_child(&handle)
        .expect("Failed to create WebView");
    WebView::new(native, window, cx)
});

webview.update(cx, |view, _| view.load_url("https://gpui-kit.com"));

// In the owning View's render method:
div().flex_1().child(webview.clone())
```

This excerpt shows the macOS and Windows child-view path; keep the `Entity<WebView>` in the owning View so it survives renders. `gpui-wry` updates the native view's bounds from GPUI layout. Use `load_url`, `back`, `show`, and `hide` through `webview.update(...)`. To access additional Wry APIs, use `raw()` or `handle().raw()` on the UI thread.

## Layout, focus, and lifetime

The WebView is a **native child view**, not a GPUI-painted Element. It occupies the bounds of its GPUI layout node and receives native browser input. `WebView` implements `Focusable`; its wrapper tracks a `FocusHandle`. Calling `hide()` returns focus to the parent before hiding the native view. Clicking outside the view's bounds also requests focus on the parent. Test keyboard and focus behavior for your own screen, especially when it combines GPUI inputs with browser inputs.

Keep the WebView and its handles within the parent window's lifetime. Dropping the owning Entity hides the child view, but cloned `WebViewHandle`s or frame-held clones can delay native destruction. Drop those handles before destroying the parent window. See [Entity](./entity) for state ownership and [Window](./window) for window-local handles.

## Current limitations

| Area | Current behavior |
| --- | --- |
| Platforms | Experimental macOS and Windows support. The Linux example contains an unfinished GTK hosting path; do not treat it as supported. |
| Overlay order | The native WebView sits above the GPUI surface and covers GPUI content in the same rectangle, including popovers, dialogs, menus, and tooltips. A GPUI overlay cannot reliably appear on top of it. |
| Windows renderer | The repository example sets `GPUI_DISABLE_DIRECT_COMPOSITION=true` before starting GPUI so this child-view approach renders. This is an example-specific requirement, not a general GPUI setting recommendation. |

When an overlay must be visible, place the WebView in a separate window or arrange the screen so the overlay does not cross its bounds. Do not present a normal GPUI overlay over the WebView as a supported interaction in the current implementation.

## Unmerged composition experiments

The following PRs explore overlay composition. **None is part of the current `gpui-wry` behavior described above.** Check their status and implementation before using a branch in an application:

- [GPUI Kit #2626](https://github.com/longbridge/gpui-kit/pull/2626) explores drawing GPUI overlays above the native WebView. Its current branch depends on [Zed/GPUI #61945](https://github.com/zed-industries/zed/pull/61945), an opt-in layered scene for deferred GPUI overlays. The GPUI Kit PR validates the macOS path; Windows composition and Linux hosting remain follow-up work in that PR.
- [Zed/GPUI #62379](https://github.com/zed-industries/zed/pull/62379) proposes a separate, broader opt-in `CompositionTree` for ordering GPUI and native surfaces, with macOS and Windows examples. It is an alternative to #61945, **not** the dependency of GPUI Kit #2626. Its Linux composition path is outside that PR's scope.

These experiments may change before merge. They explain the intended direction; they do not remove today's platform, overlay, or focus limitations.

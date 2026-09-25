---
title: WebView
description: Embed a native Wry WebView in a GPUI Kit window, with the current platform and overlay limitations.
order: -9.125
---

# WebView

[`gpui-wry`](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) is GPUI Kit's **experimental** integration with [Wry](https://github.com/tauri-apps/wry). Use it when a screen needs browser behavior; [TextView HTML](/component/text-view#html) renders document content but is not a browser. To open a URL in the user's default external browser, use [`cx.open_url`](./context#open-a-url-in-the-default-browser). The integration currently supports macOS and Windows. The Linux path in the repository's example is unfinished.

## Run the example

From the repository root:

```sh
cargo run -p webview
```

The [complete example](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/src/main.rs) is the runnable starting point. It creates the child view inside the `open_window` callback, wraps it in an `Entity<WebView>`, and renders that Entity below an address input. Enter in the input calls `load_url`; the example also contains a back handler. Run it from the repository root with the command above. For another application, match the dependency versions in [the example manifest](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/Cargo.toml): `gpui-kit`, `gpui-wry`, `wry` (package `lb-wry`), and `raw-window-handle` are direct dependencies of this integration.

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

This excerpt shows the macOS and Windows child-view path. Create the native view only after GPUI supplies a live `Window`, and keep the `Entity<WebView>` in its owning view so it survives renders. The example calls `gpui_kit::init(cx)` before creating component state and uses `gpui_kit::open_window(...)` to create the window. For the complete application setup, use the linked source rather than treating this excerpt as a standalone `main` function.

## Own the view and its layout

`WebView::new` initially sets the native bounds to an empty rectangle. Rendering the entity installs a GPUI layout element; during `prepaint`, that element sends its resolved bounds to Wry in **logical coordinates** and inserts a GPUI hitbox. Give the containing region a real size. The example uses `div().flex_1().h(px(400.)).child(self.webview.clone())`. The browser pixels themselves are painted by the operating system, so GPUI clipping, hitboxes, and content masks do not put GPUI UI above the child view.

Keep one `Entity<WebView>` for each native view. Do not build a new Wry view on each `render`. The wrapper's `visible()` and `bounds()` report its stored visibility and last layout bounds; `show()` and `hide()` change native visibility. If a page or tab no longer renders the entity, explicitly call `hide()` when it should disappear, and `show()` when it returns. `prepaint` skips bounds updates while hidden.

Call wrapper methods through `webview.update(cx, |view, _| ...)` on the GPUI UI context. `load_url(&str)` requests navigation but discards Wry's `Result`; it does not report load completion. `back()` runs JavaScript `history.back()` and returns a `Result` for script submission, not a history or navigation result. Use `view.raw()` for Wry methods such as `reload()`, `url()`, or `evaluate_script()` when their return values matter. `view.handle().raw()` gives an owned, UI-thread-local Wry handle when a short-lived callback needs one. Avoid keeping a handle merely to avoid updating the entity.

## Loading, navigation, and page messages

Configure page policy and callbacks on `wry::WebViewBuilder` **before** `build_as_child`; these are Wry facilities, not `gpui-wry` events. The pinned `lb-wry` version provides:

| Need | Wry API | Meaning |
| --- | --- | --- |
| Initial content | `with_url(...)` or `with_html(...)` | Select content before building; the repository example instead calls wrapper `load_url` after creation. |
| Loading status | `with_on_page_load_handler(|event, url| ...)` | Observe `Started` and `Finished`; `Finished` is not a success or HTTP-status report. Treat this separately from whether `load_url` accepted a request. |
| Navigation policy | `with_navigation_handler(|url| -> bool { ... })` | Return `true` to allow or `false` to cancel an incoming navigation. Decide policy for redirects and links as well as the initial URL. |
| New windows | `with_new_window_req_handler(...)` | Decide what happens to `window.open` requests; its callback has a platform-specific thread contract. |
| Rust to page | `evaluate_script(...)` on the raw Wry view | Submits JavaScript and returns a Wry `Result`; use `evaluate_script_with_callback` when a serialized result is needed. |
| Page to Rust | `with_ipc_handler(|request| ...)` | Receives strings posted by `window.ipc.postMessage(...)`. Parse and validate the request before acting on it. |

For example, a fixed-URL navigation policy can be attached while constructing the builder:

```rust
let builder = wry::WebViewBuilder::new()
    .with_navigation_handler(|url| url == "https://gpui-kit.com/")
    .with_on_page_load_handler(|event, url| {
        // Forward a small status message to the owning GPUI view if needed.
        // Do not capture &mut Window, &mut App, or &mut Context here.
        let phase = match event {
            wry::PageLoadEvent::Started => "started",
            wry::PageLoadEvent::Finished => "finished",
        };
        eprintln!("{phase}: {url}");
    });
```

This exact-match check is only an illustration of the callback shape; use parsed URL origin and an explicit scheme/host policy for a real allowlist. Keep callback work short. Pass status or IPC messages into application-owned state through a safe scheduling/channel path, then update the `Entity` on GPUI's context and call `cx.notify()` if the UI changes. Wry callbacks are not GPUI entity events, and the Windows new-window callback runs on a separate thread. Plan for callbacks arriving after a page has navigated or its owning window has closed.

## Security and failure boundaries

Treat page content as untrusted, including content served from a URL you control. IPC is an application capability: allow only expected message types and payload sizes, check the sender origin where the platform supplies it, and require application authorization before file, network, or account actions. Wry's IPC request URL is the main-frame URL for iframes on Linux/Android, so do not use it alone as iframe identity. Initialization scripts can run on each new page; the pinned Wry documentation says Windows also injects them into subframes, even when main-frame-only is requested. Avoid placing secrets in scripts or page globals.

Decide how downloads, external links, and `window.open` requests are handled before displaying remote content. Wry's default download-start handler allows downloads, so add a download policy if that is inappropriate for the application. Keep user-visible loading and error state in the owning GPUI view: `gpui-wry::WebView::load_url` ignores immediate errors, and a successful request can still lead to a failed page load. Use raw Wry results and page-load callbacks as separate signals; neither `PageLoadEvent::Finished` nor `load_url` alone proves successful content loading. Show a retry path for failures your application detects.

## Focus and lifetime

The WebView is a **native child view**, not a GPUI-painted Element. It occupies the bounds of its GPUI layout node and receives native browser input. `WebView` implements `Focusable`; its wrapper tracks a `FocusHandle`. Calling `hide()` returns focus to the parent before hiding the native view. Clicking outside the view's bounds also requests focus on the parent. Test keyboard and focus behavior for your own screen, especially when it combines GPUI inputs with browser inputs.

Keep the WebView and its handles within the parent window's lifetime. Dropping the owning Entity hides the child view, but cloned `WebViewHandle`s or frame-held clones can delay native destruction. Drop those handles before destroying the parent window. See [Entity](./entity) for state ownership and [Window](./window) for window-local handles.

## Debug and verify

The example requests Wry devtools in debug builds (or with its `inspector` feature); it does not open them automatically. On macOS release builds, Wry also requires its own `devtools` feature for this request to take effect. Wry's `open_devtools()` is available behind debug or that dependency feature. Inspect the page to distinguish JavaScript/network failures from GPUI layout issues. For a blank child view, first verify the containing GPUI element has nonzero bounds, that `visible()` is true, and that the native view was created from the current live window. On Windows, check the example's DirectComposition setting below. Confirm navigation and IPC in a real native window; GPUI's headless UI tests cannot prove native browser pixels, system focus, or compositor order.

For an application smoke test on each supported OS, exercise initial load, links and redirects, back and reload, new-window/download policy, failed/offline load, IPC input validation, resize, show/hide, keyboard focus transfer, and closing the parent window. Keep pure policy parsing tests separate from these native checks. The repository example demonstrates loading and layout but does not implement a complete navigation, bridge, or error-state test suite.

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

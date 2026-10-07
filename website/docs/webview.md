---
title: WebView
description: Embed a native Wry WebView in a GPUI Kit window, with the current platform and overlay limitations.
order: -9.125
maturity: [experimental, platform-dependent]
---

# WebView

[`gpui-webview`](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) is GPUI Kit's **experimental** integration with [Wry](https://github.com/tauri-apps/wry). Use it when a screen needs browser behavior; [TextView HTML](../component/text-view.md#html) renders document content but is not a browser. To open a URL in the user's default external browser, use [`cx.open_url`](./context#open-a-url-in-the-default-browser). Versions up to 0.7.1 were published as `gpui-wry`; replace that dependency with `gpui-webview` and `gpui_wry::` paths with `gpui_webview::`. See [Platform support](#platform-support) for the operating systems and display servers it runs on.

## Platform support

| Platform | Engine | Status |
| --- | --- | --- |
| macOS | WKWebView | Experimental. With `gpui-fast`, deferred GPUI overlays can render above the page. |
| Windows | WebView2 | Experimental. The example disables GPUI's DirectComposition so the child view renders. |
| Linux (X11) | WebKitGTK | Experimental. The application must start on X11; see [Linux](#linux). |
| Linux (Wayland) | — | Not supported. Run the application on X11 through XWayland instead. |

## Run the example

From the repository root:

```sh
cargo run -p webview
```

The [complete example](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/src/main.rs) is the runnable starting point. It creates the child view inside the `open_window` callback, wraps it in an `Entity<WebView>`, and renders that Entity below an address input. Enter in the input calls `load_url`; the example also contains a back handler. Run it from the repository root with the command above. For another application, match the dependency versions in [the example manifest](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/Cargo.toml): `gpui-kit`, `gpui-webview`, and `wry` (package `lb-wry`) are direct dependencies of this integration.

```rust
use gpui_kit::*;
use gpui_webview::WebView;

let webview = cx.new(|cx| {
    WebView::build(wry::WebViewBuilder::new(), window, cx).expect("Failed to create WebView")
});

webview.update(cx, |view, _| view.load_url("https://gpui-kit.com"));

// In the owning View's render method:
div().flex_1().child(webview.clone())
```

`WebView::build` attaches the Wry view as a child of the GPUI window and performs the platform setup Wry needs. `WebView::new` still wraps a `wry::WebView` the application built itself. Create the native view only after GPUI supplies a live `Window`, and keep the `Entity<WebView>` in its owning view so it survives renders. The example calls `gpui_kit::init(cx)` before creating component state and uses `gpui_kit::open_window(...)` to create the window. For the complete application setup, use the linked source rather than treating this excerpt as a standalone `main` function.

## GPUI Fast native composition

Enable `gpui-fast` on both `gpui-kit` and `gpui-webview` to select one backend. On macOS, the existing `WebView::new()` automatically registers the WKWebView in the window composition tree. Deferred GPUI overlays can then render above it. No separate composition feature or constructor is needed. The default backend and Windows retain the existing native child-view behavior.

Use `WebView::set_bounds(Rect) -> wry::Result<()>` for window-relative bounds and `WebView::set_visible(bool) -> wry::Result<()>` for visibility. They coordinate the managed container and its Wry child; raw Wry calls bypass that coordination. `show()` and `hide()` remain convenience methods that discard errors. If composition registration fails, construction logs the error and keeps the original attachment. Verify native overlay input and repeated resizing in a real window.

## Linux

Wry renders with WebKitGTK, and GTK can only embed into an X11 window; Wayland has no way to place one client's surface inside another's. An application that uses `gpui-webview` must therefore run on X11 on Linux, which means XWayland in a Wayland session. GPUI connects to one display server for the whole application, so this choice covers every window, not only the one hosting the WebView.

Start the application with `gpui_kit::platform::linux(WindowingModes::X11)` instead of `gpui_kit::application()`, as the example does:

```rust
use gpui_kit::*;

fn main() {
    #[cfg(target_os = "linux")]
    let app = gpui_kit::platform::linux(WindowingModes::X11);
    #[cfg(not(target_os = "linux"))]
    let app = gpui_kit::application();

    app.run(|cx| {
        gpui_kit::init(cx);
        // Open windows and create WebViews here.
    });
}
```

`WebView::build` checks that the window is an X11 window before doing any setup. On a Wayland window it returns an error naming this requirement, so a misconfigured application fails at construction rather than showing a blank area. On X11 it initializes GTK on its X11 backend, dispatches GTK events from a GPUI task, and attaches the view to the window. The desktop must provide XWayland, which GNOME, KDE Plasma, and Hyprland enable by default. GTK only supports integer scales (`GDK_SCALE`), so the wrapper sets Wry's page zoom to GPUI's scale divided by GDK's; the page renders at GDK's resolution but at GPUI's size. It reapplies this zoom when the window's scale changes, replacing any zoom the application set through `zoom()`. Under fractional scaling, an XWayland window may look softer than a native Wayland window, depending on the compositor.

Install the WebKitGTK 4.1 and GTK 3 development packages (for example `libwebkit2gtk-4.1-dev` on Debian and Ubuntu, `webkit2gtk-4.1` on Arch Linux) before building.

## Own the view and its layout

`WebView::new` initially sets the native bounds to an empty rectangle. Rendering the entity installs a GPUI layout element; during `prepaint`, that element sends its resolved bounds to Wry (logical coordinates on macOS and Windows, device pixels on Linux) and inserts a GPUI hitbox. Give the containing region a real size. The example uses `div().flex_1().h(px(400.)).child(self.webview.clone())`. The browser pixels themselves are painted by the operating system, so clipping, hitboxes, and content masks alone do not put GPUI UI above the child view; macOS with GPUI Fast uses the composition tree for overlay order.

Keep one `Entity<WebView>` for each native view. Do not build a new Wry view on each `render`. The wrapper's `visible()` and `bounds()` report its stored visibility and last layout bounds; `show()` and `hide()` change native visibility. If a page or tab no longer renders the entity, explicitly call `hide()` when it should disappear, and `show()` when it returns. `prepaint` skips bounds updates while hidden.

Call wrapper methods through `webview.update(cx, |view, _| ...)` on the GPUI UI context. `load_url(&str)` requests navigation but discards Wry's `Result`; it does not report load completion. `back()` runs JavaScript `history.back()` and returns a `Result` for script submission, not a history or navigation result. Use `view.raw()` for Wry methods such as `reload()`, `url()`, or `evaluate_script()` when their return values matter. `view.handle().raw()` gives an owned, UI-thread-local Wry handle when a short-lived callback needs one. Avoid keeping a handle merely to avoid updating the entity.

## Loading, navigation, and page messages

Configure page policy and callbacks on `wry::WebViewBuilder` **before** `build_as_child`; these are Wry facilities, not `gpui-webview` events. The pinned `lb-wry` version provides:

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

Decide how downloads, external links, and `window.open` requests are handled before displaying remote content. Wry's default download-start handler allows downloads, so add a download policy if that is inappropriate for the application. Keep user-visible loading and error state in the owning GPUI view: `gpui-webview::WebView::load_url` ignores immediate errors, and a successful request can still lead to a failed page load. Use raw Wry results and page-load callbacks as separate signals; neither `PageLoadEvent::Finished` nor `load_url` alone proves successful content loading. Show a retry path for failures your application detects.

## Focus and lifetime

The WebView is a **native child view**, not a GPUI-painted Element. It occupies the bounds of its GPUI layout node and receives native browser input. `WebView` implements `Focusable`; its wrapper tracks a `FocusHandle`. Calling `hide()` returns focus to the parent before hiding the native view. Clicking outside the view's bounds also requests focus on the parent. Test keyboard and focus behavior for your own screen, especially when it combines GPUI inputs with browser inputs.

Keep the WebView and its handles within the parent window's lifetime. Dropping the owning Entity hides the child view, but cloned `WebViewHandle`s or frame-held clones can delay native destruction. Drop those handles before destroying the parent window. See [Entity](./entity) for state ownership and [Window](./window) for window-local handles.

## Debug and verify

The example requests Wry devtools in debug builds (or with its `inspector` feature); it does not open them automatically. On macOS release builds, Wry also requires its own `devtools` feature for this request to take effect. Wry's `open_devtools()` is available behind debug or that dependency feature. Inspect the page to distinguish JavaScript/network failures from GPUI layout issues. For a blank child view, first verify the containing GPUI element has nonzero bounds, that `visible()` is true, and that the native view was created from the current live window. On Windows, check the example's DirectComposition setting below. Confirm navigation and IPC in a real native window; GPUI's headless UI tests cannot prove native browser pixels, system focus, or compositor order.

For an application smoke test on each supported OS, exercise initial load, links and redirects, back and reload, new-window/download policy, failed/offline load, IPC input validation, resize, show/hide, keyboard focus transfer, and closing the parent window. Keep pure policy parsing tests separate from these native checks. The repository example demonstrates loading and layout but does not implement a complete navigation, bridge, or error-state test suite.

## Current limitations

| Area | Current behavior |
| --- | --- |
| Platforms | Experimental on macOS, Windows, and Linux on X11. Wayland is not supported; Linux applications must start on X11 or XWayland. |
| Overlay order | The native WebView sits above the GPUI surface and covers GPUI content in the same rectangle, including popovers, dialogs, menus, and tooltips. With the default backend, on Windows, or on Linux, a GPUI overlay cannot reliably appear on top of it. macOS with `gpui-fast` uses native composition for deferred overlays. |
| Windows renderer | The repository example sets `GPUI_DISABLE_DIRECT_COMPOSITION=true` before starting GPUI so this child-view approach renders. This is an example-specific requirement, not a general GPUI setting recommendation. |

When an overlay must be visible, place the WebView in a separate window or arrange the screen so the overlay does not cross its bounds. This restriction applies to the default backend, Windows, and Linux; macOS with `gpui-fast` supports deferred GPUI overlays through native composition.

## Unmerged composition experiments

The following PRs explore overlay composition. **None is part of the current `gpui-webview` behavior described above.** Check their status and implementation before using a branch in an application:

- [GPUI Kit #2626](https://github.com/longbridge/gpui-kit/pull/2626) explores drawing GPUI overlays above the native WebView. Its current branch depends on [Zed/GPUI #61945](https://github.com/zed-industries/zed/pull/61945), an opt-in layered scene for deferred GPUI overlays. The GPUI Kit PR validates the macOS path; Windows composition and Linux hosting remain follow-up work in that PR.
- [Zed/GPUI #62379](https://github.com/zed-industries/zed/pull/62379) proposes a separate, broader opt-in `CompositionTree` for ordering GPUI and native surfaces, with macOS and Windows examples. It is an alternative to #61945, **not** the dependency of GPUI Kit #2626. Its Linux composition path is outside that PR's scope.

These experiments may change before merge. They explain the intended direction; they do not remove today's platform, overlay, or focus limitations.

---
title: Native Extensions
description: Integrate native menus and child views with GPUI Kit, including handles, layout, input, lifetime, and platform limits.
order: -9.1
---

# Native Extensions

A native extension attaches an OS control or view to a GPUI window. The two concrete integrations in this repository show different paths: [NativeMenu](https://github.com/longbridge/gpui-kit/tree/main/crates/component/src/native_menu) uses the operating system's menu API, while [gpui-wry](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) embeds a native WebView. Both cross the GPUI/OS boundary; neither is implemented by launching another application.

| Platform | NativeMenu | Embedded WebView in this repository |
| --- | --- | --- |
| macOS | AppKit `NSMenu` attached to an `NSView` | Wry child view; current example works |
| Windows | Win32 `TrackPopupMenuEx` with an `HWND` | Wry child view; current example works with its renderer setting |
| Linux | GPUI-drawn `PopupMenu` fallback, clipped to the window | GTK hosting code is unfinished; no supported path yet |

The Linux menu fallback preserves the `NativeMenu` API, but it is GPUI content rather than an OS popup. Linux native-view hosting would need its own GTK/Wayland/X11 integration and tests. See [WebView](./webview) for the current platform requirements.

## 1. Choose the integration boundary

For **platform menu integration**, keep the shared GPUI-facing API separate from platform adapters. `NativeMenu::show(position, window, cx)` chooses its macOS, Windows, or Linux fallback implementation in [the shared module](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/mod.rs). Its native adapters create and operate the system menu, then send the selected action back through GPUI.

For an **embedded child view**, native content occupies a GPUI layout slot across frames. Own it in an `Entity`, as [`gpui-wry::WebView`](https://github.com/longbridge/gpui-kit/blob/main/crates/webview/src/lib.rs) does. The owner must control show/hide, focus, bounds, and destruction before the parent window closes.

## 2. Obtain the right window handle

`Window::window_handle(window)` gives a GPUI `AnyWindowHandle` for updating that same window later. To call an OS API, use the `raw-window-handle` trait instead. These methods have similar names but different purposes:

```rust
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

let gpui_handle = Window::window_handle(window); // Later GPUI update
let native_handle = HasWindowHandle::window_handle(window)?; // OS adapter
match native_handle.as_raw() {
    RawWindowHandle::AppKit(handle) => { /* macOS: handle.ns_view */ }
    RawWindowHandle::Win32(handle) => { /* Windows: handle.hwnd */ }
    RawWindowHandle::Xcb(handle) => { /* Linux X11: handle.window */ }
    RawWindowHandle::Wayland(handle) => { /* Linux Wayland: handle.surface */ }
    _ => { /* unsupported backend */ }
}
```

This is an adapter excerpt inside a function returning `Result`. Match the actual handle variant under the corresponding target `#[cfg]`; do not retain a borrowed raw handle beyond the live window operation. GPUI's pinned Linux platform implementation exposes XCB window IDs under X11 and a Wayland surface under Wayland. A raw surface pointer alone does not provide the GTK widget hierarchy or compositor integration needed to embed a GTK control. The repository's [AppKit](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/macos.rs) and [Win32](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/windows.rs) adapters show complete handle extraction.

## 3. Platform menu example: NativeMenu

The adapter calls the platform's UI framework directly, using target-specific Rust crates:

| Backend | Rust crates and APIs used here | Native object or operation |
| --- | --- | --- |
| macOS | `objc2`, `objc2-app-kit`, `objc2-foundation`; `MainThreadMarker`, `NSMenu`, `NSMenuItem`, `define_class` | Construct AppKit menu items, receive Objective-C selection, show the menu from an `NSView` |
| Windows | `windows` crate with Win32 UI features; `CreatePopupMenu`, `TrackPopupMenuEx`, `DestroyMenu` | Build an `HMENU`, track the selection against an `HWND`, release native resources |
| Linux | GPUI's X11 backend uses `x11rb`; its Wayland backend uses `wayland-client`. Wry's unfinished path uses `gtk` and `WebViewBuilderExtUnix`. | NativeMenu currently renders a GPUI `PopupMenu` fallback in `Root`'s overlay; native widget hosting is not implemented |

These are the calls inside the actual adapters, with surrounding item construction and error handling omitted:

```rust
// macOS: objc2-app-kit, on the AppKit main thread.
let mtm = MainThreadMarker::new()?;
let menu = NSMenu::new(mtm);
menu.popUpMenuPositioningItem_atLocation_inView(None, point, Some(view));

// Windows: windows::Win32::UI::WindowsAndMessaging.
let menu = unsafe { CreatePopupMenu() }.ok()?;
let selected = unsafe { TrackPopupMenuEx(menu, flags.0, x, y, hwnd, None) };
let _ = unsafe { DestroyMenu(menu) };
```

The macOS adapter turns the AppKit `NSView` into an `NSMenu` anchor, converts GPUI's top-left logical position to AppKit view coordinates, and runs the menu tracking loop. The Windows adapter obtains an `HWND`, converts logical pixels to physical client coordinates and then screen coordinates, and calls `TrackPopupMenuEx`. Both run the blocking OS tracking loop **outside GPUI's mutable borrow**, then use a foreground task, `cx.update(...)`, and the retained `AnyWindowHandle::update(...)` to dispatch the selected `Action` through `Window::dispatch_action`. Closing the window or cancelling the menu produces no action.

A caller only supplies the semantic items and position; `Copy` and `Paste` below are application-defined GPUI Actions:

```rust
NativeMenu::new()
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
    .show(position, window, cx);
```

The [story](https://github.com/longbridge/gpui-kit/blob/main/crates/story/src/stories/native_menu_story.rs) shows focus placement and action handling. On Linux, [the fallback](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/fallback.rs) builds a GPUI `PopupMenu` in `Root`'s overlay; it is still subject to the GPUI window boundary.

## 4. Embedded view example: WebView

The [WebView example](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/src/main.rs) creates a Wry native child view from a live GPUI window and stores the wrapper in an `Entity`:

```rust
let webview = cx.new(|cx| {
    use raw_window_handle::HasWindowHandle;

    let handle = window.window_handle().expect("No window handle");
    let native = wry::WebViewBuilder::new()
        .build_as_child(&handle)
        .expect("Failed to create WebView");
    gpui_wry::WebView::new(native, window, cx)
});
```

The wrapper renders a custom `Element` in the normal GPUI tree. `request_layout` reserves the slot; `prepaint` receives its resolved `Bounds<Pixels>` and calls Wry's `set_bounds` with logical coordinates. It also inserts a GPUI hitbox. `paint` registers outside-click focus behavior. **GPUI does not paint the WebView pixels**: the OS owns that child view. A GPUI `ContentMask` or hitbox does not change its compositor order. The owner hides the native view on drop; cloned `WebViewHandle`s must be released before the parent window is destroyed.

The current WebView sits above GPUI content in the same rectangle, so GPUI popovers, dialogs, and tooltips cannot reliably cover it. On Windows, the repository example disables GPUI DirectComposition for this child-view route. The Linux GTK path in the example is marked unfinished. Details and usage are in [WebView](./webview).

On Linux, the example imports `gtk` and Wry's `WebViewBuilderExtUnix`, creates a `gtk::Fixed`, and calls `build_gtk(&fixed)`. The code itself says the host initialization is unfinished: creating a GTK widget does **not** attach it to GPUI's XCB or Wayland window. GPUI's pinned Linux backend uses `x11rb` calls such as `configure_window` on X11 and `wayland-client`'s `WlSurface::commit` on Wayland. A Linux adapter must use the matching backend's connection and event loop, own its surface, and arrange input and compositor order; the raw `XcbWindowHandle` or `WaylandWindowHandle` alone cannot supply that integration. The current `gpui-wry` example is not a working template for this final step.

## 5. Build another native control

1. Define one GPUI-facing operation and the same result semantics for each platform. State whether the Linux implementation is native, GPUI-drawn, or unsupported.
2. Put AppKit, Win32, and Linux backend code behind target-specific adapters. Keep raw handles and OS types inside those adapters.
3. For an embedded view, own its lifetime in an `Entity`; map GPUI layout bounds to native bounds in `prepaint`, and test scale changes, focus, input, and window close. For system UI such as NativeMenu, run its tracking loop without holding a GPUI mutable borrow, then return the result to GPUI.
4. Test each actual backend. A headless GPUI test can check state and actions; it cannot prove OS positioning, native focus, or compositor stacking.

### Composition work in progress

Current GPUI has no general API for placing a native child view between its base scene and overlays. [Zed/GPUI #62379](https://github.com/zed-industries/zed/pull/62379) proposes an opt-in `CompositionTree` with GPUI base, native, and GPUI overlay surfaces, including macOS AppKit and Windows DirectComposition adapters. [Zed/GPUI #61945](https://github.com/zed-industries/zed/pull/61945) is a smaller alternative for deferred overlays; [GPUI Kit #2626](https://github.com/longbridge/gpui-kit/pull/2626) experiments with that branch. These PRs are unmerged, and #62379 explicitly leaves Linux composition for later. Treat their APIs as experimental, not as instructions for the current released GPUI Kit.

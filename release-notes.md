# Release Notes

## Pending Updates

### 0.7.0 (unreleased)

#### Root owns window overlays

`gpui_component::Root` now always mounts the dialog, sheet and notification
layers above application content. Opening a dialog, sheet or notification no
longer depends on the application's view rendering its layer. Notifications use
the Root's full bounds, and cached content does not duplicate or suppress layers.

#### Breaking changes

The following `gpui-component` APIs have been removed:

- `Root::render_dialog_layer`
- `Root::render_sheet_layer`
- `Root::render_notification_layer`

Remove these calls from application render methods. There are no replacement
layer switches or manual mounting APIs; Root renders all three layers itself.
Existing custom layer positions move to Root's window-level overlay placement.

```diff
 div()
     .size_full()
     .child(self.content.clone())
-    .children(Root::render_sheet_layer(window, cx))
-    .children(Root::render_dialog_layer(window, cx))
-    .children(Root::render_notification_layer(window, cx))
```

If a render method first stored these layers in local variables, remove those
variables and the corresponding `.children(...)` calls as well.

`window.open_dialog`, `window.open_sheet` and `window.push_notification` remain
the application-facing APIs. Keep a `Root` as the window's root view, or use the
new window helper below.

#### Added: `gpui_base::Root` and `gpui_kit::open_window`

```rust
pub fn open_window<V: Render>(
    options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> Result<(AnyWindowHandle, Entity<V>)>
```

Opens a window and returns both the window handle and the content entity. The
helper always wraps content in `gpui_base::Root`, independent of Cargo features.
The helper is defined only in Kit. `component::Root` re-exports the Base type.

Base owns the root, content, overlay hosting, keyboard traversal and selection
copying. Explicit `gpui_component::init` registers a per-window extension for
styled dialogs, sheets, notifications, tooltips, menus, touch selection and
window presentation. Base does not depend on Component or its theme. Plugins
must be registered before creating windows; they do not retrofit existing roots.

Component operations belong to `WindowExt`; the previous Component-specific
Root methods and fields (including `notification`) are no longer exposed on Root.

```rust
let (window, view) = gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
    cx.new(|cx| MyApp::new(window, cx))
})?;
```

Call `gpui_kit::init(cx)` before opening component-backed windows, and do not
return a Root from this helper's builder. In an async context, call the helper
inside `cx.update`.

Kit examples and the native/web story galleries use this helper for standard window
startup. Base examples continue using `gpui_base::init` and GPUI's window API
directly, without a dependency on Kit. The FPS example disables Kit's default
features.

Quit and close-window actions, keyboard shortcuts and confirmation flows remain
application-owned. Kit initialization does not install default quit or close
bindings.

The `Root` and `WindowExt` text-selection methods are removed. Use
`gpui_base::TextSelection::{selected_text, has_selection, clear, end}`.

# gpui-test

Headless behavior and layout tests using GPUI's existing `TestAppContext`, native
input dispatch, `ElementId`, and ordinary Rust assertions. No GPUI fork, Cargo
patch, accessibility query, or separate application runner is required.

```toml
[dev-dependencies]
gpui-test = { path = "../gpui-kit/crates/test" }
gpui-component = { path = "../gpui-kit/crates/component", features = ["test-support"] }
```

Use the same GPUI dependency as GPUI Kit. The umbrella `gpui-kit/test-support`
feature also enables component instrumentation.

## Test API

```rust,ignore
use gpui::AppContext as _;
use gpui_test::TestWindowExt;

// Initialize the component library and create the window with normal GPUI APIs.
cx.update_window(handle.into(), |_, window, cx| {
    window.click("search", cx);
    window.input("GPUI 中文", cx);
    let input = window.find("search").unwrap();
    assert!(input.focused());
    assert_eq!(input.text(), Some("GPUI 中文"));
}).unwrap();
```

- `window.find(id) -> Option<TestElement>` observes the last painted state.
- `window.click(id, cx)` sends mouse move/down/up through GPUI hit testing.
- `window.input(text, cx)` uses GPUI's keystroke parser and simulated input.
- Snapshots expose `bounds()`, `visible()`, `focused()`, `disabled()` and `text()`.

Kit Button and Input register automatically when `test-support` is enabled.
Buttons use their constructor ID. Inputs accept `.id("search")`; the default
remains `("input", input_state.entity_id())`. Metadata comes from the controls'
existing label, focus handle, disabled flag and unmasked input value.

## Observing native GPUI elements

Upstream GPUI does not publicly expose a completed frame's identified elements.
Following CONTRIBUTING, this crate does **not** modify GPUI to obtain them.
Native elements must opt in using a transparent wrapper:

```rust,ignore
use gpui::{div, prelude::*, px};
use gpui_test::ObserveElement;

let content = div().id("popover").observe().w(px(200.)).h(px(80.));
```

`observe()` supports identified GPUI divs, preserves their original identity,
layout and native event handlers, and forwards their accessibility methods.
It does not traverse their children. Unobserved native IDs return `None`.
Visibility inspection evaluates the computed style once more, so style/drag
predicates may run an additional time and should not depend on invocation counts.
If a custom control knows a text value, supply it with `.text(value)`; the
wrapper cannot discover arbitrary rendered child text through public APIs.
Tracking focus after `.observe()` records the same handle; when wrapping an
already focus-tracked element, use `.focus(&handle)` to supply the handle.

This is the RFC's thin-wrapper fallback. Automatic discovery of all native
`.id(...)` elements would require a generic API contributed upstream separately.

## Frames, caching and visibility

Draw once before the first query. After direct state/focus changes, resizing or
an existing GPUI action, call `window.refresh(); window.draw(cx).clear(cx)`.
Click and text helpers do this themselves. Drive asynchronous work through
GPUI's normal test executor before the next draw.

Use `cx.update_window(handle.into(), ...)` or `VisualTestContext::update`.
Typed `WindowHandle::update` borrows the root entity and cannot safely redraw it
inside that callback. Do not query while layout/prepaint/paint is in progress.

Registration is owned by GPUI's public element-state mechanism; weak registry
entries disappear when the element/window is released, and cached views retain
their last painted facts. Previously returned snapshots remain unchanged.
Use a refresh when focus or other external state must invalidate cached facts.

Bounds are final window-local layout geometry. Visibility checks the target's
computed hidden/opacity style, positive area, viewport and content mask.
Hidden ancestors that suppress paint keep descendants invisible. Upstream does
not expose inherited paint opacity, so an unobserved transparent ancestor is
not detected. Pixel occlusion is not inferred: overlays still intercept native
clicks. A partially clipped target's center may lie outside its visible area.

Missing/invisible click targets panic with the target ID. Disabled controls
receive ordinary events so tests can verify that they remain inert. Repeated
IDs in different scopes are ambiguous and panic; choose unique target IDs or
GPUI's composite IDs. Text is explicit logical content, not rasterized glyphs;
masked input values and unspecified custom text are not reported.

## Verification

```sh
cargo test -p gpui-test --locked
```

The 28 tests cover:

- Real Kit Button/Input/Popover interactions, focus switching, Unicode/Shift,
  backspace, read-only/disabled inputs, and masked-value privacy.
- Resolved geometry, visibility, clipping and native hit testing, including
  overlays and a partially clipped target whose center cannot receive clicks.
- Cached child visibility, unmount/remount, composite IDs after list reordering,
  immutable snapshots, and isolation after closing another window.
- Anonymous/ambiguous IDs, accessibility forwarding, and stale-record removal
  when a 1,000-element list shrinks to 10 elements.

The same command runs in the existing CI platform matrix. The large-list case
checks correctness, not rendering performance.

For a complete application workflow and CI setup, see the
[UI automation testing guide](../../website/docs/ui-testing.md).

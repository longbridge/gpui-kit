# Testing

**Contents:** [Choose the test level](#choose-the-test-level) · [UI integration workflow](#ui-integration-workflow) · [Complete example](#complete-example) · [Queries and interactions](#queries-and-interactions) · [Frames and async work](#frames-and-async-work) · [Assertion boundaries](#assertion-boundaries) · [Additional resources](#additional-resources)

## UI integration testing

A UI integration test renders real components in a headless window, dispatches
clicks, keyboard input and scrolling, then checks state, focus, layout and owner
callbacks. For example: add a Checkbox UI integration test that proves clicking
toggles the owner's value and disabled controls reject the interaction.
Use this term when describing component interaction coverage.
`#[gpui_kit::test]` runs the test; `gpui_kit::test` operates and inspects its UI.

## Choose the test level

Use ordinary Rust `#[test]` for pure logic. Use `#[gpui_kit::test]` and
`TestAppContext` for entities, subscriptions, actions and async tasks; it can
create headless windows too. `VisualTestContext` is available for existing GPUI
window helpers. For an application UI flow, use `gpui_kit::test::TestWindowExt`
on the real `Window` and assert the behavior produced by native events.

Import the Kit types you use explicitly and write `#[gpui_kit::test]`. Avoid `use gpui_kit::*;` in test modules: it imports GPUI’s `test` macro and can shadow Rust’s built-in `#[test]`. Add
`test-support` to the application's `gpui-kit` development dependency, using
exactly the same source and version as its normal dependency. The helpers
require a Kit revision that includes them; check the installed API before
assuming an older published release supports them. No extra testing crate or
GPUI patch is required.

For a standalone test package beside a Kit checkout:

```toml
[package]
name = "ui-tests"
version = "0.1.0"
edition = "2024"
publish = false

[dev-dependencies]
gpui-kit = { path = "../gpui-kit/crates/kit", features = ["test-support"] }
```

Headless tests still need the platform's native build dependencies. Run
`cargo generate-lockfile` once for the new package, then
`cargo test --test ui --locked`. In the Kit repository the equivalent command
is `cargo test -p gpui-kit --features test-support --test ui --locked`.

## UI integration workflow

1. Import the production view and constructor from the application library.
   Keep state entities and subscriptions owned by that view. Define a view
   inside a test only for a self-contained example or a deliberate fixture.
2. Initialize with `cx.update(gpui_kit::init)`, open a headless window of an
   explicit size through `gpui_kit::open_window`. Its Base `Root` renders
   the dialog, sheet and notification layers above the view automatically.
   Do not mount overlay layers in the application view.
3. Render a frame and locate existing control IDs. Custom native elements opt
   in with `.id("status").test_support()` through `TestSupportExt`. Place it
   before `.track_focus(&handle)`; custom wrappers must forward that method
   to their inner element. Registration adds no layout container.
4. Dispatch clicks, input and named keys through `TestWindowExt`. Exercise the
   actual handler path; invoking callbacks or directly setting control state
   would bypass the behavior being tested.
5. Assert the initial state, the visible outcome after each meaningful action,
   and the application result. Include a relevant negative case, such as a
   disabled control rejecting the interaction or invalid input preventing save.
6. Run the affected test target and report its result. An uncompiled example or
   a test which never checks the action's outcome is not completion evidence.

## Complete example

Copy this into `tests/ui.rs`. It follows the Kit integration-test example
from `crates/kit/tests/ui.rs`, with window setup inlined so this reference works
without a repository checkout. In an application, replace that view definition
with an import of your production view.

```rust
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Bounds, Context, Entity, SharedString, TestAppContext, Window, WindowBounds,
    WindowOptions,
    component::{
        button::Button,
        input::{Input, InputState},
    },
    div,
    prelude::*,
    px, size,
};

struct Profile {
    name: Entity<InputState>,
    submitted: Option<SharedString>,
}

impl Render for Profile {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.submitted.as_ref().map_or_else(
            || SharedString::from("Not saved"),
            |name| SharedString::from(format!("Saved: {name}")),
        );
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(Input::new(&self.name).id("name").w(px(240.)))
            .child(
                Button::new("save")
                    .label("Save")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submitted = Some(this.name.read(cx).value());
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("status")
                    .role(gpui_kit::Role::Status)
                    .test_support()
                    .aria_label(status.clone())
                    .child(status),
            )
    }
}

#[gpui_kit::test]
fn saves_a_profile_through_the_ui(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, profile) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(px(640.), px(480.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                let view = cx.new(|cx| Profile {
                    name: cx.new(|cx| InputState::new(window, cx)),
                    submitted: None,
                });
                view
            },
        )
        .expect("open test window")
    });

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("status").label(), Some("Not saved"));

        window.click("name", cx);
        window.input("Ada 中文", cx);
        let name = window.find("name");
        assert_eq!(name.focused(), Some(true));
        assert_eq!(name.value(), Some("Ada 中文"));
        assert!(name.bounds().size.width > px(0.));
        // Named keys share the same Window API and refresh the resulting frame.
        window.press("backspace", cx);
        assert_eq!(window.find("name").value(), Some("Ada 中"));

        window.click("save", cx);
        let status = window.find("status");
        assert!(status.visible());
        assert_eq!(status.label(), Some("Saved: Ada 中"));
        assert!(status.bounds().top() >= window.find("save").bounds().bottom());
    })
    .unwrap();

    // Verify the application result as well as the native properties.
    cx.update(|cx| {
        assert_eq!(profile.read(cx).submitted.as_deref(), Some("Ada 中"));
    });
}
```

## Queries and interactions

Import `TestWindowExt` for queries and interactions from `gpui_kit::test`.
For custom registration, import `TestSupportExt`. Use normal Rust assertions with snapshots.

| API | Behavior |
| --- | --- |
| `window.find(id)` | Requires a unique observed target; errors list registered paths. |
| `window.try_find(id)` | Returns `None` when absent; ambiguous IDs still panic. |
| `window.find_all(id)` | Returns all registered matches, including invisible ones, by current-frame bounds origin (y, then x); equal-origin order is unspecified. Empty when absent. |
| `window.within(id)` | Resolves a native GPUI identity scope, including an unobserved ancestor. |
| `click`, `right_click`, `double_click`, `hover` | Dispatch real pointer events at the target center. |
| `click_at(id, offset, cx)` | Uses an offset from the target bounds' top-left corner. |
| `window.click_with_modifiers(id, modifiers, cx)` | Centered left click with modifiers; sends modifier changes and restores the previous modifier state afterward. |
| `window.click_with_options(id, options, cx)` | Combines offset, button, count, and modifiers using `ClickOptions`; restores the previous modifier state afterward. |
| `scroll(id, delta, cx)` | Dispatches a GPUI `ScrollDelta` wheel event. |
| `drag_to(from_id, to_id, cx)` | Drags between target centers within the current scope. |
| `window.drag(from, to, cx)` | Drags between window-local points; use `bounds()` for precise or cross-scope geometry. |
| `press(key, cx)` | Sends named GPUI keys such as `backspace`, `escape` or `secondary-a`. |
| `input(text, cx)` | Types Unicode characters at current focus; click the input first. |

`ClickOptions::new()` defaults to one left click at the center without modifiers.
For example, `ClickOptions::new().with_offset(point(px(8.), px(8.)))
.with_button(MouseButton::Right).with_count(2).with_modifiers(Modifiers::shift())`
combines a local offset, right double-click, and Shift. Counts must be positive.
Configurable clicks preserve caps lock. `find_all` ordering is geometry for the
current frame, not stable identity, paint order, or business order; filter with
`visible()` when counting visible matches.

GPUI IDs need only be unique within their native scope. Use
`window.within("dialog").find("name")` for repeated local IDs. Scoped windows
also provide pointer operations, `press` and `input`; keyboard helpers require
observed focus inside that scope and recheck it between characters. They do
not focus the target or replace its value. For a submenu with another
`"popup-menu"` ID, retain the resolved outer scope before opening the submenu,
then query its descendants.

## Inspect accessibility targets and individual events

Import `gpui_kit::test::{TestQueryExt, TestEventExt}` for additional observation
and event control. `elements()` returns observed elements in the current frame;
`find_by_label(label)`, `try_find_by_label(label)` and `find_all_by_label(label)`
match exact, case-sensitive native accessibility labels. `find_all_by_role(role)` selects an
AccessKit `Role`. Results include invisible registrations, ordered by y, then x
and a path tie-breaker. Required queries panic on absence; all unique queries
panic on ambiguity. Scoped queries include strict descendants, excluding the
scope node itself. These queries also work inside `within` and do not discover
unobserved text or offscreen virtual rows. A label is an accessibility name,
not rendered text; use stable IDs when localization changes the name.

For gestures whose intermediate state matters, dispatch each step explicitly:

```rust
use gpui_kit::test::TestEventExt;

let start = window.find("handle").bounds().center();
window.pointer_down(start, MouseButton::Left, cx);
window.pointer_move(start + point(px(24.), px(0.)), Some(MouseButton::Left), cx);
// Assert the state while the pointer is still held.
window.pointer_up(start + point(px(24.), px(0.)), MouseButton::Left, cx);
```

Positions use window-local pixels. `pointer_move(position, pressed_button, cx)`
receives the held button explicitly; these helpers do not remember button state.
`pointer_down` and `pointer_up` dispatch one click event each. Ordinary click, hover, scroll, drag and individual pointer steps use
the window's current modifiers. Configurable clicks temporarily use their
explicit modifiers and restore the preceding state afterward. `change_modifiers(modifiers, cx)` changes them and
preserves caps lock; restore the previous modifiers when the gesture ends.
`key_down(key, is_held, cx)` and `key_up(key, cx)` allow separate key transitions
and auto-repeat (`is_held = true`). Key events combine modifiers written in the
key string with the window's held modifiers, without changing the retained
modifier state. Each step refreshes before dispatch and
completes a frame afterward. These key events do not insert text; use `input`
for typing. Leave the window update and wait for deferred results when needed.

## Exercise text composition through an input handler

`gpui_kit::test::TestInput` bridges an explicit GPUI `InputHandler` to a window.
For an entity implementing `EntityInputHandler`, construct the adapter with
`gpui_kit::ElementInputHandler::new(bounds, entity.clone())`, then pass it to
`TestInput::new(handler, window)`. Keep the entity owned by the
production view and use its actual input geometry. This bridge calls that
handler directly; it does not select the focused control or run an OS IME.

`commit_text(text, cx)` commits through `replace_text_in_range`.
`compose_text(replacement, text, selection, cx)` updates marked text through
`replace_and_mark_text_in_range`. Replacement and selection ranges use UTF-16
code units, not UTF-8 byte offsets; selection is relative to the composed text.
With no replacement range, the handler replaces its current marked range or
selection. Empty composition text cancels preedit according to the handler.
`marked_text_range(cx)` returns a document-relative range;
`selected_text_range(cx)` returns `Option<UTF16Selection>`, including direction;
`unmark_text(cx)` ends marking while retaining its text. Mutation helpers complete a frame after the
handler call. They can test composition transitions and commit behavior, while
platform candidate windows, focus routing and OS IME integration still need
native tests. Use `window.input` for ordinary text typed at the current focus.

## Resize, display scale and clipboard

Use GPUI's existing `TestAppContext::simulate_window_resize(handle, size)` and
`simulate_window_scale_factor_change(handle, scale)` outside a window update,
then call `render_frame` before inspecting layout. Pointer positions remain
window-local logical pixels at every display scale. Assert both preserved
editing state and resized geometry, then activate a control at its new bounds.

Exercise clipboard behavior through production shortcuts (`secondary-c`,
`secondary-x`, `secondary-v`) and inspect `cx.read_from_clipboard()` when needed.
The test platform shares its clipboard across windows; editor state should
remain window-specific. This verifies the test-platform adapter and application
bindings, not the operating system's clipboard service.

After activating an external Link, inspect `cx.opened_url()` to verify the URL
received by the test platform and assert the application's click callback.
This does not establish that a browser opened the destination. The test
platform exposes no cursor-style reader; URL and callback assertions do not
verify the pointer cursor.

## Frames and async work

Snapshots are owned facts from the last completed frame, not live elements.
Fetch a new snapshot after an interaction. Interaction helpers refresh frames;
after an external entity update, focus change or resize, call
`window.render_frame(cx)` before querying.

For deferred actions, subscriptions or popup updates, leave `update_window`
so GPUI can process its effects. Use `cx.run_until_parked()` for queued work,
or import `TestAppContextExt` and await a bounded condition:

```rust
cx.wait_for(handle.into(), std::time::Duration::from_secs(1), |window, _| {
    window.try_find("dialog").is_none()
}).await;
```

`wait_for` runs outside a borrowed window update and uses GPUI test time.
Process a queued action before changing any input that the action is meant to
validate. Otherwise it can read the later input instead of the value at the
intended test step.

For timers, check the UI before its deadline as well as after expiration.
GPUI's non-synced `Animation` uses wall-clock time; advancing the test clock
does not complete it. Use reduced motion when supported, or a bounded wait
appropriate to that production animation before asserting final geometry.

## Assertion boundaries

- `checked()`, `selected()`, `expanded()`, `value()` and `label()` come from
  native accessibility properties. `None` means unavailable; masked input
  deliberately omits its value. Do not supply a second set of test-only state.
- `disabled()` currently reports `Some(true)` or `None`. To prove a control is
  enabled, perform the interaction and assert the result. Absence of a flag
  is not proof that the control accepts input.
- `focused()` measures the observed native focus scope. Missing bindings are
  diagnosed when the native element advertises `Action::Focus`; controls
  without that action can still return `None`. Register the real binding.
- `bounds()` supports containment, relative position, overlap, scrolling and
  drag assertions. `visible()` checks geometry, clipping and observed style;
  it does not prove a target is unobstructed. Real hit testing decides whether
  a click reaches the control.
- Native properties can be wrong too. Pair them with real interactions,
  geometry and the application result. A label is not rendered text, and a
  correct accessibility value does not establish correct pixels.
- These tests run in-process. Pixel comparisons need GPUI's separate renderer;
  packaged-app behavior and full OS IME composition need other coverage. Kit's
  explicit macOS rendering target is
  `cargo test -p gpui-kit --features test-support --test rendering --locked`.

## Additional resources

- [Website testing guide](https://gpui-kit.com/docs/test) — bilingual setup,
  component coverage, platform requirements and CI.
- [GPUI test examples](test-examples.md) — entity and context patterns.
- [GPUI test reference](test-reference.md) — re-entrancy and property tests.

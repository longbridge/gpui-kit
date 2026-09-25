---
title: Accessibility
description: Build and test accessible GPUI Kit interfaces with AccessKit semantics and actions.
order: -3.4
---

# Accessibility

GPUI sends an accessibility tree to platform assistive technology through [AccessKit](https://accesskit.dev/). A useful control has a **role** (what it is), a **name** (how a person identifies it), **state** (such as value, checked, selected, or expanded), and **actions** (what assistive technology can request). Its stable element ID preserves identity across frames; it is not the name a person hears. The same interface must work with a keyboard and show where focus is. Tree properties alone do not implement keyboard behavior.

Applications normally use GPUI Kit's styled components from `gpui_kit::component`. Their interaction comes from the unstyled `gpui_kit::base` layer. Both are available through one `gpui-kit` dependency. Use a standard control before composing a new interactive `div`: it already coordinates pointer input, keyboard input, focus, state, and AccessKit semantics.

For a custom keyboard target, follow [Focus](./focus) first: `track_focus`, Tab order, and an accessible role solve different parts of the interaction.

Think of accessibility as one path through the application, not a separate description attached at the end:

| Step | In the Profile example | What to verify |
| --- | --- | --- |
| Retained state | `Entity<InputState>` owns the draft; `Profile::submitted` owns the saved value. | Typing and Save change the intended model, including after a rerender. |
| Rendering | `Input`, `Button`, and the status `div` read that state. | The visible label and result match the model. |
| Semantics | The controls expose a role, name, relevant state, and available actions through AccessKit. | A painted-frame snapshot contains the expected properties. |
| Operation | Pointer, keyboard, and assistive technology reach the same command. | Save works with each input method; focus moves visibly and predictably. |

Start with the first two steps, then inspect the semantic tree and operate the real window. A passing headless tree assertion cannot prove what a screen reader announces or whether a focus ring is visible.

## Follow one complete Save flow

Run the repository's [Profile UI test](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/tests/ui.rs) from the workspace root:

```sh
cargo test -p gpui-kit --features test-support --test ui saves_a_profile_through_the_ui -- --exact
```

The test opens a headless window, enters a name, activates **Save**, and checks both application state and accessibility properties written by the rendered GPUI elements. This gives you a concrete path from the model to a visible control and its AccessKit node. For setup of your own test crate, continue with [Testing](./test).

### 1. Name the control that receives input

The view keeps an `Entity<InputState>` so the text survives rerenders. It renders the input with a stable ID for GPUI and an explicit human-readable name for assistive technology:

```rust
Input::new(&self.name)
    .id("name")
    .aria_label("Profile name")
    .w(px(240.))
```

The ID and label answer different questions. `"name"` lets GPUI and the test identify this particular element; `"Profile name"` tells a person what to enter. The example also renders **Profile name** as visible text directly above the input. Proximity does not create an accessibility relationship by itself, so the input has its own name. A placeholder would disappear as the user types and is a poor replacement for a persistent label.

### 2. Update one model, then render the status from it

The **Save** button's listener copies the current input value into the view's `submitted` field and calls `cx.notify()`. The next `render` computes a status string from `submitted` and uses the same string for pixels and the accessibility name:

```rust
let status = self.submitted.as_ref().map_or_else(
    || SharedString::from("Not saved"),
    |name| SharedString::from(format!("Saved: {name}")),
);

div()
    .id("status")
    .role(Role::Status)
    .test_support()
    .aria_label(status.clone())
    .child(status)
```

The status `div` is not a button; it describes the result of an action. Its stable ID plus `Role::Status` gives it a semantic node. `.test_support()` only makes this custom node discoverable to GPUI Kit's test helper when that feature is enabled. A role and label do not change the model; `cx.notify()` is what causes the new model value to reach the next frame.

### 3. Assert the contract after a painted frame

The test calls `render_frame` before its first query, then interacts with the native elements and reads fresh snapshots:

```rust
window.render_frame(cx);
assert_eq!(window.find("status").role(), Some(Role::Status));
assert_eq!(window.find("status").label(), Some("Not saved"));

window.click("name", cx);
window.input("Ada José", cx);
assert_eq!(window.find("name").label(), Some("Profile name"));
assert_eq!(window.find("name").value(), Some("Ada José"));

window.press("backspace", cx);
assert_eq!(window.find("name").value(), Some("Ada Jos"));
window.click("save", cx);
assert_eq!(window.find("status").label(), Some("Saved: Ada Jos"));
```

The repository test also checks input focus, status geometry, and `profile.read(cx).submitted`. Checking the model prevents a test from passing when only the label changes. `window.click` exercises pointer activation here; the `backspace` step removes the accented `é` after the pointer has focused the input, testing Unicode editing rather than keyboard navigation to **Save**. As an experiment, remove `.aria_label("Profile name")` and rerun the test: the label assertion shows why visible proximity to other text cannot be assumed to name the input.

To check the keyboard path in a real window, put this complete version of the same view in the existing `examples/hello_world/src/main.rs`, then run `cargo run -p hello_world` from the workspace root. This is a local exercise; restore the example file when finished. The headless test above does not run this keyboard sequence.

```rust
use gpui_kit::{
    AppContext, Context, Entity, Role, SharedString, Window, WindowOptions,
    component::{button::Button, input::{Input, InputState}},
    div, prelude::*, px,
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
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(
                div().flex().flex_col().gap_1().child("Profile name").child(
                    Input::new(&self.name)
                        .id("name")
                        .aria_label("Profile name")
                        .w(px(240.)),
                ),
            )
            .child(Button::new("save").label("Save").on_click(
                cx.listener(|this, _, _, cx| {
                    this.submitted = Some(this.name.read(cx).value());
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .id("status")
                    .role(Role::Status)
                    .aria_label(status.clone())
                    .child(status),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Profile {
                name: cx.new(|cx| InputState::new(window, cx)),
                submitted: None,
            })
        })
        .expect("failed to open window");
    });
}
```

The window starts with **Not saved**. Starting before the form, Tab to **Profile name** and confirm a visible focus indication; type `Ada José` and use Backspace once, leaving `Ada Jos`. Tab to **Save**, confirm its focus indication, then press Enter. The visible result should become **Saved: Ada Jos**. While **Save** remains focused, press Space to activate it again. Use Shift+Tab to return to the input, then Tab back to **Save** to verify reverse and forward navigation. If either activation key does not work, inspect the button's focus and key handling instead of treating the pointer test as evidence. Finally, activate **Save** through your target platform's assistive technology and check the announced role, name, and result; the status role alone does not guarantee a live announcement.

To adapt this exercise to your own form, follow the same order: give each actual input a stable ID and name, make the action update retained state, render any result from that state, then assert both the model and the freshly painted semantics. Keep the visible label beside the control for sighted users; the accessible name is an additional contract. If a field becomes invalid, expose the error in visible text and check that a person using assistive technology can discover the error and recover. A color change by itself does not communicate the reason or correction.

## Start with semantic controls

The following controls expose useful semantics from their actual state:

| Control | Accessibility contract |
| --- | --- |
| Button, Link | Button or Link role, accessible name, activation. Use Button for an application command and Link for an external destination. |
| Checkbox, Switch, Toggle, Radio | Their respective role and checked or toggled state. Checkbox also reports mixed state. |
| Input | Text input role chosen from its content type, name, non-sensitive value, and an accessible `SetValue` action when not disabled. Masked and password values are withheld. |
| Select | ComboBox role, name, committed value, expanded state, and an accessible activation path. |
| Tab, tab list | Tab and TabList roles; tabs report selection and, when supplied, position in the set. |
| Slider, Progress | Numeric value and range; Slider handles accessible Increment and Decrement. Indeterminate Progress omits its value. |
| Table | Table, row, header and cell roles with indices and optional counts. Name the Table root. |

For a labeled button, the visible label is the accessible name by default. Name an icon-only button explicitly:

```rust
use gpui_kit::component::{IconName, button::Button};

Button::new("search-documents")
    .icon(IconName::Search)
    .accessibility_label("Search documents")
    .tooltip("Search documents")
    .on_click(|_, window, cx| {
        // Invoke the same application command used by its keyboard route.
    })
```

`accessibility_label` names the control for assistive technology; a tooltip is a separate hint and cannot replace that name. Keep the name specific to the action and update it if the action changes. Do not assume that arbitrary text nested inside a custom container becomes its accessible name. A visible form label and a neighboring input likewise do not gain an automatic label relationship merely by being next to each other: name the actual input with `Input::aria_label(...)`, or use a component that provides the relationship. Input can fall back to a placeholder as its name, but an explicit label remains clearer; a generated mask placeholder is deliberately not used as the name.

## Identity and roles

GPUI includes an element in its accessibility tree when it has both an [ElementId](./element_id) and a non-empty accessibility role. The global identity also contains IDs of ancestors. Keep IDs stable across frames and derive repeated item IDs from domain keys, so reordering does not look like a series of removals and insertions to assistive technology. An `id` alone is not a role; an unroled `div` is a layout container, not an announced control. A role alone does not make a control focusable or operable.

For a semantic status message, a GPUI element can supply both:

```rust
use gpui_kit::*;

div()
    .id("save-status")
    .role(Role::Status)
    .test_support()
    .aria_label("Saved")
    .child("Saved")
```

The explicit label is the announced name. `.test_support()` lets the later integration test find this custom `div` when `test-support` is enabled; it adds no layout container and is inert in normal builds. `Role::GenericContainer` is filtered from the accessibility tree; use an actual role for a meaningful node. `accessibility_id(...)` is a separate, author-provided identifier exposed to platform automation. It maps to identifiers such as UIA `AutomationId` on Windows and `AXIdentifier` on macOS, with Linux AT-SPI support depending on the deployed adapter. It is not a substitute for GPUI's `.id(...)` or for a human-readable name.

## Names, states, and relationships

On an identified `div`, GPUI's `StatefulInteractiveElement` provides `.role(...)`, `.aria_label(...)`, `.aria_description(...)`, `.aria_selected(...)`, `.aria_expanded(...)`, `.aria_toggled(...)`, `.aria_value(...)`, `.aria_numeric_value(...)`, and range and collection properties. Numeric controls can also report minimum, maximum, step, and orientation; headings can report level; list and table items can report positions and counts. Update these from the same model that draws the visible UI; the owning view uses its [Context](./context) to notify GPUI after a state change. A description supplements the name; it does not replace it. `.aria_keyshortcuts(...)` announces a shortcut but does not bind the key: register the real GPUI keybinding separately.

GPUI's current `div` API has no general `.aria_disabled(...)` builder. Base controls such as Button and Checkbox gate focus and activation when disabled, but that does not guarantee a native disabled property for every node. Verify both the available tree state and the actual disabled behavior. Likewise, `.track_focus(...)` gives a node a focus path and advertises the accessible Focus action; a role or `.focusable()` alone does not implement a useful keyboard command.

The element tree establishes parent/child relationships. Composite controls may keep keyboard focus on a parent and mark the current child with `.aria_active_descendant()`. GPUI applies that child-side marker only when an ancestor actually has focus. The child needs its own ID and role. This is specialized composite behavior; use the built-in Select, menu, or list behavior when it fits.

Do not infer a web `aria-labelledby`, `aria-describedby`, or `aria-controls` builder from the `aria_` prefix. These are not general builders on GPUI's current `div` API. For a Table with a visible caption, set `.accessibility_label(...)` on the Table root; the caption container does not automatically name it.

## Accessible actions and keyboard input

An AccessKit action is distinct from a GPUI [Action](./action) dispatched by a keybinding or menu. GPUI exposes it as `AccessibleAction`. For an identified `div`, `.on_a11y_action(action, handler)` registers one requested action; the handler receives optional `ActionData`, `&mut Window`, and `&mut App`. `.on_click(...)` already advertises accessible Click activation and routes it to the click handler, so a second Click handler can perform the command twice. A custom slider, for example, must offer Increment and Decrement as well as its pointer and keyboard controls, and update its numeric accessibility value after the model changes. GPUI Kit's Slider already does this.

Keep the interaction promise consistent: the visible label, accessible name, shortcut, pointer behavior, keyboard behavior, and assistive action should all perform the same command. A clickable painted shape with a hitbox is still missing semantics and keyboard operation until those are implemented. After dialogs or sheets close, restore focus to their trigger. Keep focus visible and ordered according to the task.

For the Profile flow above, use the runnable window exercise to verify keyboard operation separately from the pointer test. The visible result changes only after `submitted` changes and `cx.notify()` triggers a rerender. For a composite control, also test its documented arrow keys and Escape behavior. Test these paths on the platforms you ship; a successful pointer test or an announced shortcut does not establish them.

### Build one interactive custom control

The earlier `EventSurface` exercise adds a Status node to a painted shape, but it does not make the shape operable without a pointer. For a control whose appearance can be built from normal elements, start with composition. Replace `examples/hello_world/src/main.rs` temporarily with this complete example, then run `cargo run -p hello_world` from the repository root. Restore the example file when finished.

```rust
use gpui_kit::{
    *,
    component::ThemeStyled as _,
    prelude::*,
};

struct CounterControl {
    focus: FocusHandle,
    activations: usize,
}

impl CounterControl {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            activations: 0,
        }
    }
}

impl Render for CounterControl {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let status = format!("Activations: {}", self.activations);

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .child(
                div()
                    .id("activate-counter")
                    .role(Role::Button)
                    .aria_label("Activate counter")
                    .track_focus(&self.focus)
                    .p_3()
                    .border_1()
                    .when(focused, |control| control.focus_ring_style(window, cx))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.activations += 1;
                        cx.notify();
                    }))
                    .child("Activate counter"),
            )
            .child(
                div()
                    .id("activation-status")
                    .role(Role::Status)
                    .aria_label(status.clone())
                    .child(status),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(CounterControl::new)
        })
        .expect("failed to open window");
    });
}
```

The retained `FocusHandle` survives rerenders. Its `.tab_stop(true)` includes the control in Tab navigation, while `.track_focus(...)` registers the rendered node. The stable ID and `Role::Button` expose a control node, and `.aria_label(...)` names the operation. The focus ring is rendered only while that handle is focused. The single `.on_click(...)` listener changes the retained count and calls `cx.notify()`; GPUI routes primary pointer clicks, Enter/Space keyboard activation on the focused node, and AccessKit Click to that listener. Do not register a second `AccessibleAction::Click` for this node, or the command may run twice.

Check each path in the running window. A pointer click should change **Activations: 0** to **Activations: 1**. From outside the control, Tab to it and confirm the ring is visible; Enter should produce **Activations: 2**, and Space should produce **Activations: 3**. Tab away and Shift+Tab back to confirm it remains in the navigation order. With a platform accessibility inspector, find a Button named **Activate counter** and a Status whose label matches the visible count. Invoke the Button's accessible Click operation with the target assistive technology and confirm the count increases exactly once. A headless snapshot can check the role, name, and updated model, but cannot prove the ring is visible, the platform adapter exposes the node, or a screen reader announces the changing Status.

If Tab skips the control, check both `.tab_stop(true)` on the retained handle and `.track_focus(...)` on the current rendered node. If its role or name is missing, check that the same node has a stable ID, role, and label. If an activation produces two increments, look for a second Click or key handler calling the same command. This exercise demonstrates one button-like control; for real application commands, prefer the standard Button, which also handles its other component states and styling.

## Custom `Element` implementations

When composition with `div()` is insufficient, a low-level `Element` has explicit hooks. Continue the [EventSurface exercise](./element#make-the-surface-visible-and-respond-to-a-press), which already implements `IntoElement`, layout, prepaint, and paint in `examples/hello_world/src/main.rs`. Make these three edits inside that existing example:

1. In `Render for SurfaceDemo`, pass the current counter text into `EventSurface` alongside `owner` and `color`: `status: format!("Pointer presses: {}", self.presses),`.
2. Add `status: String,` to the `EventSurface` struct.
3. Replace its `id()` method and add the two accessibility methods inside `impl Element for EventSurface`:

```rust
fn id(&self) -> Option<ElementId> { Some("pointer-status".into()) }

fn a11y_role(&self) -> Option<Role> { Some(Role::Status) }

fn write_a11y_info(&self, node: &mut accesskit::Node) {
    node.set_label(self.status.clone());
}
```

`use gpui_kit::*;` in the complete EventSurface example already imports `ElementId`, `Role`, and `accesskit`. Run `cargo run -p hello_world` again. With assistive technology or a platform accessibility inspector active, find the rectangle's **Status** node named **Pointer presses: 0**. A left press inside the rectangle updates the visible counter and, after rerender, the node's name to **Pointer presses: 1**; the stable ID keeps its identity across those frames. An inspector can show the tree properties, while only a screen reader check can establish whether the change is announced. The accessibility tree is built when assistive technology is active; an absent tree with no client attached does not by itself prove the methods failed.

This is a narrowly scoped semantics exercise. `EventSurface` still responds only to pointer input, and `Role::Status` describes its counter; it does not make the rectangle a keyboard or assistive-technology-activatable control. Use a standard Button for a real command, or separately implement tracked focus, keyboard activation, an appropriate control role, and an accessible action. GPUI only calls `write_a11y_info` for an element that contributes an identified role. GPUI Kit's `window.find(...)` helper observes registered controls and `div().test_support()`, not this raw `Element`; do not expect it to find `pointer-status`. `a11y_synthetic_children(...)` can add AccessKit child nodes after prepaint, for example text runs in a custom editor. `A11ySubtreeBuilder::synthetic_node_id(key)` derives a child ID from its parent and a stable key; `push_child(...)` attaches the node. Keys must be unique among a parent's synthetic children. This is an advanced path: the implementer owns hit testing, event dispatch, focus, keyboard handling, and accessible actions in addition to the tree data.

## Test the contract

Enable GPUI Kit's `test-support` feature for UI integration tests and import `gpui_kit::test::TestWindowExt`; import `TestSupportExt` when observing a custom `div`. Query the **painted** element after `window.render_frame(cx)`. `ElementSnapshot` exposes `role()`, `label()`, `value()`, `focused()`, `checked()`, `indeterminate()`, `selected()`, and `expanded()`. Re-query after each interaction because a snapshot describes one completed frame. Use the runnable Profile test above as the assertion example; it checks role, name, value, focus, geometry, and the saved model value.

`None` from a snapshot state reader means the property is unavailable, not `false`. In particular, `.disabled()` is `Some(true)` only if the node exposes that flag; test disabled behavior by attempting activation and checking that the result did not change. `ElementSnapshot::value()` reads a string accessibility value, not Slider's numeric value or painted text. Masked and password inputs intentionally expose no accessibility value. The current Input implementation registers `SetValue` when it is not disabled, including in read-only mode; its handler uses a programmatic replacement path. Do not treat `readonly(true)` as protection against this accessible write path without verifying the behavior you need. See [Testing](./test) for test setup, frame, and focus details.

Headless snapshots read AccessKit properties produced by GPUI elements; they do not inspect the final platform adapter tree, a screen reader announcement, or pixels. Inspect the running app with assistive technology on each target platform for announcement order, focus movement, editing, and actions. In particular, verify that a changing `Role::Status` is actually announced rather than assuming its role alone guarantees a live announcement. Platform adapters differ, and [WebAssembly support](./webassembly.md) must be checked independently from native desktop behavior. Pair this with visual checks for focus contrast, readable text, target size, reduced motion, and information that must not depend on color alone; see [Design Guides](./design-guides).

### A practical acceptance pass

Use this sequence on every platform you plan to ship. Record the operating system, assistive technology, app build, and whether each result was observed; an AccessKit property in a test is evidence for the GPUI tree, not for every platform bridge.

1. **Keyboard only:** start before the form. Tab to the input and Save button, use Shift+Tab to return, and verify visible focus at each stop. Type, edit, and activate Save with Enter or Space. Confirm the visible result and saved model. For a composite widget, also try its documented arrow keys, Escape, and Tab exit.
2. **Assistive technology:** navigate to the same input and button. Confirm their announced role and name, the input value where disclosure is appropriate, and the state after Save. Activate the button through the assistive technology's control action. Check the resulting status announcement in the actual application; `Role::Status` alone is not proof that it will be spoken.
3. **State changes:** test empty or invalid input, disabled and enabled transitions, and an overlay if the flow opens one. Check that the user can find the error, that disabled commands cannot execute, and that focus reaches a sensible target after the overlay closes.
4. **Presentation:** zoom or enlarge text, inspect focus contrast and clipping, reduce motion where supported, and check that color is not the only way to distinguish success, error, or selection.

### If a check fails

| Symptom | Inspect first |
| --- | --- |
| The custom node is absent from the painted snapshot. | Does it have a stable `.id(...)` and a meaningful `.role(...)`? Has `window.render_frame(cx)` run? For a custom `div` located by GPUI Kit's test helper, did you add `.test_support()` and enable `test-support`? |
| The role is present but the name is missing or wrong. | Name the control itself (`Input::aria_label`, `Button::accessibility_label`, or `div().aria_label`); nearby text and tooltips are not a guaranteed naming relationship. |
| The status text changes but its snapshot stays old. | Re-query after the interaction; check that the owner updated retained state and called `cx.notify()`. A saved `ElementSnapshot` describes one frame. |
| A shortcut is announced but pressing it does nothing. | `.aria_keyshortcuts(...)` only reports a shortcut. Register the GPUI binding and attach its action handler to the intended focus path; see [Actions](./action). |
| Pointer activation works but keyboard or assistive activation fails. | Inspect Tab stop, tracked focus, key context, action handler, and accessible action. A role and a hitbox alone do not provide these paths; consider using a built-in control. |
| A headless assertion passes but a screen reader behaves differently. | Reproduce on the target platform and inspect the platform adapter's result. The headless helper does not validate announcements, focus visuals, or adapter behavior. |

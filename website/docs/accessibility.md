---
title: Accessibility
description: Build and test accessible GPUI Kit interfaces with AccessKit semantics and actions.
order: -3.4
---

# Accessibility

GPUI sends an accessibility tree to platform assistive technology through [AccessKit](https://accesskit.dev/). A useful node tells a screen reader **what** a control is (role), **which** control it is (name and stable identity), **what state it is in** (value, selected, checked, expanded), and **what it can do** (actions). The same interface must also work with a keyboard and have a visible focus state. An accessibility node alone does not supply keyboard behavior.

Applications normally use GPUI Kit's styled components from `gpui_kit::component`. Their interaction comes from the unstyled `gpui_kit::base` layer. Both are available through one `gpui-kit` dependency. Use a standard control before composing a new interactive `div`: it already coordinates pointer input, keyboard input, focus, state, and AccessKit semantics.

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

GPUI includes an element in its accessibility tree when it has both an [ElementId](./element_id) and a non-empty accessibility role. The global identity also contains IDs of ancestors. Keep IDs stable across frames and derive repeated item IDs from domain keys, so reordering does not look like a series of removals and insertions to assistive technology. An `id` alone is not a role; an unroled `div` is a layout container, not an announced control.

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

## Custom `Element` implementations

When composition with `div()` is insufficient, a low-level `Element` has explicit hooks:

```rust
use gpui_kit::{Role, accesskit::Node};

fn a11y_role(&self) -> Option<Role> { Some(Role::Status) }

fn write_a11y_info(&self, node: &mut Node) {
    node.set_label("Download complete");
}
```

These methods belong inside an `impl Element for ...` that also provides a stable `id()` and the required layout, prepaint and paint methods; see [Element](./element). GPUI only calls `write_a11y_info` for an element that contributes an identified role. `a11y_synthetic_children(...)` can add AccessKit child nodes after prepaint, for example text runs in a custom editor. `A11ySubtreeBuilder::synthetic_node_id(key)` derives a child ID from its parent and a stable key; `push_child(...)` attaches the node. Keys must be unique among a parent's synthetic children. This is an advanced path: the implementer owns hit testing, event dispatch, focus, keyboard handling, and accessible actions in addition to the tree data.

## Test the contract

Enable GPUI Kit's `test-support` feature for UI integration tests and import `gpui_kit::test::TestWindowExt`. Query the **painted** native element after `window.render_frame(cx)`. `ElementSnapshot` exposes `role()`, `label()`, `value()`, `focused()`, `checked()`, `indeterminate()`, `selected()`, and `expanded()`. Re-query after each interaction because a snapshot describes one completed frame:

```rust
window.render_frame(cx);
assert_eq!(window.find("save-status").role(), Some(Role::Status));
assert_eq!(window.find("save-status").label(), Some("Saved"));

window.click("save", cx);
assert_eq!(window.find("save-status").label(), Some("Saved: Ada"));
```

The example assumes the application updates the status label in its Save handler and that the status `div` uses `.test_support()` as shown above. Also assert the actual application result and exercise keyboard input. `None` from a snapshot state reader means the native property is unavailable, not `false`. In particular, `.disabled()` is `Some(true)` only if the node exposes that flag; test disabled behavior by attempting activation and checking that the result did not change. `ElementSnapshot::value()` reads a string accessibility value, not Slider's numeric value or painted text. Masked and password inputs intentionally expose no accessibility value. The current Input implementation registers `SetValue` when it is not disabled, including in read-only mode; its handler uses a programmatic replacement path. Do not treat `readonly(true)` as protection against this accessible write path without verifying the behavior you need. See [Testing](./test) for a complete runnable test and focus, frame, and platform details.

Headless snapshots verify the properties exposed by the native tree and real interaction, not a packaged screen reader session or pixels. Inspect the running app with assistive technology on each target platform for announcement order, focus movement, editing, and actions. Platform adapters differ, and [WebAssembly support](./webassembly.md) must be checked independently from native desktop behavior. Pair this with visual checks for focus contrast, readable text, target size, reduced motion, and information that must not depend on color alone; see [Design Guides](./design-guides).

---
title: Focus Trap
description: A utility element that traps keyboard focus within a container, preventing Tab navigation from escaping.
---

# Focus Trap

Focus trap utility for constraining keyboard focus within a specific container. Essential for modal dialogs, sheets, and overlay components to provide proper keyboard navigation accessibility.

**Note:** [Dialog](/component/dialog) and [Sheet](/component/sheet) components have focus trap built-in. You only need to manually use `focus_trap()` for custom modal-like components.

## Import

```rust
use gpui_kit::component::FocusTrapElement;
```

## Usage

### Basic Focus Trap

```rust
let container_handle = cx.focus_handle();

v_flex()
    .child(Button::new("btn1").label("Button 1"))
    .child(Button::new("btn2").label("Button 2"))
    .child(Button::new("btn3").label("Button 3"))
    .focus_trap("trap1", &container_handle)
// Pressing Tab will cycle: btn1 -> btn2 -> btn3 -> btn1
// Focus will not escape to elements outside this container
```

### Multiple Focus Traps

You can have multiple independent focus trap areas in your application. Each trap operates independently:

```rust
let trap1_handle = cx.focus_handle();
let trap2_handle = cx.focus_handle();

v_flex()
    .gap_4()
    // First focus trap area
    .child(
        h_flex()
            .gap_2()
            .child(Button::new("trap1-1").label("Area 1 - Button 1"))
            .child(Button::new("trap1-2").label("Area 1 - Button 2"))
            .child(Button::new("trap1-3").label("Area 1 - Button 3"))
            .focus_trap("trap1", &trap1_handle)
    )
    // Second focus trap area
    .child(
        h_flex()
            .gap_2()
            .child(Button::new("trap2-1").label("Area 2 - Button 1"))
            .child(Button::new("trap2-2").label("Area 2 - Button 2"))
            .focus_trap("trap2", &trap2_handle)
    )
```

### Focus Trap with Dialog

[Dialog] components have focus trap built-in automatically. You don't need to manually add `focus_trap()`:

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Settings")
        .child(
            v_flex()
                .gap_3()
                .child(Button::new("save").label("Save"))
                .child(Button::new("cancel").label("Cancel"))
                .child(Button::new("reset").label("Reset"))
        )
    // Dialog internally uses focus_trap()
    // Tab navigation automatically cycles: save -> cancel -> reset -> save
})
```

### Focus Trap with Sheet

[Sheet] components also have focus trap built-in automatically:

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Filter Options")
        .child(
            v_flex()
                .gap_2()
                .child(Checkbox::new("option1").label("Option 1"))
                .child(Checkbox::new("option2").label("Option 2"))
                .child(Button::new("apply").label("Apply Filters"))
        )
    // Sheet internally uses focus_trap()
    // Focus automatically cycles within the sheet panel
})
```

## How It Works

The focus trap system consists of three key components:

1. **FocusTrapContainer**: Wraps any container element and registers it as a focus trap area
2. **FocusTrapManager**: Global state manager that tracks all active focus traps
3. **Root Integration**: The [Root] view intercepts Tab/Shift-Tab events and enforces focus cycling

When Tab or Shift-Tab is pressed:

1. [Root] detects if the currently focused element is inside a focus trap
2. If yes, it calculates the next focusable element within the same trap
3. If focus would escape the trap, it cycles back to the beginning (Tab) or end (Shift-Tab)
4. This prevents focus from leaving the trapped container

### Built-in Focus Trap Components

The following components have focus trap functionality built-in and don't require manual `focus_trap()` calls:

- **[Dialog]** - Modal dialogs automatically trap focus (see `dialog.rs:437`)
- **[Sheet]** - Side panels automatically trap focus (see `sheet.rs:197`)

## API Reference

- [FocusTrapElement](https://docs.rs/gpui-component/latest/gpui_component/trait.FocusTrapElement.html)
- [FocusTrapContainer](https://docs.rs/gpui-component/latest/gpui_component/struct.FocusTrapContainer.html)

## Examples

### Custom Modal with Focus Trap

```rust
struct CustomModal {
    container_handle: FocusHandle,
}

impl CustomModal {
    fn new(cx: &mut App) -> Self {
        Self {
            container_handle: cx.focus_handle(),
        }
    }
}

impl Render for CustomModal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .gap_4()
                    .p_6()
                    .bg(cx.theme().background)
                    .rounded(cx.theme().radius_lg)
                    .shadow_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child("This is a modal dialog")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("ok").primary().label("OK"))
                            .child(Button::new("cancel").label("Cancel"))
                    )
                    .focus_trap("modal", &self.container_handle)
            )
    }
}
```

### Nested Focus Traps

Focus traps support nesting. When multiple traps are active, the innermost trap takes precedence:

```rust
let outer_handle = cx.focus_handle();
let inner_handle = cx.focus_handle();

div()
    .child(
        v_flex()
            .gap_4()
            .p_4()
            .border_1()
            .border_color(cx.theme().border)
            .child(Button::new("outer-1").label("Outer Button 1"))
            .child(
                // Inner trap takes precedence when focused
                h_flex()
                    .gap_2()
                    .p_4()
                    .bg(cx.theme().accent.opacity(0.1))
                    .child(Button::new("inner-1").label("Inner Button 1"))
                    .child(Button::new("inner-2").label("Inner Button 2"))
                    .focus_trap("inner", &inner_handle)
            )
            .child(Button::new("outer-2").label("Outer Button 2"))
            .focus_trap("outer", &outer_handle)
    )
```

### Conditional Focus Trap

You can conditionally apply focus trapping based on application state:

```rust
struct ModalView {
    is_modal: bool,
    container_handle: FocusHandle,
}

impl Render for ModalView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = v_flex()
            .gap_2()
            .child(Button::new("btn1").label("Button 1"))
            .child(Button::new("btn2").label("Button 2"))
            .child(Button::new("btn3").label("Button 3"));

        if self.is_modal {
            // Apply focus trap when in modal mode
            content.focus_trap("conditional", &self.container_handle)
                .into_any_element()
        } else {
            // Normal behavior without focus trap
            content.into_any_element()
        }
    }
}
```

## Accessibility Notes

- Focus trapping is essential for modal dialogs and overlays to meet WCAG accessibility guidelines
- Always provide a way to close or dismiss trapped focus areas (ESC key, close button)
- The first focusable element in the trap should receive focus when the trap is activated
- Use focus traps sparingly - only for truly modal interactions
- Ensure keyboard navigation order is logical within the trapped area

## See Also

- [Root View System](/component/root) - Manages focus trap behavior at the window level
- [Dialog](/component/dialog) - Uses focus trap automatically
- [Sheet](/component/sheet) - Uses focus trap automatically
- [focus-trap-react](https://github.com/focus-trap/focus-trap-react) - Similar concept for React applications

[Root]: https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html
[FocusTrapElement]: https://docs.rs/gpui-component/latest/gpui_component/trait.FocusTrapElement.html
[Dialog]: /component/dialog
[Sheet]: /component/sheet

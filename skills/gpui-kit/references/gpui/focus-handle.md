# Focus & Keyboard Navigation

**Contents:** [Overview](#overview) · [Quick Start](#quick-start) · [Focus Events](#focus-events) · [Keyboard Navigation](#keyboard-navigation) · [Common Patterns](#common-patterns) · [Best Practices](#best-practices)

## Overview

GPUI's focus system enables keyboard navigation and focus management.

**Key Concepts:**
- **FocusHandle**: Reference to focusable element
- **Focus tracking**: Current focused element
- **Keyboard navigation**: Tab/Shift-Tab between elements
- **Focus events**: on_focus, on_blur

## Quick Start

### Creating Focus Handles

```rust
struct FocusableComponent {
    focus_handle: FocusHandle,
}

impl FocusableComponent {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}
```

### Making Elements Focusable

```rust
impl Render for FocusableComponent {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_enter))
            .child("Focusable content")
    }

    fn on_enter(&mut self, _: &Enter, cx: &mut Context<Self>) {
        // Handle Enter key when focused
        cx.notify();
    }
}
```

### Focus Management

```rust
impl MyComponent {
    fn focus(&mut self, cx: &mut Context<Self>) {
        self.focus_handle.focus(cx);
    }

    fn is_focused(&self, cx: &App) -> bool {
        self.focus_handle.is_focused(cx)
    }

    fn blur(&mut self, cx: &mut Context<Self>) {
        cx.blur();
    }
}
```

## Focus Events

### Handling Focus Changes

```rust
impl Render for MyInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(cx);

        div()
            .track_focus(&self.focus_handle)
            .on_focus(cx.listener(|this, _event, cx| {
                this.on_focus(cx);
            }))
            .on_blur(cx.listener(|this, _event, cx| {
                this.on_blur(cx);
            }))
            .when(is_focused, |el| {
                el.bg(cx.theme().focused_background)
            })
            .child(self.render_content())
    }
}

impl MyInput {
    fn on_focus(&mut self, cx: &mut Context<Self>) {
        // Handle focus gained
        cx.notify();
    }

    fn on_blur(&mut self, cx: &mut Context<Self>) {
        // Handle focus lost
        cx.notify();
    }
}
```

## Keyboard Navigation

### Tab Order

A tracked handle is a Tab stop only when the handle says so. `cx.focus_handle()`
starts with `tab_stop = false`, and an element's own `.tab_index()` /
`.tab_stop()` do not apply to a handle passed to `track_focus`: build the handle
with the flag.

```rust
// In the owning entity's constructor:
let focus1 = cx.focus_handle().tab_stop(true);          // Tab order: 1
let focus2 = cx.focus_handle().tab_stop(true);          // Tab order: 2
let hidden = cx.focus_handle();                         // focusable, never a Tab stop

div()
    .child(div().track_focus(&focus1).child("first"))
    .child(div().track_focus(&focus2).child("second"))
    .child(div().track_focus(&hidden).child("container"))
```

The Tab table is rebuilt every frame from the elements that painted with a
tracked handle, in tree order (`tab_index` reorders within a `tab_group`). A
stateless component may create its handle in `render` through keyed state —
`window.use_keyed_state(id, cx, |_, cx| cx.focus_handle().tab_stop(true))` —
which survives re-renders, so the order stays stable; `Button` does exactly
this. A focused element receives `ClickEvent::Keyboard` in `on_click` when
Enter or Space is released, so a `track_focus` + `on_click` row activates from
the keyboard without extra bindings.

### Focus Within Containers

```rust
impl Container {
    fn focus_first(&mut self, cx: &mut Context<Self>) {
        if let Some(first) = self.children.first() {
            first.update(cx, |child, cx| {
                child.focus_handle.focus(cx);
            });
        }
    }

    fn focus_next(&mut self, cx: &mut Context<Self>) {
        // Custom focus navigation logic
    }
}
```

## Common Patterns

### 1. Auto-focus on Mount

```rust
impl MyDialog {
    fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Focus when created
        focus_handle.focus(cx);

        Self { focus_handle }
    }
}
```

### 2. Focus Trap (Modal)

```rust
impl Modal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, cx| {
                if event.key == Key::Tab {
                    // Keep focus within modal
                    this.focus_next_in_modal(cx);
                    cx.stop_propagation();
                }
            }))
            .child(self.render_content())
    }
}
```

### 3. Conditional Focus

```rust
impl Searchable {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .when(self.search_active, |el| {
                el.on_mount(cx.listener(|this, _, cx| {
                    this.focus_handle.focus(cx);
                }))
            })
            .child(self.search_input())
    }
}
```

## Best Practices

### ✅ Track Focus on Interactive Elements

```rust
// ✅ Good: Track focus for keyboard interaction
input()
    .track_focus(&self.focus_handle)
    .on_action(cx.listener(Self::on_enter))
```

### ✅ Provide Visual Focus Indicators

Draw the framework's ring so custom controls match `Button` and `Input`:

```rust
use gpui_kit::component::ThemeStyled as _;

let is_focused = self.focus_handle.is_focused(window);

div()
    .track_focus(&self.focus_handle)
    .when(is_focused, |el| el.focus_ring_style(window, cx))
```

The ring is painted 3px outside the element, so an ancestor with
`overflow_hidden()` clips it; leave it room or draw the ring inside the element.

### ❌ Don't: Forget to Track Focus

```rust
// ❌ Bad: No track_focus, keyboard navigation won't work
div()
    .on_action(cx.listener(Self::on_enter))
```


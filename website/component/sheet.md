---
title: Sheet
description: A sliding panel that appears from the edges of the screen for displaying content.
---

# Sheet

A Sheet (also known as a sidebar or slide-out panel) is a navigation component that slides in from the edges of the screen. It provides additional space for content without taking up the main view, and can be used for navigation menus, forms, or any supplementary content.

## Import

```rust
use gpui_kit::component::WindowExt;
use gpui_kit::component::Placement;
```

## Usage

### Setup application root view for display of sheets

You need to set up your application's root view to render the sheet layer. This is typically done in your main application struct's render method.

The [Root::render_sheet_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_sheet_layer) function handles rendering any active modals on top of your app content.

```rust
use gpui_kit::component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            // Render the sheet layer on top of the app content
            .children(sheet_layer)
    }
}
```

### Basic Sheet

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Navigation")
        .child("Sheet content goes here")
})
```

### Sheet with Placement

```rust
// Left sheet (default)
window.open_sheet_at(Placement::Left, cx, |sheet, _, _| {
    sheet.title("Left Sheet")
})

// Right sheet
window.open_sheet_at(Placement::Right, cx, |sheet, _, _| {
    sheet.title("Right Sheet")
})

// Top sheet
window.open_sheet_at(Placement::Top, cx, |sheet, _, _| {
    sheet.title("Top Sheet")
})

// Bottom sheet
window.open_sheet_at(Placement::Bottom, cx, |sheet, _, _| {
    sheet.title("Bottom Sheet")
})
```

### Sheet with Custom Size

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Wide Sheet")
        .size(px(500.))  // Custom width for left/right, height for top/bottom
        .child("This sheet is 500px wide")
})
```

### Sheet with Form Content

```rust
let input = cx.new(|cx| InputState::new(window, cx));
let date = cx.new(|cx| DatePickerState::new(window, cx));

window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("User Profile")
        .child(
            v_flex()
                .gap_4()
                .child("Enter your information:")
                .child(Input::new(&input).placeholder("Full Name"))
                .child(DatePicker::new(&date).placeholder("Date of Birth"))
        )
        .footer(
            h_flex()
                .gap_3()
                .child(Button::new("save").primary().label("Save"))
                .child(Button::new("cancel").label("Cancel"))
        )
})
```

### Overlay Options

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Settings")
        .overlay(true)              // Show overlay background (default: true)
        .overlay_closable(true)     // Click overlay to close (default: true)
        .child("Sheet settings content")
})

// No overlay
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Side Panel")
        .overlay(false)             // No overlay background
        .child("This sheet has no overlay")
})
```

### Resizable Sheet

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Resizable Panel")
        .resizable(true)            // Allow user to resize (default: true)
        .size(px(300.))
        .child("You can resize this sheet by dragging the edge")
})
```

### Custom Margin and Positioning

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Below Title Bar")
        .margin_top(px(32.))        // Space for window title bar
        .child("This sheet appears below the title bar")
})
```

### Sheet with List

```rust
let delegate = ListDelegate::new(items);
let list = cx.new(|cx| List::new(delegate, window, cx));

window.open_sheet_at(Placement::Left, cx, |sheet, _, _| {
    sheet
        .title("File Explorer")
        .size(px(400.))
        .child(
            div()
                .border_1()
                .border_color(cx.theme().border)
                .rounded(cx.theme().radius)
                .size_full()
                .child(list.clone())
        )
})
```

### Close Event Handling

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Sheet with Handler")
        .child("This sheet has a custom close handler")
        .on_close(|_, window, cx| {
            window.push_notification("Sheet was closed", cx);
        })
})
```

### Navigation Sheet

```rust
window.open_sheet_at(Placement::Left, cx, |sheet, _, _| {
    sheet
        .title("Navigation")
        .size(px(280.))
        .child(
            v_flex()
                .gap_2()
                .child(Button::new("home").ghost().label("Home").w_full())
                .child(Button::new("profile").ghost().label("Profile").w_full())
                .child(Button::new("settings").ghost().label("Settings").w_full())
                .child(Button::new("logout").ghost().label("Logout").w_full())
        )
})
```

### Custom Styling

```rust
window.open_sheet(cx, |sheet, _, cx| {
    sheet
        .title("Styled Sheet")
        .bg(cx.theme().accent)
        .text_color(cx.theme().accent_foreground)
        .border_color(cx.theme().primary)
        .child("Custom styled sheet content")
})
```

### Programmatic Close

```rust
// Close sheet from inside
Button::new("close")
    .label("Close Sheet")
    .on_click(|_, window, cx| {
        window.close_sheet(cx);
    })

// Close sheet from outside
window.close_sheet(cx);
```

## API Reference

### Window Extensions

| Method                             | Description                               |
| ---------------------------------- | ----------------------------------------- |
| `open_sheet(cx, fn)`               | Open sheet with default placement (Right) |
| `open_sheet_at(placement, cx, fn)` | Open sheet at specific placement          |
| `close_sheet(cx)`                  | Close current sheet                       |

### Sheet Builder

| Method                   | Description                             |
| ------------------------ | --------------------------------------- |
| `title(str)`             | Set sheet title                         |
| `child(el)`              | Add content to sheet body               |
| `footer(el)`             | Set footer content                      |
| `size(px)`               | Set sheet size (width or height)        |
| `margin_top(px)`         | Set top margin (for title bars)         |
| `resizable(bool)`        | Allow resizing (default: true)          |
| `overlay(bool)`          | Show overlay background (default: true) |
| `overlay_closable(bool)` | Click overlay to close (default: true)  |
| `on_close(fn)`           | Close event callback                    |

### Placement Options

| Value               | Description                         |
| ------------------- | ----------------------------------- |
| `Placement::Left`   | Slides in from left edge            |
| `Placement::Right`  | Slides in from right edge (default) |
| `Placement::Top`    | Slides in from top edge             |
| `Placement::Bottom` | Slides in from bottom edge          |

### Styling Methods

| Method                | Description              |
| --------------------- | ------------------------ |
| `bg(color)`           | Set background color     |
| `text_color(color)`   | Set text color           |
| `border_color(color)` | Set border color         |
| `px_*()/py_*()`       | Custom padding           |
| `gap_*()`             | Spacing between children |

## Examples

### Settings Panel

```rust
window.open_sheet_at(Placement::Right, cx, |sheet, _, _| {
    sheet
        .title("Settings")
        .size(px(350.))
        .child(
            v_flex()
                .gap_4()
                .child("Appearance")
                .child(Checkbox::new("dark-mode").label("Dark Mode"))
                .child(Checkbox::new("animations").label("Enable Animations"))
                .child("Notifications")
                .child(Checkbox::new("push-notifications").label("Push Notifications"))
        )
        .footer(
            h_flex()
                .justify_end()
                .gap_2()
                .child(Button::new("apply").primary().label("Apply"))
                .child(Button::new("cancel").label("Cancel"))
        )
})
```

### File Browser

```rust
window.open_sheet_at(Placement::Left, cx, |sheet, _, _| {
    sheet
        .title("Files")
        .size(px(300.))
        .child(
            v_flex()
                .size_full()
                .child(
                    h_flex()
                        .gap_2()
                        .p_2()
                        .child(Button::new("new-folder").small().icon(IconName::FolderPlus))
                        .child(Button::new("upload").small().icon(IconName::Upload))
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(file_tree_list)
                )
        )
})
```

### Help Panel

```rust
window.open_sheet_at(Placement::Bottom, cx, |sheet, _, _| {
    sheet
        .title("Help & Documentation")
        .size(px(200.))
        .child(
            h_flex()
                .gap_4()
                .child("Keyboard Shortcuts")
                .child(Kbd::new("⌘").child("K"))
                .child("Search")
                .child(Kbd::new("⌘").child("P"))
                .child("Command Palette")
        )
})
```

## Best Practices

1. **Appropriate Placement**: Use left/right for navigation, top/bottom for temporary content
2. **Consistent Sizing**: Maintain consistent sheet sizes across your application
3. **Clear Headers**: Always provide descriptive titles
4. **Close Options**: Provide multiple ways to close (ESC, overlay click, close button)
5. **Content Organization**: Use proper spacing and grouping for sheet content
6. **Responsive Design**: Consider sheet behavior on smaller screens
7. **Performance**: Lazy load sheet content when possible for better performance

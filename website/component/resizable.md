---
title: Resizable
description: A flexible panel layout system with draggable resize handles and adjustable panels.
---

# Resizable

The resizable component system provides a flexible way to create layouts with resizable panels. It supports both horizontal and vertical resizing, nested layouts, size constraints, and drag handles. Perfect for creating paned interfaces, split views, and adjustable dashboards.

## Import

```rust
use gpui_kit::component::resizable::{
    h_resizable, v_resizable, resizable_panel,
    ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent
};
```

## Usage

Use `h_resizable` to create a horizontal layout, `v_resizable` to create a vertical layout.

The first argument is the `id` for this [ResizablePanelGroup].

:::tip
In GPUI, the `id` must be unique within the layout scope (The nearest parent has presents `id`).
:::

```rust
h_resizable("my-layout")
    .on_resize(|state, window, cx| {
        // Handle resize event
        // You can read the panel sizes from the state.
        let state = state.read(cx);
        let sizes = state.sizes();
    })
    .child(
        // Use resizable_panel() to create a sized panel.
        resizable_panel()
            .size(px(200.))
            .child("Left Panel")
    )
    .child(
        // Or you can just add AnyElement without a size.
        div()
            .child("Right Panel")
            .into_any_element()
    )
```

The `v_resizable` component is used to create a vertical layout.

```rust
v_resizable("vertical-layout")
    .child(
        resizable_panel()
            .size(px(100.))
            .child("Top Panel")
    )
    .child(
        div()
            .child("Bottom Panel")
            .into_any_element()
    )
```

### Panel Size Constraints

```rust
resizable_panel()
    .size(px(200.))                    // Initial size
    .size_range(px(150.)..px(400.))    // Min and max size
    .child("Constrained Panel")
```

### Multiple Panels

```rust
h_resizable("multi-panel", state)
    .child(
        resizable_panel()
            .size(px(200.))
            .size_range(px(150.)..px(300.))
            .child("Left Panel")
    )
    .child(
        resizable_panel()
            .child("Center Panel")
    )
    .child(
        resizable_panel()
            .size(px(250.))
            .child("Right Panel")
    )
```

### Nested Layouts

```rust
v_resizable("main-layout", window, cx)
    .child(
        resizable_panel()
            .size(px(300.))
            .child(
                h_resizable("nested-layout", window, cx)
                    .child(
                        resizable_panel()
                            .size(px(200.))
                            .child("Top Left")
                    )
                    .child(
                        resizable_panel()
                            .child("Top Right")
                    )
            )
    )
    .child(
        resizable_panel()
            .child("Bottom Panel")
    )
```

### Nested Panel Groups

```rust
h_resizable("outer", window, cx)
    .child(
        resizable_panel()
            .size(px(200.))
            .child("Left Panel")
    )
    .group(
        v_resizable("inner", window, cx)
            .child(
                resizable_panel()
                    .size(px(150.))
                    .child("Top Right")
            )
            .child(
                resizable_panel()
                    .child("Bottom Right")
            )
    )
```

### Conditional Panel Visibility

```rust
resizable_panel()
    .visible(self.show_sidebar)
    .size(px(250.))
    .child("Sidebar Content")
```

### Panel with Size Limits

```rust
// Panel with minimum size only
resizable_panel()
    .size_range(px(100.)..Pixels::MAX)
    .child("Flexible Panel")

// Panel with both min and max
resizable_panel()
    .size_range(px(200.)..px(500.))
    .child("Constrained Panel")

// Panel with exact constraints
resizable_panel()
    .size(px(300.))
    .size_range(px(300.)..px(300.))  // Fixed size
    .child("Fixed Panel")
```

## Examples

### File Explorer Layout

```rust
struct FileExplorer {
    show_sidebar: bool,
}

impl Render for FileExplorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_resizable("file-explorer", window, cx)
            .child(
                resizable_panel()
                    .visible(self.show_sidebar)
                    .size(px(250.))
                    .size_range(px(200.)..px(400.))
                    .child(
                        v_flex()
                            .p_4()
                            .child("📁 Folders")
                            .child("• Documents")
                            .child("• Pictures")
                            .child("• Downloads")
                    )
            )
            .child(
                v_flex()
                    .p_4()
                    .child("📄 Files")
                    .child("file1.txt")
                    .child("file2.pdf")
                    .child("image.png")
                    .into_any_element()
            )
    }
}
```

### IDE Layout

```rust
struct IDELayout {
    main_state: Entity<ResizableState>,
    sidebar_state: Entity<ResizableState>,
    bottom_state: Entity<ResizableState>,
}

impl Render for IDELayout {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_resizable("ide-main", self.main_state.clone())
            .child(
                resizable_panel()
                    .size(px(300.))
                    .size_range(px(200.)..px(500.))
                    .child(
                        v_resizable("sidebar", self.sidebar_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(200.))
                                    .child("File Explorer")
                            )
                            .child(
                                resizable_panel()
                                    .child("Outline")
                            )
                    )
            )
            .child(
                resizable_panel()
                    .child(
                        v_resizable("editor-area", self.bottom_state.clone())
                            .child(
                                resizable_panel()
                                    .child("Code Editor")
                            )
                            .child(
                                resizable_panel()
                                    .size(px(150.))
                                    .size_range(px(100.)..px(300.))
                                    .child("Terminal / Output")
                            )
                    )
            )
    }
}
```

### Dashboard with Widgets

```rust
struct Dashboard {
    layout_state: Entity<ResizableState>,
    widget_state: Entity<ResizableState>,
}

impl Render for Dashboard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_resizable("dashboard", self.layout_state.clone())
            .child(
                resizable_panel()
                    .size(px(120.))
                    .child("Header / Navigation")
            )
            .child(
                resizable_panel()
                    .child(
                        h_resizable("widgets", self.widget_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(300.))
                                    .child("Chart Widget")
                            )
                            .child(
                                resizable_panel()
                                    .child("Data Table")
                            )
                            .child(
                                resizable_panel()
                                    .size(px(250.))
                                    .child("Stats Panel")
                            )
                    )
            )
            .child(
                resizable_panel()
                    .size(px(60.))
                    .child("Footer")
            )
    }
}
```

### Settings Panel

```rust
struct SettingsPanel {
    settings_state: Entity<ResizableState>,
}

impl SettingsPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        let settings_state = ResizableState::new(cx);

        // Listen for resize events to save layout preferences
        cx.subscribe(&settings_state, |this, _, event: &ResizablePanelEvent, cx| {
            match event {
                ResizablePanelEvent::Resized => {
                    this.save_layout_preferences(cx);
                }
            }
        });

        Self { settings_state }
    }

    fn save_layout_preferences(&self, cx: &mut Context<Self>) {
        let sizes = self.settings_state.read(cx).sizes();
        // Save to preferences
        println!("Saving layout: {:?}", sizes);
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_resizable("settings", self.settings_state.clone())
            .child(
                resizable_panel()
                    .size(px(200.))
                    .size_range(px(150.)..px(300.))
                    .child(
                        v_flex()
                            .gap_2()
                            .p_4()
                            .child("Categories")
                            .child("• General")
                            .child("• Appearance")
                            .child("• Advanced")
                    )
            )
            .child(
                resizable_panel()
                    .child(
                        div()
                            .p_6()
                            .child("Settings Content Area")
                    )
            )
    }
}
```

## Best Practices

1. **State Management**: Use separate ResizableState for independent layouts
2. **Size Constraints**: Always set reasonable min/max sizes for panels
3. **Event Handling**: Subscribe to ResizablePanelEvent for layout persistence
4. **Nested Layouts**: Use `.group()` method for clean nested structures
5. **Performance**: Avoid excessive nesting for better performance
6. **User Experience**: Provide adequate handle padding for easier interaction

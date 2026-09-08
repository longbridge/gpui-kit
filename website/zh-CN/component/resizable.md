---
title: Resizable
description: 具有拖拽分隔条与尺寸约束的可调整面板布局系统。
---

# Resizable

Resizable 组件系统用于构建可拖拽调整大小的面板布局，支持横向与纵向分割、嵌套布局、尺寸限制和拖拽句柄，适合实现 IDE、仪表盘或分栏视图。

## 导入

```rust
use gpui_kit::component::resizable::{
    h_resizable, v_resizable, resizable_panel,
    ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent
};
```

## 用法

使用 `h_resizable` 创建横向布局，使用 `v_resizable` 创建纵向布局。

第一个参数是 [ResizablePanelGroup] 的 `id`。

:::tip
在 GPUI 中，`id` 在当前布局作用域内必须唯一。
:::

```rust
h_resizable("my-layout")
    .on_resize(|state, window, cx| {
        let state = state.read(cx);
        let sizes = state.sizes();
    })
    .child(
        resizable_panel()
            .size(px(200.))
            .child("Left Panel")
    )
    .child(
        div()
            .child("Right Panel")
            .into_any_element()
    )
```

纵向布局写法类似：

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

### 面板尺寸约束

```rust
resizable_panel()
    .size(px(200.))
    .size_range(px(150.)..px(400.))
    .child("Constrained Panel")
```

### 多面板布局

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

### 嵌套布局

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

### 嵌套面板组

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

### 条件显示面板

```rust
resizable_panel()
    .visible(self.show_sidebar)
    .size(px(250.))
    .child("Sidebar Content")
```

### 带尺寸上下限的面板

```rust
resizable_panel()
    .size_range(px(100.)..Pixels::MAX)
    .child("Flexible Panel")

resizable_panel()
    .size_range(px(200.)..px(500.))
    .child("Constrained Panel")

resizable_panel()
    .size(px(300.))
    .size_range(px(300.)..px(300.))
    .child("Fixed Panel")
```

## 示例

### 文件浏览器布局

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

### IDE 布局

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

### 仪表盘布局

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

### 设置面板

```rust
struct SettingsPanel {
    settings_state: Entity<ResizableState>,
}

impl SettingsPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        let settings_state = ResizableState::new(cx);

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

## 最佳实践

1. 为独立布局使用各自的 `ResizableState`。
2. 始终为面板设置合理的最小和最大尺寸。
3. 通过订阅 `ResizablePanelEvent` 持久化用户布局。
4. 使用 `.group()` 构建清晰的嵌套结构。
5. 避免无意义的深层嵌套以减少复杂度。
6. 为拖拽句柄保留足够交互空间，提升体验。

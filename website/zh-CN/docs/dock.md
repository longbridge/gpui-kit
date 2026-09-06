---
title: Dock
description: 支持标签页、分割面板、边缘 Dock 与状态持久化的生产级工作区布局。
order: -6
example: dock
---

# Dock

Dock 用可拖动标签组、嵌套分割和可收起的左、右、底部 Dock 构建应用工作区。它是 Longbridge 在商业产品中长期使用的布局基础，而不是一个脱离实际项目的 UI Demo。

`gpui-base` 负责数据模型、布局计算和拖放行为，`gpui-component` 提供完整控件与统一视觉。需要直接用于真实应用的 Dock 时，请使用 `gpui_kit::component::dock`。

如果你需要了解与渲染器无关的架构或实现自定义渲染器，请阅读英文版 [Dock — gpui-base](/base/dock)。

## 创建 DockArea

通过 `DockSkin` 创建 DockArea。如果之后需要调整外观，请保留返回的 skin。

```rust
use gpui_kit::component::dock::{DockArea, DockSkin};

struct Workspace {
    dock_area: Entity<DockArea>,
    dock_skin: Rc<DockSkin>,
}

impl Workspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (dock_area, dock_skin) =
            DockSkin::dock_area("main-dock", Some(1), window, cx);

        Self { dock_area, dock_skin }
    }
}
```

可选的 version 属于你的持久化布局 schema。当应用无法再恢复旧布局时再增加它。

## 定义 Panel

带样式的 Dock Panel 通过 `BasePanel` 提供身份与持久化能力，通过 `Panel` 提供标题、标签页和工具栏表现。

```rust
use gpui_kit::component::dock::{BasePanel, Panel, PanelEvent};

struct FilesPanel {
    focus_handle: FocusHandle,
}

impl EventEmitter<PanelEvent> for FilesPanel {}

impl Focusable for FilesPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for FilesPanel {
    fn panel_name(&self) -> &'static str {
        "FilesPanel"
    }
}

impl Panel for FilesPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Files"
    }
}

impl Render for FilesPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_3().child("Project files")
    }
}
```

请用 `panel_handle` 包装带样式的 Panel。这样 base Dock 通过与渲染无关的 handle 保存 Panel 时，仍会保留 `gpui-component` 的完整面板外观。

## 描述初始布局

`DockLayout` 是纯数据：可以在 Window 存在之前组合，也可以序列化、比较或从应用状态生成。标签组与分割布局可以任意嵌套。

```rust
use gpui_kit::component::dock::{DockLayout, panel_handle};

let files = cx.new(|cx| FilesPanel {
    focus_handle: cx.focus_handle(),
});
let editor = cx.new(|cx| EditorPanel::new(cx));

let layout = DockLayout::h_split()
    .child(
        DockLayout::tabs().panel_view(panel_handle(files), cx),
        Some(px(240.)),
    )
    .child(
        DockLayout::tabs().panel_view(panel_handle(editor), cx),
        None,
    );

self.dock_area.update(cx, |area, cx| {
    area.set_center(layout, window, cx);
});
```

用 `h_split()` 和 `v_split()` 创建横向与纵向分割，用 `tabs()` 创建标签组，用 `tiles()` 创建可自由定位的画布。分割尺寸为 `None` 时会填满剩余空间。

DockArea 还通过 `DockPlacement` 支持左、右和底部区域。运行时可以添加、移除、激活、最大化或移动 Panel；用户操作会触发 `DockEvent`，其中 `LayoutChanged` 可用于持久化。

## 恢复与持久化

将整个工作区导出为 `DockAreaState`，通过 Serde 保存，并在下次启动时恢复。

```rust
use gpui_kit::component::dock::DockAreaState;

// 保存。
let state = self.dock_area.read(cx).dump(cx);
let json = serde_json::to_string_pretty(&state)?;

// 恢复。
let state: DockAreaState = serde_json::from_str(&json)?;
self.dock_area.update(cx, |area, cx| {
    area.load(state, window, cx)
})?;
```

应用初始化时需要注册每一种可恢复的 Panel。注册的工厂通过 `panel_handle` 重新创建带样式的 Panel。

```rust
register_panel(cx, "FilesPanel", |state, window, cx| {
    let panel = cx.new(|cx| FilesPanel::from_state(state, window, cx));
    panel_handle(panel)
});
```

Dock 状态兼容旧版本保存的布局。对于已经从应用移除的 Panel 或主动调整的 schema，仍应准备合理的默认布局作为回退。

## 调整工作区样式

`DockSkin` 将渲染决策与布局引擎分离。你可以在不改变 Dock 行为的情况下配置常用面板外观：

```rust
self.dock_skin.set_panel_style(PanelStyle::default(), cx);
self.dock_skin.set_toggle_button_visible(true, cx);
self.dock_skin
    .set_tiles_scrollbar_mode(Some(ScrollbarMode::Auto), cx);
```

如果需要完全不同的视觉，可以实现 `gpui-base` 的渲染器 traits；同一份布局数据和操作逻辑仍然可以复用。

## 可运行示例

仓库内提供了包含边缘 Dock、运行时 Panel 操作、布局持久化与键盘操作的完整工作区：

```sh
cargo run -p example-dock
```

完整实现见 [`examples/dock/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/dock/src/main.rs)。

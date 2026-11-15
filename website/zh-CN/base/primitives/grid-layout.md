---
title: Grid Layout
description: 可由用户拖动和调整大小的网格，包含布局模型及其计算。
order: 12.5
---

# Grid Layout

可由用户拖动并调整大小的网格，适合仪表盘、小组件面板等排列方式。item 之间会相互推开；空间腾出后，item 会向上靠拢，并自动绕开锁定的 item。

和所有 GPUI Base 原语一样，Grid Layout 只提供行为以及实现行为所需的几何计算——每个 item 的位置、拖动、调整大小、键盘模型和布局计算——不规定视觉语言。落点占位框、调整大小的手柄、拖动中与选中 item 的样式，以及 item 归位时的动画，都由应用提供。GPUI Component 的 [GridLayout](../../component/grid-layout.md) 会根据主题提供这些外观。

## 示例

[统一的原生 Cargo 入口](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) 会从 [共享的 showcase 实现](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs) 中选择这个原语。上方的 WASM 预览编译的也是同一份 showcase。

```bash
cargo run -p gpui-base-examples -- grid-layout
```

## 引入

```rust
use gpui_kit::base::{GridArea, GridItem, GridLayout, GridLayoutEvent, GridLayoutState, grid_drag_handle};
```

## 结构与 API

- `GridLayoutState` 持有布局及其整理规则：`cols(n)`、`compaction(Option<Axis>)`、`prevent_collision(bool)`、`draggable(bool)`、`resizable(bool)`，以及初始的 `default_areas(...)`。
- `GridArea` 是一个 item 的位置，以从零开始的单元格计：`col`、`row`、`col_span`、`row_span`，调整大小的限制 `min_span` / `max_span`，以及 `locked`。应用持久化的就是这条记录。
- `GridLayout` 是元素本身，它把每个 `GridItem` 放进 id 相同的 area。用 `row_height(rems(..))` 设置行高，用 GPUI 自带的 `gap`、`gap_x` 和 `gap_y` 设置间距，与 CSS grid 一致。
- `GridItem` 是一个 item，实现了 `Styled` 和 `ParentElement`，并且与普通元素一样支持 `.when(..)` 和 `.map(..)`。
- `grid_drag_handle(&state, id)` 返回拖动手柄元素，供设置了 `drag_handle_only(true)` 的 item 使用。

```rust
let state = cx.new(|cx| {
    GridLayoutState::new(cx).cols(12).default_areas([
        GridArea::new("revenue").col_span(8).row_span(4).min_span(4, 2),
        GridArea::new("orders").col(8).col_span(4).row_span(2),
        GridArea::new("notes").col(8).row(2).col_span(4).row_span(2).locked(true),
    ])
});

GridLayout::new("dashboard", &state)
    .row_height(rems(2.5))
    .gap_3()
    .item(GridItem::new("revenue").child(revenue_card))
    .item(GridItem::new("orders").child(orders_card))
    .item(GridItem::new("notes").child(notes_card))
```

`col` 是从零开始的单元格序号，而不是 GPUI `col_start` 所用的、从 1 开始的 CSS 网格线编号，因此布局在代码和保存的数据中读起来一致。

### 外观

Grid Layout 默认不绘制任何内容。可以通过以下方式提供外观：

- `dragging_item(|style| ..)`——叠加在拖动中的 item 上的样式；
- `selected_item(|style| ..)`——叠加在选中 item 上的样式，无论它是通过指针还是键盘选中的；
- `focused_item(|style| ..)`——在键盘操作网格时再叠加其上，即焦点环；
- `placeholder(|window, cx| ..)`——填充拖动 item 即将落入的单元格；
- `resize_handle(|window, cx| ..)`——可调整大小的 item 在用于调整大小的末尾下角显示的内容；
- `transition(Transition)`——item 移到新位置时的过渡。未设置时，item 立即移动。

### 窄窗口

`reflow(cols)` 以较少的列数显示 item，而不改变 state 中保存的布局——例如在断点以下改为单列堆叠：

```rust
GridLayout::new("dashboard", &state)
    .when(window.viewport_size().width < px(768.), |grid| grid.reflow(1))
```

显示用的 area 由 state 中的 area 推导而来：先适配列宽，再按宽布局的阅读顺序向上靠拢，锁定不生效。reflow 期间网格为只读——item 不能拖动、调整大小，也不能用键盘移动——因此不会以较窄的宽度保存任何内容。选中仍然可用。

## 状态与事件

订阅 `GridLayoutEvent::Change` 即可保存用户的排列。它携带完整的布局，每次确定的变化只发出一次——拖动或调整大小松开时、键盘命令之后，以及调用 `move_item` 或 `resize_item` 之后——不会随指针移动反复发出。`set_areas` 替换布局时不会发出该事件，因为应用已经持有它设置的布局。`GridLayoutEvent::Select` 报告用户选中的 item——点击它，或使用 `Tab`、`Shift-Tab`——每次变化只发出一次，因此跟随选中项的视图（例如设置面板）无需监听每次重绘。`set_selected_item` 不会发出该事件。`GridLayoutEvent` 标记为 `#[non_exhaustive]`，匹配时请保留通配分支。

`GridArea` 实现了 `Serialize` 和 `Deserialize`。位置之后的字段都有默认值，因此只保存了 `id`、`col`、`row`、`col_span` 和 `row_span` 的布局仍可加载。

请先设置 state 的规则，再调用 `default_areas`：area 会按此前已设置的规则整理。

## 键盘

点击 item 会选中它并让网格获得焦点。网格获得焦点后，方向键将选中的 item 移动一格，`Shift` + 方向键调整其大小，`Tab` 和 `Shift-Tab` 切换选中的 item，`Escape` 取消拖动。越过最后一个或第一个 item 时，网格不再处理 `Tab`，焦点会移出网格。这些按键绑定在 `GridLayout` 键盘上下文中的 `MoveLeft`、`MoveRight`、`MoveUp`、`MoveDown`、`ShrinkWidth`、`GrowWidth`、`ShrinkHeight`、`GrowHeight`、`SelectNextItem` 和 `SelectPreviousItem` 动作上。

## 完整 Rust 示例

可运行 showcase 使用的完整实现直接嵌入自 Rust 源码：

<<< ../../../../crates/base/examples/showcase/components/grid_layout.rs{rust}

上面的命令负责应用初始化、窗口创建和共享的 `BaseShowcase` 状态。

## 无障碍

网格是一个带标签的 group，默认标签为 "Grid layout"；可以用 `GridLayout::accessibility_label` 说明其中的内容。每个 item 都是一个 group，并报告自己是否被选中；请用 `GridItem::accessibility_label` 为它命名，通常使用其可见标题。

## 说明

请为每个 item 提供稳定的 id，不要在重新排序时改变。网格宽度与容器相同，高度由行数决定；行数可能超出窗口时，请把它放在可滚动区域中。

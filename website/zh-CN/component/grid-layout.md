---
title: GridLayout
description: 可由用户拖动和调整大小的网格布局。
---

# GridLayout

GridLayout 把 item 排列在由列和行组成的网格中，用户可以用指针或键盘移动 item、调整其大小。item 之间会相互推开；空间腾出后，item 会向上靠拢，并自动绕开锁定的 item。适合仪表盘、小组件面板等由用户决定排列方式的界面。

其行为与布局模型来自 `gpui-base` 的 [Grid Layout](../base/primitives/grid-layout.md) 原语；本模块重新导出它，并提供 `grid_layout`，按主题为其加上外观。

## 引入

```rust
use gpui_kit::component::grid_layout::{
    GridArea, GridItem, GridLayoutEvent, GridLayoutState, grid_drag_handle, grid_layout,
};
```

## 使用

用网格规则和 area 创建一个 `GridLayoutState`，再让每个 `GridItem` 通过相同的 id 对应一个 area。

```rust
let state = cx.new(|cx| {
    GridLayoutState::new(cx).default_areas([
        GridArea::new("revenue").col_span(8).row_span(4),
        GridArea::new("orders").col(8).col_span(4).row_span(2),
        GridArea::new("visitors").col(8).row(2).col_span(4).row_span(2),
    ])
});

grid_layout("dashboard", &state, cx)
    .item(GridItem::new("revenue").child(revenue_card))
    .item(GridItem::new("orders").child(orders_card))
    .item(GridItem::new("visitors").child(visitors_card))
```

`GridArea` 以从零开始的网格单元描述 item 的位置：`col` 是起始列，`row` 是顶部所在行，`col_span` 和 `row_span` 是横跨的列数和行数。没有 area 的 item 不会显示。

网格宽度与容器相同，高度由行数决定。行数可能超出窗口时，请把它放在可滚动区域中。

## 网格规则

请先设置规则，再设置 area：`default_areas` 会按此前已设置的规则整理 area。

```rust
use gpui_kit::Axis;

let state = cx.new(|cx| {
    GridLayoutState::new(cx)
        .cols(6)
        .compaction(Some(Axis::Vertical))
        .default_areas(areas)
});
```

| 规则 | 默认值 | 说明 |
| --- | --- | --- |
| `cols` | `12` | 宽度均分的列数 |
| `compaction` | `Some(Axis::Vertical)` | item 靠拢的方向；为 `None` 时 item 停留在放置的位置 |
| `prevent_collision` | `false` | 移动或调整大小会与其他 item 重叠时直接拒绝，而不是推开对方 |
| `draggable` | `true` | 是否可以移动 item |
| `resizable` | `true` | 是否可以调整 item 大小 |

每条规则都有对应的 `set_*` 方法用于之后修改，例如用 `set_compaction`，或用 `set_draggable` 在查看与编辑仪表盘之间切换。

行高和间距属于元素本身，与 CSS grid 一致：

```rust
grid_layout("dashboard", &state, cx)
    .row_height(rems(3.))
    .gap_4()
```

`grid_layout` 默认使用主题的中等间距，可用 `.gap(..)`、`.gap_x(..)` 或 `.gap_y(..)` 覆盖。行高默认为 `2.5rem`，随应用缩放变化。

## item 限制

可以限制 item 的大小范围，也可以把它锁定在原位。锁定的 item 不能移动、调整大小，也不会被推开，其他 item 会绕开它。

```rust
GridArea::new("revenue").col_span(6).row_span(4).min_span(3, 2).max_span(8, 6);

GridArea::new("notes").col(8).col_span(4).row_span(3).locked(true);
```

## 保存布局

`GridArea` 实现了 `Serialize` 和 `Deserialize`。订阅 `GridLayoutEvent::Change`，即可在用户每次修改后保存排列方式，之后用 `default_areas` 或 `set_areas` 恢复。

```rust
cx.subscribe(&state, |this, _, event: &GridLayoutEvent, cx| {
    if let GridLayoutEvent::Change(areas) = event {
        this.save_dashboard_layout(areas, cx);
    }
})
.detach();
```

拖动或调整大小结束时、键盘命令改变布局时，以及调用 `move_item` 或 `resize_item` 之后，都会发出 `Change`。一次拖动只在松开时报告一次变化，不会随指针移动反复发出。`set_areas` 不会发出 `Change`，因为应用已经持有它设置的布局。`GridLayoutEvent` 标记为 `#[non_exhaustive]`，请用 `if let` 或保留通配分支来匹配。

## 跟随选中项

`GridLayoutEvent::Select` 报告用户选中的 item——点击它，或使用 `Tab`、`Shift-Tab`——每次变化只发出一次。设置面板可以据此跟随选中项，无需监听每次重绘：

```rust
cx.subscribe(&state, |this, _, event: &GridLayoutEvent, cx| {
    if let GridLayoutEvent::Select(id) = event {
        this.inspected_tile = id.clone();
        cx.notify();
    }
})
.detach();
```

`set_selected_item` 修改选中项时不会发出该事件。

## 窄窗口

`reflow(cols)` 以较少的列数显示 item，而不改变保存的布局——例如在断点以下改为单列堆叠：

```rust
grid_layout("dashboard", &state, cx)
    .when(window.viewport_size().width < px(768.), |grid| grid.reflow(1))
```

reflow 期间网格为只读，因此不会以较窄的宽度保存任何内容；选中仍然可用。

## 拖动手柄

默认情况下，按住 item 的任意位置都能移动它。如果 item 的内容有自己的指针交互（例如表格或图表），可以只允许通过手柄移动，手柄通常是标题栏：

```rust
GridItem::new("orders")
    .drag_handle_only(true)
    .child(
        v_flex()
            .size_full()
            .child(grid_drag_handle(&state, "orders").p_2().child("订单"))
            .child(orders_table),
    )
```

## 以代码修改

```rust
state.update(cx, |state, cx| {
    state.move_item("orders", 0, 4, cx);
    state.resize_item("revenue", 6, 3, cx);
});
```

两者与指针操作一样会推开 item 并重新靠拢，返回值表示布局是否发生变化。

## 外观

`grid_layout` 会添加：拖动 item 将落下位置的虚线占位框、每个可调整大小 item 角上的调整手柄、拖动中 item 的浮起阴影，以及主题的移动动画（遵循应用的减少动态效果设置）。网格可编辑时，选中的 item 带有主色描边，便于编辑器表明设置作用于哪个 item；键盘操作网格时由焦点环取代它，两者不会同时出现。只读网格不标记选中项。这些都可以在返回的 builder 上用 `placeholder`、`resize_handle`、`dragging_item`、`selected_item`、`focused_item` 或 `transition` 替换。

`GridItem` 本身不绘制任何内容，外观由其中的内容提供，通常是卡片或 GroupBox。请为 item 设置与内容相同的圆角，使浮起阴影和描边沿用同样的轮廓。

```rust
GridItem::new("orders")
    .rounded(cx.theme().radius_lg)
    .child(orders_card)
```

## 键盘

点击 item 会选中它并让网格获得焦点。网格获得焦点后：

| 按键 | 操作 |
| --- | --- |
| `Tab` / `Shift-Tab` | 按阅读顺序选中下一个或上一个 item；越过两端时焦点移出网格 |
| 方向键 | 将选中的 item 移动一格 |
| `Shift` + 方向键 | 将选中的 item 调整一格大小 |
| `Escape` | 取消正在进行的拖动 |

命令无法执行时（例如已到达起始边缘），按键会交给外层视图处理。

## 无障碍

网格是一个带标签的 group，默认标签为 "Grid layout"；可以在网格上用 `accessibility_label` 说明其中的内容。每个 item 都是一个 group，并报告自己是否被选中。请用 `GridItem::accessibility_label` 为 item 命名，通常使用其可见标题。

---
title: GridLayout
description: 可由用户拖动和调整大小的网格布局。
---

# GridLayout

GridLayout 把 item 排列在由列和行组成的网格中，用户可以用指针或键盘移动 item、调整其大小。item 之间会相互推开；空间腾出后，item 会向上靠拢，并自动绕开锁定的 item。适合仪表盘、小组件面板等由用户决定排列方式的界面。

## 引入

```rust
use gpui_kit::component::grid_layout::{
    GridDragHandle, GridItem, GridLayout, GridLayoutEvent, GridLayoutItem, GridLayoutState,
};
```

## 使用

用网格规则和 item 创建一个 `GridLayoutState`，再让每个 `GridLayoutItem` 通过相同的 id 对应一条布局记录。

```rust
let state = cx.new(|cx| {
    GridLayoutState::new(cx).with_items([
        GridItem::new("revenue", 0, 0, 8, 4),
        GridItem::new("orders", 8, 0, 4, 2),
        GridItem::new("visitors", 8, 2, 4, 2),
    ])
});

GridLayout::new("dashboard", &state)
    .item(GridLayoutItem::new("revenue").child(revenue_card))
    .item(GridLayoutItem::new("orders").child(orders_card))
    .item(GridLayoutItem::new("visitors").child(visitors_card))
```

`GridItem` 以网格单元描述 item 的位置：`x` 是起始列，`y` 是顶部所在行，`w` 和 `h` 是横跨的列数和行数。没有布局记录的 item 不会显示。

网格宽度与容器相同，高度由行数决定。行数可能超出窗口时，请把它放在可滚动区域中。

## 网格规则

请先设置规则，再设置 item：`with_items` 会按此前已设置的规则整理 item。

```rust
use gpui_kit::{Axis, px, rems};

let state = cx.new(|cx| {
    GridLayoutState::new(cx)
        .with_columns(6)
        .with_row_height(rems(3.))
        .with_gap(px(8.))
        .with_compaction(Some(Axis::Vertical))
        .with_items(items)
});
```

| 规则 | 默认值 | 说明 |
| --- | --- | --- |
| `with_columns` | `12` | 宽度均分的列数 |
| `with_row_height` | `2.5rem` | 单行高度，随应用缩放变化 |
| `with_gap` | 主题间距 `md` | 相邻 item 之间的间距 |
| `with_compaction` | `Some(Axis::Vertical)` | item 靠拢的方向；为 `None` 时 item 停留在放置的位置 |
| `with_prevent_collision` | `false` | 移动或调整大小会与其他 item 重叠时直接拒绝，而不是推开对方 |
| `with_draggable` | `true` | 是否可以移动 item |
| `with_resizable` | `true` | 是否可以调整 item 大小 |

每条规则都有对应的 `set_*` 方法用于之后修改，例如用 `set_compaction`，或用 `set_draggable` 在查看与编辑仪表盘之间切换。

## item 限制

可以限制 item 的大小范围，也可以把它锁定在原位。锁定的 item 不能移动、调整大小，也不会被推开，其他 item 会绕开它。

```rust
GridItem::new("revenue", 0, 0, 6, 4)
    .with_min_size(3, 2)
    .with_max_size(8, 6);

GridItem::new("notes", 8, 0, 4, 3).with_locked(true);
```

## 保存布局

`GridItem` 实现了 `Serialize` 和 `Deserialize`。订阅 `GridLayoutEvent::Change`，即可在用户每次修改后保存排列方式，之后用 `with_items` 或 `set_items` 恢复。

```rust
cx.subscribe(&state, |this, _, event: &GridLayoutEvent, cx| {
    let GridLayoutEvent::Change(items) = event;
    this.save_dashboard_layout(items, cx);
})
.detach();
```

拖动或调整大小结束时、键盘命令改变布局时，以及调用 `move_item` 或 `resize_item` 之后，都会发出 `Change`。一次拖动只在松开时报告一次变化，不会随指针移动反复发出。`set_items` 不会发出 `Change`，因为应用已经持有它设置的布局。

## 拖动手柄

默认情况下，按住 item 的任意位置都能移动它。如果 item 的内容有自己的指针交互（例如表格或图表），可以只允许通过手柄移动，手柄通常是标题栏：

```rust
GridLayoutItem::new("orders")
    .drag_handle_only(true)
    .child(
        v_flex()
            .size_full()
            .child(GridDragHandle::new(&state, "orders").p_2().child("订单"))
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

`GridLayoutItem` 本身不绘制任何内容，外观由其中的内容提供，通常是卡片或 GroupBox。请为 item 设置与内容相同的圆角，使拖动占位框和焦点环沿用同样的轮廓。

```rust
GridLayoutItem::new("orders")
    .rounded(cx.theme().radius_lg)
    .child(orders_card)
```

拖动时，item 跟随指针显示在其他 item 上方，虚线占位框标出它将落下的位置。其他 item 会平滑移到新的位置；动画遵循应用的减少动态效果设置。

## 键盘

点击 item 会选中它并让网格获得焦点。网格获得焦点后：

| 按键 | 操作 |
| --- | --- |
| `Tab` / `Shift-Tab` | 按阅读顺序选中下一个或上一个 item |
| 方向键 | 将选中的 item 移动一格 |
| `Shift` + 方向键 | 将选中的 item 调整一格大小 |
| `Escape` | 取消正在进行的拖动 |

命令无法执行时（例如已到达起始边缘），按键会交给外层视图处理。

## 无障碍

网格是一个带标签的 group，默认标签为 "Grid layout"；可以用 `GridLayout::accessibility_label` 说明其中的内容。每个 item 都是一个 group，并报告自己是否被选中。请用 `GridLayoutItem::accessibility_label` 为 item 命名，通常使用其可见标题。

---
title: Carousel
description: 用于浏览相关内容的可组合 Carousel 组件。
---

# Carousel

Carousel 在可吸附的 viewport 中展示一个或多个相关 item，支持横向和纵向布局、键盘导航、指针与触控板手势、循环以及受控选中项。

## 引入

```rust
use gpui_kit::Axis;
use gpui_kit::component::carousel::{
    Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext,
    CarouselPagination, CarouselPaginationItem, CarouselPrevious, CarouselState,
};
```

## 使用

为内容创建一个 `CarouselState`，并将它传给所有 Carousel 部件。

```rust
let state = cx.new(|_| CarouselState::new(3));

Carousel::new("projects-carousel", &state)
    .child(
        CarouselContent::new(&state)
            .child(CarouselItem::new("project-1", 0, &state).child("项目一"))
            .child(CarouselItem::new("project-2", 1, &state).child("项目二"))
            .child(CarouselItem::new("project-3", 2, &state).child("项目三")),
    )
    .child(CarouselPrevious::new(&state))
    .child(CarouselNext::new(&state))
```

`CarouselContent` 管理 viewport 与吸附布局，`CarouselItem` 标识一个逻辑 slide。到达对应边界时，上一项和下一项按钮会自动禁用。

state 的 item 数量应与直接 `CarouselItem` 子元素的数量一致。一个 state 及其 scroll handle 只服务一个 viewport。

## 组合结构

Carousel 由一个内容 viewport、其中的 item 和可选控制按钮组成：

```text
Carousel
├── CarouselContent
│   ├── CarouselItem
│   └── CarouselItem
├── CarouselPrevious
└── CarouselNext
```

可以在 `Carousel` 根节点上使用 `.w_full().max_w_96()` 约束整个 Carousel；需要单独设置 viewport 的宽度或高度时，可以直接设置 `CarouselContent` 的样式。`track_style` 仅用于间距等内部 track 调整。根节点会把常规子元素按列排布并留出 16px 间距，因此放在内容后面的 `CarouselPagination` 会自然与内容拉开；需要其他排布时直接在根节点上覆盖样式。

## 每屏多个 item

`CarouselItem` 实现了 `Styled`。设置 flex basis 可以在 viewport 中同时显示多个 item；再通过 `CarouselContent::track_style` 设置负的起始 margin，并为每个 item 设置数值相同的起始 padding，即可调整它们之间的间距。这与 shadcn/ui 采用的成对间距模型一致。

```rust
use gpui_kit::{ParentElement as _, StyleRefinement, Styled as _, relative};

let state = cx.new(|_| CarouselState::new(6));

CarouselContent::new(&state)
    .track_style(StyleRefinement::default().ml_neg_1())
    .children((0..6).map(|index| {
        CarouselItem::new(("project", index), index, &state)
            .flex_basis(relative(1. / 3.))
            .pl_1()
            .child(format!("项目 {}", index + 1))
    }))
```

flex basis 控制的是 item 几何尺寸，与按钮等控件使用的语义 `Size` 相互独立。

横向 Carousel 默认在 content track 上使用 `.ml_neg_4()`，在 item 上使用 `.pl_4()`；纵向 Carousel 使用对应的 `.mt_neg_4()` 与 `.pt_4()`。覆盖间距时应同步修改两侧，并使用相同的 spacing scale，这样首个 item 会继续与 viewport 对齐，同时改变可见间距。

## 方向

创建 state 时使用 `with_axis`：

```rust
let state = cx.new(|_| {
    CarouselState::new(3).with_axis(Axis::Vertical)
});
```

横向 Carousel 使用 Left 和 Right，纵向 Carousel 使用 Up 和 Down。
纵向 `CarouselContent` 需要设置明确的高度，让每个全高 item 都有可供吸附的 viewport。

Carousel 根节点可通过 Tab 获得焦点，因此省略可选控制按钮时仍可使用键盘导航。Home 和 End 用于选择第一项和最后一项。点击 Carousel 内部或它的控制按钮同样会让它获得焦点以便键盘导航，但不会显示焦点环；焦点环只在通过键盘聚焦时出现。

## 循环

启用循环后，从最后一项继续向后会回到第一项：

```rust
let state = cx.new(|_| CarouselState::new(5).with_looping(true));
```

## 受控选中项

应用可以控制 `CarouselState`。使用 `with_selected_index` 设置初始选中项，使用 `set_selected_index` 进行程序化切换。

```rust
let state = cx.new(|_| CarouselState::new(4).with_selected_index(1));

state.update(cx, |state, cx| {
    state.set_selected_index(3, cx);
});
```

如果应用需要同步当前 slide，可以监听 `CarouselEvent::Change`：

```rust
cx.subscribe(&state, |this, _, event: &CarouselEvent, cx| {
    let CarouselEvent::Change(index) = event;
    this.selected_index = *index;
    cx.notify();
});
```

## 事件

| 事件 | 说明 |
| --- | --- |
| `CarouselEvent::Change(index)` | 用户导航选中新的内容时触发。 |

键盘导航和上一项/下一项按钮使用同一套 state 状态转换，并触发相同事件。指针和触控板手势结束时，会吸附到最近的 snap 点。鼠标滚轮每格移动一项；在边界处开始的手势会交给外层容器滚动。

## 分页指示器

分页是可选部件，不会固定一种视觉样式。使用 `CarouselPaginationItem` 组合指示器，再按需要设置每一项的样式或内容：

```rust
CarouselPagination::new().children((0..3).map(|index| {
    CarouselPaginationItem::new(("project-page", index), index, &state)
        .child((index + 1).to_string())
}))
```

`CarouselPaginationItem` 与指针、键盘和上一项/下一项导航使用同一套 selection 状态转换。

## 控件尺寸

`CarouselPrevious`、`CarouselNext` 和 `CarouselPaginationItem` 实现了 `Sizable`。需要让这些控件同步缩放时，为它们设置相同的语义尺寸：

```rust
use gpui_kit::component::{Sizable as _, Size};

CarouselPrevious::new(&state).with_size(Size::Large);
CarouselNext::new(&state).with_size(Size::Large);
```

上一项和下一项控件默认使用 `Size::Medium`，分页项默认使用 `Size::XSmall`。

## 自定义控制按钮

`CarouselPrevious` 和 `CarouselNext` 实现了 `ParentElement` 与 `Styled`。没有子元素时，它们会根据方向显示对应的箭头；添加子元素后，可以替换可见内容，同时保留自动导航和边界禁用状态。`accessibility_label` 也会同步替换控件的 tooltip。

```rust
use gpui_kit::ParentElement as _;

CarouselPrevious::new(&state)
    .accessibility_label("上一个项目")
    .child("返回");

CarouselNext::new(&state)
    .accessibility_label("下一个项目")
    .child("继续");
```

需要完全自定义控制按钮时，可以省略对应的 Carousel 部件，并使用公开 state API 组合任意控件：

```rust
use gpui_kit::ParentElement as _;
use gpui_kit::component::{Disableable as _, button::Button};

let previous_state = state.clone();
let previous_disabled = !state.read(cx).has_previous();

Button::new("projects-previous")
    .label("返回")
    .disabled(previous_disabled)
    .on_click(move |_, _, cx| {
        previous_state.update(cx, |state, cx| {
            state.select_previous(cx);
        });
    })
```

## 无障碍

Carousel 会提供带 label 的区域，每个 item 会报告自己在内容集合中的位置。当默认的“轮播”无法准确描述内容时，使用 `accessibility_label` 设置更明确的名称。

Carousel 动画会遵循应用的减少动效设置。

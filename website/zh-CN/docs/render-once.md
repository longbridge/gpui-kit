---
title: RenderOnce
description: 使用持有数据的方式构建可复用、声明式的 GPUI 组件。
order: -2.6
---

# RenderOnce

核心区别是所有权。**`RenderOnce::render(self, ...)` 消费组件值**：父级通常在自己 render 时构造一份新的、轻量的界面描述。**[`Render::render(&mut self, ...)`](./render) 借用保留的 View**，这个 View 存放在 [Entity](./entity) 中。用 `RenderOnce` 表达一次 render 所需的声明式输入；当 View 自身需要跨 render 保存状态与生命周期时，用 `Render`。`Entity<T>` 也可以只保存没有实现 `Render` 的 model 或数据；类型实现 `Render` 后，该 Entity 才是可渲染的 View。

```rust
// RenderOnce
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
// Render
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;
```

这也是为什么多数可复用的 GPUI Kit 组件优先采用 `RenderOnce`：调用方提供属性和 handler，组件返回现成的语义元素，无须给每个按钮、列表行或 Badge 创建 Entity。持有集合、订阅、异步任务或协同选中状态的文件树、聊天列表、图表工作区等功能 View，通常需要保留的 `Entity<T>` 和 `Render`。复杂 View 内部仍可以构造许多 `RenderOnce` 子组件。

```rust
use gpui_kit::*;
use gpui_kit::prelude::*;

#[derive(IntoElement)]
struct MessageRow {
    author: SharedString,
    body: SharedString,
}

impl MessageRow {
    fn new(author: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            author: author.into(),
            body: body.into(),
        }
    }
}

impl RenderOnce for MessageRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .child(div().font_semibold().child(self.author))
            .child(self.body)
    }
}
```

`#[derive(IntoElement)]` 会生成转换实现，让这个值可以直接加入 GPUI 的 [Element](./element) tree：

```rust
div().child(MessageRow::new("You", "Explain RenderOnce"))
```

这个 derive 不会立即 render 组件。它生成 `IntoElement` 实现，其 Element 类型为 `ViewElement<Self>`；GPUI 处理外层元素树时才消费并 render 这个值。单独实现 `RenderOnce` 会让值实现 `View`，但要把它直接传给 `.child(...)`，还需要 `IntoElement`；这里由 derive 提供。derive **不会**为你的类型实现 `Styled`、`ParentElement`、焦点或无障碍语义。只有实际构造这些元素能力，或另行实现相应 trait，它们才会存在。

## 持有的值与 render 生命周期

[`Render`](./render) Guide 详述保留的 View。`RenderOnce` 要求 `Self: 'static`，值式组件的实际签名是：

```rust
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
```

因为 `self` 的所有权属于 `render`，所以可以直接把字段移动到 Element tree 和 `'static` handler 中。组件值只使用一次；父级下一次 render 时再构造新值。Builder 方法可以在构造期间修改这个值；最终属性描述的是一次 render，而不是持久的可变模型。

多个字段需要移动到不同位置时，可以先解构，让所有权关系更清楚：

```rust
fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
    let MessageRow { author, body } = self;

    div()
        .child(author)
        .child(body)
}
```

这并不表示界面显示一帧后就会消失，也不表示每次屏幕刷新都要构造新值。“Once” 指同一个组件实例只使用一次：父级每次执行 `render` 才构造另一个实例。普通窗口重绘可能让未变化的子 View 再次 render，而显式的[缓存 View](./view-cache) 可跳过未变化的子树。GPUI 在这些 render 之间保留 Entity 和 keyed state，因此不能简单等同于每次屏幕刷新都重建整个应用的传统 immediate-mode。不要把 `&mut Window` 或 `&mut App` 引用保留到本次调用结束后。需要身份的重复行应从领域数据生成稳定 ID；见 [ElementId](./element_id)。

## 状态应放在组件值之外

`RenderOnce` **并不表示“完全没有状态”**。值可以携带本次 render 的 label、disabled、checked 等属性。GPUI 可通过稳定的 [ElementId](./element_id) 保留微小的交互状态，`RenderOnce` 组件也可以持有外部 `Entity<T>` handle。关键边界是：变化的应用状态必须由这个被消费的值以外的对象持久拥有。它可以是实现 `Render` 的 View、只存 model 的 Entity，或其他应用状态 owner；再把当前值、回调或 handle 传给组件。

GPUI Kit 带样式的 `Button` 和 `Checkbox` 都实现 `RenderOnce`。Button 接收 label 与点击 handler。Checkbox 接收受控的 `checked` 布尔值，并通过 `on_click` 报告请求的下一个值；owner 写入该值并调用 `cx.notify()`。Base 层提供焦点、键盘和无障碍行为。父级可以轻量地重新构造它们，同时让真正的状态只有一个 owner。

```rust
use gpui_kit::component::checkbox::Checkbox;

// 位于 owner 的 Render::render 内，self.show_hidden 和 cx 已存在：
Checkbox::new("show-hidden")
    .checked(self.show_hidden)
    .label("Show hidden files")
    .on_click(cx.listener(|this, checked, _, cx| {
        this.show_hidden = *checked;
        cx.notify();
    }))
```

```rust
use gpui_kit::component::button::{Button, ButtonVariants};

struct Editor {
    saved: bool,
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(if self.saved { "Saved" } else { "Unsaved" })
            .child(
                Button::new("save")
                    .label("Save")
                    .primary()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.saved = true;
                        cx.notify();
                    })),
            )
    }
}
```

这段代码沿用第一个示例的 `use gpui_kit::*;`。`Editor` 挂载在 `Entity<Editor>` 中；`Button::new` 建立稳定的元素 ID。按钮的名称、焦点、键盘激活和无障碍 role 由组件层与 Base 层提供。Element handler 要求 `'static`，因此子组件若捕获回调或 `Entity<T>`，必须使用持有的值。如果调用方还需要某个 handle，应先 clone，再把副本移入 handler。

文字编辑展示了边界的另一侧。`Input::new(&state)` 返回 `RenderOnce` 可视组件，但 `state` 是所属 View 保留的 `Entity<InputState>`。`InputState` 实现 `Render`，跨父级 render 保留文字、选区、焦点、编辑历史和输入行为。重新构造 `Input::new(&state)` 不会重新创建编辑器状态：

```rust
use gpui_kit::component::input::{Input, InputState};

struct SearchView {
    query: Entity<InputState>,
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.query).id("search-query")
    }
}
```

构造 `SearchView` 时创建一次 `query`，例如 `cx.new(|cx| InputState::new(window, cx))`；不要在 `render` 中重新创建。持有大量集合数据的 View 也遵循相同的所有权规则：记录、过滤条件、选中项 ID 和订阅保留在 owner 中，再从它们构造值式 row 与控件。

捕获 `Entity<T>` 会让该 Entity 在生成的 handler 存续期间保持存活。子组件操作其 owner 时，这通常符合预期。如果 handler 不应该延长目标的生命周期，应使用 `WeakEntity<T>`，并处理 `weak.update(...)` 已经无法访问目标的情况。

`RenderOnce::render` 得到 `&mut App`，而不是 `&mut Context<Self>`。因此 `RenderOnce` 组件没有自己的 Entity [Context](./context)：不能为自己使用 `cx.listener`、保留自己的 Subscription 或 Task，也不能通过 `cx.notify()` 安排自己重新 render。应传入 handler、派发 [Action](./action)，或更新真正持有状态的 `Entity`。Keyed element state 可以保留局部交互细节，但不能代替持久应用数据的 owner。

## Builder 风格的组件

持有的私有字段很适合 Builder API。GPUI Kit 组件也使用这种模式。Builder 接收并返回 `Self`，调用方可在保持组件有效的同时继续配置它：

```rust
use gpui_kit::component::ActiveTheme;

#[derive(IntoElement)]
struct StatusBadge {
    label: SharedString,
    muted: bool,
}

impl StatusBadge {
    fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            muted: false,
        }
    }

    fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

impl RenderOnce for StatusBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .rounded_full()
            .px_2()
            .text_color(cx.theme().muted_foreground)
            .when(!self.muted, |this| this.text_color(cx.theme().foreground))
            .child(self.label)
    }
}
```

这段代码也使用 `use gpui_kit::*;`。返回的 `div()` 实现了 [`Styled`](./style) 和 `ParentElement`，所以能调用 `.rounded_full()`、`.text_color()` 和 `.child()`。如果调用方需要**直接对 `StatusBadge` 调用**这些方法，就要为该类型实现 `Styled` 和／或 `ParentElement`，存储样式或子元素，再在 `render` 中应用。`IntoElement` derive 不会转发返回元素的 Fluent trait。GPUI Kit 的 `Button` 明确实现了这两个 trait。小范围调整可用 `.when(...)`、`.when_some(...)`；结构差异明显时用普通 Rust 分支。

## 选择合适的层级

| 使用 | 适用情况 |
| --- | --- |
| `RenderOnce` + `IntoElement` | 可复用组件消费本次 render 的属性与 handler；也可使用 keyed state 或外部 Entity，渲染时得到 `&mut Window` 和 `&mut App`。 |
| `Entity` 上的 [`Render`](./render) | 保留的 View 拥有变化的数据、集合、Subscription、Task 或生命周期；渲染时得到 `&mut Context<Self>`，可修改并通知它。 |
| [`Element`](./element) | 内置元素无法表达所需的 layout、prepaint、paint、hit testing 等底层阶段。 |

常见的组合方式是：`Render` View 持有状态，由它创建 `RenderOnce` 组件来描述可复用 UI，而这些组件返回内置 Element。这让多数 GPUI Kit 组件拥有精简、明确的 API，同时把复杂状态放在少数有实际意义的 owner 中。只有标准 Element API 无法表达所需渲染行为时，才需要直接实现 `Element`。

:::info
如果一个组件开始积累可变状态、Subscription 或后台 Task，应把这些生命周期移入 `Entity`，并为它实现 `Render`。把它们放在每次 render 都会被消费的值里，会破坏 `RenderOnce` 简单明确的所有权模型。
:::

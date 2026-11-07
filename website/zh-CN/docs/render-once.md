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
    action: Option<AnyElement>,
}

impl MessageRow {
    fn new(author: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            author: author.into(),
            body: body.into(),
            action: None,
        }
    }

    fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl RenderOnce for MessageRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let MessageRow { author, body, action } = self;

        div()
            .flex()
            .gap_2()
            .child(div().font_semibold().child(author))
            .child(body)
            .when_some(action, |row, action| row.child(action))
    }
}
```

`#[derive(IntoElement)]` 会生成转换实现，让这个值可以直接加入 GPUI 的 [Element](./element) tree：

```rust
div().child(MessageRow::new("You", "Explain RenderOnce"))
```

这个 derive 不会立即 render 组件。它生成 `IntoElement` 实现，其 Element 类型为 `ViewElement<Self>`；GPUI 处理外层元素树时才消费并 render 这个值。单独实现 `RenderOnce` 会让值实现 `View`，但要把它直接传给 `.child(...)`，还需要 `IntoElement`；这里由 derive 提供。derive **不会**为你的类型实现 `Styled`、`ParentElement`、焦点或无障碍语义。只有实际构造这些元素能力，或另行实现相应 trait，它们才会存在。

## 动手练习：重建组件值，保留计数

在依赖 `gpui-kit` 的应用中，用下面的完整代码替换 `src/main.rs`，然后运行 `cargo run`。`CounterLabel` 是一个 `RenderOnce` 值；`CounterView` 是持有计数的、保留在 `Entity` 中的 View。

```rust
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

#[derive(IntoElement)]
struct CounterLabel {
    count: u32,
}

impl RenderOnce for CounterLabel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child(format!("Count: {}", self.count))
    }
}

struct CounterView {
    count: u32,
}

impl Render for CounterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(CounterLabel { count: self.count })
            .child(
                Button::new("increment")
                    .primary()
                    .label("Add one")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();
                    })),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| CounterView { count: 0 })
        })
        .expect("Failed to open window");
    });
}
```

窗口初始显示 `Count: 0`。连续点击两次 **Add one**，应依次看到 `Count: 1` 和 `Count: 2`。每次调用 `CounterView::render` 都会用当前计数构造新的 `CounterLabel`，旧值已被消费；计数能继续增长，是因为同一个 `CounterView` Entity 持有它。点击回调修改 owner 后，`cx.notify()` 请求再次渲染。组件名并不表示每个显示帧恰好 render 一次；具体时机见 [Render](./render)。

如果文本一直为零，检查监听器是否写入 `this.count` 并调用 `cx.notify()`。如果每次都回到一，检查 `CounterView { count: 0 }` 是否只在窗口创建闭包中构造，而没有放在 `render` 中。如果 `.child(CounterLabel { ... })` 编译失败，检查 `#[derive(IntoElement)]` 和 `use gpui_kit::*;` 是否都在作用域内。本页后续片段分别展示不同变化；复制运行时以这个完整示例为起点。

## 从构造到调用

`MessageRow` 的 builder 接口很小：`new` 提供必需的文本，`action` 是可选的 **slot**。`AnyElement` 能保存调用方传入的不同具体元素。这个 slot 总是放在正文之后；重复调用 `.action(...)` 会替换前一个元素。仓库中的 [`Empty` 组件](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/empty.rs) 也使用有名称的可选 slot，并按固定顺序渲染。如果组件要接收多个任意子元素，可保存 `Vec<AnyElement>` 并实现 `ParentElement`；单个具名 slot 不需要该 trait。

调用方可以把现成的语义控件放进 slot，由持久的 View 保存行为。下面的 `Editor` 持有 `opened`；点击回调修改字段，并通知 GPUI 重新渲染：

```rust
// In Editor::render, where cx: &mut Context<Self> is available.
MessageRow::new("You", "Explain RenderOnce").action(
    Button::new("open-message")
        .label("Open")
        .on_click(cx.listener(|this, _, _, cx| {
            this.opened = true;
            cx.notify();
        })),
)
```

使用 `use gpui_kit::component::button::Button;` 导入按钮，并为 `Editor` 添加 `opened: bool` 字段。渲染多条消息时，每个按钮应使用由该消息的领域 ID 派生的稳定 ID（例如当 `message.id` 的类型可作为 `ElementId` 的一部分时，使用 `("open-message", message.id)`）。显示文字不能作为身份。`MessageRow` 本身没有 Entity、listener context 或回调所有权；它只消费调用方已经构造的 action。若回调本身属于自定义组件的接口，也可以在私有字段中保存持有所有权的 `'static` handler，并在 `render` 中转交给语义控件。

## 持有的值与 render 生命周期

[`Render`](./render) Guide 详述保留的 View。`RenderOnce` 要求 `Self: 'static`，值式组件的实际签名是：

```rust
fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
```

因为 `self` 的所有权属于 `render`，所以可以直接把字段移动到 Element tree 和 `'static` handler 中。组件值只使用一次；父级下一次 render 时再构造新值。Builder 方法可以在构造期间修改这个值；最终属性描述的是一次 render，而不是持久的可变模型。

多个字段需要移动到不同位置时，可以先解构，让所有权关系更清楚。上方完整的 `MessageRow::render` 就这样处理文本与 action slot。

slot 的转换发生在 builder 中，早于 `render`；`.when_some(...)` 只在它存在时添加子元素。`SharedString` 持有可复用文本，`AnyElement` 持有被类型擦除的子元素，因此两者都不会借用短生命周期的局部变量。

这并不表示界面显示一帧后就会消失，也不表示每次屏幕刷新都要构造新值。“Once” 指同一个组件实例只使用一次：父级每次执行 `render` 才构造另一个实例。普通窗口重绘可能让未变化的子 View 再次 render，而显式的[缓存 View](./view-cache) 可跳过未变化的子树。GPUI 在这些 render 之间保留 Entity 和 keyed state，因此不能简单等同于每次屏幕刷新都重建整个应用的传统 immediate-mode。不要把 `&mut Window` 或 `&mut App` 引用保留到本次调用结束后。需要身份的重复行应从领域数据生成稳定 ID；见 [ElementId](./element_id)。

## 状态应放在组件值之外

`RenderOnce` **并不表示“完全没有状态”**。值可以携带本次 render 的 label、disabled、checked 等属性。GPUI 可通过稳定的 [ElementId](./element_id) 保留微小的交互状态，`RenderOnce` 组件也可以持有外部 `Entity<T>` handle。关键边界是：变化的应用状态必须由这个被消费的值以外的对象持久拥有。它可以是实现 `Render` 的 View、只存 model 的 Entity，或其他应用状态 owner；再把当前值、回调或 handle 传给组件。

GPUI Kit 带样式的 `Button` 和 `Checkbox` 都实现 `RenderOnce`。Button 接收 label 与点击 handler。Checkbox 接收受控的 `checked` 布尔值，并通过 `on_click` 报告请求的下一个值；owner 写入该值并调用 `cx.notify()`。Base 层提供焦点、键盘和无障碍行为。父级可以轻量地重新构造它们，同时让真正的状态只有一个 owner。

```rust
use gpui_kit::component::checkbox::Checkbox;

// In the owner's Render::render, where self.show_hidden and cx exist:
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

## 常见编译错误与检查方法

| 现象 | 检查方法 |
| --- | --- |
| `.child(...)` 报告 `MessageRow` 未实现 `IntoElement` | 除了 `impl RenderOnce`，还要添加 `#[derive(IntoElement)]`，并导入 `use gpui_kit::*;`。 |
| 找不到 `when_some` 或其他链式样式方法 | 导入 `gpui_kit::prelude::*;` 以获得 `FluentBuilder`，并确认方法属于返回的 `div()`，而非自定义组件类型。 |
| slot 或 handler 中借用的局部值“存活时间不够长” | 移入持有数据的 `SharedString`、`AnyElement` 或克隆后的 `Entity` handle。GPUI handler 必须是 `'static`。 |
| 在 `RenderOnce::render` 中找不到 `cx.listener` 或 `cx.notify()` | 这里拿到的是 `&mut App`，不是 Entity 的 `Context<Self>`。如上例，在 owner View 的 `Render::render` 中创建 listener，再传入组件。 |
| 直接对 `MessageRow` 调用 `.child(...)` 失败 | derive 提供的是 `IntoElement`，不是 `ParentElement`。使用 `.action(...)` 等具名 builder，或实现 `ParentElement` 并显式保存子元素。 |

检查复制的示例时，先把类型、builder 和 `RenderOnce` 实现放在同一模块，加入上面的两条 import，并在持久 View 中将它用作子元素，然后在应用工作区运行 `cargo check -p your-app`。文中的几段 `Editor` 代码分别说明不同用法；若要合并编译，需加上文中提到的字段与导入。当前仓库的 API 可对照 [`Empty`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/empty.rs)、[`Checkbox`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/checkbox.rs) 与 [`Button`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/button/button.rs)。

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
| [`RenderOnce`](./render-once) + `IntoElement` | 可复用组件消费本次 render 的属性与 handler；也可使用 keyed state 或外部 Entity，渲染时得到 `&mut Window` 和 `&mut App`。 |
| `Entity` 上的 [`Render`](./render) | 保留的 View 拥有变化的数据、集合、Subscription、Task 或生命周期；渲染时得到 `&mut Context<Self>`，可修改并通知它。 |
| [`Element`](./element) | 内置元素无法表达所需的 layout、prepaint、paint、hit testing 等底层阶段。 |

常见的组合方式是：`Render` View 持有状态，由它创建 `RenderOnce` 组件来描述可复用 UI，而这些组件返回内置 Element。这让多数 GPUI Kit 组件拥有精简、明确的 API，同时把复杂状态放在少数有实际意义的 owner 中。只有标准 Element API 无法表达所需渲染行为时，才需要直接实现 `Element`。

:::info
如果一个组件开始积累可变状态、Subscription 或后台 Task，应把这些生命周期移入 `Entity`，并为它实现 `Render`。把它们放在每次 render 都会被消费的值里，会破坏 `RenderOnce` 简单明确的所有权模型。
:::

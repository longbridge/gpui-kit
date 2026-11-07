---
title: ElementId
description: 为 GPUI 元素提供稳定标识，并理解带 key 的状态如何跨帧保留。
order: -2.4
---

# ElementId

`ElementId` 是 GPUI 渲染树中元素的**局部 key**。GPUI 将它与带 key 的祖先元素 ID 组合，形成 `GlobalElementId`。凭借这条路径，[View 再次 render](./render) 时，GPUI 能把交互和元素状态关联到同一个逻辑元素。当元素有 role 时，这条路径也让它在[无障碍树](./accessibility)中保持节点身份。

ID 不是 [Entity] 的 handle，也不能像 HTML DOM ID 一样用来查找元素。共享的应用状态用 `Entity<T>`；元素树中的身份用 `ElementId`，例如组件内部按 key 保存的状态、组件提供的焦点或滚动行为，以及自定义 [Element] 保留的状态。

## 赋予元素 ID

在 `div()` 等交互元素上调用 `.id(...)`，会得到一个 `Stateful<Div>`：

```rust
let save = div()
    .id("save-button")
    .on_click(|_, window, cx| {
        // Handle the click.
    })
    .child("Save");
```

`Stateful<E>` wrapper 提供 GPUI 的有状态交互方法，并携带元素 ID。自定义 `Element` 可以在 `id()` 方法中返回 `Some(id)`，而不经过这个 wrapper。普通、没有 key 的 `div()` 仍可以作为布局父节点，只是不会在 ID 路径中增加一段。

字符串、整数，以及名称加整数的元组，都是常见的 `ElementId` 输入：

```rust
use gpui_kit::component::button::Button;

div().id("search")
div().id(("message", message.id))
Button::new(("delete-project", project.id)).label("Delete")
```

key 应取自对象的身份，而非显示给用户的文字。翻译后的标签、当前选中项、每次新生成的随机值，都可能在对象仍然相同的时候改变。

## GlobalElementId 与祖先路径

GPUI 从元素路径上的 ID 组成 `GlobalElementId`。只有带 key 的祖先才会增加路径段：

```text
div().id("workspace")
├── div().id("inbox")
│   └── div().id(("row", 42))  → ["workspace", "inbox", ("row", 42)]
└── div().id("archive")
    └── div().id(("row", 42))  → ["workspace", "archive", ("row", 42)]
```

两个 row 可以复用局部 ID，因为它们的带 key 祖先路径不同。图中只展示手写的 ID：由 Entity 支撑的 View 还会将其 `EntityId` 加入路径，[`RenderOnce`](./render-once) 组件则会加入类型名命名空间。这条路径属于 [Window](./window) 的渲染树；`GlobalElementId` 是 GPUI 的内部路径，无须在调用处自行构造，也不是整个进程通用的字符串。若自定义绘制 API 需要为自身的 key 取得路径，可用 `window.with_global_id(key, |global_id, window| { … })` 在回调期间创建。

实际的唯一性规则是：**在同一个最近的带 key 祖先之下，各条带 key 的后代分支需要不同的 ID**。没有 key 的容器不会开辟新命名空间：

```rust
div().id("workspace")
    .child(div().child(div().id("item")))
    .child(div().child(div().id("item"))) // Duplicate keyed path: IDs collide.
```

可以给两条分支赋予各自稳定的 ID，或让两个 item 使用不同 ID。路径重复会使保留状态和交互关联到错误的逻辑元素。

## 变化列表中的稳定 key

列表项可能插入、删除、过滤或重新排序时，每一行的 ID 应来自稳定的业务标识：

```rust
div().id("messages").children(messages.iter().map(|message| {
    div()
        .id(("message", message.id))
        .child(message.preview.clone())
}))
```

`("message", message.id)` 将 row 的用途与其他使用相同数字 ID 的控件区分开。调整顺序会改变绘制位置，但每条消息仍保持原来的 key 路径。相反，`.id(index)` 将状态绑定到**位置**：插入一行后，原来第一行的焦点、滚动、动画或其他带 key 状态，可能被另一个消息复用。只有位置本身就是身份且不会移动时，索引才合适。

如果重复的 row 中还有多个控件，先给 row 设置 key，再给子控件设置 `"edit"`、`"delete"` 等不同的局部 key。这样移动 row 时，它下面整棵带 key 的子树都会跟着移动。

### 完整示例：消息行及其控件

下面使用实际的 `div().id(...)` 和 `Button::new(id)` API。`Message::id` 是稳定的数据库 ID；`preview` 是可能变化的显示内容。每一行都为自己的控件提供命名空间：

```rust
use gpui_kit::*;
use gpui_kit::component::button::Button;

struct Message {
    id: u64,
    preview: SharedString,
}

fn message_list(messages: &[Message]) -> impl IntoElement {
    div().id("messages").children(messages.iter().map(|message| {
        div()
            .id(("message", message.id))
            .child(message.preview.clone())
            .child(Button::new("archive").label("Archive"))
    }))
}
```

消息 `42` 的按钮路径中，手写的部分是 `"messages" → ("message", 42) → "archive"`（GPUI 还可能加入 View 和组件的命名空间）。每行的按钮都能叫 `"archive"`，因为行 ID 不同。列表从 `[42, 7]` 重排为 `[7, 42]` 后，路径仍跟随对应的消息。如果消息 `42` 在某个已渲染帧中被移除，它的元素局部状态就会结束；以后重新插入会创建新状态。需要在移除期间保留的应用级选中项或草稿，应由 `Entity<T>` 或模型持有。

行 key 不会自动区分**同一行内**的兄弟控件：该行下若有两个 `Button::new("archive")`，仍会发生冲突。应给它们不同的局部 ID。消息预览或本地化标签变化时，也应继续使用相同的业务 ID；把文字当成 key 会重置 UI 身份。

### 动手练习：重排、隐藏与恢复行

在已有的 `examples/hello_world` 包中，把 `src/main.rs` 替换为下面的完整示例，运行 `cargo run -p hello_world`。先点两次 **Add to 42**，再点 **Swap rows**。记录 42 移到记录 7 下方后仍显示 `2`。点 **Hide 42**，确认该行已从画面消失，再点 **Show 42**。计数从 `0` 开始，因为这份 keyed state 曾缺席于一个已渲染帧。若计数必须在隐藏期间保留，应由应用拥有的 Entity 保存。

```rust
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::Button;
use gpui_kit::*;

struct Example {
    reversed: bool,
    show_42: bool,
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let order = if self.reversed { [7_u64, 42] } else { [42, 7] };
        let rows = order
            .into_iter()
            .filter(|id| *id != 42 || self.show_42)
            .map(|id| {
                // This call happens while the parent View renders, before the row div is drawn.
                // Give the state its own domain-derived key; the row ID below keys its subtree.
                let count = window.use_keyed_state(("row-count", id), cx, |_, _| 0_u32);
                let value = *count.read(cx);

                div()
                    .id(("row", id))
                    .h_flex()
                    .gap_2()
                    .child(format!("Record {id}: {value}"))
                    .child(
                        Button::new("increment")
                            .label(format!("Add to {id}"))
                            .on_click(move |_, _, cx| {
                                count.update(cx, |value, cx| {
                                    *value += 1;
                                    cx.notify();
                                });
                            }),
                    )
            })
            .collect::<Vec<_>>();

        div()
            .v_flex()
            .gap_2()
            .p_4()
            .child(Button::new("swap").label("Swap rows").on_click(cx.listener(
                |this, _, _, cx| {
                    this.reversed = !this.reversed;
                    cx.notify();
                },
            )))
            .child(
                Button::new("toggle-42")
                    .label(if self.show_42 { "Hide 42" } else { "Show 42" })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_42 = !this.show_42;
                        cx.notify();
                    })),
            )
            .child(div().id("rows").v_flex().gap_2().children(rows))
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example {
                reversed: false,
                show_42: true,
            })
        })
        .expect("failed to open window");
    });
}
```

行的 `("row", id)` 路径使控件在顺序变化后仍属于原记录。两个行里的 `Button::new("increment")` 可以复用局部名称，因为它们的行路径不同。计数器另用 `("row-count", id)` 作 key：它在父 View 构建行时取得，此时行的元素 ID 还没有进入路径。两个 key 都必须使用稳定的记录 ID。只有 GPUI 渲染过缺少该行的帧，隐藏才会使旧状态失效；如果在两次渲染之间连续触发隐藏和显示，旧状态仍可能保留。

## ID 如何保留状态

GPUI 会在 render 时重新构建元素。`div()` 或 `RenderOnce` 组件返回的 Rust 值是临时的；赋予 ID 不会让它变成持久的 `Entity<T>`。ID 的作用是让 GPUI 在连续帧之间重新关联元素局部状态。

`window.use_keyed_state(key, cx, init)` 使用当前带 key 的祖先路径加上 `key`。它返回 `Entity<S>`；只要连续渲染帧都使用这个 key，状态就会保留。`Window` 管理这份 keyed state；`cx` 提供应用访问（不同类型的说明见 [Context](./context)）。没有旧状态时才调用 `init`。GPUI 还会观察这份状态 Entity；状态变化时通知当前 View：

```rust
let focus_handle = window
    .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
    .read(cx)
    .clone();
```

GPUI Kit 的 `Button` 用这种方式保存焦点 handle。它的公开 ID 让每次重新构建的按钮都能找到相同的状态 key。焦点行为仍由 focus handle 承担；元素 ID 决定这个 handle 的状态归属何处。只有路径在连续帧中被访问，窗口才会保留其 keyed state；[缓存 View](./view-cache) 重用子树时会重放这些访问。单独保存的强 `Entity<S>` 句柄可以让该 Entity 在窗口状态条目消失后继续存活，但路径重新出现时仍会执行 `init`。

自定义 `Element` 的 `id()` 返回 ID 时，GPUI 会在 `request_layout`、`prepaint` 和 `paint` 中传入 `Option<&GlobalElementId>`。绘制期间，`window.with_element_state(global_id, ...)` 可以读取上一帧状态，并返回需要保存到下一帧的值：

```rust
let state = window.with_element_state(
    id.expect("this element always has an ID"),
    |previous: Option<AnimationState>, _window| {
        let state = previous.unwrap_or_default();
        (state.clone(), state)
    },
);
```

GPUI Kit 的 `ScrollBounce` 元素就采用这个模式，在 `prepaint` 中保留运动状态。`with_element_state` 是供元素作者在绘制阶段使用的 API；普通 View 应优先使用 Entity 状态或组件文档提供的 API。GPUI 按全局路径**以及状态类型**存储元素状态；元素不再参与后续渲染帧时，该状态会被释放。

:::info
`window.use_state(cx, init)` 使用调用位置作为局部 key，只要求**完整路径**唯一：如果每行已有稳定的带 key 祖先，同一个调用位置用于多行也是安全的。如果多次调用共享同一带 key 祖先路径，应改用带稳定条目 key 的 `use_keyed_state`，或为每个条目建立带 key 的命名空间。
:::

## 身份变化也会改变状态

- 改变元素 ID 或带 key 祖先的 ID，会生成新路径，并重置相应的元素状态。
- 移除元素会终止其连续帧状态的生命周期；以后重新创建时会再次初始化。
- ID 本身不会保留 View 的 `Entity<T>`；其生命周期由强 Entity owner 决定。
- 带 key 的状态属于 Window 的渲染上下文。不要依靠 `GlobalElementId` 在不同窗口之间转移状态。

## 对其他行为的影响

组件可能把 ID 用作焦点、滚动、测量或动画状态的一个输入。例如，Kit 的 `Button` 通过 `window.use_keyed_state(self.id.clone(), ...)` 取得 focus handle，而 `ScrollBounce` 用 `window.with_element_state(...)` 保留运动状态。因此，两个同时存在的控件复用同一路径可能混淆行为；路径变化可能使行为重新开始。`FocusHandle` 或 `ScrollHandle` 仍分别拥有对应的行为，改变 `ElementId` 本身也不会重置保存在别处的所有 handle。

稳定路径也关系到缓存 View。只有 Entity 和元素路径仍匹配，缓存的 Entity View 才能复用工作；命中缓存时，GPUI 会重放子树对元素状态的访问。ID 不是通用的 render 缓存：给 `div()` 加 `.id(...)` 不会让它的父级跳过 `render`（见 [View 缓存](./view-cache)）。

对无障碍而言，自定义元素同时需要 ID 和 role 才能成为节点。稳定 ID 让 GPUI 在重绘后保持该节点的身份，但不会自动提供 role、标签、键盘行为或可聚焦能力。这些属性应使用组件的无障碍 API 设置（见[无障碍](./accessibility)）。

## 排查错误的 ID 路径

1. 找出焦点、滚动位置、动画或其他状态错误转移或重置的逻辑对象。记下它稳定的业务 ID，以及上方每一个带 key 的祖先。检查兄弟元素是否拥有相同的**完整路径**，或祖先 key 是否随数据重排而变化。
2. 检查重复控件之间是否只有不带 key 的容器；这样的容器无法区分路径。可以用稳定的对象 ID 替代共享的固定文字，或者给重复的父级设置 key。不要用每次 render 新生成的随机值解决冲突，那只会把状态混用变成状态丢失。
3. 如果状态只在条目消失后重置，确认它是否曾在某个已渲染帧中缺席。如果是，长期状态应保存在 Entity 或模型中。如果条目一直存在却重置，则分别检查祖先 ID 的变化、重新创建的 View Entity，以及缓存失效。

Kit 中有一个真实例子：[DockSkin::render_resize_handle](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/dock/dock.rs)。左、右、下三个 resize handle 可能位于同一个带 key 的祖先之下。若它们都使用 `"resize-handle"`，就会得到相同的 `GlobalElementId`；Dock 的源码指出，按住一个 handle 可能触发另一个 handle 的拖动。因此实现根据稳定的方位选择 `"resize-handle-left"`、`"resize-handle-right"` 和 `"resize-handle-bottom"`。

在 UI 集成测试里，ID 是已观测元素的查询选择器，不是第二套身份系统。导入 `gpui_kit::test::TestWindowExt`；`window.find("archive")` 要求只匹配一个已观测目标，因此重复控件需要原生路径作用域：

```rust
let archive = window.within(("message", 42_u64)).find("archive");
assert!(archive.visible());
```

即使祖先本身没有被观测，`window.within(...)` 也能沿其带 key 的路径限定范围。`find` 或 `try_find` 报目标不唯一，意味着查询需要限定范围；不一定表示 GPUI 状态冲突，因为不同行下的控件可以正确地复用同一个局部 ID。除了检查快照，也应测试真实点击或焦点结果（见[测试](./test)）。

布局、prepaint 和 paint 生命周期见 [Element](./element)；需要在元素离开渲染树后继续存在的状态见 [Entity](./entity)。

[Element]: ./element.md
[Entity]: ./entity.md

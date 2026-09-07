---
title: Nav Stack
description: 支持 push、pop、forward 与 replace 的视图导航栈，过渡生命周期可动画。
order: 16
---

# Nav Stack

后进先出的视图栈，同一时刻只显示一个：把新视图 push 到当前视图之上，pop 回到下面那个，或 replace 掉栈顶。它对应 SwiftUI 的 `NavigationStack`、Qt 的 `StackView` 和 WinUI 的 `Frame`。底层是一份视图的 [History](../history.md)，活动条目从根页面排列到当前页面。pop 掉的页面会成为前进条目，直到下一次 push 丢弃这条前进分支，所以 `forward` 能像 WinUI 的 `GoForward` 一样把它带回来。

和所有 GPUI Base 原语一样，Nav Stack 只提供行为和语义结构，不规定产品视觉语言。页面是你创建的视图，页面之间怎么切换由你的 item renderer 决定。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- nav-stack
```

## 导入

```rust
use gpui_kit::base::{NavMotion, NavOperation, NavPage, NavStack, NavStackState};
use gpui_kit::base::motion::{PresencePhase, Transition};
```

## 结构与 API

`NavStackState` 就是栈。它放在 GPUI entity 里，按根在前的顺序持有 `AnyView`，每次变化后 emit `NavStackEvent`。

| 方法                                                               | 作用                                                                                  |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| `push(view, motion, cx)`                                           | 压到当前栈顶之上。压入空栈时立即生效，与 Qt 的 `initialItem` 一致。                   |
| `pop(motion, cx)`                                                  | 弹出栈顶并返回它。根页面永远不会被弹出，深度为 1 时返回 `None`。                      |
| `pop_to_root(motion, cx)`                                          | 用一次过渡弹掉根以上的全部页面，并返回它们。                                          |
| `forward(motion, cx)`                                              | 把最近弹掉的页面带回到当前栈顶之上并返回它。上次 push 之后没有弹过页面时返回 `None`。 |
| `replace(view, motion, cx)`                                        | 用 `view` 换掉栈顶并返回被换掉的页面，前进页保留。空栈时等于 push。                   |
| `clear(cx)`                                                        | 立即清空栈和前进页。                                                                  |
| `depth()`、`is_empty()`、`current()`、`views()`、`forward_views()` | 读取栈。`depth() > 1` 时显示返回按钮，`forward_views()` 非空时显示前进按钮。          |

`NavStack` 是元素。它持有 entity，用 `transition` 指定每次变化的时长，把每个已挂载的视图作为 `NavPage` 交给 `item` renderer。元素本身负责尺寸、背景和裁剪；它已经设置了定位，让一次变化中的两个页面可以重叠。

`NavPage` 是 renderer 收到的东西，已经铺满容器。读取 `phase()`（`Entering`、`Present` 或 `Exiting`）、`operation()`（`Push`、`Pop`、`Replace`，稳定后为 `None`）和 `progress()`（已缓动，`0.0` 到 `1.0`，一次变化中两个页面共用），用 GPUI 样式修饰后返回。

权威实现位于 [`components/nav_stack.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/nav_stack.rs)，原生与浏览器预览编译的是同一文件。

## 动画

动画在两个层面决定，默认都没有：

- **整个栈。** 不带 `transition` 的 `NavStack` 永远不动画，每次变化立即切换。给它一个 `Transition` 就会动画，再用 `item` renderer 决定怎么动。
- **单次变化。** `push`、`pop`、`pop_to_root` 和 `replace` 都接收一个 `NavMotion`，对应 UIKit 的 `animated:` 和 Qt 的 `StackView.Immediate`。`NavMotion::Animated` 走栈的 transition；`NavMotion::Immediate` 即便栈配了动画也立即切换，启动时恢复栈、从命令直接跳到某页时用它。

```rust
stack.update(cx, |stack, cx| stack.push(detail, NavMotion::Animated, cx));
stack.update(cx, |stack, cx| stack.push(restored, NavMotion::Immediate, cx));
```

## 过渡

push、pop 或 replace 之后，出场的视图会一直挂载到元素的 `Transition` 结束。绘制顺序跟随操作：push 或 replace 进来的页面盖在被它覆盖的页面上，pop 出去的页面盖在被它露出的页面上，所以滑动两个方向都正确。

```rust
NavStack::new(&self.stack)
    .size_full()
    .overflow_hidden()
    .transition(Transition::new(Duration::from_millis(220)))
    .item(|page, _, _| {
        let offset = match (page.phase(), page.operation()) {
            (PresencePhase::Entering, Some(NavOperation::Push)) => 1.0 - page.progress(),
            (PresencePhase::Exiting, Some(NavOperation::Pop)) => page.progress(),
            _ => 0.0,
        };
        page.left(relative(offset)).into_any_element()
    })
```

系统要求减少动态效果时，无论 renderer 想画什么，栈都立即切换。过渡进行中来了新操作，新操作接管，页面从当前位置反向，不会跳变。过渡进行期间两个页面都不接收指针输入。

## 状态与事件

把 `NavStackState` entity 放在渲染栈的视图上并 observe 它，这样从任何地方 push 都会让宿主重绘。需要导航的页面持有栈的 `WeakEntity`，示例页面就是这么做的。

`views()` 和 `forward_views()` 足够做一个历史菜单：把两者列出来，选中后连续 pop 或 forward 到那一页。示例页面把这个列表画成一行页码，前方的页面灰显。

栈不会移动焦点。`AnyView` 不带 focus handle；需要焦点的页面在被 push 时自己拿，和在其他地方一样。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/nav_stack.rs{rust}

## 可访问性

页面切换由页面自己宣告：每页顶部放一个标题，辅助技术在 push 之后有地方落脚。过渡结束后栈只保留当前页面可交互。

## 注意事项

页面是 entity。栈会保留在栈上的页面，以及上次 push 之后弹掉、`forward` 能带回来的页面，所以页面自己的订阅和定时器会一直活到某次 push 丢弃它或栈被清空。请在消费端设计系统中验证减少动态效果时的表现。

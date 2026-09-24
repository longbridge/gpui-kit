---
title: Action
description: 定义有类型的命令，并通过 Focus、Key Context 和 GPUI Dispatch Path 路由。
order: -2.62
---

# Action

**Action** 表达应用可以执行的操作。快捷键、菜单项、命令面板、按钮或另一个 Action handler 都可以派发同一个有类型的值。GPUI 把它路由到 [Element](./element) 树中负责该命令的区域。[Event](./event) 则沿另一个方向工作：状态改变后，它报告已经发生的事情。

本页说明命令定义与派发。按键写法、Context 匹配和 Keymap 设置见 [KeyBinding](./keybinding)。

## 一条命令，多个入口

使用 namespace 定义 unit Action。`actions!` 会生成类型，并注册稳定名称，这里的名称是 `chat::SendMessage`：

```rust
use gpui_kit::*;

actions!(chat, [SendMessage]);
```

元素 handler 接收有类型的 Action、窗口和所属实体的 [Context](./context)。`cx.listener` 将方法适配成元素 callback：

```rust
impl Chat {
    fn on_action_send_message(
        &mut self,
        _: &SendMessage,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.submit_draft();
        cx.notify();
    }
}

// 在 Chat::render 中：
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
    .child("Chat")
```

把快捷键绑定到 `SendMessage`，其他入口也使用同一个 Action：

```rust
window.dispatch_action(Box::new(SendMessage), cx); // 按钮或命令面板
MenuItem::action("Send Message", SendMessage)      // 原生应用菜单
```

命令逻辑只保存在一个 handler 中。直接调用 `window.dispatch_action(...)` **不需要** KeyBinding，也不会匹配 Key Context；按键要先转换为 Action，才会用到这两项。绑定方法见 [KeyBinding](./keybinding)。

## 定义携带数据的 Action

Action 可以携带数据。derive `Action` 时需要 `Clone` 和 `PartialEq`。如果还要通过有名称的 JSON Keymap 项构造它，则同时 derive `Deserialize` 和 `JsonSchema`：

```rust
#[derive(Action, Clone, PartialEq, serde::Deserialize, schemars::JsonSchema)]
#[action(namespace = chat)]
struct InsertPrompt {
    text: String,
}
```

Action registry 根据 namespace 和类型名，用 Action 名称及可选 JSON payload 构造有类型的值。这样，可配置的 Keymap 与命令界面可以引用同一命令。名称必须唯一；重复注册会在创建应用时 panic。

上面的 JSON 示例还要求应用依赖启用了 `derive` 功能的 `serde` 以及 `schemars`，供这两个 derive 使用。

运行时命令的 payload 不应来自 JSON 时，`no_json` 保留有类型的派发，但不允许从 JSON 构造：

```rust
#[derive(Action, Clone, PartialEq)]
#[action(namespace = workspace, no_json)]
struct OpenConversation {
    conversation_id: ConversationId,
}
```

给命令选择稳定的动词型名称。含义相同的各个入口应共用一个 Action 类型；状态改变由命令 owner 实现，而不是散落在不同的输入 callback 中。

## Focus 决定路由

<img class="architecture-light" src="/focus-action-flow.svg?v=20260922-3" alt="Focus 决定 Dispatch Path；Key Context 将快捷键匹配为 Action，再向获得 Focus 的元素派发">
<img class="architecture-dark" src="/focus-action-flow-dark.svg?v=20260922-3" alt="Focus 决定 Dispatch Path；Key Context 将快捷键匹配为 Action，再向获得 Focus 的元素派发">

[FocusHandle](./window) 标识一个键盘目标。由负责交互的实体保存 handle，并在每次渲染时将它附加到元素：

```rust
struct Chat {
    focus_handle: FocusHandle,
}

impl Chat {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle() }
    }
}

impl Focusable for Chat {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// 在 Chat::render 中：
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
```

`track_focus` 把 handle 注册在元素的 dispatch node 上。默认情况下，元素内部的 mouse down 会让此 handle 获得 Focus。内层控件需要保留自己的 Focus 时，可在它的 mouse down handler 中调用 `window.prevent_default()`，阻止祖先的默认 Focus 转移。注册 handle 不会在 render 时立即聚焦；打开视图或用户进入视图时调用 `self.focus_handle.focus(window, cx)`。

`handle.is_focused(window)` 检查当前目标是否正好获得 Focus；`handle.contains_focused(window, cx)` 也接受获得 Focus 的子元素，适合子控件活动时保持面板激活。注册的 handle 不会自动成为 Tab stop：创建它时可配置 `cx.focus_handle().tab_stop(true)`。Stateless component 可用 `window.use_keyed_state(...)` 在多次 render 间保留 handle。

GPUI 从获得 Focus 的元素经过各级祖先组成 **Dispatch Path**。路径上的 `key_context("Chat")` 让相应的上下文快捷键有资格匹配；匹配的按键生成 Action。随后 Action 沿这条路径派发。Sibling 上的 handler 无法从这条路径到达。

## Handler 顺序与传播

Action 派发分为两个阶段：

1. **Capture**：从根节点到目标节点调用匹配的 `.capture_action(...)` listener。
2. **Bubble**：从目标节点到根节点调用匹配的 `.on_action(...)` listener；如果继续传播，最后调用全局 `cx.on_action(...)` listener。

因此，离目标最近的 bubble handler 先处理命令。Action handler 默认会停止 bubble 传播。如果当前 handler 不处理该 Action，应该调用 `cx.propagate()`，让父级或全局 handler 继续尝试：

```rust
fn on_action_close(
    &mut self,
    _: &ClosePanel,
    _: &mut Window,
    cx: &mut Context<Self>,
) {
    if !self.can_close() {
        cx.propagate();
        return;
    }
    self.close();
    cx.notify();
}
```

Capture listener 可以调用 `cx.stop_propagation()`，阻止派发到达目标。全局 bubble handler 默认也会停止传播，因此不处理命令的全局 fallback 应调用 `cx.propagate()`。这些方法控制 Action 派发；`window.prevent_default()` 控制 mouse Focus 转移等默认输入行为。Pointer 与 keyboard Event 的传播见 [Event](./event)。

`window.dispatch_action(Box::new(action), cx)` 记录调用时的 Focus 目标，并把派发延迟到已渲染的帧。明确要派发到某个 owner 时，`focus_handle.dispatch_action(&action, window, cx)` 从渲染该 handle 的元素开始，前提是它仍在当前帧中。`cx.dispatch_action(&action)` 派发到活跃窗口；没有活跃窗口时则派发到全局 handler。Popup 或点击改变 Focus 后，这些方法的区别尤其重要。

## 通过共同 owner 协调并列区域

假设在 Sidebar 选中会话后，Chat 需要打开它。Sidebar 用 `OpenConversation` 描述意图；共同 owner `Workspace` 处理 Action，再更新 Chat Entity：

```rust
impl Workspace {
    fn on_action_open_conversation(
        &mut self,
        action: &OpenConversation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.chat.update(cx, |chat, cx| {
            chat.open(action.conversation_id.clone(), window, cx);
        });
    }
}

// Workspace 在自己的 handler 下渲染两个区域。
h_flex()
    .on_action(cx.listener(Self::on_action_open_conversation))
    .child(self.sidebar.clone())
    .child(self.chat.clone())

// Sidebar 交互期间，其 Focus Path 仍然活跃：
window.dispatch_action(Box::new(OpenConversation { conversation_id }), cx);
```

路由是 **Sidebar → Workspace**。然后 `Workspace` 通过 [Entity](./entity) API 更新 Chat。若只把 handler 挂在 Chat 上，来自 Sidebar 的 Action 无法到达它：Chat 是 sibling，位于 Sidebar 的 Dispatch Path 之外。必须无视当前 Focus、明确派发到某个已渲染区域时，可保留该区域的 `FocusHandle`，使用它的 `dispatch_action` 方法。

GPUI Kit 的 Command palette 和 Popup Menu 也使用这种方式：选中项提供一个 boxed Action，窗口再派发它。Command palette 还在自身元素上持有 Focus handle、Key Context 与导航 Action handler。框架组件负责选择与键盘交互机制；应用 owner 负责命令的具体含义。

## 排查丢失的命令

快捷键只有点击某个区域之后才生效时，按顺序检查路由：

1. 哪个 `FocusHandle` 获得 Focus？它是否通过 `track_focus` 附加在已渲染树上？
2. 所需的 `key_context` 是否在该元素或祖先上？绑定匹配见 [KeyBinding](./keybinding)。
3. 有类型的 `.on_action(...)` handler 是否在生成的 Dispatch Path 上？
4. 是否被更近的 handler 消费了 Action？不处理的 handler 是否忘了调用 `cx.propagate()`？
5. 如果直接派发前 Focus 已改变，是否需要用明确的 `FocusHandle` 指定目标？

让负责命令的区域同时持有 handle、context 和 handler。全局 handler 只用于真正适用于整个应用的操作。

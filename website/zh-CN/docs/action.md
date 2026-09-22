---
title: Action
description: 理解 GPUI 如何路由 Focus、快捷键、Action 与 Event。
order: -5
---

# Action

GPUI 提供 **Focus**、**Key Context**、**Action**、**KeyBinding** 与 [**Event**](./event)，它们是 GPUI 的核心交互机制。应用通过这些机制，把操作命令路由到窗口中当前活跃的区域，并在 Entity 之间传递类型明确的状态变化。

这篇 Guide 说明如何配合使用这些机制：

- **Focus** 表示键盘交互此刻发生在哪里；
- **`track_focus`** 在 Element 上注册稳定的 `FocusHandle`，让鼠标交互与命令路由可以使用它；
- **Action** 表示一条命令，可以来自快捷键、菜单、按钮或代码；
- [**Event**](./event) 表示某个 Entity 已经发生了什么，并通知它的订阅者。

## 快捷键怎样生效

<img class="architecture-light" src="/focus-action-flow.svg?v=20260922-3" alt="GPUI 从 Focus 组成 Dispatch Path，用路径上的 Key Context 匹配 KeyBinding，再把 Action 派发给最具体的 handler">
<img class="architecture-dark" src="/focus-action-flow-dark.svg?v=20260922-3" alt="GPUI 从 Focus 组成 Dispatch Path，用路径上的 Key Context 匹配 KeyBinding，再把 Action 派发给最具体的 handler">

假设窗口左侧是 Sidebar，右侧是 Chat。点击 Sidebar 后，Focus Path 包含 `Sidebar`；点击聊天输入区后，Focus Path 包含 `Chat`。因此绑定到 `Chat` 的快捷键只会在右侧区域激活。

可以在同一段布局代码中明确声明两个键盘交互区域：

```rust
h_flex()
    .size_full()
    .child(
        // 左侧：点击后激活 Sidebar Key Context。
        div()
            .w_64()
            .track_focus(&self.sidebar_focus)
            .key_context("Sidebar")
            .child("Sidebar"),
    )
    .child(
        // 右侧：点击后激活 Chat Key Context。
        div()
            .flex_1()
            .track_focus(&self.chat_focus)
            .key_context("Chat")
            .on_action(cx.listener(Self::on_action_send_message))
            .child("Chat"),
    )
```

两个区域各自拥有稳定的 `FocusHandle` 与 Key Context。点击 Chat 后，Focus 移到 `chat_focus`，`Chat` 进入当前 Dispatch Path，`SendMessage` handler 才能收到匹配后的 Action。点击 Sidebar 则会激活 `Sidebar`，此时只属于 Chat 的 binding 不会匹配。

按下一个键时，GPUI 会：

1. 从获得 Focus 的元素出发，沿祖先节点组成 Dispatch Path；
2. 收集路径上的 `key_context`，用它们匹配 `KeyBinding`；
3. 把匹配到的 Action 沿同一条路径派发，最具体的 handler 最先处理。

活跃的 Focus Path 让同一个按键可以在窗口的不同区域表达不同含义，不需要额外维护一套全局快捷键分发开关。

## Focus 是位置

`FocusHandle` 是键盘目标的稳定身份。让拥有这段交互的 Entity 保存它：

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
```

- `handle.is_focused(window)`：Focus 正好在这个目标上；
- `handle.contains_focused(window, cx)`：Focus 也可以在它的子树里；
- `handle.focus(window, cx)`：主动把 Focus 移到这里。

输入光标、选中的控件通常检查 exact Focus；当子控件获得 Focus 时整个面板仍应保持激活，则检查 containment。

## `track_focus` 到底做了什么

`track_focus` 把 handle 绑定到当前帧里的具体元素：

```rust
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
```

调用 `track_focus` 会把 `FocusHandle` 注册到这个 Element 对应的 dispatch node，并把该 Element 标记为可以接收 Focus。这会产生几项关联行为：

- 在 Element 内按下鼠标时，GPUI 默认把 Focus 移到这个 handle；
- `focus`、`in_focus` 与 `focus_visible` style 可以读取它的状态；
- GPUI 可以计算 Focus containment 与 Focus Path；
- 这条路径上的 Key Context 和 Action handler 会参与按键匹配与 Action 派发。

嵌套控件需要保留自己的 Focus 时，可以通过 `cx.prevent_default()` 阻止父元素在 mouse down 时接管 Focus。

`track_focus` **不会在 render 时立刻让 Element 获得 Focus**。打开视图或进入交互时，调用 `focus_handle.focus(window, cx)`。不要在 `render` 里无条件请求 Focus，否则每次渲染都会把 Focus 抢回来。

### Focus 与 Tab 顺序是两件事

被 track 的 handle 不会自动成为 Tab stop。Tab 行为要声明在 handle 本身：

```rust
let focus_handle = cx.focus_handle().tab_stop(true);
```

需要明确顺序时使用 `tab_index(...)`。在元素上调用 `.tab_stop(...)` 不会改变传给 `track_focus` 的 handle。

Stateless component 可以用 keyed state 让 handle 跨 render 保持稳定：

```rust
let focus_handle = window.use_keyed_state(id, cx, |_, cx| {
    cx.focus_handle().tab_stop(true)
});
```

## Action 是命令协议

Action 是 GPUI 表达应用操作的核心方式：它是一个有类型的命令值，sender 无需耦合 receiver 就能 dispatch，GPUI 再沿当前 Focus Path 路由。同一个 Action 同时服务三个层次：

1. **输入映射**：`KeyBinding` 把按键映射为 Action；
2. **命令派发**：命令面板、按钮、Popup Menu 或其他 handler dispatch 这个 Action；
3. **配置**：Keymap 把命令序列化为稳定的 Action name 与可选 JSON payload，GPUI 的 Action registry 再将其反序列化为有类型的 Action；这是 Zed 风格用户 Keymap 的基础。

例如，命令面板保存 Action，而不是为每一行各存一份 callback：

```rust
let commands: Vec<(&str, Box<dyn Action>)> = vec![
    ("发送消息", Box::new(SendMessage)),
    ("切换侧边栏", Box::new(ToggleSidebar)),
];

// 用户确认当前命令时：
window.dispatch_action(commands[selected].1.boxed_clone(), cx);
```

应用可以按自己的模型保存 owned 或 cloneable command entry；这里的关键边界是：选择命令后得到一个 Action 并 dispatch，当前 focused owner 仍然负责如何处理它。

Native application menu 也使用同一套协议。在 macOS 上，菜单命令通过 Action 接入，而不是普通 element click callback：

```rust
MenuItem::action("发送消息", SendMessage)
```

通过 `actions!` 声明的 unit Action 会按名称注册。Action 需要携带配置数据时，derive `Action` 与 `Deserialize`，并设置 namespace：

```rust
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = chat)]
struct InsertPrompt {
    text: SharedString,
}
```

Keymap 因此可以用稳定的 Action name 标识命令，并在需要时附带 JSON payload。`#[action(no_json)]` 会明确禁止从 JSON 构造该 Action，适用于不应该出现在用户配置中的 runtime-only command。

### 通过共同 owner 协调并列组件

假设用户在 Sidebar 选择一条会话后，Chat 需要打开这条会话。Sidebar 只需要用 `OpenConversation` 表达这个意图，不需要持有 Chat 的 callback 或引用。二者最近的共同 owner `Workspace` 负责处理 Action，再更新 Chat：

```rust
#[derive(Action, Clone, PartialEq)]
#[action(namespace = workspace, no_json)]
struct OpenConversation {
    conversation_id: ConversationId,
}

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

// Workspace 是 Sidebar 与 Chat 的共同祖先。
h_flex()
    .on_action(cx.listener(Self::on_action_open_conversation))
    .child(self.sidebar.clone())
    .child(self.chat.clone())

// Sidebar 中的会话行派发命令。
window.dispatch_action(
    Box::new(OpenConversation { conversation_id }),
    cx,
);
```

现在路由关系很明确：**Sidebar → Workspace → Chat**。Action 从 Sidebar 当前的 Dispatch Path 向上走，由 `Workspace` 接收；`Workspace` 再通过 Entity API 调用 Chat。Action 本身不会从 Sidebar 横向跳到 Chat。

:::info INFO — sibling 不在当前 Dispatch Path 上

如果只把 `on_action_open_conversation` handler 挂在 Chat 上，当 Sidebar 拥有 Focus 时派发的 Action 无法到达它：Chat 是 sibling，不是当前 Dispatch Path 上的祖先。同一种错误也会导致快捷键看起来没有响应——`on_action` handler 位于 Focus 选中的路径之外。跨区域 handler 应放在最近的共同 owner 上；注册 `KeyBinding` 后，还要把对应的 `key_context` 与 handler 放在快捷键应该生效的路径上。

:::

只有真正属于整个应用的命令才使用 global handler。

## 完整实现一条键盘命令

先定义并绑定一次命令：

```rust
actions!(chat, [SendMessage]);
const CHAT_CONTEXT: &str = "Chat";

fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-enter", SendMessage, Some(CHAT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-enter", SendMessage, Some(CHAT_CONTEXT)),
    ]);
}
```

把 Focus、Key Context 和 handler 放在同一个 owner region：

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

impl Render for Chat {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .key_context(CHAT_CONTEXT)
            .on_action(cx.listener(Self::on_action_send_message))
            .child("Chat")
    }
}
```

所有入口都派发同一个 `SendMessage` Action：`KeyBinding` 负责快捷键，按钮在 click handler 中调用 `window.dispatch_action(...)`，Popup Menu 与 macOS native menu 保存同一个 Action。真正的消息提交逻辑只实现一次。

Action handler 默认停止冒泡。如果内层 handler 不处理，希望父级继续尝试，调用 `cx.propagate()`。使用 `cx.on_action(...)` 注册的全局 handler 在不适用时也必须 propagate。

先执行 `cx.bind_keys(...)`，再执行 `cx.set_menus(...)`。Native menu 创建时会固定当时的快捷键显示；之后只改 binding，不会自动更新已有菜单。

## Action 与 Event 如何配合

Action 与 Event 描述同一次交互中的两个相反方向：

```text
⌘ Enter → SendMessage Action → Chat 发送 → MessageSent Event → Workspace 更新
```

Action 把“**想做什么**”向内传给 command owner；操作改变状态后，Event 再把“**发生了什么**”向外传给感兴趣的 owner。阅读 [Event](./event)，继续了解 `EventEmitter`、`emit`、订阅生命周期，以及完整的 Action/Event 选择方法。

## 全局与上下文快捷键

编辑和导航快捷键通常都应有 Key Context。它们只在特定区域包含 Focus 时有意义，也应该让更具体的 child 优先处理。

真正的全局 fallback 或 service command 才使用 `cx.on_action(...)`。危险的全局快捷键还要在 owner 中检查运行时状态。Longbridge Pro 的交易快捷键既限制在 Workspace Key Context，又会在输入框或对话框获得 Focus 时拒绝执行。Key Context 决定“**命令在哪里有资格匹配**”，handler 决定“**它现在是否允许执行**”。

## 排查“点击以后才生效”

如果快捷键只有点击某个区域以后才生效，通常是这次点击把 Focus 移进了包含目标 Key Context 和 handler 的路径。把它当作路由问题，沿 GPUI 使用的同一条链路检查。

按整条链路依次检查：

1. **Binding**：按键是否绑定到预期的 Action 和 Key Context？
2. **Focus**：点击前后，究竟哪个 `FocusHandle` 获得 Focus？
3. **Tracking**：同一个 handle 是否在已渲染元素上调用 `track_focus`？
4. **Context**：目标 `key_context` 是否位于 focused element 或其祖先上？
5. **Handler**：`on_action` 是否也在同一条 Dispatch Path 上？
6. **Propagation**：是否有更具体的 handler 提前吞掉 Action？
7. **Lifetime**：全局 handler 或 Event subscription 是否已释放？过期 handler 是否忘了 propagate？

最常见的修复，是让同一个 region 一起拥有稳定 handle、`track_focus`、`key_context` 与 `on_action`，然后在用户进入该区域时把 Focus 移进去。

这些模式来自 GPUI 的 dispatch 实现，并在 GPUI Kit 的 Menu、Tree、Input、Dialog、Color Picker，Zed 的 panel 与 editor，以及 Longbridge Pro 的 workspace 和交易快捷键中得到实际验证。

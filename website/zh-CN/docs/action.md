---
title: Action
description: 定义有类型的命令，并通过 Focus、Key Context 和 GPUI Dispatch Path 路由。
order: -2.62
---

# Action

**Action** 表达应用可以执行的操作。快捷键、菜单项、命令面板、按钮或另一个 Action handler 都可以派发同一个有类型的值。GPUI 把它路由到 [Element](./element) 树中负责该命令的区域。[Event](./event) 则沿另一个方向工作：状态改变后，它报告已经发生的事情。

[GPUI Action](https://docs.rs/crate/gpui-pre/0.3.6/source/src/action.rs) 源码定义了本页介绍的宏、trait 和 registry。

本页说明命令定义与派发。若尚未建立键盘目标，先读 [Focus](./focus)；按键写法、Context 匹配和 Keymap 设置见 [KeyBinding](./keybinding)。

## 运行仓库中的 Action 示例

在仓库根目录直接打开 Story Gallery 的 Tree 页面：

```sh
cargo run -p gpui-component-story -- Tree
```

选中一行文件树，再按 **Enter**；终端会打印 `Renaming item: ...`。换一行再试，可以看到 handler 读取的是当前选择。如果 Enter 没反应，先点击树中的一行：这个快捷键只在 Tree story 的 Focus Path 中生效。这个示例不会真的重命名文件。

对照 [`crates/story/src/stories/tree_story.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/story/src/stories/tree_story.rs) 阅读实现：

1. `actions!(story, [Rename, OpenFile, Delete])` 定义有类型的命令。
2. `init` 把 `enter` 绑定到 `TreeStory` Context 下的 `Rename`；应用初始化时由 [`stories::init`](https://github.com/longbridge/gpui-kit/blob/main/crates/story/src/stories/mod.rs) 调用。
3. `TreeStory::render` 在容器上设置 `.key_context(CONTEXT)` 和 `.on_action(cx.listener(Self::on_action_rename))`；子级 Tree 提供活动的 Focus Path。
4. `on_action_rename` 从 `tree_state` 读取选中项。选中状态属于 Tree state；Action 表达用户请求的操作。

下文从这条路由逐步展开。菜单或按钮以后也能派发同一个 Action，无需复制 handler。

## 在 `hello_world` 中写一个可运行的 Action

Tree story 适合追踪现成应用。要亲手搭起整条路由，可暂时把 `examples/hello_world/src/main.rs` 替换为下面的完整程序。它复用现有 `hello_world` 包，不需要新增 crate 或依赖。

```rust
use gpui_kit::component::button::Button;
use gpui_kit::*;

actions!(counter, [Increment]);

struct Counter {
    count: usize,
    focus: FocusHandle,
}

impl Counter {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        focus.focus(window, cx);
        Self { count: 0, focus }
    }

    fn on_increment(&mut self, _: &Increment, _: &mut Window, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}

impl Render for Counter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .track_focus(&self.focus)
            .key_context("Counter")
            .on_action(cx.listener(Self::on_increment))
            .child(format!("Count: {}", self.count))
            .child("Press Enter while this region has focus")
            .child(
                Button::new("increment")
                    .label("Increment")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.focus.dispatch_action(&Increment, window, cx);
                    })),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        cx.bind_keys([KeyBinding::new("enter", Increment, Some("Counter"))]);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Counter::new(window, cx))
        })
        .expect("failed to open window");
    });
}
```

在仓库根目录运行：

```sh
cargo run -p hello_world --bin hello_world
```

窗口从 `Count: 0` 开始。在计数器区域拥有 Focus 时按 **Enter**，或点击 **Increment**，每次都会让界面上的数字增加 1。计数器的 `FocusHandle` 保存在 Entity 中，并附加到渲染出的容器。`cx.bind_keys` 只在路径中包含 `Counter` 时才把 Enter 映射为 `Increment`。容器上的 `.on_action` 调用 `on_increment`，修改保留状态，再通过 `cx.notify()` 让新数字显示出来。按钮借助保存的 handle 向同一个容器派发同一个 Action；即使点击按钮改变 Focus，命令仍能到达计数器。

可以做两个小实验，逐项检查路由，然后还原源码。先删掉 `.key_context("Counter")`：Enter 不再匹配该绑定，按钮仍然有效，因为直接派发不会查询 Key Context。恢复 Context 后删掉 `.on_action(...)`：两个入口都无法改变计数，因为路径上没有 handler。如果 handler 已执行而数字未刷新，检查它是否修改了保留状态并调用 `cx.notify()`。实验结束后恢复 `examples/hello_world/src/main.rs`。

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

// In Chat::render:
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
    .child("Chat")
```

把快捷键绑定到 `SendMessage`，其他入口也使用同一个 Action：

```rust
window.dispatch_action(Box::new(SendMessage), cx); // Button or command palette
MenuItem::action("Send Message", SendMessage)      // Native application menu
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

如果 handler 能从 owner 读取所需状态，就用 unit Action，例如 Tree story 的 `Rename`。如果调用方必须指定目标或提供值，就用携带数据的 Action。Action payload 是命令输入，不是存放 owner 中动态 UI 状态的地方。

运行时命令的 payload 不应来自 JSON 时，`no_json` 保留有类型的派发，但不允许从 JSON 构造：

```rust
#[derive(Action, Clone, PartialEq)]
#[action(namespace = workspace, no_json)]
struct OpenConversation {
    conversation_id: String,
}
```

给命令选择稳定的动词型名称。含义相同的各个入口应共用一个 Action 类型；状态改变由命令 owner 实现，而不是散落在不同的输入 callback 中。

## Focus 决定路由

<svg class="focus-action-diagram" viewBox="0 0 1120 250" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="focus-action-title-zh focus-action-desc-zh">
  <title id="focus-action-title-zh">GPUI 快捷键派发的三个步骤</title>
  <desc id="focus-action-desc-zh">Focus 建立派发路径，Key Context 选择匹配的快捷键，Action 沿路径交给 handler。</desc>
  <defs><marker id="focus-action-arrow-zh" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 0 10 5 0 10z" class="fa-arrow-head" /></marker></defs>
  <rect x="24" y="24" width="310" height="202" rx="14" class="fa-box" />
  <text x="50" y="58" class="fa-step">1 · FOCUS</text><text x="50" y="88" class="fa-title">Build the Dispatch Path</text>
  <rect x="50" y="111" width="258" height="58" rx="9" class="fa-active" /><text x="70" y="136" class="fa-code">Chat → Workspace</text><text x="70" y="157" class="fa-body">focused Element → ancestors</text>
  <text x="50" y="201" class="fa-body">Start at the Element with Focus.</text>
  <path d="M348 125H393" class="fa-arrow" marker-end="url(#focus-action-arrow-zh)" />
  <rect x="407" y="24" width="310" height="202" rx="14" class="fa-box" />
  <text x="433" y="58" class="fa-step">2 · KEY CONTEXT</text><text x="433" y="88" class="fa-title">Match a KeyBinding</text>
  <rect x="433" y="111" width="258" height="58" rx="9" class="fa-active" /><text x="453" y="136" class="fa-code">⌘ Enter + "Chat"</text><text x="453" y="157" class="fa-code">→ SendMessage</text>
  <text x="433" y="201" class="fa-body">Use key_context values on the path.</text>
  <path d="M731 125H776" class="fa-arrow" marker-end="url(#focus-action-arrow-zh)" />
  <rect x="790" y="24" width="306" height="202" rx="14" class="fa-box" />
  <text x="816" y="58" class="fa-step">3 · ACTION</text><text x="816" y="88" class="fa-title">Dispatch along the path</text>
  <rect x="816" y="111" width="254" height="58" rx="9" class="fa-action" /><text x="836" y="136" class="fa-code">Chat handler</text><text x="836" y="157" class="fa-body">then parents if propagated</text>
  <text x="816" y="201" class="fa-body">The most specific handler runs first.</text>
</svg>

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

// In Chat::render:
div()
    .track_focus(&self.focus_handle)
    .key_context("Chat")
    .on_action(cx.listener(Self::on_action_send_message))
```

`track_focus` 把 handle 注册在元素的 dispatch node 上。默认情况下，元素内部的 mouse down 会让此 handle 获得 Focus。内层控件需要保留自己的 Focus 时，可在它的 mouse down handler 中调用 `window.prevent_default()`，阻止祖先的默认 Focus 转移。注册 handle 不会在 render 时立即聚焦；打开视图或用户进入视图时调用 `self.focus_handle.focus(window, cx)`。

`handle.is_focused(window)` 检查当前目标是否正好获得 Focus；`handle.contains_focused(window, cx)` 也接受获得 Focus 的子元素，适合子控件活动时保持面板激活。注册的 handle 不会自动成为 Tab stop：创建它时可配置 `cx.focus_handle().tab_stop(true)`。Stateless component 可用 `window.use_keyed_state(...)` 在多次 render 间保留 handle。

GPUI 从获得 Focus 的元素经过各级祖先组成 **Dispatch Path**。路径上的 `key_context("Chat")` 让相应的上下文快捷键有资格匹配；匹配的按键生成 Action。随后 Action 沿这条路径派发。Sibling 上的 handler 无法从这条路径到达。

### 追踪一次快捷键

假设获得 Focus 的元素位于 `Chat` 内，而 `Chat` 位于 `Workspace` 内。`KeyBinding::new("secondary-enter", SendMessage, Some("Chat"))` 只在当前 Focus Path 中有 `Chat` 时才有资格匹配。GPUI 选出匹配的 binding，再沿该路径派发 `SendMessage`。`Chat` 的 bubble handler 会先于 `Workspace` 的 handler 运行。如果路径上的元素都没有处理这个 Action，全局 `cx.on_action` handler 可以接收它。Context 负责选择 **binding**，不会自行选定 handler。绑定竞争与 predicate 写法见 [KeyBinding](./keybinding)。

点击触发命令时，`window.dispatch_action(Box::new(SendMessage), cx)` 使用调用时捕获的 Focus。确保预期路径已获得 Focus，或通过保留的 `FocusHandle` 派发。如果不需要共享命令路由，按钮 callback 也可以直接调用 owner 的方法。

## Handler 顺序与传播

Action 派发分为两个阶段：

1. **Capture**：先调用全局 capture listener，再从根节点到目标节点调用匹配的 `.capture_action(...)` listener。
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

要观察命令而不消费它，capture listener 保持传播开启即可；capture 开始时已默认开启传播。在 bubble 阶段，每个 listener 开始时都默认停止传播。只有希望父级或全局 fallback 继续接收 Action 时，才调用 `cx.propagate()`。单纯提前 `return` 不会把命令传下去。

`window.dispatch_action(Box::new(action), cx)` 记录调用时的 Focus 目标，并把派发延迟到已渲染的帧。明确要派发到某个 owner 时，`focus_handle.dispatch_action(&action, window, cx)` 从渲染该 handle 的元素开始，前提是它仍在当前帧中。`cx.dispatch_action(&action)` 派发到活跃窗口；没有活跃窗口时则派发到全局 handler。Popup 或点击改变 Focus 后，这些方法的区别尤其重要。

| 调用 | 目标 | 适用场景 |
| --- | --- | --- |
| `window.dispatch_action(Box::new(action), cx)` | 调用时本窗口的当前 Focus | 面向获得 Focus 的区域的菜单或按钮命令；派发会延后执行。 |
| `focus_handle.dispatch_action(&action, window, cx)` | 当前帧中渲染该 handle 的元素 | 即使其他控件已获得 Focus，仍要命中指定的已渲染区域；handle 未渲染时不会派发。 |
| `cx.dispatch_action(&action)` | 活跃窗口；没有活跃窗口时为全局 handler | 调用方只有 `App` context 的应用级命令。 |

这些调用都不会计算 Key Context predicate。只有**按键**通过 keymap 选取 Action 时，Key Context 才参与匹配。派发后的 Action 仍需要在选定目标的路径上找到 handler。

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

// Workspace renders both regions under its own handler.
h_flex()
    .on_action(cx.listener(Self::on_action_open_conversation))
    .child(self.sidebar.clone())
    .child(self.chat.clone())

// The Sidebar focus path stays active during interaction:
window.dispatch_action(Box::new(OpenConversation { conversation_id }), cx);
```

路由是 **Sidebar → Workspace**。然后 `Workspace` 通过 [Entity](./entity) API 更新 Chat。若只把 handler 挂在 Chat 上，来自 Sidebar 的 Action 无法到达它：Chat 是 sibling，位于 Sidebar 的 Dispatch Path 之外。必须无视当前 Focus、明确派发到某个已渲染区域时，可保留该区域的 `FocusHandle`，使用它的 `dispatch_action` 方法。更大的功能模块如何安排 owner，见 [Coding Guides](./coding-guides)。

GPUI Kit 的 Command palette 和 Popup Menu 也使用这种方式：选中项提供一个 boxed Action，窗口再派发它。Command palette 还在自身元素上持有 Focus handle、Key Context 与导航 Action handler。框架组件负责选择与键盘交互机制；应用 owner 负责命令的具体含义。

## 排查丢失的命令

快捷键只有点击某个区域之后才生效时，按顺序检查路由：

1. 哪个 `FocusHandle` 获得 Focus？它是否通过 `track_focus` 附加在已渲染树上？
2. 所需的 `key_context` 是否在该元素或祖先上？绑定匹配见 [KeyBinding](./keybinding)。
3. 有类型的 `.on_action(...)` handler 是否在生成的 Dispatch Path 上？
4. 是否被更近的 handler 消费了 Action？不处理的 handler 是否忘了调用 `cx.propagate()`？
5. 如果直接派发前 Focus 已改变，是否需要用明确的 `FocusHandle` 指定目标？

按键没有反应时，先区分**没有匹配的 binding**与**找不到可到达的 handler**。尝试通过目标 `FocusHandle` 直接派发 Action：如果能到达 handler，应检查按键写法与 Context predicate；如果仍无法到达，应检查 handle 是否已渲染、Dispatch Path 和传播。handler 已执行但画面未更新时，应确认它修改了 owner 的状态，并在需要重绘时调用 `cx.notify()`。

让负责命令的区域同时持有 handle、context 和 handler。全局 handler 只用于真正适用于整个应用的操作。

## 用 Tree story 练习

1. **跟踪路径。** 运行 `cargo run -p gpui-component-story -- Tree`，选中一行并按 Enter。在 `tree_story.rs` 中找出 binding、context 和 handler。哪个组件持有选中项？哪个 entity 持有命令 handler？
2. **更换输入。** 在自己的分支中，临时将 Tree story 的 binding 从 `"enter"` 改成 `"secondary-r"`。重新运行 Story Gallery，确认新按键仍调用同一个 `Rename` handler，而 Enter 不再调用。实验后还原源码。
3. **预测传播。** 在一个小视图的子级和父级都挂上 `Rename` handler。子级仅在没有选中项时调用 `cx.propagate()`。先预测两种情况下各有哪些 handler 运行，再用可见输出验证。handler 直接返回而不调用 `cx.propagate()`，就会消费 Action。
4. **添加目标。** 用 `#[action(namespace = story, no_json)]` 给必须携带行 ID 的操作建模。从行 callback 派发，把状态修改保留在 owner 的 handler 中。与从当前选中项读取目标的 `Rename` 比较。

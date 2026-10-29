---
title: AI Elements
description: 为智能体和聊天界面提供推理过程、工具调用、计划、审批、引用来源和输入框。
---

# AI Elements

`ai` 模块提供智能体和聊天界面中围绕对话本身的部件：模型的推理过程、工具调用及其输出、计划与待办队列、审批请求、引用来源，以及输入框。它们参考了
<a href="https://ui.shadcn.com/docs/ai-elements" target="_blank" rel="noopener noreferrer">shadcn AI Elements</a>
的模式，对话记录本身则由现有的 [Message](message) 和 [MessageScroller](message-scroller) 组合而成。

所有元素都是受控的 `RenderOnce` 值。应用持有对话记录、流式状态和每个展开状态，并在每次渲染时传入；回调只报告用户请求的变化。

## 导入

```rust
use gpui_kit::component::ai::{
    AgentTask, AgentTaskItem, ChainOfThought, ChainOfThoughtStep, Checkpoint, Confirmation,
    ConfirmationState, ContextUsage, FileChange, FileTree, FileTreeEntry, InlineCitation,
    ModelOption, ModelSelector, Plan, PlanStep, PromptInput, PromptStatus, Queue, QueueItem,
    Reasoning, Source, Sources, StackFrame, StackTrace, Subagent, Suggestion, Suggestions,
    Terminal, TestCase, TestResults, TestSuite, Tool, Trace, TraceStatus,
};
```

## Trace 行

大多数元素都由 `Trace` 组合而成：一行可展开的标题，包含图标、标题、可选的元信息和状态，展开后显示内容。共用这一行，让智能体的每一步工作都有相同的几何结构和键盘路径——标题本身是 `Button`，可以用 Tab 聚焦，用 Enter 或空格展开。

`TraceStatus` 记录进度：`Pending`、`Running`、`Completed`、`Failed` 或 `Skipped`。每种状态都有不同的图形，不只靠颜色区分；运行中的步骤显示旋转指示器。

```rust
Trace::new("lint")
    .icon(IconName::Wrench)
    .title("cargo clippy")
    .status(TraceStatus::Running)
    .open(self.lint_open)
    .on_toggle(cx.listener(|this, open: &bool, _, cx| {
        this.lint_open = *open;
        cx.notify();
    }))
    .content("Checking 312 files…")
```

## 推理过程

`Reasoning` 显示模型的思考。`streaming` 为 true 时，标题以闪光效果显示「思考中…」；结束后显示模型思考了多久。通常在推理流式输出时保持展开，回答开始后收起。

```rust
Reasoning::new(("reasoning", message.id))
    .streaming(message.is_thinking())
    .duration(message.thought_for)
    .open(message.reasoning_open)
    .on_toggle(cx.listener(move |this, open: &bool, _, cx| this.set_reasoning_open(id, *open, cx)))
    .child(message.reasoning.clone())
```

`ChainOfThought` 是分步版本：一行摘要，展开后是若干 `ChainOfThoughtStep`，每一步包含标签、可选说明、表示步骤类型的可选图标以及自己的状态。

## 工具调用与审批

`Tool` 展示一次工具调用从输入到输出的过程。按照与模型交换的原样传入文本，它会以等宽字体显示。`error` 会把调用标记为失败。

```rust
Tool::new("search-1", "web_search")
    .status(TraceStatus::Completed)
    .input(r#"{ "query": "GPUI layout" }"#)
    .output("3 results")
```

`Confirmation` 在工具运行前请用户批准。处于 `ConfirmationState::Requested` 时，它显示请求以及「拒绝」和默认提交的「批准」；有更具体的动词时，用 `approve_label` 写明结果。做出决定后，它收起为一行，记录这次决定。

```rust
Confirmation::new("publish", "Run “cargo publish”?")
    .description("Publishes the crate to crates.io. This cannot be undone.")
    .approve_label("Publish")
    .state(self.publish_state)
    .on_approve(cx.listener(|this, _, _, cx| this.approve(cx)))
    .on_deny(cx.listener(|this, _, _, cx| this.deny(cx)))
```

`Terminal` 显示一条命令及其流式输出；`exit_code` 结束命令并设置状态。只显示最新的 `max_lines` 行，让最新输出始终可见。`StackTrace` 显示一次失败及其栈帧，`StackTrace::parse` 可以解析 JavaScript、Python 和 Rust 的回溯，来自依赖和运行时的栈帧会弱化显示。`TestResults` 统计一次测试运行的结果并列出每个 `TestSuite`，失败的用例下方显示失败信息。

## 计划、任务与子智能体

- `Plan` 列出 `PlanStep`，并以「2 / 5」概括进度；整体状态由各步骤推导。
- `Queue` 是智能体的待办列表：未完成的 `QueueItem` 始终可见，已完成的项目位于由 `done_open` 控制的区域中。
- `AgentTask` 按发生顺序列出一个任务的步骤，每一步可以注明它涉及的文件。它没有命名为 `Task`，以免遮蔽 GPUI 的异步 `Task`。
- `Subagent` 显示一次子智能体运行：它收到的指令、作为子元素的自身记录，以及结果。
- `FileTree` 以文件夹树显示智能体修改过的文件，包含 `FileChange` 和行数统计。文件夹的展开和选中的文件都由应用控制。
- `Checkpoint` 在对话记录中标出一个可以恢复的位置。

## 来源与引用

`Sources` 把回复使用过的网页收在「引用了 N 个来源」摘要之后。`InlineCitation` 在句子中标出来源：显示来源的域名，悬停时展示来源，激活时打开它——它链接的是网页资源。

```rust
Sources::new(("sources", id))
    .sources(message.sources.clone())
    .open(message.sources_open)
    .on_toggle(cx.listener(move |this, open: &bool, _, cx| this.set_sources_open(id, *open, cx)))
```

## 输入框

`PromptInput` 是消息输入框：附件显示在上方，工具位于工具栏的起始一侧，末尾是发送按钮。`PromptStatus` 决定按钮的行为：`Ready` 发送，`Submitted` 等待，`Streaming` 停止，`Error` 允许重新发送。空文本永远不会发送。

应用持有一个 `TextareaState`。用 `.auto_grow(1, 8).submit_on_enter(true)` 创建它，并订阅 `InputEvent::PressEnter`，以便从键盘发送，调用与 `on_submit` 相同的方法；`RenderOnce` 元素本身无法持有这个订阅。

```rust
let prompt = cx.new(|cx| {
    TextareaState::new(window, cx)
        .auto_grow(1, 8)
        .submit_on_enter(true)
        .placeholder("Message…")
});

PromptInput::new("prompt", &self.prompt)
    .status(self.status())
    .tool(
        ModelSelector::new("model")
            .models([ModelOption::new("gpt-5", "GPT-5")])
            .efforts([ModelOption::new("low", "Low"), ModelOption::new("high", "High")])
            .model(self.model.clone())
            .effort(self.effort.clone())
            .on_model_change(cx.listener(|this, id: &SharedString, _, cx| this.set_model(id, cx)))
            .on_effort_change(cx.listener(|this, id: &SharedString, _, cx| this.set_effort(id, cx))),
    )
    .tool(ContextUsage::new("context", self.used_tokens, 400_000))
    .on_submit(cx.listener(|this, text: &SharedString, window, cx| this.send(text, window, cx)))
    .on_stop(move |_, cx| _ = view.update(cx, |this, cx| this.stop(cx)))
```

`ModelSelector` 包含两个菜单：模型，以及推理强度；只有提供了强度选项时才显示第二个菜单。`ContextUsage` 显示上下文窗口的占用比例，激活后展开 token 明细。`Suggestion` 一键发送一条简短提示，多个建议放在 `Suggestions` 行中。

## 无障碍与状态

- 展开标题都是 Button：可以获得键盘焦点，并朗读标题。标题要具体，写明工具或步骤的名称。
- 对于未完成的状态，图形总是与文字标签一起出现。
- 回复流式输出时，禁用 `Suggestion` 和 `ModelSelector`；`PromptInput` 的发送按钮在文本为空和提交中时会自动禁用。
- 只在有实际后果的操作上使用 `Confirmation`。它的批准按钮是主要提交，应当用结果来命名。

## API 参考

| Type | 用途 |
| --- | --- |
| `Trace`, `TraceStatus` | 各元素共用的展开行与进度 |
| `Reasoning` | 带思考时长摘要的流式推理 |
| `ChainOfThought`, `ChainOfThoughtStep` | 可展开为步骤的摘要 |
| `Tool` | 从输入到输出或错误的工具调用 |
| `Confirmation`, `ConfirmationState` | 工具运行前的审批请求 |
| `Plan`, `PlanStep` | 计划步骤与进度 |
| `Queue`, `QueueItem` | 分为待办和已完成的待办列表 |
| `AgentTask`, `AgentTaskItem` | 按发生顺序列出的任务步骤 |
| `Subagent` | 带记录和结果的子智能体运行 |
| `Terminal` | 带流式输出和退出码的命令 |
| `TestResults`, `TestSuite`, `TestCase`, `TestSummary` | 包含测试套件和失败信息的测试运行 |
| `StackTrace`, `StackFrame` | 失败及其栈帧，附带回溯解析 |
| `Sources`, `Source`, `InlineCitation` | 引用的网页与句内引用 |
| `PromptInput`, `PromptStatus` | 带附件、工具和发送/停止的输入框 |
| `ModelSelector`, `ModelOption` | 模型与推理强度菜单 |
| `ContextUsage` | 带 token 明细的上下文窗口用量 |
| `Suggestion`, `Suggestions` | 一键发送的提示 |
| `FileTree`, `FileTreeEntry`, `FileChange` | 以文件夹树显示的已修改文件 |
| `Checkpoint` | 对话记录中可恢复的位置 |

完整示例见 `examples/chatgpt`：一个使用 ChatGPT 账号登录、通过这些元素流式调用 OpenAI Responses API 的客户端。

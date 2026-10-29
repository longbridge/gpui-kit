---
title: AI Elements
description: Reasoning, tool calls, plans, approvals, sources and the prompt composer for agent and chat interfaces.
---

# AI Elements

The `ai` module provides the pieces of an agent or chat interface that sit
around the conversation itself: the model's reasoning, tool calls and their
output, plans and to-do queues, approval requests, cited sources, and the
prompt composer. They follow the patterns of
<a href="https://ui.shadcn.com/docs/ai-elements" target="_blank" rel="noopener noreferrer">shadcn AI Elements</a>
and compose the existing [Message](message) and
[MessageScroller](message-scroller) components for the transcript.

Every element is a controlled `RenderOnce` value. The application owns the
transcript, the streaming state, and every open state, and passes them in on
each render; callbacks report the change the user requested.

## Import

```rust
use gpui_kit::component::ai::{
    AgentTask, AgentTaskItem, ChainOfThought, ChainOfThoughtStep, Checkpoint, Confirmation,
    ConfirmationState, ContextUsage, FileChange, FileTree, FileTreeEntry, InlineCitation,
    ModelOption, ModelSelector, Plan, PlanStep, PromptInput, PromptStatus, Queue, QueueItem,
    Reasoning, Source, Sources, StackFrame, StackTrace, Subagent, Suggestion, Suggestions,
    Terminal, TestCase, TestResults, TestSuite, Tool, Trace, TraceStatus,
};
```

## The trace row

Most elements are compositions of `Trace`: a disclosure row with an icon, a
title, optional metadata, a status, and content revealed when the row is open.
Sharing one row gives every step of agent work the same geometry and the same
keyboard path — the header is a `Button`, so Tab reaches it and Enter or Space
toggles it.

`TraceStatus` records progress: `Pending`, `Running`, `Completed`, `Failed`, or
`Skipped`. Each state has a distinct glyph, so color is never the only cue; a
running step shows a spinner.

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

## Reasoning

`Reasoning` shows the model's thinking. While `streaming` is true the title
shimmers as `Thinking…`; afterwards it reports how long the model thought.
Applications usually keep it open while reasoning streams and close it when
the answer begins.

```rust
Reasoning::new(("reasoning", message.id))
    .streaming(message.is_thinking())
    .duration(message.thought_for)
    .open(message.reasoning_open)
    .on_toggle(cx.listener(move |this, open: &bool, _, cx| this.set_reasoning_open(id, *open, cx)))
    .child(message.reasoning.clone())
```

`ChainOfThought` is the stepwise variant: a summary that expands into
`ChainOfThoughtStep`s, each with a label, optional description, an optional
icon naming the kind of step, and its own status.

## Tool calls and approval

`Tool` shows one call from its input to its output. Pass the text exactly as
exchanged with the model; it is shown monospaced. `error` marks the call
failed.

```rust
Tool::new("search-1", "web_search")
    .status(TraceStatus::Completed)
    .input(r#"{ "query": "GPUI layout" }"#)
    .output("3 results")
```

`Confirmation` asks the user to approve a tool before it runs. While
`ConfirmationState::Requested`, it shows the request with `Deny` and the
default commit `Approve`; name the result with `approve_label` when a more
specific verb exists. Once decided it collapses to a line that records the
decision.

```rust
Confirmation::new("publish", "Run “cargo publish”?")
    .description("Publishes the crate to crates.io. This cannot be undone.")
    .approve_label("Publish")
    .state(self.publish_state)
    .on_approve(cx.listener(|this, _, _, cx| this.approve(cx)))
    .on_deny(cx.listener(|this, _, _, cx| this.deny(cx)))
```

`Terminal` shows a command with its streaming output; `exit_code` finishes it
and sets the status. Only the newest `max_lines` lines are shown, so the
latest output stays in view. `StackTrace` shows a failure and its frames, and
`StackTrace::parse` reads JavaScript, Python, and Rust backtraces; frames from
dependencies and the runtime are de-emphasized. `TestResults` counts a test
run's outcomes and lists each `TestSuite`, showing failure messages under
failing cases.

## Plans, tasks and subagents

- `Plan` lists `PlanStep`s and summarizes progress as `2 of 5`; its status is
  derived from the steps.
- `Queue` is the agent's to-do list: remaining `QueueItem`s are always
  visible, and done items sit in a section controlled by `done_open`.
- `AgentTask` lists the steps of one task as they happen, each optionally
  naming the file it touched. It is not called `Task` so it does not shadow
  GPUI's async `Task`.
- `Subagent` shows a subagent run: its instruction, its own trace as children,
  and its result.
- `FileTree` shows the files the agent changed as a folder tree with
  `FileChange` and line counts. Folder expansion and the selected file are
  controlled.
- `Checkpoint` divides the transcript at a point the user can restore.

## Sources and citations

`Sources` lists the pages a reply used behind a `Used N sources` summary.
`InlineCitation` marks a source within a sentence: it shows the source's host,
reveals the source on hover, and opens it when activated, since it links to a
web resource.

```rust
Sources::new(("sources", id))
    .sources(message.sources.clone())
    .open(message.sources_open)
    .on_toggle(cx.listener(move |this, open: &bool, _, cx| this.set_sources_open(id, *open, cx)))
```

## Prompt input

`PromptInput` is the message field with attachments above it, tools at the
leading edge of its toolbar, and a send button. `PromptStatus` decides the
button: `Ready` sends, `Submitted` waits, `Streaming` stops, and `Error` lets
the user send again. Empty text is never sent.

The application owns a `TextareaState`. Create it with
`.auto_grow(1, 8).submit_on_enter(true)` and subscribe to
`InputEvent::PressEnter` to send from the keyboard, calling the same method as
`on_submit`; a `RenderOnce` element cannot hold that subscription itself.

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

`ModelSelector` holds two menus — the model and how hard it thinks; the effort
menu appears only when effort options are given. `ContextUsage` shows how much
of the context window is used and opens a token breakdown when activated.
`Suggestion` sends a short prompt in one click; place several in a
`Suggestions` row.

## Accessibility and state guidance

- Disclosure headers are Buttons: they take keyboard focus and announce their
  title. Keep titles specific — name the tool or the step.
- Status always pairs a glyph with a text label for non-completed states.
- Disable `Suggestion` and `ModelSelector` while a reply streams; the
  `PromptInput` send button disables itself for empty text and while
  submitting.
- Use `Confirmation` only for consequential actions. Its approve button is the
  primary commit, so label it with the result.

## API reference

| Type | Purpose |
| --- | --- |
| `Trace`, `TraceStatus` | Disclosure row and progress shared by the elements |
| `Reasoning` | Streaming reasoning behind a duration summary |
| `ChainOfThought`, `ChainOfThoughtStep` | Summary that expands into steps |
| `Tool` | Tool call from input to output or error |
| `Confirmation`, `ConfirmationState` | Approval request before a tool runs |
| `Plan`, `PlanStep` | Planned steps and progress |
| `Queue`, `QueueItem` | To-do list split into remaining and done |
| `AgentTask`, `AgentTaskItem` | Steps of one task as they happen |
| `Subagent` | Subagent run with its trace and result |
| `Terminal` | Command with streaming output and exit code |
| `TestResults`, `TestSuite`, `TestCase`, `TestSummary` | Test run with suites and failures |
| `StackTrace`, `StackFrame` | Failure and its frames, with a backtrace parser |
| `Sources`, `Source`, `InlineCitation` | Cited pages and in-sentence citations |
| `PromptInput`, `PromptStatus` | Composer with attachments, tools and send/stop |
| `ModelSelector`, `ModelOption` | Model and reasoning-effort menus |
| `ContextUsage` | Context-window meter with token breakdown |
| `Suggestion`, `Suggestions` | One-click prompts |
| `FileTree`, `FileTreeEntry`, `FileChange` | Changed files as a folder tree |
| `Checkpoint` | Restorable point in the transcript |

See `examples/chatgpt` for a complete client that signs in with ChatGPT and
streams the OpenAI Responses API through these elements.

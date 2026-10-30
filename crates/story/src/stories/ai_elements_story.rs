use std::{collections::HashSet, time::Duration};

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    ai::{
        AgentTask, AgentTaskItem, ChainOfThought, ChainOfThoughtStep, Checkpoint, Confirmation,
        ConfirmationState, ContextUsage, FileChange, FileTree, FileTreeEntry, InlineCitation,
        ModelOption, ModelSelector, Plan, PlanStep, PromptInput, PromptStatus, Queue, QueueItem,
        Reasoning, Source, Sources, StackTrace, Subagent, Suggestion, Suggestions, Terminal,
        TestCase, TestResults, TestSuite, Tool, TraceStatus,
    },
    bubble::Bubble,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{InputEvent, TextareaState},
    message::{Message, MessageAlignment, MessageContent},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, prelude::FluentBuilder as _, rems,
};

use crate::{Story, section};

const REASONING: &str = "reasoning";
const TOOL: &str = "tool";
const FAILED_TOOL: &str = "failed-tool";
const CHAIN: &str = "chain";
const PLAN: &str = "plan";
const TASK: &str = "task";
const SUBAGENT: &str = "subagent";
const TERMINAL: &str = "terminal";
const TESTS: &str = "tests";
const STACK: &str = "stack";
const SOURCES: &str = "sources";
const QUEUE_DONE: &str = "queue-done";

pub struct AiElementsStory {
    focus_handle: FocusHandle,
    open: HashSet<&'static str>,
    prompt: Entity<TextareaState>,
    status: PromptStatus,
    model: SharedString,
    effort: SharedString,
    confirmation: ConfirmationState,
    collapsed_folders: Vec<SharedString>,
    selected_file: Option<SharedString>,
    sent: Vec<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl AiElementsStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 6)
                .submit_on_enter(true)
                .placeholder("Ask anything…")
        });
        let _subscriptions = vec![cx.subscribe_in(
            &prompt,
            window,
            |this, state, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { shift: false, .. } = event {
                    let text = state.read(cx).value();
                    this.submit(text, window, cx);
                }
            },
        )];

        Self {
            focus_handle: cx.focus_handle(),
            open: HashSet::from([TOOL, PLAN, TASK, TERMINAL]),
            prompt,
            status: PromptStatus::Ready,
            model: "gpt-5".into(),
            effort: "medium".into(),
            confirmation: ConfirmationState::Requested,
            collapsed_folders: Vec::new(),
            selected_file: Some("crates/component/src/ai/mod.rs".into()),
            sent: Vec::new(),
            _subscriptions,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn is_open(&self, key: &'static str) -> bool {
        self.open.contains(key)
    }

    fn set_open(&mut self, key: &'static str, open: bool, cx: &mut Context<Self>) {
        if open {
            self.open.insert(key);
        } else {
            self.open.remove(key);
        }
        cx.notify();
    }

    fn toggle_handler(
        key: &'static str,
        cx: &mut Context<Self>,
    ) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
        let view = cx.entity().downgrade();
        move |open, _, cx| {
            let open = *open;
            _ = view.update(cx, |this, cx| this.set_open(key, open, cx));
        }
    }

    fn submit(&mut self, text: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if text.trim().is_empty() || self.status != PromptStatus::Ready {
            return;
        }
        self.sent.push(text);
        self.status = PromptStatus::Streaming;
        self.prompt
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    fn render_chat(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let citation = InlineCitation::new("citation", "1")
            .source(
                Source::new("GPUI Kit", "https://gpui-kit.com")
                    .description("A Rust desktop application framework built on GPUI."),
            )
            .source(Source::new("Zed", "https://zed.dev"));

        section("Chat")
            .description("A conversation composed from Message, Reasoning, Tool, Sources and Prompt Input.")
            .max_w(rems(48.))
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .child(
                        Message::new().alignment(MessageAlignment::End).content(
                            MessageContent::new()
                                .bubble(Bubble::new().child("What is GPUI Kit built on?")),
                        ),
                    )
                    .child(
                        Reasoning::new(REASONING)
                            .duration(Duration::from_secs(6))
                            .open(self.is_open(REASONING))
                            .on_toggle(Self::toggle_handler(REASONING, cx))
                            .child("The user asks about the framework's foundation. Check the docs before answering."),
                    )
                    .child(
                        Tool::new(TOOL, "web_search")
                            .status(TraceStatus::Completed)
                            .input(r#"{ "query": "GPUI Kit framework" }"#)
                            .output(r#"[{ "title": "GPUI Kit", "url": "https://gpui-kit.com" }]"#)
                            .open(self.is_open(TOOL))
                            .on_toggle(Self::toggle_handler(TOOL, cx)),
                    )
                    .child(
                        h_flex()
                            .flex_wrap()
                            .gap_1()
                            .text_sm()
                            .child("GPUI Kit is built on GPUI, the GPU-accelerated UI framework from Zed.")
                            .child(citation),
                    )
                    .child(
                        Sources::new(SOURCES)
                            .source(Source::new("GPUI Kit", "https://gpui-kit.com"))
                            .source(Source::new("Zed", "https://zed.dev"))
                            .open(self.is_open(SOURCES))
                            .on_toggle(Self::toggle_handler(SOURCES, cx)),
                    )
                    .children(self.sent.iter().map(|text| {
                        Message::new().alignment(MessageAlignment::End).content(
                            MessageContent::new().bubble(Bubble::new().child(text.clone())),
                        )
                    }))
                    .child(
                        Suggestions::new()
                            .child(self.suggestion("s1", "Compare with Tauri", cx))
                            .child(self.suggestion("s2", "Show a hello world", cx)),
                    )
                    .child(self.render_prompt(cx)),
            )
    }

    fn suggestion(
        &self,
        id: &'static str,
        text: &'static str,
        cx: &mut Context<Self>,
    ) -> Suggestion {
        Suggestion::new(id, text)
            .disabled(self.status != PromptStatus::Ready)
            .on_click(cx.listener(|this, text: &SharedString, window, cx| {
                this.submit(text.clone(), window, cx)
            }))
    }

    fn render_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        PromptInput::new("prompt", &self.prompt)
            .status(self.status)
            .tool(
                Button::new("attach")
                    .ghost()
                    .small()
                    .icon(IconName::Paperclip)
                    .accessibility_label("Attach files"),
            )
            .tool(
                ModelSelector::new("model")
                    .models([
                        ModelOption::new("gpt-5", "GPT-5"),
                        ModelOption::new("gpt-5-codex", "GPT-5 Codex"),
                    ])
                    .efforts([
                        ModelOption::new("low", "Low"),
                        ModelOption::new("medium", "Medium"),
                        ModelOption::new("high", "High"),
                    ])
                    .model(self.model.clone())
                    .effort(self.effort.clone())
                    .disabled(self.status != PromptStatus::Ready)
                    .on_model_change(cx.listener(|this, id: &SharedString, _, cx| {
                        this.model = id.clone();
                        cx.notify();
                    }))
                    .on_effort_change(cx.listener(|this, id: &SharedString, _, cx| {
                        this.effort = id.clone();
                        cx.notify();
                    })),
            )
            .tool(
                ContextUsage::new("context", 46_200, 400_000)
                    .input(42_000)
                    .output(3_400)
                    .reasoning(800)
                    .cached(30_000),
            )
            .on_submit(cx.listener(|this, text: &SharedString, window, cx| {
                this.submit(text.clone(), window, cx)
            }))
            .on_stop({
                let view = cx.entity().downgrade();
                move |_, cx| {
                    _ = view.update(cx, |this, cx| {
                        this.status = PromptStatus::Ready;
                        cx.notify();
                    });
                }
            })
    }

    fn render_agent_work(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Agent work")
            .description("Plans, tasks, chains of thought and subagents share one trace row.")
            .max_w(rems(48.))
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        ChainOfThought::new(CHAIN)
                            .open(self.is_open(CHAIN))
                            .on_toggle(Self::toggle_handler(CHAIN, cx))
                            .step(
                                ChainOfThoughtStep::new("Searching the documentation")
                                    .icon(IconName::Search)
                                    .description("gpui-kit.com, docs.rs"),
                            )
                            .step(ChainOfThoughtStep::new("Reading the architecture notes"))
                            .step(
                                ChainOfThoughtStep::new("Drafting the answer")
                                    .status(TraceStatus::Running),
                            ),
                    )
                    .child(
                        Plan::new(PLAN)
                            .title("Add AI elements")
                            .open(self.is_open(PLAN))
                            .on_toggle(Self::toggle_handler(PLAN, cx))
                            .steps([
                                PlanStep::new("Read the design guides")
                                    .status(TraceStatus::Completed),
                                PlanStep::new("Build the trace row").status(TraceStatus::Completed),
                                PlanStep::new("Compose the elements").status(TraceStatus::Running),
                                PlanStep::new("Write the gallery page"),
                            ]),
                    )
                    .child(
                        AgentTask::new(TASK, "Searching the codebase")
                            .status(TraceStatus::Completed)
                            .open(self.is_open(TASK))
                            .on_toggle(Self::toggle_handler(TASK, cx))
                            .item(
                                AgentTaskItem::new("Read").file("crates/component/src/message.rs"),
                            )
                            .item(
                                AgentTaskItem::new("Read")
                                    .file("crates/component/src/collapsible.rs"),
                            )
                            .item(AgentTaskItem::new("Found 2 composable building blocks")),
                    )
                    .child(
                        Subagent::new(SUBAGENT, "Explorer")
                            .task("Find where themes are loaded")
                            .status(TraceStatus::Completed)
                            .open(self.is_open(SUBAGENT))
                            .on_toggle(Self::toggle_handler(SUBAGENT, cx))
                            .child(
                                Tool::new("subagent-grep", "grep")
                                    .status(TraceStatus::Completed)
                                    .input(r#"{ "pattern": "Theme::change" }"#),
                            )
                            .result("Themes load in crates/story/src/themes.rs."),
                    )
                    .child(
                        Tool::new(FAILED_TOOL, "read_file")
                            .input(r#"{ "path": "missing.rs" }"#)
                            .error("No such file or directory (os error 2)")
                            .open(self.is_open(FAILED_TOOL))
                            .on_toggle(Self::toggle_handler(FAILED_TOOL, cx)),
                    ),
            )
    }

    fn render_execution(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Execution")
            .description("Commands, test runs and failures report what actually happened.")
            .max_w(rems(48.))
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        Terminal::new(TERMINAL, "cargo test -p gpui-component ai::")
                            .output("running 30 tests\n...\ntest result: ok. 30 passed; 0 failed")
                            .exit_code(0)
                            .open(self.is_open(TERMINAL))
                            .on_toggle(Self::toggle_handler(TERMINAL, cx)),
                    )
                    .child(
                        TestResults::new(TESTS)
                            .open(self.is_open(TESTS))
                            .on_toggle(Self::toggle_handler(TESTS, cx))
                            .suite(
                                TestSuite::new("ai::plan")
                                    .case(TestCase::new("test_plan_builder").duration(Duration::from_millis(2)))
                                    .case(TestCase::new("test_aggregate_status").duration(Duration::from_millis(1))),
                            )
                            .suite(
                                TestSuite::new("ai::stack_trace")
                                    .case(TestCase::new("test_parse_rust").failure("assertion `left == right` failed\n  left: 1\n right: 2"))
                                    .case(TestCase::new("test_parse_go").status(TraceStatus::Skipped)),
                            ),
                    )
                    .child(
                        StackTrace::parse(
                            STACK,
                            "TypeError: Cannot read properties of undefined (reading 'map')\n    at renderList (/app/src/list.ts:42:17)\n    at render (/app/src/view.ts:12:5)\n    at /app/node_modules/react-dom/index.js:3:1",
                        )
                        .open(self.is_open(STACK))
                        .on_toggle(Self::toggle_handler(STACK, cx)),
                    ),
            )
    }

    fn render_review(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Review")
            .description("Approvals, changed files, the to-do queue and checkpoints.")
            .max_w(rems(48.))
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .child(
                        Confirmation::new("confirmation", "Run “cargo publish”?")
                            .description("Publishes gpui-component 0.8.0 to crates.io. This cannot be undone.")
                            .approve_label("Publish")
                            .state(self.confirmation)
                            .on_approve(cx.listener(|this, _, _, cx| {
                                this.confirmation = ConfirmationState::Approved;
                                cx.notify();
                            }))
                            .on_deny(cx.listener(|this, _, _, cx| {
                                this.confirmation = ConfirmationState::Denied;
                                cx.notify();
                            })),
                    )
                    .when(self.confirmation != ConfirmationState::Requested, |this| {
                        this.child(
                            Button::new("reset-confirmation")
                                .small()
                                .label("Ask again")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.confirmation = ConfirmationState::Requested;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        FileTree::new("files")
                            .entries([
                                FileTreeEntry::new("crates/component/src/ai/mod.rs")
                                    .change(FileChange::Added)
                                    .line_counts(42, 0),
                                FileTreeEntry::new("crates/component/src/ai/trace.rs")
                                    .change(FileChange::Added)
                                    .line_counts(280, 0),
                                FileTreeEntry::new("crates/component/src/lib.rs").line_counts(1, 0),
                                FileTreeEntry::new("docs/OLD.md").change(FileChange::Deleted),
                            ])
                            .collapsed(self.collapsed_folders.clone())
                            .selected(self.selected_file.clone())
                            .on_toggle_folder(cx.listener(|this, path: &SharedString, _, cx| {
                                if let Some(ix) = this.collapsed_folders.iter().position(|p| p == path) {
                                    this.collapsed_folders.remove(ix);
                                } else {
                                    this.collapsed_folders.push(path.clone());
                                }
                                cx.notify();
                            }))
                            .on_select(cx.listener(|this, path: &SharedString, _, cx| {
                                this.selected_file = Some(path.clone());
                                cx.notify();
                            })),
                    )
                    .child(
                        Queue::new("queue")
                            .items([
                                QueueItem::new("q1", "Read the issue").done(true),
                                QueueItem::new("q2", "Reproduce the crash").done(true),
                                QueueItem::new("q3", "Write the fix").description("crates/component/src/input"),
                                QueueItem::new("q4", "Add a regression test"),
                            ])
                            .done_open(self.is_open(QUEUE_DONE))
                            .on_toggle_done(Self::toggle_handler(QUEUE_DONE, cx)),
                    )
                    .child(
                        Checkpoint::new("checkpoint")
                            .label("Before refactor")
                            .on_restore(|_, _, _| {}),
                    ),
            )
    }
}

impl Story for AiElementsStory {
    fn title() -> &'static str {
        "AI Elements"
    }

    fn description() -> &'static str {
        "Reasoning, tool calls, plans, approvals, sources and the prompt composer for agent interfaces."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for AiElementsStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AiElementsStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .text_color(cx.theme().foreground)
            .child(self.render_chat(cx))
            .child(self.render_agent_work(cx))
            .child(self.render_execution(cx))
            .child(self.render_review(cx))
    }
}

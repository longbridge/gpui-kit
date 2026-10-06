use gpui_kit::component::{
    ActiveTheme, Disableable, IconName, Selectable, Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    diff::{
        Diff, DiffDocument, DiffFile, DiffLineAnnotation, DiffLinePosition, DiffMode, DiffSide,
        DiffState,
    },
    h_flex, v_flex,
};
use gpui_kit::{
    Action, App, AppContext as _, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, Window, div, prelude::FluentBuilder as _, rems,
};
use serde::Deserialize;

use crate::story_toolbar_group;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = diff_story, no_json)]
enum DiffStoryAction {
    Example(usize),
    Context(Option<usize>),
    Expand,
    Collapse,
}

const EXAMPLES: [&str; 9] = [
    "Code review",
    "Added file",
    "Deleted file",
    "Unicode",
    "Long lines",
    "Large file",
    "Identical files",
    "Line endings",
    "Empty files",
];

pub struct DiffStory {
    state: Entity<DiffState>,
    example: usize,
    annotations: Vec<DiffLineAnnotation>,
    comment_resolved: bool,
    layout_revision: u64,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for DiffStory {
    fn title() -> &'static str {
        "Diff"
    }

    fn description() -> &'static str {
        "Readonly code review with split and unified layouts, inline changes and expandable context."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DiffStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let state = cx.new(|cx| DiffState::new(example_document(0), cx));
            let subscription = cx.observe(&state, |_, _, cx| cx.notify());
            Self {
                state,
                example: 0,
                annotations: vec![DiffLineAnnotation::new(
                    "retry-delay-review",
                    DiffLinePosition::new(DiffSide::Modified, 21),
                )],
                comment_resolved: false,
                layout_revision: 0,
                _subscriptions: vec![subscription],
            }
        })
    }

    fn on_action(&mut self, action: &DiffStoryAction, window: &mut Window, cx: &mut Context<Self>) {
        match *action {
            DiffStoryAction::Example(example) => {
                self.example = example;
                self.comment_resolved = false;
                self.layout_revision += 1;
                let document = example_document(example);
                self.state
                    .update(cx, |state, cx| state.set_document(document, window, cx));
            }
            DiffStoryAction::Context(lines) => {
                self.state
                    .update(cx, |state, cx| state.set_context_lines(lines, cx));
            }
            DiffStoryAction::Expand => self.state.update(cx, |state, cx| state.expand_all(cx)),
            DiffStoryAction::Collapse => self.state.update(cx, |state, cx| state.collapse_all(cx)),
        }
        cx.notify();
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let example = self.example;
        let mode = self.state.read(cx).mode();
        let context = self.state.read(cx).context_lines();
        let has_changes = self.state.read(cx).document().has_changes();
        h_flex()
            .w_full()
            .gap_2()
            .child(
                story_toolbar_group()
                    .w_auto()
                    .child(
                        Button::new("diff-unified")
                            .label("Unified")
                            .selected(mode == DiffMode::Unified)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state
                                    .update(cx, |state, cx| state.set_mode(DiffMode::Unified, cx));
                            })),
                    )
                    .child(
                        Button::new("diff-split")
                            .label("Split")
                            .selected(mode == DiffMode::Split)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state
                                    .update(cx, |state, cx| state.set_mode(DiffMode::Split, cx));
                            })),
                    ),
            )
            .child(
                story_toolbar_group()
                    .w_auto()
                    .child(
                        Button::new("diff-previous")
                            .icon(IconName::ArrowUp)
                            .accessibility_label("Previous change")
                            .tooltip("Previous change · Shift+F7")
                            .disabled(!has_changes)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |state, cx| state.previous_change(cx));
                            })),
                    )
                    .child(
                        Button::new("diff-next")
                            .icon(IconName::ArrowDown)
                            .accessibility_label("Next change")
                            .tooltip("Next change · F7")
                            .disabled(!has_changes)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |state, cx| state.next_change(cx));
                            })),
                    ),
            )
            .child(div().flex_1())
            .child(story_toolbar_group().w_auto().dropdown_child(
                Button::new("diff-options").label("Options"),
                move |menu, window, cx| {
                    menu.submenu("Example", window, cx, move |menu, _, _| {
                        EXAMPLES
                            .into_iter()
                            .enumerate()
                            .fold(menu, |menu, (index, label)| {
                                menu.menu_with_check(
                                    label,
                                    example == index,
                                    Box::new(DiffStoryAction::Example(index)),
                                )
                            })
                    })
                    .submenu("Context", window, cx, move |menu, _, _| {
                        menu.menu_with_check(
                            "Three lines",
                            context == Some(3),
                            Box::new(DiffStoryAction::Context(Some(3))),
                        )
                        .menu_with_check(
                            "All lines",
                            context.is_none(),
                            Box::new(DiffStoryAction::Context(None)),
                        )
                        .separator()
                        .menu("Expand all", Box::new(DiffStoryAction::Expand))
                        .menu("Collapse all", Box::new(DiffStoryAction::Collapse))
                    })
                },
            ))
    }
}

impl Render for DiffStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let story = cx.entity().downgrade();
        let resolved = self.comment_resolved;
        v_flex()
            .w_full()
            .gap_3()
            .on_action(cx.listener(Self::on_action))
            .child(self.render_toolbar(cx))
            .child(v_flex().w_full().gap_2()
                .child(div().font_medium().child(EXAMPLES[self.example]))
                .child(
                Diff::new(&self.state).w_full().h(rems(32.))
                .with_layout_revision(self.layout_revision)
                .when(self.example == 0, |diff| {
                    diff.with_annotations(self.annotations.clone())
                        .render_header_metadata(|_, _, cx| {
                            div().text_xs().text_color(cx.theme().muted_foreground)
                                .child("main → retry-backoff")
                        })
                        .render_annotation(move |annotation, _, cx| {
                            let story = story.clone();
                            v_flex().w_full().gap_2()
                                .child(h_flex().w_full().justify_between().gap_2()
                                    .child(h_flex().gap_2()
                                        .child(div().text_sm().font_medium().child("Alex"))
                                        .child(div().text_xs().text_color(cx.theme().muted_foreground)
                                            .child(if resolved { "Resolved" } else { "Review comment" })))
                                    .child(Button::new(annotation.id().clone())
                                        .w_auto().ghost().xsmall()
                                        .label(if resolved { "Reopen" } else { "Resolve" })
                                        .on_click(move |_, _, cx| {
                                            let _ = story.update(cx, |this, cx| {
                                                this.comment_resolved = !this.comment_resolved;
                                                this.layout_revision += 1;
                                                cx.notify();
                                            });
                                        })))
                                .when(!resolved, |this| this.child(div().text_sm()
                                    .child("Should callers be able to configure the 6.4-second delay cap?")))
                        })
                })))
    }
}

fn example_document(example: usize) -> DiffDocument {
    match example {
        1 => DiffDocument::added(DiffFile::new("src/retry.rs", ADDED).with_language("rust")),
        2 => DiffDocument::deleted(
            DiffFile::new("src/legacy_retry.rs", DELETED).with_language("rust"),
        ),
        3 => compare(
            "src/greeting.rs",
            "// Greeting for every visitor\nfn greeting(name: &str) -> String {\n\tformat!(\"Hello, {name} 👋\")\n}\n\n// café: e\u{301}, 中文，مرحبا\n",
            "// 为每位访客生成问候\nfn greeting(name: &str) -> String {\n\tformat!(\"你好，{name} 🌏\")\n}\n\n// café: é, 中文，مرحبا\n",
        ),
        4 => {
            let prefix = "const ENDPOINT: &str = \"https://api.example.com/v1/changes?";
            compare(
                "src/endpoint.rs",
                &format!("{prefix}{}&limit=20\";\n", "scope=repository&".repeat(24)),
                &format!("{prefix}{}&limit=100\";\n", "scope=repository&".repeat(24)),
            )
        }
        5 => {
            let original = (1..=5000)
                .map(|line| format!("entry_{line:04} = enabled\n"))
                .collect::<String>();
            let modified = original
                .replace("entry_0040 = enabled", "entry_0040 = disabled")
                .replace("entry_2500 = enabled", "entry_2500 = pending")
                .replace("entry_4960 = enabled", "entry_4960 = disabled");
            compare("config/features.conf", &original, &modified)
        }
        6 => compare("src/retry.rs", ADDED, ADDED),
        7 => compare(
            "config/review.conf",
            "enabled=true\r\nlimit=20\r\n",
            "enabled=true\nlimit=20",
        ),
        8 => compare("empty.txt", "", ""),
        _ => compare("src/retry.rs", REVIEW_ORIGINAL, REVIEW_MODIFIED),
    }
}

fn compare(name: &'static str, original: &str, modified: &str) -> DiffDocument {
    DiffDocument::new(
        DiffFile::new(name, original.to_owned()),
        DiffFile::new(name, modified.to_owned()),
    )
}

const ADDED: &str =
    "pub fn retry_delay(attempt: u32) -> u64 {\n    100 * 2_u64.pow(attempt.min(6))\n}\n";
const DELETED: &str = "pub fn retry_delay(_: u32) -> u64 {\n    1000\n}\n";
const REVIEW_ORIGINAL: &str = r#"use std::time::Duration;

/// Retry policy for network requests.
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
        }
    }
}

impl RetryPolicy {
    /// Returns the delay before the next attempt.
    pub fn delay(&self, attempt: u32) -> Duration {
        self.base_delay * attempt
    }

    /// Whether another request may be attempted.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

pub fn describe(policy: &RetryPolicy) -> String {
    format!("{} attempts", policy.max_attempts)
}
"#;
const REVIEW_MODIFIED: &str = r#"use std::time::Duration;

/// Retry policy for network requests.
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
        }
    }
}

impl RetryPolicy {
    /// Returns the delay before the next attempt.
    pub fn delay(&self, attempt: u32) -> Duration {
        self.base_delay * 2_u32.pow(attempt.min(6))
    }

    /// Whether another request may be attempted.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

pub fn describe(policy: &RetryPolicy) -> String {
    format!("Up to {} attempts", policy.max_attempts)
}
"#;

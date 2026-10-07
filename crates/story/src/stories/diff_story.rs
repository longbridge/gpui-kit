use gpui_kit::component::{
    ActiveTheme, Disableable, IconName, Selectable, Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    diff::{
        Diff, DiffAnnotation, DiffChangeIndicator, DiffFile, DiffHoverHighlight, DiffHunkSeparator,
        DiffInlineUnit, DiffLinePosition, DiffLineRange, DiffMode, DiffSide, DiffState,
    },
    h_flex, v_flex,
};
use gpui_kit::{
    Action, App, AppContext as _, Context, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window, div,
    prelude::FluentBuilder as _, rems,
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
    Inline(usize),
    Separator(usize),
    Indicator(usize),
    Background,
    Hover,
    Wrap,
}

const EXAMPLES: [&str; 11] = [
    "Pull request",
    "Added file",
    "Deleted file",
    "Unicode",
    "Long lines",
    "Large file",
    "Renamed file",
    "Missing final newline",
    "File mode",
    "Merge conflict",
    "Source file",
];

const INLINE: [(&str, Option<DiffInlineUnit>); 3] = [
    ("Words", Some(DiffInlineUnit::Word)),
    ("Characters", Some(DiffInlineUnit::Character)),
    ("Whole lines", None),
];
const SEPARATORS: [(&str, DiffHunkSeparator); 3] = [
    ("Hunk header", DiffHunkSeparator::Metadata),
    ("Hidden line count", DiffHunkSeparator::LineInfo),
    ("Divider", DiffHunkSeparator::Simple),
];
const INDICATORS: [(&str, DiffChangeIndicator); 3] = [
    ("Signs", DiffChangeIndicator::Signs),
    ("Bars", DiffChangeIndicator::Bars),
    ("None", DiffChangeIndicator::None),
];

/// An application-owned review comment anchored to the end of its lines.
struct Comment {
    id: ElementId,
    range: DiffLineRange,
    body: SharedString,
    resolved: bool,
}

pub struct DiffStory {
    state: Entity<DiffState>,
    example: usize,
    comments: Vec<Comment>,
    inline: usize,
    separator: usize,
    indicator: usize,
    background: bool,
    hover: bool,
    wrap: bool,
    _subscription: Subscription,
}

impl super::Story for DiffStory {
    fn title() -> &'static str {
        "Diff"
    }

    fn description() -> &'static str {
        "Readonly unified and Git patches, merge conflicts and source files, with review comments."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DiffStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let state = cx.new(|cx| DiffState::new(example_files(0), cx));
            Self {
                _subscription: cx.observe(&state, |_, _, cx| cx.notify()),
                state,
                example: 0,
                comments: vec![Comment {
                    id: ElementId::from("retry-delay-review"),
                    range: DiffLineRange::new("src/retry.rs", DiffSide::Modified, 21, 21),
                    body: "Should callers be able to configure the 6.4-second delay cap?".into(),
                    resolved: false,
                }],
                inline: 0,
                separator: 0,
                indicator: 0,
                background: true,
                hover: false,
                wrap: false,
            }
        })
    }

    /// Rebuilds the state when an option prepared with it changes.
    fn rebuild_state(&mut self, cx: &mut Context<Self>) {
        let mode = self.state.read(cx).mode();
        let inline = INLINE[self.inline].1;
        let files = example_files(self.example);
        self.state = cx.new(|cx| {
            DiffState::new(files, cx)
                .with_mode(mode)
                .with_inline_unit(inline)
        });
        self._subscription = cx.observe(&self.state, |_, _, cx| cx.notify());
    }

    fn on_action(&mut self, action: &DiffStoryAction, _: &mut Window, cx: &mut Context<Self>) {
        match *action {
            DiffStoryAction::Example(example) => {
                self.example = example;
                let files = example_files(example);
                self.state
                    .update(cx, |state, cx| state.set_files(files, cx));
            }
            DiffStoryAction::Context(lines) => {
                self.state
                    .update(cx, |state, cx| state.set_context_lines(lines, cx));
            }
            DiffStoryAction::Expand => self
                .state
                .update(cx, |state, cx| state.expand_unchanged(cx)),
            DiffStoryAction::Collapse => self
                .state
                .update(cx, |state, cx| state.collapse_unchanged(cx)),
            DiffStoryAction::Inline(inline) => {
                self.inline = inline;
                self.rebuild_state(cx);
            }
            DiffStoryAction::Separator(separator) => self.separator = separator,
            DiffStoryAction::Indicator(indicator) => self.indicator = indicator,
            DiffStoryAction::Background => self.background = !self.background,
            DiffStoryAction::Hover => self.hover = !self.hover,
            DiffStoryAction::Wrap => self.wrap = !self.wrap,
        }
        cx.notify();
    }

    fn add_comment(&mut self, range: &DiffLineRange, cx: &mut Context<Self>) {
        let (start, end) = (
            range.start().min(range.end()),
            range.start().max(range.end()),
        );
        let lines = if start == end {
            format!("line {end}")
        } else {
            format!("lines {start}–{end}")
        };
        self.comments.push(Comment {
            id: ElementId::from(SharedString::from(format!(
                "comment-{}",
                self.comments.len()
            ))),
            range: range.clone(),
            body: format!("New comment on {lines}.").into(),
            resolved: false,
        });
        cx.notify();
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let example = self.example;
        let mode = self.state.read(cx).mode();
        let context = self.state.read(cx).context_lines();
        let (inline, separator, indicator) = (self.inline, self.separator, self.indicator);
        let (background, hover, wrap) = (self.background, self.hover, self.wrap);
        let has_changes = self
            .state
            .read(cx)
            .files()
            .iter()
            .any(|file| file.has_changes());
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
                            .tooltip("Previous change")
                            .disabled(!has_changes)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |state, cx| state.previous_change(cx));
                            })),
                    )
                    .child(
                        Button::new("diff-next")
                            .icon(IconName::ArrowDown)
                            .accessibility_label("Next change")
                            .tooltip("Next change")
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
                            "Three context lines",
                            context == Some(3),
                            Box::new(DiffStoryAction::Context(Some(3))),
                        )
                        .menu_with_check(
                            "All supplied context",
                            context.is_none(),
                            Box::new(DiffStoryAction::Context(None)),
                        )
                        .separator()
                        .menu("Expand all", Box::new(DiffStoryAction::Expand))
                        .menu("Collapse all", Box::new(DiffStoryAction::Collapse))
                    })
                    .submenu("Inline changes", window, cx, move |menu, _, _| {
                        INLINE
                            .into_iter()
                            .enumerate()
                            .fold(menu, |menu, (ix, (label, _))| {
                                menu.menu_with_check(
                                    label,
                                    inline == ix,
                                    Box::new(DiffStoryAction::Inline(ix)),
                                )
                            })
                    })
                    .submenu("Hunk separator", window, cx, move |menu, _, _| {
                        SEPARATORS
                            .into_iter()
                            .enumerate()
                            .fold(menu, |menu, (ix, (label, _))| {
                                menu.menu_with_check(
                                    label,
                                    separator == ix,
                                    Box::new(DiffStoryAction::Separator(ix)),
                                )
                            })
                    })
                    .submenu("Change indicator", window, cx, move |menu, _, _| {
                        INDICATORS
                            .into_iter()
                            .enumerate()
                            .fold(menu, |menu, (ix, (label, _))| {
                                menu.menu_with_check(
                                    label,
                                    indicator == ix,
                                    Box::new(DiffStoryAction::Indicator(ix)),
                                )
                            })
                    })
                    .separator()
                    .menu_with_check(
                        "Tint changed lines",
                        background,
                        Box::new(DiffStoryAction::Background),
                    )
                    .menu_with_check(
                        "Highlight hovered line",
                        hover,
                        Box::new(DiffStoryAction::Hover),
                    )
                    .menu_with_check(
                        "Wrap long lines",
                        wrap,
                        Box::new(DiffStoryAction::Wrap),
                    )
                },
            ))
    }
}

impl Render for DiffStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let annotations = self
            .comments
            .iter()
            .map(|comment| {
                DiffAnnotation::line(
                    comment.id.clone(),
                    DiffLinePosition::new(
                        comment.range.path().clone(),
                        comment.range.end_side(),
                        comment.range.end(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        // The renderer receives `App`, so it reads a snapshot of the comments.
        let comments = std::rc::Rc::new(
            self.comments
                .iter()
                .map(|comment| (comment.id.clone(), comment.body.clone(), comment.resolved))
                .collect::<Vec<_>>(),
        );
        let story = cx.entity().downgrade();
        let add_story = story.clone();
        v_flex()
            .w_full()
            .gap_3()
            .on_action(cx.listener(Self::on_action))
            .child(self.render_toolbar(cx))
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(div().font_medium().child(EXAMPLES[self.example]))
                    .child(
                        Diff::new(&self.state)
                            .w_full()
                            .h(rems(32.))
                            .hunk_separator(SEPARATORS[self.separator].1)
                            .change_indicator(INDICATORS[self.indicator].1)
                            .change_background(self.background)
                            .hover_highlight(if self.hover {
                                DiffHoverHighlight::Both
                            } else {
                                DiffHoverHighlight::None
                            })
                            .soft_wrap(self.wrap)
                            .when(self.example == 0, |diff| {
                                diff.annotations(annotations)
                                    .render_annotation(move |annotation, _, cx| {
                                        let (id, body, resolved) = comments
                                            .iter()
                                            .find(|(id, ..)| id == annotation.id())
                                            .cloned()
                                            .unwrap_or_else(|| {
                                                (
                                                    annotation.id().clone(),
                                                    SharedString::default(),
                                                    false,
                                                )
                                            });
                                        let story = story.clone();
                                        v_flex()
                                            .w_full()
                                            .gap_2()
                                            .child(
                                                h_flex()
                                                    .w_full()
                                                    .justify_between()
                                                    .gap_2()
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_medium()
                                                                    .child("Alex"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(
                                                                        cx.theme().muted_foreground,
                                                                    )
                                                                    .child(if resolved {
                                                                        "Resolved"
                                                                    } else {
                                                                        "Review comment"
                                                                    }),
                                                            ),
                                                    )
                                                    .child(
                                                        Button::new(id.clone())
                                                            .w_auto()
                                                            .ghost()
                                                            .xsmall()
                                                            .label(if resolved {
                                                                "Reopen"
                                                            } else {
                                                                "Resolve"
                                                            })
                                                            .on_click(move |_, _, cx| {
                                                                let id = id.clone();
                                                                let _ =
                                                                    story.update(cx, |this, cx| {
                                                                        if let Some(comment) = this
                                                                            .comments
                                                                            .iter_mut()
                                                                            .find(|comment| {
                                                                                comment.id == id
                                                                            })
                                                                        {
                                                                            comment.resolved =
                                                                                !comment.resolved;
                                                                        }
                                                                        cx.notify();
                                                                    });
                                                            }),
                                                    ),
                                            )
                                            .when(!resolved, |this| {
                                                this.child(div().text_sm().child(body))
                                            })
                                    })
                                    .on_add_annotation(move |range, _, cx| {
                                        let _ = add_story
                                            .update(cx, |this, cx| this.add_comment(range, cx));
                                    })
                            }),
                    ),
            )
    }
}

fn example_files(example: usize) -> Vec<DiffFile> {
    let patch = match example {
        1 => ADDED_PATCH.to_owned(),
        2 => DELETED_PATCH.to_owned(),
        3 => UNICODE_PATCH.to_owned(),
        4 => {
            let prefix = "const ENDPOINT: &str = \"https://api.example.com/v1/changes?";
            format!(
                "--- a/src/endpoint.rs\n+++ b/src/endpoint.rs\n@@ -1 +1 @@\n-{prefix}{}&limit=20\";\n+{prefix}{}&limit=100\";\n",
                "scope=repository&".repeat(24),
                "scope=repository&".repeat(24),
            )
        }
        5 => {
            // The fixture is a supplied patch: no source comparison is performed.
            let mut patch = String::from(
                "--- a/config/features.conf\n+++ b/config/features.conf\n@@ -1,5000 +1,5000 @@\n",
            );
            for line in 1..=5000 {
                match line {
                    40 | 4960 => patch.push_str(&format!(
                        "-entry_{line:04} = enabled\n+entry_{line:04} = disabled\n"
                    )),
                    2500 => patch.push_str(&format!(
                        "-entry_{line:04} = enabled\n+entry_{line:04} = pending\n"
                    )),
                    _ => patch.push_str(&format!(" entry_{line:04} = enabled\n")),
                }
            }
            patch
        }
        6 => RENAMED_PATCH.to_owned(),
        7 => NO_NEWLINE_PATCH.to_owned(),
        8 => MODE_PATCH.to_owned(),
        9 => {
            return vec![
                DiffFile::parse_conflicts("src/retry.rs", MERGE_CONFLICT)
                    .expect("Story conflict is valid"),
            ];
        }
        10 => return vec![DiffFile::unchanged("src/retry.rs", SOURCE_FILE)],
        // A pull request touches several files; they share one scrolling list.
        _ => [REVIEW_PATCH, ADDED_PATCH, DELETED_PATCH, MODE_PATCH].concat(),
    };
    DiffFile::parse(&patch).expect("Story patch is valid")
}

const MERGE_CONFLICT: &str = "impl Default for RetryPolicy {\n    fn default() -> Self {\n        Self {\n<<<<<<< HEAD\n            max_attempts: 5,\n            base_delay: Duration::from_millis(100),\n=======\n            max_attempts: 3,\n            base_delay: Duration::from_millis(250),\n>>>>>>> retry-backoff\n        }\n    }\n}\n";
const SOURCE_FILE: &str = "use std::time::Duration;\n\n/// How often a failed request is retried.\npub struct RetryPolicy {\n    pub max_attempts: u32,\n    pub base_delay: Duration,\n}\n\nimpl RetryPolicy {\n    /// Returns the delay before the next attempt.\n    pub fn delay(&self, attempt: u32) -> Duration {\n        self.base_delay * 2_u32.pow(attempt.min(6))\n    }\n}\n";
const ADDED_PATCH: &str = r#"diff --git a/src/retry.rs b/src/retry.rs
new file mode 100644
--- /dev/null
+++ b/src/retry.rs
@@ -0,0 +1,3 @@
+pub fn retry_delay(attempt: u32) -> u64 {
+    100 * 2_u64.pow(attempt.min(6))
+}
"#;
const DELETED_PATCH: &str = r#"diff --git a/src/legacy_retry.rs b/src/legacy_retry.rs
deleted file mode 100644
--- a/src/legacy_retry.rs
+++ /dev/null
@@ -1,3 +0,0 @@
-pub fn retry_delay(_: u32) -> u64 {
-    1000
-}
"#;
const UNICODE_PATCH: &str = "--- a/src/greeting.rs\n+++ b/src/greeting.rs\n@@ -1,6 +1,6 @@\n-// Greeting for every visitor\n+// 为每位访客生成问候\n fn greeting(name: &str) -> String {\n-\tformat!(\"Hello, {name} 👋\")\n+\tformat!(\"你好，{name} 🌏\")\n }\n \n-// café: e\u{301}, 中文，مرحبا\n+// café: é, 中文，مرحبا\n";
const RENAMED_PATCH: &str = r#"diff --git a/src/retry.rs b/src/retry_policy.rs
similarity index 100%
rename from src/retry.rs
rename to src/retry_policy.rs
"#;
const NO_NEWLINE_PATCH: &str = r#"--- a/config/review.conf
+++ b/config/review.conf
@@ -1,2 +1,2 @@
 enabled=true
-limit=20
+limit=100
\ No newline at end of file
"#;
const MODE_PATCH: &str = r#"diff --git a/scripts/review.sh b/scripts/review.sh
old mode 100644
new mode 100755
"#;
const REVIEW_PATCH: &str = "diff --git a/src/retry.rs b/src/retry.rs\nindex 30a471b..604db2a 100644\n--- a/src/retry.rs\n+++ b/src/retry.rs\n@@ -9,7 +9,7 @@ impl Default for RetryPolicy {\n impl Default for RetryPolicy {\n     fn default() -> Self {\n         Self {\n-            max_attempts: 3,\n+            max_attempts: 5,\n             base_delay: Duration::from_millis(100),\n         }\n     }\n@@ -18,7 +18,7 @@ impl RetryPolicy {\n impl RetryPolicy {\n     /// Returns the delay before the next attempt.\n     pub fn delay(&self, attempt: u32) -> Duration {\n-        self.base_delay * attempt\n+        self.base_delay * 2_u32.pow(attempt.min(6))\n     }\n \n     /// Whether another request may be attempted.\n@@ -29,4 +29,4 @@ impl RetryPolicy {\n \n pub fn describe(policy: &RetryPolicy) -> String {\n-    format!(\"{} attempts\", policy.max_attempts)\n+    format!(\"Up to {} attempts\", policy.max_attempts)\n }\n";

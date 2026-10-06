use gpui_kit::component::{
    ActiveTheme, Disableable, IconName, Selectable, Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    diff::{Diff, DiffAnnotation, DiffFile, DiffLinePosition, DiffMode, DiffSide, DiffState},
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
    "Pull request",
    "Added file",
    "Deleted file",
    "Unicode",
    "Long lines",
    "Large file",
    "Renamed file",
    "Missing final newline",
    "File mode",
];

pub struct DiffStory {
    state: Entity<DiffState>,
    example: usize,
    annotations: Vec<DiffAnnotation>,
    comment_resolved: bool,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for DiffStory {
    fn title() -> &'static str {
        "Diff"
    }

    fn description() -> &'static str {
        "Readonly unified and Git patches with split layouts, source line numbers and review comments."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DiffStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let state = cx.new(|cx| DiffState::new(example_documents(0), cx));
            let subscription = cx.observe(&state, |_, _, cx| cx.notify());
            Self {
                state,
                example: 0,
                annotations: vec![DiffAnnotation::line(
                    "retry-delay-review",
                    DiffLinePosition::new("src/retry.rs", DiffSide::Modified, 21),
                )],
                comment_resolved: false,
                _subscriptions: vec![subscription],
            }
        })
    }

    fn on_action(&mut self, action: &DiffStoryAction, _: &mut Window, cx: &mut Context<Self>) {
        match *action {
            DiffStoryAction::Example(example) => {
                self.example = example;
                self.comment_resolved = false;
                let documents = example_documents(example);
                self.state
                    .update(cx, |state, cx| state.set_files(documents, cx));
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
        }
        cx.notify();
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let example = self.example;
        let mode = self.state.read(cx).mode();
        let context = self.state.read(cx).context_lines();
        let has_changes = self
            .state
            .read(cx)
            .files()
            .iter()
            .any(|document| document.has_changes());
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
                .when(self.example == 0, |diff| {
                    diff.annotations(self.annotations.clone())
                        .annotation_content(move |annotation, _, cx| {
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
                                                cx.notify();
                                            });
                                        })))
                                .when(!resolved, |this| this.child(div().text_sm()
                                    .child("Should callers be able to configure the 6.4-second delay cap?")))
                        })
                })))
    }
}

fn example_documents(example: usize) -> Vec<DiffFile> {
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
        // A pull request touches several files; they share one scrolling list.
        _ => [REVIEW_PATCH, ADDED_PATCH, DELETED_PATCH, MODE_PATCH].concat(),
    };
    DiffFile::parse(&patch).expect("Story patch is valid")
}

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

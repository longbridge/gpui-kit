use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, StyledExt as _,
    ai::trace::{ToggleHandler, Trace, TraceStatus},
    h_flex, v_flex,
};

/// One entry of the agent's to-do list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueItem {
    id: SharedString,
    title: SharedString,
    description: Option<SharedString>,
    done: bool,
}

impl QueueItem {
    /// Create a remaining item with a stable id.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            done: false,
        }
    }

    /// Set supporting detail shown under the title.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set whether the item is done.
    pub fn done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    /// The item's stable id.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Whether the item is done.
    pub fn is_done(&self) -> bool {
        self.done
    }
}

/// The agent's to-do list, split into remaining and done.
///
/// Remaining items are always visible. Done items sit in a section the user
/// can reveal; its open state is controlled through [`Self::done_open`].
#[derive(IntoElement)]
pub struct Queue {
    id: ElementId,
    style: StyleRefinement,
    items: Vec<QueueItem>,
    done_open: bool,
    on_toggle_done: Option<ToggleHandler>,
}

impl Queue {
    /// Create an empty queue.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            items: Vec::new(),
            done_open: false,
            on_toggle_done: None,
        }
    }

    /// Add an item.
    pub fn item(mut self, item: QueueItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add several items.
    pub fn items(mut self, items: impl IntoIterator<Item = QueueItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Set whether the done section is revealed.
    pub fn done_open(mut self, open: bool) -> Self {
        self.done_open = open;
        self
    }

    /// Called with the requested open state of the done section.
    pub fn on_toggle_done(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_done = Some(Rc::new(handler));
        self
    }

    fn render_item(item: QueueItem, cx: &App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let status = if item.done {
            TraceStatus::Completed
        } else {
            TraceStatus::Pending
        };

        h_flex()
            .items_start()
            .gap_2()
            .px_2()
            .child(div().pt_0p5().child(status.indicator(cx)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        div()
                            .when(item.done, |this| {
                                this.text_color(muted_foreground).line_through()
                            })
                            .child(item.title),
                    )
                    .when_some(item.description, |this, description| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(muted_foreground)
                                .child(description),
                        )
                    }),
            )
    }
}

impl Styled for Queue {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Queue {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let (done, remaining): (Vec<_>, Vec<_>) =
            self.items.into_iter().partition(|item| item.done);
        let remaining_count = remaining.len();
        let done_count = done.len();

        v_flex()
            .w_full()
            .min_w_0()
            .gap_1p5()
            .text_sm()
            .child(
                div()
                    .px_2()
                    .text_xs()
                    .font_medium()
                    .text_color(muted_foreground)
                    .child(SharedString::from(t!(
                        "Queue.remaining",
                        count = remaining_count
                    ))),
            )
            .children(
                remaining
                    .into_iter()
                    .map(|item| Self::render_item(item, cx)),
            )
            .when(done_count > 0, |this| {
                this.child(
                    Trace::new((self.id, "done"))
                        .title(SharedString::from(t!("Queue.done", count = done_count)))
                        .open(self.done_open)
                        .with_toggle_handler(self.on_toggle_done)
                        .content(
                            v_flex()
                                .gap_1p5()
                                .children(done.into_iter().map(|item| Self::render_item(item, cx))),
                        ),
                )
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_builder() {
        let queue = Queue::new("queue")
            .item(QueueItem::new("a", "Read the issue").done(true))
            .items([
                QueueItem::new("b", "Write the fix").description("crates/component"),
                QueueItem::new("c", "Run tests"),
            ])
            .done_open(true)
            .on_toggle_done(|_, _, _| {});

        assert_eq!(queue.items.len(), 3);
        assert!(queue.items[0].is_done());
        assert_eq!(queue.items[1].id(), "b");
        assert!(queue.done_open);
        assert!(queue.on_toggle_done.is_some());
    }
}

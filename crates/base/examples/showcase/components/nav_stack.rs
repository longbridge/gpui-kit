use super::*;
use gpui::{AnyElement, WeakEntity, relative};
use gpui_base::NavPage;
use gpui_base::motion::{PresencePhase, Transition};
use std::time::Duration;

/// One page of the stack. A page knows its depth and holds the stack it lives
/// in, so its own buttons can push over it, replace it, or pop it.
pub(in super::super) struct ShowcasePage {
    depth: usize,
    stack: WeakEntity<NavStackState>,
}

impl ShowcasePage {
    pub(in super::super) fn new(depth: usize, stack: WeakEntity<NavStackState>) -> Self {
        Self { depth, stack }
    }

    /// A click handler that builds a page at `depth` and hands it to `apply`:
    /// a pushed page sits one deeper, a replacement at the same depth.
    fn navigate(
        &self,
        depth: usize,
        apply: impl Fn(&mut NavStackState, gpui::Entity<ShowcasePage>, &mut Context<NavStackState>)
        + 'static,
    ) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static {
        let stack = self.stack.clone();
        move |_, _, cx| {
            _ = stack.update(cx, |state, cx| {
                let page = cx.new(|_| ShowcasePage::new(depth, stack.clone()));
                apply(state, page, cx);
            });
        }
    }
}

impl Render for ShowcasePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let depth = self.depth;
        // The trail is the stack's `History`: the pages behind this one, then
        // the pages popped off it, which `forward` brings back one at a time.
        let (behind, ahead) = self
            .stack
            .upgrade()
            .map(|stack| {
                let stack = stack.read(cx);
                (stack.depth(), stack.forward_views().len())
            })
            .unwrap_or((depth, 0));
        let button = |id: &'static str, label: &'static str| {
            Button::new(id)
                .h_7()
                .px_2()
                .flex()
                .items_center()
                .border_1()
                .border_color(example_rgb(0x171717))
                .bg(example_rgb(0xffffff))
                .child(label)
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .bg(example_rgb(if depth % 2 == 1 {
                0xffffff
            } else {
                0xf5f5f5
            }))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("Page {depth}")),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .text_color(example_rgb(0x737373))
                    .children((1..=behind + ahead).map(|page| {
                        div()
                            .px_1()
                            .when(page == depth, |this| {
                                this.text_color(example_rgb(0x171717))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                            })
                            .when(page > behind, |this| this.text_color(example_rgb(0xd4d4d4)))
                            .child(page.to_string())
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(button("push", "Push").on_click(
                        self.navigate(depth + 1, |stack, page, cx| {
                            stack.push(page, NavMotion::Animated, cx)
                        }),
                    ))
                    .child(button("replace", "Replace").on_click(self.navigate(
                        depth,
                        |stack, page, cx| {
                            stack.replace(page, NavMotion::Animated, cx);
                        },
                    )))
                    .when(depth > 1, |this| {
                        let stack = self.stack.clone();
                        this.child(button("pop", "Pop").on_click(move |_, _, cx| {
                            _ = stack.update(cx, |stack, cx| {
                                stack.pop(NavMotion::Animated, cx);
                            });
                        }))
                    })
                    .when(ahead > 0, |this| {
                        let stack = self.stack.clone();
                        this.child(button("forward", "Forward").on_click(move |_, _, cx| {
                            _ = stack.update(cx, |stack, cx| {
                                stack.forward(NavMotion::Animated, cx);
                            });
                        }))
                    }),
            )
    }
}

impl BaseShowcase {
    pub(in super::super) fn nav_stack(&self) -> impl IntoElement {
        NavStack::new(&self.stack)
            .w_72()
            .h_40()
            .overflow_hidden()
            .border_1()
            .border_color(example_rgb(0xd4d4d4))
            .transition(Transition::new(Duration::from_millis(220)))
            .item(|page, _, _| slide(page))
    }
}

/// A pushed page slides in from the right and slides back out when popped;
/// the page underneath drifts a little to show depth. A replacement slides in
/// over the page it replaces. The showcase's own shell uses this too.
pub(in super::super) fn slide(page: NavPage) -> AnyElement {
    let offset = match (page.phase(), page.operation()) {
        (PresencePhase::Entering, Some(NavOperation::Push | NavOperation::Replace)) => {
            1.0 - page.progress()
        }
        (PresencePhase::Exiting, Some(NavOperation::Pop)) => page.progress(),
        (PresencePhase::Exiting, Some(NavOperation::Push)) => -0.3 * page.progress(),
        (PresencePhase::Entering, Some(NavOperation::Pop)) => -0.3 * (1.0 - page.progress()),
        _ => 0.0,
    };
    page.left(relative(offset)).into_any_element()
}

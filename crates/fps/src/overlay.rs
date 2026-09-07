use gpui::{
    Anchor, App, Entity, IntoElement, ParentElement, Pixels, RenderOnce, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use std::time::Duration;

use crate::monitor::FpsMonitor;

/// Distance from the edges the HUD is pinned to.
const MARGIN: Pixels = px(12.);

/// Pins an [`FpsMonitor`] to an edge or corner of its parent, the way a game
/// overlays its frame counter.
///
/// Most applications want [`fps_monitor`](crate::fps_monitor) instead, which
/// creates and reuses the monitor for you. Reach for this when you already hold
/// a configured [`FpsMonitor`].
///
/// The overlay positions itself absolutely, so **the parent must be
/// `relative()`**:
///
/// ```no_run
/// # use gpui::*;
/// # use gpui_fps::{FpsMonitor, FpsOverlay};
/// # fn example(monitor: &Entity<FpsMonitor>, content: impl IntoElement) -> impl IntoElement {
/// div()
///     .relative()
///     .size_full()
///     .child(content)
///     .child(FpsOverlay::new(monitor).anchor(Anchor::BottomLeft))
/// # }
/// ```
#[derive(IntoElement)]
pub struct FpsOverlay {
    monitor: Entity<FpsMonitor>,
    anchor: Anchor,
    frame_budget: Option<Duration>,
}

impl FpsOverlay {
    pub fn new(monitor: &Entity<FpsMonitor>) -> Self {
        Self {
            monitor: monitor.clone(),
            anchor: Anchor::TopRight,
            frame_budget: None,
        }
    }

    /// Where in the parent the HUD sits. Defaults to [`Anchor::TopRight`].
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// The per-frame budget used for chart grading and its vertical scale.
    pub fn frame_budget(mut self, budget: Duration) -> Self {
        self.frame_budget = Some(budget);
        self
    }
}

impl RenderOnce for FpsOverlay {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(budget) = self.frame_budget {
            self.monitor
                .update(cx, |monitor, _| monitor.set_frame_budget(budget));
        }
        let margin = MARGIN;

        // Corners are placed by their own two offsets so the overlay stays the
        // size of the HUD. The centered anchors need a strip to center within,
        // but it is only stretched along the one axis that needs it, keeping
        // the area laid over the content as small as possible.
        div()
            .absolute()
            .flex()
            .map(|this| match self.anchor {
                Anchor::TopLeft => this.top(margin).left(margin),
                Anchor::TopRight => this.top(margin).right(margin),
                Anchor::BottomLeft => this.bottom(margin).left(margin),
                Anchor::BottomRight => this.bottom(margin).right(margin),
                Anchor::TopCenter => this.top(margin).left_0().right_0().justify_center(),
                Anchor::BottomCenter => this.bottom(margin).left_0().right_0().justify_center(),
                Anchor::LeftCenter => this.left(margin).top_0().bottom_0().items_center(),
                Anchor::RightCenter => this.right(margin).top_0().bottom_0().items_center(),
            })
            .child(self.monitor)
    }
}

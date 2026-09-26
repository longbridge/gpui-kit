//! Hover tracking shared by every [`Plot`]: which datum the cursor is on, how
//! far its hover has faded in, and where a pointer following it has glided to.
//!
//! This is behavior only. A styled layer draws the crosshair, dots and tooltip
//! box, and projects its timing through [`PlotMotion`](crate::PlotMotion).
use gpui::{App, Pixels, Point, Window};

use crate::{
    Spring, Theme,
    motion::{TransitionId, spring, transition},
};

#[derive(Clone)]
pub struct TooltipState {
    pub index: usize,
    pub cross_line: Point<Pixels>,
    pub dots: Vec<Point<Pixels>>,
}

impl TooltipState {
    pub fn new(index: usize, cross_line: Point<Pixels>, dots: Vec<Point<Pixels>>) -> Self {
        Self {
            index,
            cross_line,
            dots,
        }
    }
}

/// The datum a plot has in focus this frame, handed to [`Plot::hover`](super::Plot::hover).
///
/// Carries the [`TooltipState`] the cursor resolved to and how far the hover has
/// faded in. After the cursor leaves, the state lingers here while the focus
/// eases back to zero, so a hover-driven presentation can fade out over the
/// last datum instead of vanishing.
#[derive(Clone)]
pub struct PlotHover {
    state: TooltipState,
    focus: f32,
    hovered: bool,
}

impl PlotHover {
    /// The datum in focus: the one under the cursor, or the last one while the
    /// hover fades out.
    pub fn state(&self) -> &TooltipState {
        &self.state
    }

    /// How far the hover has faded in, from `0` to `1`.
    ///
    /// Rises over the styled layer's fast duration when the cursor lands on a
    /// datum and falls back after it leaves, during which [`Self::is_hovered`]
    /// is false.
    pub fn focus(&self) -> f32 {
        self.focus
    }

    /// Whether the cursor is on the datum, as opposed to the state lingering
    /// while its hover fades out.
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Whether this is the first frame the cursor is on a datum: the hover has
    /// not started fading in yet. A position that follows the hovered datum
    /// adopts it here instead of travelling from where the last hover ended.
    pub fn is_entering(&self) -> bool {
        self.hovered && self.focus == 0.
    }

    /// Follow `target` on the [pointer spring](pointer_spring), adopting the
    /// target on the entering frame instead of travelling from where the last
    /// hover ended.
    ///
    /// For a position a plot paints with, such as the center of a highlighted
    /// band or the crosshair a styled tooltip draws.
    pub fn glide(
        &self,
        id: impl Into<TransitionId>,
        target: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> Pixels {
        let policy = pointer_spring(cx).with_travel(!self.is_entering());
        spring(id, target, policy, window, cx)
    }
}

/// The spring a hover pointer — the crosshair, highlight band or hover dot —
/// follows the hovered datum with: the active [`PlotMotion`](crate::PlotMotion)'s
/// pointer, which snaps unless a styled layer projects one.
pub fn pointer_spring(cx: &App) -> Spring {
    Theme::global(cx).plot.motion().pointer()
}

/// The last datum the cursor resolved to, where the cursor was and how far the
/// hover has faded in, kept in element state so the hover can fade out over it
/// after the cursor leaves and so an overlay can read the fade without being
/// handed it; see [`hover_focus`].
struct HoverMemory {
    state: Option<TooltipState>,
    cursor: Point<Pixels>,
    focus: f32,
    /// Whether this frame is the first the cursor is on a datum; see
    /// [`PlotHover::is_entering`].
    entering: bool,
}

impl Default for HoverMemory {
    fn default() -> Self {
        Self {
            state: None,
            cursor: Point::default(),
            // An overlay rendered outside a plot's tracking is fully opaque.
            focus: 1.,
            entering: false,
        }
    }
}

/// The element-state key of a plot's [`HoverMemory`], within the plot's scope.
const HOVER_MEMORY: &str = "__plot-hover";

/// Resolve the datum a plot shows this frame from the `live` state the cursor
/// resolved to.
///
/// While `live` is `Some` it is shown as is. After the cursor leaves, the last
/// state lingers with its focus easing to zero over the active
/// [`PlotMotion`](crate::PlotMotion)'s exit, then is dropped. Called by
/// [`PlotElement`](super::PlotElement) within the plot's element scope; the
/// returned cursor is the live one, or the last one while the state lingers.
pub(super) fn track_hover(
    live: Option<TooltipState>,
    cursor: Option<Point<Pixels>>,
    window: &mut Window,
    cx: &mut App,
) -> Option<(PlotHover, Point<Pixels>)> {
    let hovered = live.is_some();
    let memory = window.use_keyed_state(HOVER_MEMORY, cx, |_, _| HoverMemory::default());

    let theme = Theme::global(cx);
    let motion = theme.plot.motion();
    let policy = if hovered {
        motion.enter().clone()
    } else {
        motion.exit().clone()
    };
    let focus = transition(
        (HOVER_MEMORY, "focus"),
        if hovered { 1. } else { 0. },
        policy,
        window,
        cx,
    );

    memory.update(cx, |memory, _| {
        if let (Some(live), Some(cursor)) = (live, cursor) {
            memory.state = Some(live);
            memory.cursor = cursor;
        }
        memory.focus = focus;
        memory.entering = hovered && focus == 0.;
        if !hovered && focus <= 0. {
            memory.state = None;
        }
    });

    let memory = memory.read(cx);
    let state = memory.state.clone()?;
    Some((
        PlotHover {
            state,
            focus,
            hovered,
        },
        memory.cursor,
    ))
}

/// How far the enclosing plot's hover has faded in this frame, from `0` to `1`.
///
/// For an overlay a plot returns from [`Plot::tooltip`](super::Plot::tooltip),
/// which renders within the plot's element scope and fades with its hover
/// without being handed the focus. Outside a plot this is `1`.
pub fn hover_focus(window: &mut Window, cx: &mut App) -> f32 {
    window
        .use_keyed_state(HOVER_MEMORY, cx, |_, _| HoverMemory::default())
        .read(cx)
        .focus
}

/// Whether this frame is the first the enclosing plot's cursor is on a datum;
/// see [`PlotHover::is_entering`]. Outside a plot this is `false`.
pub fn is_hover_entering(window: &mut Window, cx: &mut App) -> bool {
    window
        .use_keyed_state(HOVER_MEMORY, cx, |_, _| HoverMemory::default())
        .read(cx)
        .entering
}

#[cfg(test)]
mod tests {
    use gpui::{point, px};

    use super::*;

    #[test]
    fn test_plot_hover_readers() {
        let state = TooltipState::new(2, point(px(10.), px(20.)), vec![]);
        let hover = PlotHover {
            state,
            focus: 1.,
            hovered: true,
        };
        assert_eq!(hover.state().index, 2);
        assert!(hover.is_hovered());
        // Fully in focus: a pointer keeps travelling rather than snapping.
        assert!(!hover.is_entering());

        // The first hovered frame, before the fade has started.
        let entering = PlotHover {
            focus: 0.,
            ..hover.clone()
        };
        assert!(entering.is_entering());

        // Fading out after the cursor left: neither hovered nor entering.
        let lingering = PlotHover {
            focus: 0.4,
            hovered: false,
            ..hover
        };
        assert!(!lingering.is_hovered());
        assert!(!lingering.is_entering());
    }
}

//! The appear motion shared by every [`Plot`](super::Plot): how far its data
//! marks have drawn in since the plot was first painted.
//!
//! This is behavior only. A plot decides what appearing looks like — a line
//! revealed from the left, bars growing from zero — and a styled layer
//! projects the timing through [`PlotMotion`](crate::PlotMotion).
use gpui::{App, ElementId, Window};

use crate::{
    Theme,
    motion::{Easing, Presence},
};

/// How far a plot's data marks have appeared this frame, handed to
/// [`Plot::appear`](super::Plot::appear).
///
/// The appear starts on the first frame a plot's id is painted and runs over
/// the active [`PlotMotion`](crate::PlotMotion)'s appear. Base's default
/// duration is zero, and reduced motion skips it, so a plot is then complete
/// from its first frame.
#[derive(Clone)]
pub struct PlotAppear {
    /// Linear time through the appear, from `0` to `1`.
    time: f32,
    easing: Easing,
}

impl PlotAppear {
    /// A finished appear: every mark is complete.
    pub fn complete() -> Self {
        Self {
            time: 1.,
            easing: Easing::Linear,
        }
    }

    /// How far the whole plot has appeared, from `0` to `1`, eased.
    pub fn progress(&self) -> f32 {
        // Charts read this per mark on every frame, long after the appear is
        // done, so a finished appear skips sampling the curve.
        if self.time >= 1. {
            return 1.;
        }
        self.easing.sample(self.time)
    }

    /// Whether the appear is still running.
    pub fn is_appearing(&self) -> bool {
        self.time < 1.
    }

    /// How far mark `index` of `count` has appeared, from `0` to `1`, eased.
    ///
    /// The marks start one after another across the first `spread` of the
    /// appear (`0..1`) and each runs for the rest of it, so the last mark
    /// still finishes with the appear however many marks there are. A `spread`
    /// of `0` moves every mark together.
    pub fn staggered(&self, index: usize, count: usize, spread: f32) -> f32 {
        if count <= 1 || self.time >= 1. {
            return self.progress();
        }
        let spread = spread.clamp(0., 0.95);
        let start = spread * index.min(count - 1) as f32 / (count - 1) as f32;
        let time = ((self.time - start) / (1. - spread)).clamp(0., 1.);
        self.easing.sample(time)
    }
}

/// The element-state key of a plot's appear, within the plot's scope.
const APPEAR: &str = "__plot-appear";

/// Sample the appear of the plot painting under the window's current element
/// id. A new `generation` starts it over. Called by
/// [`PlotElement`](super::PlotElement) within the plot's element scope on every
/// frame, so it borrows the theme rather than cloning it and builds its key
/// without allocating.
pub(super) fn track_appear(generation: u64, window: &mut Window, cx: &mut App) -> PlotAppear {
    let Some(policy) = cx
        .try_global::<Theme>()
        .map(|theme| theme.plot.motion().appear().clone())
    else {
        return PlotAppear::complete();
    };
    let easing = policy.curve().clone();
    // Presence keeps the linear time so the marks can each ease over their
    // own slice of it; see `PlotAppear::staggered`.
    let sample = Presence::new((ElementId::Integer(generation), APPEAR), true)
        .transition(policy.easing(Easing::Linear))
        .sample(window, cx);
    PlotAppear {
        time: sample.progress,
        easing,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, time::Duration};

    use gpui::{
        Bounds, Context, ElementId, IntoElement, Pixels, Render, TestAppContext, WindowHandle, px,
        size,
    };

    use super::*;
    use crate::{
        PlotMotion, PlotTheme,
        motion::Transition,
        plot::{Plot, PlotElement},
    };

    /// A plot that records the appear progress it is handed each frame.
    struct Recorder {
        samples: Rc<RefCell<Vec<f32>>>,
        generation: Option<u64>,
    }

    impl IntoElement for Recorder {
        type Element = PlotElement<Self>;

        fn into_element(self) -> Self::Element {
            PlotElement::new(self)
        }
    }

    impl Plot for Recorder {
        fn paint(&mut self, _: Bounds<Pixels>, _: &mut Window, _: &mut App) {}

        fn id(&self) -> Option<ElementId> {
            Some("recorder".into())
        }

        // Appear motion rides on the id alone, without the interactive layer.
        fn interactive(&self) -> bool {
            false
        }

        fn appear(&mut self, appear: PlotAppear, _: &mut Window, _: &mut App) {
            self.samples.borrow_mut().push(appear.progress());
        }

        fn appear_generation(&self) -> Option<u64> {
            self.generation
        }
    }

    struct RecorderView {
        samples: Rc<RefCell<Vec<f32>>>,
        generation: Option<u64>,
    }

    impl Render for RecorderView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Recorder {
                samples: self.samples.clone(),
                generation: self.generation,
            }
        }
    }

    fn open(
        cx: &mut TestAppContext,
        generation: Option<u64>,
    ) -> (WindowHandle<RecorderView>, Rc<RefCell<Vec<f32>>>) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
            Theme::global_mut(cx).plot = PlotTheme::new().with_motion(
                PlotMotion::default()
                    .with_appear(Transition::new(Duration::from_millis(100)).ease(|t| t)),
            );
        });
        let samples = Rc::new(RefCell::new(Vec::new()));
        let window = cx.open_window(size(px(100.), px(100.)), {
            let samples = samples.clone();
            move |_, _| RecorderView {
                samples,
                generation,
            }
        });
        cx.run_until_parked();
        (window, samples)
    }

    fn next_frame(window: WindowHandle<RecorderView>, cx: &mut TestAppContext) -> usize {
        let frames = window
            .update(cx, |_, window, cx| window.simulate_next_frame(cx))
            .unwrap();
        cx.run_until_parked();
        frames
    }

    #[gpui::test]
    fn test_plot_appears_once_over_the_theme_duration(cx: &mut TestAppContext) {
        let (window, samples) = open(cx, Some(0));
        assert_eq!(samples.borrow().last(), Some(&0.));

        cx.executor().advance_clock(Duration::from_millis(50));
        next_frame(window, cx);
        assert_eq!(samples.borrow().last(), Some(&0.5));

        cx.executor().advance_clock(Duration::from_millis(50));
        next_frame(window, cx);
        assert_eq!(samples.borrow().last(), Some(&1.));

        // Once whole, the plot stops asking for frames and stays whole.
        assert_eq!(next_frame(window, cx), 0);
        window.update(cx, |_, window, _| window.refresh()).unwrap();
        cx.run_until_parked();
        assert_eq!(samples.borrow().last(), Some(&1.));
    }

    #[gpui::test]
    fn test_reduced_motion_skips_the_appear(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_reduce_motion(true));
        let (window, samples) = open(cx, Some(0));
        assert_eq!(samples.borrow().first(), Some(&1.));
        assert_eq!(next_frame(window, cx), 0);
    }

    /// A plot that does not opt in is whole at once and asks for no frames,
    /// even with an appear duration in the theme.
    #[gpui::test]
    fn test_plot_without_a_generation_does_not_appear(cx: &mut TestAppContext) {
        let (window, samples) = open(cx, None);
        assert_eq!(samples.borrow().first(), Some(&1.));
        assert_eq!(next_frame(window, cx), 0);
    }

    /// A new generation starts the appear over.
    #[gpui::test]
    fn test_new_generation_replays_the_appear(cx: &mut TestAppContext) {
        let (window, samples) = open(cx, Some(0));
        cx.executor().advance_clock(Duration::from_millis(100));
        next_frame(window, cx);
        assert_eq!(samples.borrow().last(), Some(&1.));

        window
            .update(cx, |view, _, cx| {
                view.generation = Some(1);
                cx.notify();
            })
            .unwrap();
        cx.run_until_parked();
        assert_eq!(samples.borrow().last(), Some(&0.));
    }

    fn at(time: f32) -> PlotAppear {
        PlotAppear {
            time,
            easing: Easing::Linear,
        }
    }

    #[test]
    fn test_complete_appear() {
        let appear = PlotAppear::complete();
        assert!(!appear.is_appearing());
        assert_eq!(appear.progress(), 1.);
        assert_eq!(appear.staggered(3, 10, 0.5), 1.);
    }

    #[test]
    fn test_staggered_marks_share_the_appear() {
        // The first mark starts at once, the last once the spread has passed.
        assert_eq!(at(0.).staggered(0, 5, 0.5), 0.);
        assert_eq!(at(0.25).staggered(0, 5, 0.5), 0.5);
        assert_eq!(at(0.5).staggered(4, 5, 0.5), 0.);
        assert_eq!(at(0.75).staggered(4, 5, 0.5), 0.5);
        // Every mark finishes with the appear.
        for index in 0..5 {
            assert_eq!(at(1.).staggered(index, 5, 0.5), 1.);
        }
    }

    #[test]
    fn test_staggered_without_spread_moves_together() {
        assert_eq!(at(0.4).staggered(0, 3, 0.), 0.4);
        assert_eq!(at(0.4).staggered(2, 3, 0.), 0.4);
        // A lone mark ignores the spread.
        assert_eq!(at(0.4).staggered(0, 1, 0.5), 0.4);
    }
}

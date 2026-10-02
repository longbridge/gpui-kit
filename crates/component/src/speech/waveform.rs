use gpui::{
    App, Bounds, Entity, IntoElement, ParentElement as _, Pixels, RenderOnce, StyleRefinement,
    Styled, Window, canvas, div, fill, point, px, size,
};

use crate::{ActiveTheme as _, Sizable, Size, StyledExt as _};

use super::{SpeechState, level::LEVEL_INTERVAL};

/// The width of a waveform the caller does not size: 24 bars at the smaller
/// sizes.
const DEFAULT_WIDTH: Pixels = px(96.);

/// A live waveform of a [`SpeechState`]'s input levels.
///
/// Bars fill the waveform's width and grow up and down from its midline. The
/// newest level enters at the trailing edge and older ones scroll toward the
/// leading edge, redrawn every frame while audio is captured. Without capture
/// the bars rest as a muted baseline. With reduced motion the bars still show
/// each level but do not scroll between them.
///
/// Size the waveform like any element, e.g. `.w_full()` to span a row; it is
/// 96 px wide by default, and its height follows [`Sizable`].
#[derive(IntoElement)]
pub struct SpeechWaveform {
    state: Entity<SpeechState>,
    size: Size,
    style: StyleRefinement,
}

impl SpeechWaveform {
    /// A waveform for `state`.
    pub fn new(state: &Entity<SpeechState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }

    fn height(&self) -> Pixels {
        match self.size {
            Size::Size(height) => height,
            Size::XSmall => px(12.),
            Size::Small => px(16.),
            Size::Medium => px(20.),
            Size::Large => px(24.),
        }
    }
}

impl Sizable for SpeechWaveform {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for SpeechWaveform {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for SpeechWaveform {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let height = self.height();
        // Bars thicken with the waveform so a tall one does not read as hairlines.
        let bar = (height * 0.125).round().clamp(px(2.), px(4.));
        let state = self.state.read(cx);
        let capturing = state.status().is_capturing();
        let levels: Vec<f32> = state.levels().collect();
        let animate = capturing && !cx.reduce_motion();
        // How far the bars have scrolled toward the next level, so the motion
        // stays smooth when levels arrive slower than the display refreshes.
        let scroll = state
            .last_level_at()
            .filter(|_| animate)
            .map(|at| (at.elapsed().as_secs_f32() / LEVEL_INTERVAL.as_secs_f32()).min(1.))
            .unwrap_or(0.);
        if animate {
            window.request_animation_frame();
        }
        let color = if capturing {
            cx.theme().primary
        } else {
            cx.theme().muted_foreground
        };

        div()
            .h(height)
            .w(DEFAULT_WIDTH)
            .flex_shrink_0()
            .refine_style(&self.style)
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        for rect in bar_rects(bounds, &levels, scroll, bar, bar) {
                            window.paint_quad(fill(rect, color).corner_radii(bar / 2.));
                        }
                    },
                )
                .size_full(),
            )
    }
}

/// The bars for `levels` (oldest first) in `bounds`: the newest at the trailing
/// edge, `scroll` of a step further toward the leading edge, each centered on
/// the midline and at least as tall as it is wide.
fn bar_rects(
    bounds: Bounds<Pixels>,
    levels: &[f32],
    scroll: f32,
    bar: Pixels,
    gap: Pixels,
) -> Vec<Bounds<Pixels>> {
    let pitch = bar + gap;
    let height = bounds.size.height;
    let middle = bounds.origin.y + height / 2.;
    // One more bar than fits, to scroll in from the trailing edge.
    let count = ((bounds.size.width + gap) / pitch).floor().max(0.) as usize + 1;
    (0..count)
        .filter_map(|from_end| {
            let right = bounds.right() - pitch * (from_end as f32 + scroll);
            let left = right - bar;
            if left < bounds.left() - px(0.5) {
                return None;
            }
            let level = levels
                .len()
                .checked_sub(from_end + 1)
                .map_or(0., |ix| levels[ix]);
            let bar_height = (height * level.clamp(0., 1.)).max(bar);
            Some(Bounds::new(
                point(left, middle - bar_height / 2.),
                size(bar, bar_height),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(width: f32, height: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(10.), px(20.)), size(px(width), px(height)))
    }

    #[test]
    fn bars_fill_the_width_with_the_newest_at_the_trailing_edge() {
        let bounds = bounds(98., 20.);
        let rects = bar_rects(bounds, &[0.2, 0.9], 0., px(2.), px(2.));
        // 98 px at a 4 px pitch fits 25 bars.
        assert_eq!(rects.len(), 25);
        assert_eq!(rects[0].right(), bounds.right());
        assert_eq!(rects[0].size.height, px(18.));
        assert_eq!(rects[1].size.height, px(4.));
    }

    #[test]
    fn bars_grow_from_the_midline_and_rest_as_a_baseline() {
        let bounds = bounds(40., 20.);
        for rect in bar_rects(bounds, &[0.5], 0., px(2.), px(2.)) {
            let middle = rect.origin.y + rect.size.height / 2.;
            assert_eq!(middle, px(30.));
            assert!(rect.size.height >= px(2.));
        }
    }

    #[test]
    fn scrolling_moves_bars_toward_the_leading_edge() {
        let bounds = bounds(40., 20.);
        let still = bar_rects(bounds, &[1.], 0., px(2.), px(2.));
        let moving = bar_rects(bounds, &[1.], 0.5, px(2.), px(2.));
        assert_eq!(still[0].left() - moving[0].left(), px(2.));
        assert!(
            moving
                .iter()
                .all(|rect| rect.left() >= bounds.left() - px(0.5))
        );
    }
}

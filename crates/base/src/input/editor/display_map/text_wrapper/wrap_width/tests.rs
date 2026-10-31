//! Differential wrapping and edit invalidation checks over the original display-map implementation.

use super::super::monospace::MonospaceWidths;
use super::super::{
    GraphemeEnds, TextWrapper, WrapScratch, WrappingIndent, measured_wrap,
    measured_wrap_boundaries, monospace_wrap,
};
use super::*;
use crate::input::RopeExt as _;
use gpui::{App, TestAppContext, font};
use ropey::Rope;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation as _;

/// Deterministic fractional glyph widths include multibyte graphemes and contextual ligatures.
fn measure(text: &str) -> Pixels {
    let width: f32 = text
        .graphemes(true)
        .map(|g| {
            if g == "\t" {
                28.25
            } else if g.is_ascii() {
                7.125
            } else {
                15.75
            }
        })
        .sum();
    px(width - text.matches("fi").count() as f32 * 0.875)
}

/// Compare complete logical-to-visual row geometry, including continuation indentation.
fn rows(wrapper: &TextWrapper) -> Vec<(usize, u32, Vec<Range<usize>>)> {
    wrapper
        .iter_lines()
        .map(|line| (line.len(), line.indent, line.wrapped_lines.to_vec()))
        .collect()
}

/// Run the same row-splice path with deterministic glyph widths and count actual rewraps.
fn resize(wrapper: &mut TextWrapper, text: &Rope, width: Pixels, reuse: bool) -> usize {
    wrapper.wrap_width = Some(width);
    let indent = wrapper.wrapping_indent;
    let mut calls = 0;
    wrapper.update_lines(
        text,
        &(0..text.len()),
        text,
        reuse,
        &mut |line, width, _| {
            calls += 1;
            measured_wrap(line, width, indent, &mut WrapScratch::default(), measure)
        },
    );
    calls
}

#[test]
fn oversized_words_keep_measurement_work_local() {
    for length in [8192, 16_384] {
        for prefix in ["", "short ", "    "] {
            let text = format!("{prefix}{}", "x".repeat(length));
            for indent in [WrappingIndent::None, WrappingIndent::Same] {
                for width in [80., 256., 512.] {
                    let mut measured_bytes = 0;
                    let mut longest_probe = 0;
                    let measured = measured_wrap(
                        &text,
                        px(width),
                        indent,
                        &mut WrapScratch::default(),
                        |part| {
                            measured_bytes += part.len();
                            longest_probe = longest_probe.max(part.len());
                            px(part.len() as f32)
                        },
                    );
                    let original = measured_wrap_boundaries(&text, px(width), indent, |part| {
                        px(part.len() as f32)
                    });
                    assert_eq!(measured.boundaries, original);
                    assert!(
                        longest_probe <= 4 * width as usize,
                        "a {width}-pixel row measured a {longest_probe}-byte suffix"
                    );
                    assert!(
                        measured_bytes <= 32 * text.len(),
                        "{} bytes required {measured_bytes} bytes of shaping",
                        text.len()
                    );
                }
            }
        }
    }
}

#[test]
fn cached_resize_matches_cold_unicode_wrapping_at_fractional_thresholds() {
    let text = Rope::from(
        "    alphabet 中文😀 e\u{301} family 👨‍👩‍👧‍👦 finish\n\tfi ffi office\nمرحبا بالعالم\n\r\nend",
    );
    for indent in [WrappingIndent::None, WrappingIndent::Same] {
        let mut cached = TextWrapper::new(font("test"), px(14.), Some(px(160.)));
        cached.wrapping_indent = indent;
        resize(&mut cached, &text, px(160.), false);
        let mut reused = 0;
        // Narrow, widen, then jump across multiple break thresholds; test exact and adjacent floats.
        for value in (1..1440).chain((1..1440).rev()).chain([4096, 1, 1600, 23]) {
            for width in [
                value as f32 / 8.,
                (value as f32 / 8.).next_up(),
                (value as f32 / 8.).next_down(),
            ] {
                let wrapped = resize(&mut cached, &text, px(width), true);
                reused += text.lines_len().saturating_sub(wrapped);
                let mut cold = TextWrapper::new(font("test"), px(14.), Some(px(width)));
                cold.wrapping_indent = indent;
                cold._update(&text, &(0..text.len()), &text, &mut |line, width, _| {
                    measured_wrap_boundaries(line, width, indent, measure)
                });
                assert_eq!(
                    rows(&cached),
                    rows(&cold),
                    "width={width}, indent={indent:?}"
                );
            }
        }
        assert!(reused > 1000, "nearby widths must reuse real row geometry");
    }
}

/// A short changed line invalidates only itself while distant rows retain their measured decisions.
#[gpui::test]
fn resize_reuses_measured_lines_and_invalidates_edits_fonts_indent_and_tokens(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        let text = Rope::from("short\n    a longer line containing 中文 and fi\nlast\n");
        let mut cached = TextWrapper::new(font("Arial"), px(14.), Some(px(1000.)));
        cached.update(&text, &(0..0), &text, cx);
        MEASURED_LINES.with(|count| count.set(0));
        for width in (800..1000).rev() {
            cached.set_wrap_width(Some(px(width as f32)), cx);
        }
        assert_eq!(
            MEASURED_LINES.with(|count| count.get()),
            0,
            "fitting lines must not be shaped again during a width drag"
        );
        cached.set_wrap_width(Some(px(1.)), cx);
        assert!(MEASURED_LINES.with(|count| count.get()) > 0);
        assert_cold(&cached, cx);

        let mut edited = text.clone();
        edited.replace(0..5, "edited 中文😀 new line\ninserted");
        cached.update(
            &edited,
            &(0..5),
            &Rope::from("edited 中文😀 new line\ninserted"),
            cx,
        );
        cached.set_wrap_width(Some(px(82.5)), cx);
        assert_cold(&cached, cx);
        cached.set_font(font("Arial"), px(22.), cx);
        assert_cold(&cached, cx);
        cached.set_wrapping_indent(WrappingIndent::None, cx);
        assert_cold(&cached, cx);
        cached.set_inline_metrics(std::rc::Rc::from([(0..6, px(80.))]), cx);
        cached.set_wrap_width(Some(px(120.)), cx);
        assert_cold(&cached, cx);
        cached.set_wrap_width(None, cx);
        assert_cold(&cached, cx);
        cached.set_wrap_width(Some(px(130.)), cx);
        assert_cold(&cached, cx);
    });
}

/// A width change that moves the breaks of a few lines in a long document rewraps only those
/// lines, while every row still matches a cold wrap.
#[gpui::test]
fn a_width_change_rewraps_only_the_lines_it_breaks_differently(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let long_line = "    a much longer line that wraps at narrower widths 中文 and office";
        let source: String = (0..1000)
            .map(|row| if row % 100 == 7 { long_line } else { "short" })
            .collect::<Vec<_>>()
            .join("\n");
        let text = Rope::from(source);
        let mut cached = TextWrapper::new(font("Arial"), px(14.), Some(px(1000.)));
        cached.update(&text, &(0..0), &text, cx);
        for width in [300., 120., 119.5, 400., 90., 250.] {
            assert!(
                cached.stale_rows(px(width)).is_some(),
                "width={width} must rewrap stale runs, not every line"
            );
            MEASURED_LINES.with(|count| count.set(0));
            cached.set_wrap_width(Some(px(width)), cx);
            let measured = MEASURED_LINES.with(|count| count.get());
            assert!(measured <= 10, "width={width} measured {measured} lines");
            assert_cold(&cached, cx);
        }
    });
}

/// A fresh wrapper with the same source and rendering inputs is the uncached layout oracle.
fn assert_cold(cached: &TextWrapper, cx: &mut App) {
    let mut cold = TextWrapper::new(cached.font.clone(), cached.font_size, cached.wrap_width);
    cold.wrapping_indent = cached.wrapping_indent;
    cold.tab_size = cached.tab_size;
    cold.inline_metrics = cached.inline_metrics.clone();
    cold.update(&cached.text, &(0..0), &cached.text, cx);
    assert_eq!(rows(cached), rows(&cold));
}

/// Native shaping must preserve the original breaks across ligatures, fallback fonts and thresholds.
/// Windows headless mode uses no-op fonts, so borrow its real text system without opening a window.
#[cfg(target_os = "windows")]
#[test]
fn native_cached_resize_matches_original_word_and_grapheme_breaks() {
    let platform = gpui_platform::current_platform(false);
    let cx = TestAppContext::build_with_text_system(
        gpui::TestDispatcher::new(0),
        None,
        platform.text_system(),
    );
    cx.update(|cx| {
        for family in ["Arial", "Consolas"] {
            for font_size in [px(14.), px(19.5)] {
                let font = font(family);
                let system = gpui::WindowTextSystem::new(cx.text_system().clone());
                for indent in [WrappingIndent::None, WrappingIndent::Same] {
                    for text in [
                        "    office fi ffi alpha beta 中文😀 e\u{301} family 👨‍👩‍👧‍👦 finish",
                        "\tlong_word_without_a_break_0123456789\r",
                        "ab\tc\t中文\t\tend\t",
                        "    مرحبا بالعالم العربية السلام عليكم",
                    ] {
                        let source = Rope::from(text);
                        let mut cached = TextWrapper::new(font.clone(), font_size, Some(px(500.)));
                        cached.wrapping_indent = indent;
                        cached.update(&source, &(0..0), &source, cx);
                        // Reuse the same warm wrapper across tab-size changes as well as widths.
                        for tab_size in [2, 8, 4, 0, 4] {
                            cached.set_tab_size(tab_size, cx);
                            assert_cold(&cached, cx);
                            let measure = |text: &str| {
                                native_width(&system, &font, font_size, tab_size, text)
                            };
                            let threshold = f32::from(measure("ab\tc\t"));
                            let widths = (1..500).step_by(7).map(|n| n as f32).chain([
                                threshold,
                                threshold.next_down(),
                                threshold.next_up(),
                            ]);
                            for width in widths {
                                cached.set_wrap_width(Some(px(width)), cx);
                                let mut original =
                                    TextWrapper::new(font.clone(), font_size, Some(px(width)));
                                original.wrapping_indent = indent;
                                original._update(&source, &(0..0), &source, &mut |line, width, _| {
                                    measured_wrap_boundaries(line, width, indent, measure)
                                });
                                assert_eq!(
                                    rows(&cached),
                                    rows(&original),
                                    "font={family}, size={font_size:?}, tab_size={tab_size}, indent={indent:?}, width={width}, text={text:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Measure `text` with native `font` shaping at `font_size`, then apply `tab_size` space stops.
#[cfg(target_os = "windows")]
fn native_width(
    system: &gpui::WindowTextSystem,
    font: &gpui::Font,
    font_size: Pixels,
    tab_size: usize,
    text: &str,
) -> Pixels {
    let layout = system.layout_line(
        text,
        font_size,
        &[gpui::TextRun {
            len: text.len(),
            font: font.clone(),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }],
        None,
    );
    let stop = crate::input::tab_stops::tab_stop_width(system, font, font_size, tab_size);
    crate::input::tab_stops::expanded_width(&layout, text, stop)
}

/// Changing tab stops invalidates both measured widths and reusable rows at an unchanged viewport.
#[cfg(target_os = "windows")]
#[test]
fn native_tab_size_changes_invalidate_warm_wrap_caches() {
    let platform = gpui_platform::current_platform(false);
    let cx = TestAppContext::build_with_text_system(
        gpui::TestDispatcher::new(0),
        None,
        platform.text_system(),
    );
    cx.update(|cx| {
        let font = font("Consolas");
        let font_size = px(14.);
        let system = gpui::WindowTextSystem::new(cx.text_system().clone());
        let width = native_width(&system, &font, font_size, 4, "abcdef") + px(0.5);
        let source = Rope::from("ab\tc\tde\n\txy\tz");
        let mut cached = TextWrapper::new(font, font_size, Some(width));
        cached.set_tab_size(2, cx);
        cached.update(&source, &(0..0), &source, cx);
        let narrow_stops = rows(&cached);
        MEASURED_LINES.with(|count| count.set(0));
        assert!(cached.set_tab_size(8, cx));
        assert!(MEASURED_LINES.with(|count| count.get()) > 0);
        assert_ne!(rows(&cached), narrow_stops);
        assert_cold(&cached, cx);
        MEASURED_LINES.with(|count| count.set(0));
        assert!(!cached.set_tab_size(8, cx));
        assert_eq!(MEASURED_LINES.with(|count| count.get()), 0);
        assert!(cached.set_tab_size(2, cx));
        assert_eq!(rows(&cached), narrow_stops);
        assert_cold(&cached, cx);
    });
}

/// Positions of a monospace run, accumulated in `f64` and rounded per glyph like CoreText's.
fn monospace_starts(text: &str) -> Vec<Pixels> {
    let mut x = 0_f64;
    let mut starts = vec![px(0.)];
    for _ in text.chars() {
        x += 7.826_660_633_087_158;
        starts.push(px(x as f32));
    }
    starts
}

fn monospace_measure(text: &str) -> Pixels {
    *monospace_starts(text).last().unwrap()
}

/// Deterministic xorshift, enough to vary documents without a test dependency.
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound as u64) as usize
    }
}

/// Indented code- and JSON-like printable ASCII with short words, punctuation and long tokens.
fn printable_ascii_line(rng: &mut Rng) -> String {
    const PUNCTUATION: &[u8] = b",.;:-/()[]{}\"'!?=<>+*&|%$#@^~`_";
    let mut line = " ".repeat(rng.below(10));
    for _ in 0..rng.below(40) {
        match rng.below(10) {
            0 => line.push(char::from(PUNCTUATION[rng.below(PUNCTUATION.len())])),
            1 => line.push_str(&"x".repeat(50 + rng.below(400))),
            2 => line.push_str(&" ".repeat(1 + rng.below(3))),
            _ => {
                for _ in 0..1 + rng.below(12) {
                    line.push(char::from(b'a' + rng.below(26) as u8));
                }
                line.push(' ');
            }
        }
    }
    line
}

#[test]
fn printable_ascii_grapheme_ends_match_segmentation() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut buffer = Vec::new();
    for _ in 0..500 {
        let line = printable_ascii_line(&mut rng);
        let segmented: Vec<usize> = line
            .grapheme_indices(true)
            .map(|(ix, grapheme)| ix + grapheme.len())
            .collect();
        let segmented = GraphemeEnds::Segmented(&segmented);
        let bytes = GraphemeEnds::new(&line, &mut buffer);
        assert!(matches!(bytes, GraphemeEnds::Bytes(_)));
        assert_eq!(bytes.len(), segmented.len());
        for index in 0..segmented.len() {
            assert_eq!(bytes.get(index), segmented.get(index));
        }
        for offset in 0..=line.len() + 1 {
            assert_eq!(bytes.count_through(offset), segmented.count_through(offset));
            assert_eq!(bytes.contains(offset), segmented.contains(offset));
        }
    }
}

#[test]
fn table_widths_wrap_like_the_original_algorithm() {
    let widths = MonospaceWidths::detect(monospace_starts, monospace_measure).unwrap();
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    // One scratch across lines, as a document load uses it.
    let mut scratch = WrapScratch::default();
    for _ in 0..400 {
        let line = printable_ascii_line(&mut rng);
        for _ in 0..4 {
            // Wide enough at times for the whole line to fit on one row.
            let width = px(rng.below(32_000) as f32 / 8. + 1.);
            for indent in [WrappingIndent::None, WrappingIndent::Same] {
                let table = monospace_wrap(&line, width, indent, &mut scratch, &widths, |_| {
                    unreachable!("rows stay within the table")
                });
                let shaped = measured_wrap(
                    &line,
                    width,
                    indent,
                    &mut WrapScratch::default(),
                    monospace_measure,
                );
                let original = measured_wrap_boundaries(&line, width, indent, monospace_measure);
                assert_eq!(table.boundaries, original, "{line:?} at {width:?}");
                assert_eq!(table.boundaries, shaped.boundaries);
                assert_eq!(
                    format!("{:?}", table.valid_widths),
                    format!("{:?}", shaped.valid_widths)
                );
            }
        }
    }
}

#[test]
fn table_resize_matches_cold_wrapping_at_fractional_thresholds() {
    let widths = MonospaceWidths::detect(monospace_starts, monospace_measure).unwrap();
    let mut rng = Rng(0xd1b5_4a32_d192_ed03);
    let text = Rope::from(
        (0..40)
            .map(|_| printable_ascii_line(&mut rng))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let table = |text: &str| widths.width(text.len()).unwrap();
    for indent in [WrappingIndent::None, WrappingIndent::Same] {
        let mut cached = TextWrapper::new(font("test"), px(14.), Some(px(160.)));
        cached.wrapping_indent = indent;
        let mut scratch = WrapScratch::default();
        let mut resize = |wrapper: &mut TextWrapper, width: Pixels, reuse: bool| {
            wrapper.wrap_width = Some(width);
            wrapper.update_lines(
                &text,
                &(0..text.len()),
                &text,
                reuse,
                &mut |line, width, _| {
                    monospace_wrap(line, width, indent, &mut scratch, &widths, table)
                },
            );
        };
        resize(&mut cached, px(160.), false);
        for value in (1..2000).step_by(7).chain((1..2000).rev().step_by(11)) {
            for width in [
                value as f32 / 4.,
                (value as f32 / 4.).next_up(),
                (value as f32 / 4.).next_down(),
            ] {
                resize(&mut cached, px(width), true);
                let mut cold = TextWrapper::new(font("test"), px(14.), Some(px(width)));
                cold.wrapping_indent = indent;
                cold._update(&text, &(0..text.len()), &text, &mut |line, width, _| {
                    measured_wrap_boundaries(line, width, indent, monospace_measure)
                });
                assert_eq!(
                    rows(&cached),
                    rows(&cold),
                    "width={width}, indent={indent:?}"
                );
            }
        }
    }
}

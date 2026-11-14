//! Differential wrapping and edit invalidation checks over the original display-map implementation.

use super::super::char_widths::CharWidths;
use super::super::monospace::MonospaceWidths;
use super::super::{
    GraphemeEnds, TextWrapper, WrapScratch, WrappingIndent, fast_wrap, measured_wrap,
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

const EXACT_ASCII: f64 = 7.125;
const EXACT_WIDE: f64 = 15.75;
const EXACT_TAB_STOP: f32 = 28.5;

/// Width of `text` from `x = 0`, with tabs ending at the next stop, in exact binary fractions.
fn exact_measure(text: &str) -> Pixels {
    let stop = f64::from(EXACT_TAB_STOP);
    let mut x = 0.0;
    for c in text.chars() {
        if c == '\t' {
            x = crate::input::tab_stops::next_tab_stop(x, stop);
        } else if matches!(c, ' '..='~') {
            x += EXACT_ASCII;
        } else {
            x += EXACT_WIDE;
        }
    }
    px(x as f32)
}

/// A monospace table whose advance equals the ASCII advance of [`exact_measure`].
fn exact_monospace() -> MonospaceWidths {
    let starts = |text: &str| {
        let mut starts = vec![px(0.)];
        for (ix, _) in text.chars().enumerate() {
            starts.push(px((EXACT_ASCII * (ix + 1) as f64) as f32));
        }
        starts
    };
    let measure = |text: &str| px((EXACT_ASCII * text.chars().count() as f64) as f32);
    MonospaceWidths::detect(starts, measure).unwrap()
}

/// Lines of eligible characters that always hold a character outside printable ASCII, so they
/// take the summed-width path.
fn eligible_line(rng: &mut Rng) -> String {
    const OTHERS: &[char] = &['中', '文', 'あ', 'カ', '한', 'é', 'Ж', 'β', '—', '、', 'Ａ'];
    let mut line = match rng.below(3) {
        0 => " ".repeat(rng.below(10)),
        1 => "\t".repeat(1 + rng.below(3)),
        _ => String::new(),
    };
    for _ in 0..1 + rng.below(40) {
        match rng.below(8) {
            0 | 1 => {
                for _ in 0..1 + rng.below(12) {
                    line.push(OTHERS[rng.below(OTHERS.len())]);
                }
            }
            2 => line.push('\t'),
            3 => line.push_str(&" ".repeat(1 + rng.below(3))),
            4 => line.push_str(&"x".repeat(30 + rng.below(100))),
            _ => {
                for _ in 0..1 + rng.below(10) {
                    line.push(char::from(b'a' + rng.below(26) as u8));
                }
                line.push(' ');
            }
        }
    }
    line.push('中');
    line
}

/// `fast_wrap` over `line`, with a `shaped` closure that counts its calls.
fn exact_fast_wrap(
    chars: &mut CharWidths,
    monospace: &MonospaceWidths,
    line: &str,
    width: Pixels,
    indent: WrappingIndent,
    scratch: &mut WrapScratch,
    shaping_calls: &std::cell::Cell<usize>,
) -> Option<MeasuredWrap> {
    let mut shaped = |text: &str| {
        shaping_calls.set(shaping_calls.get() + 1);
        exact_measure(text)
    };
    let mut glyph_starts = |text: &str| {
        shaping_calls.set(shaping_calls.get() + 1);
        let ends = text.char_indices().map(|(i, c)| i + c.len_utf8());
        std::iter::once(px(0.))
            .chain(ends.map(|end| exact_measure(&text[..end])))
            .collect()
    };
    let widths = chars.line_widths(
        line,
        monospace,
        px(EXACT_TAB_STOP),
        &mut shaped,
        &mut glyph_starts,
    )?;
    Some(fast_wrap(line, width, indent, scratch, &widths))
}

#[test]
fn fast_wrap_matches_the_original_algorithm_for_tabs_and_wide_text() {
    let monospace = exact_monospace();
    let mut rng = Rng(0x7f4a_7c15_9e37_79b9);
    let mut chars = CharWidths::default();
    let mut scratch = WrapScratch::default();
    let calls = std::cell::Cell::new(0);
    for _ in 0..400 {
        let line = eligible_line(&mut rng);
        for _ in 0..4 {
            let width = px(rng.below(32_000) as f32 / 8. + 1.);
            for indent in [WrappingIndent::None, WrappingIndent::Same] {
                let fast = exact_fast_wrap(
                    &mut chars,
                    &monospace,
                    &line,
                    width,
                    indent,
                    &mut scratch,
                    &calls,
                )
                .expect("an eligible line takes the summed-width path");
                let original = measured_wrap_boundaries(&line, width, indent, exact_measure);
                assert_eq!(fast.boundaries, original, "{line:?} at {width:?}");
            }
        }
    }
}

#[test]
fn fast_wrap_keeps_the_measured_width_range_of_the_shaped_path() {
    let monospace = exact_monospace();
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let mut chars = CharWidths::default();
    let mut scratch = WrapScratch::default();
    let calls = std::cell::Cell::new(0);
    for _ in 0..300 {
        let line = eligible_line(&mut rng);
        for _ in 0..4 {
            let width = px(rng.below(32_000) as f32 / 8. + 1.);
            for indent in [WrappingIndent::None, WrappingIndent::Same] {
                let fast = exact_fast_wrap(
                    &mut chars,
                    &monospace,
                    &line,
                    width,
                    indent,
                    &mut scratch,
                    &calls,
                )
                .unwrap();
                let shaped = measured_wrap(
                    &line,
                    width,
                    indent,
                    &mut WrapScratch::default(),
                    exact_measure,
                );
                assert_eq!(fast.boundaries, shaped.boundaries);
                assert_eq!(
                    format!("{:?}", fast.valid_widths),
                    format!("{:?}", shaped.valid_widths),
                    "{line:?} at {width:?}"
                );
            }
        }
    }
}

#[test]
fn fast_resize_matches_cold_wrapping_at_fractional_thresholds() {
    let monospace = exact_monospace();
    let mut rng = Rng(0xa076_1d64_78bd_642f);
    let text = Rope::from(
        (0..40)
            .map(|_| eligible_line(&mut rng))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    for indent in [WrappingIndent::None, WrappingIndent::Same] {
        let mut cached = TextWrapper::new(font("test"), px(14.), Some(px(160.)));
        cached.wrapping_indent = indent;
        let mut chars = CharWidths::default();
        let mut scratch = WrapScratch::default();
        let calls = std::cell::Cell::new(0);
        let mut resize = |wrapper: &mut TextWrapper, width: Pixels, reuse: bool| {
            wrapper.wrap_width = Some(width);
            wrapper.update_lines(
                &text,
                &(0..text.len()),
                &text,
                reuse,
                &mut |line, width, _| {
                    exact_fast_wrap(
                        &mut chars,
                        &monospace,
                        line,
                        width,
                        indent,
                        &mut scratch,
                        &calls,
                    )
                    .unwrap_or_else(|| {
                        measured_wrap(line, width, indent, &mut scratch, exact_measure)
                    })
                },
            );
        };
        resize(&mut cached, px(160.), false);
        for value in (1..4000).step_by(7).chain((1..4000).rev().step_by(11)) {
            for width in [
                value as f32 / 4.,
                (value as f32 / 4.).next_up(),
                (value as f32 / 4.).next_down(),
            ] {
                resize(&mut cached, px(width), true);
                let mut cold = TextWrapper::new(font("test"), px(14.), Some(px(width)));
                cold.wrapping_indent = indent;
                cold._update(&text, &(0..text.len()), &text, &mut |line, width, _| {
                    measured_wrap_boundaries(line, width, indent, exact_measure)
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

/// Eligible non-ASCII lines are shaped on the first pass only, however often the width changes.
#[test]
fn a_second_pass_at_another_width_makes_no_shaping_call() {
    let monospace = exact_monospace();
    let mut rng = Rng(0x0dd_ba11_cafe_f00d);
    let text = Rope::from(
        (0..200)
            .map(|_| eligible_line(&mut rng))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let indent = WrappingIndent::Same;
    let mut wrapper = TextWrapper::new(font("test"), px(14.), Some(px(160.)));
    wrapper.wrapping_indent = indent;
    let mut chars = CharWidths::default();
    let mut scratch = WrapScratch::default();
    let calls = std::cell::Cell::new(0);
    let mut pass = |wrapper: &mut TextWrapper, width: f32| {
        wrapper.wrap_width = Some(px(width));
        wrapper.update_lines(
            &text,
            &(0..text.len()),
            &text,
            false,
            &mut |line, width, _| {
                exact_fast_wrap(
                    &mut chars,
                    &monospace,
                    line,
                    width,
                    indent,
                    &mut scratch,
                    &calls,
                )
                .expect("an eligible line takes the summed-width path")
            },
        );
    };
    pass(&mut wrapper, 160.);
    assert!(calls.get() > 0, "the first pass checks each line");
    let first = calls.get();
    pass(&mut wrapper, 237.5);
    assert_eq!(calls.get(), first, "the second pass must not shape");
}

/// Rows from the summed-width path equal rows from the shaped path through the production wrapper.
#[gpui::test]
fn production_wrapper_with_and_without_the_summed_width_path_agree(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let long_ascii = "plain words that wrap at narrower widths ".repeat(6);
        let source = [
            long_ascii.as_str(),
            "中文字符混合的一行很长的文字需要在窄宽度下折行显示出来",
            "mixed 中文 and latin words that go on for a while 中文 again",
            "\t\tindented\twith tabs and 中文 text that is long enough to wrap",
            "    spaces then 日本語のテキスト and more words to wrap around",
            "an emoji 😀 line with enough words to need wrapping at small widths",
            "",
        ]
        .join("\n");
        let text = Rope::from(source);
        let mut build = |fast: bool| {
            let mut wrapper = TextWrapper::new(font("Menlo"), px(14.), Some(px(400.)));
            wrapper.set_fast_non_ascii(fast);
            wrapper.update(&text, &(0..0), &text, cx);
            wrapper
        };
        let mut fast = build(true);
        let mut shaped = build(false);
        // Without a detected monospace font the summed-width path never runs and this test only
        // guards equality. CoreText provides Menlo, so the path runs on macOS.
        #[cfg(target_os = "macos")]
        assert!(matches!(fast.monospace, Some(Some(_))));
        for width in [400., 180., 97.5, 60., 333.25, 1000.] {
            fast.set_wrap_width(Some(px(width)), cx);
            shaped.set_wrap_width(Some(px(width)), cx);
            assert_eq!(rows(&fast), rows(&shaped), "width={width}");
        }
        fast.set_tab_size(2, cx);
        shaped.set_tab_size(2, cx);
        assert_eq!(rows(&fast), rows(&shaped));
    });
}

/// A tab after text that ends on a tab stop sits on a floating point tie. Lines of printable
/// ASCII and tabs share one comparison with the shaper, so the summed path must resolve that
/// tie the way the shaped path does on every line.
#[gpui::test]
fn summed_and_shaped_paths_agree_on_tabs_after_text_that_ends_on_a_stop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        // The first line of ASCII and tabs passes the shared comparison. The later lines put a
        // tab after every count of columns, so some of them end on a stop at each tab size.
        let mut source = String::from("\tindented first line\n");
        for columns in 0..40 {
            source.push_str(&"a".repeat(columns));
            source.push('\t');
            source.push_str(&"word ".repeat(12));
            source.push('\n');
        }
        let text = Rope::from(source);
        for font_size in [12., 13., 14., 15., 16.] {
            for tab_size in [2, 3, 4, 8] {
                let mut build = |fast: bool| {
                    let mut wrapper =
                        TextWrapper::new(font("Menlo"), px(font_size), Some(px(400.)));
                    wrapper.tab_size = tab_size;
                    wrapper.set_fast_non_ascii(fast);
                    wrapper.update(&text, &(0..0), &text, cx);
                    wrapper
                };
                let mut fast = build(true);
                let mut shaped = build(false);
                // A width of a whole number of columns is a tie of its own: the two paths may
                // differ there by less than the line tolerance. The fraction keeps clear of it.
                for width in (200..700).step_by(7).map(|width| width as f32 + 0.37) {
                    fast.set_wrap_width(Some(px(width)), cx);
                    shaped.set_wrap_width(Some(px(width)), cx);
                    assert_eq!(
                        rows(&fast),
                        rows(&shaped),
                        "font size {font_size}, tab size {tab_size}, width {width}"
                    );
                }
            }
        }
    });
}

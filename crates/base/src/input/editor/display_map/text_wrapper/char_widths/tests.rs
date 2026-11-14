//! Summed widths, with synthetic shapers standing in for fonts. Advances are exact binary
//! fractions, so sums compare exactly.

use std::cell::Cell;

use super::*;

const ASCII: f64 = 7.125;
const WIDE: f64 = 15.75;
const TAB_STOP: f32 = 28.5;

/// Wide characters take two columns, everything else one.
fn advance(c: char) -> f64 {
    if c >= '\u{3000}' { WIDE } else { ASCII }
}

/// Shaped width of `text` from `x = 0`, with tabs expanded to the next stop.
fn reference(text: &str, tab_stop: f64) -> f64 {
    let mut x = 0.0;
    for c in text.chars() {
        if c == '\t' {
            x = tab_stop * ((x / tab_stop).floor() + 1.0);
        } else {
            x += advance(c);
        }
    }
    x
}

fn shaper(text: &str) -> Pixels {
    px(reference(text, f64::from(TAB_STOP)) as f32)
}

/// Glyph starts of `text` as `measure` shapes it: the width of each prefix, then of the text.
fn starts_from(measure: impl Fn(&str) -> Pixels, text: &str) -> Vec<Pixels> {
    let ends = text.char_indices().map(|(i, c)| i + c.len_utf8());
    std::iter::once(px(0.))
        .chain(ends.map(|end| measure(&text[..end])))
        .collect()
}

fn starts(text: &str) -> Vec<Pixels> {
    starts_from(shaper, text)
}

fn monospace() -> MonospaceWidths {
    let starts = |text: &str| {
        let mut x = 0.0;
        let mut starts = vec![px(0.)];
        for _ in text.chars() {
            x += ASCII;
            starts.push(px(x as f32));
        }
        starts
    };
    let measure = |text: &str| px((text.chars().count() as f64 * ASCII) as f32);
    MonospaceWidths::detect(starts, measure).unwrap()
}

/// Widths of `line` with the default shaper, or `None`.
fn widths_of(chars: &mut CharWidths, line: &str) -> Option<Vec<Pixels>> {
    let monospace = monospace();
    let widths = chars.line_widths(line, &monospace, px(TAB_STOP), &mut shaper, &mut starts)?;
    Some(
        (0..=line.len())
            .filter(|&i| line.is_char_boundary(i))
            .map(|i| widths.width(0..i))
            .collect(),
    )
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % bound
    }
}

fn random_line(rng: &mut Lcg, len: usize) -> String {
    const POOL: &[char] = &[
        'a', 'Z', ' ', '~', '\t', 'é', 'ÿ', 'Ā', 'β', 'Ж', '—', '、', 'あ', 'カ', '中', '한', 'Ａ',
    ];
    (0..len).map(|_| POOL[rng.next(POOL.len())]).collect()
}

#[test]
fn fast_characters_cover_each_row_of_the_table() {
    for c in [
        ' ', '~', 'a', '\t', '¡', '¬', '®', 'ÿ', 'Ā', 'ſ', 'Α', 'Ρ', 'Σ', 'ω', 'Ё', 'А', 'я', 'ё',
        '‐', '‧', '‰', '›', '\u{3000}', '〩', '〰', '〿', 'ぁ', 'ゖ', 'ァ', 'ヺ', 'ー', '㐀', '䶿',
        '一', '鿿', '가', '힣', '！', '｠', '￠', '￦',
    ] {
        assert!(is_fast_char(c), "{c:?} should be fast");
    }
}

#[test]
fn characters_that_combine_or_join_are_not_fast() {
    for c in [
        '\r', '\n', '\u{00A0}', '\u{00AD}', '\u{0301}', '\u{3099}', '\u{302A}', '😀', 'ع', 'क',
        '\u{0000}', '\u{007F}',
    ] {
        assert!(!is_fast_char(c), "{c:?} should not be fast");
    }
}

#[test]
fn widths_of_random_ranges_equal_the_reference() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let mut rng = Lcg(7);
    for _ in 0..300 {
        let len = 1 + rng.next(40);
        let line = random_line(&mut rng, len);
        let Some(widths) =
            chars.line_widths(&line, &monospace, px(TAB_STOP), &mut shaper, &mut starts)
        else {
            assert!(is_printable_ascii(&line), "{line:?}");
            continue;
        };
        let bounds: Vec<usize> = (0..=line.len())
            .filter(|&i| line.is_char_boundary(i))
            .collect();
        for _ in 0..30 {
            let a = bounds[rng.next(bounds.len())];
            let b = bounds[rng.next(bounds.len())];
            let range = a.min(b)..a.max(b);
            let expected = reference(&line[range.clone()], f64::from(TAB_STOP));
            assert_eq!(
                widths.width(range.clone()),
                px(expected as f32),
                "{line:?} {range:?}"
            );
        }
    }
}

#[test]
fn tabs_count_stops_from_the_start_of_the_range() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let line = "\t中\t\tab\t";
    let widths = chars
        .line_widths(line, &monospace, px(TAB_STOP), &mut shaper, &mut starts)
        .unwrap();
    let tab = TAB_STOP as f64;
    // A tab at the start of a range.
    assert_eq!(widths.width(0..1), px(TAB_STOP));
    // Two tabs in a row.
    let (second, third) = (1 + '中'.len_utf8(), 2 + '中'.len_utf8());
    assert_eq!(widths.width(second..third + 1), px((2.0 * tab) as f32));
    // A tab as the last character.
    assert_eq!(widths.width(line.len() - 3..line.len()), px(TAB_STOP));
    // A range with no tab.
    assert_eq!(widths.width(third + 1..third + 3), px((2.0 * ASCII) as f32));
    // The whole line.
    assert_eq!(widths.width(0..line.len()), shaper(line));
}

#[test]
fn a_character_whose_pair_width_differs_is_rejected() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let kerned = |text: &str| {
        let base = shaper(text);
        if text.starts_with('Ж') && text.chars().count() > 1 {
            base + px(1.0)
        } else {
            base
        }
    };
    let mut kerned_starts = |text: &str| starts_from(kerned, text);
    let mut kerned = kerned;
    assert!(
        chars
            .line_widths(
                "aЖb",
                &monospace,
                px(TAB_STOP),
                &mut kerned,
                &mut kerned_starts
            )
            .is_none()
    );
    assert_eq!(chars.others.get(&'Ж'), Some(&None));
    // The rejection is remembered, so a second line makes no shaping call.
    let calls = Cell::new(0);
    let mut counted = |text: &str| {
        calls.set(calls.get() + 1);
        shaper(text)
    };
    assert!(
        chars
            .line_widths("Жx", &monospace, px(TAB_STOP), &mut counted, &mut starts)
            .is_none()
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn a_line_that_differs_from_its_shaped_width_is_rejected_once() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let calls = Cell::new(0);
    let mut off = |text: &str| {
        calls.set(calls.get() + 1);
        if text.chars().count() > 8 {
            shaper(text) + px(0.5)
        } else {
            shaper(text)
        }
    };
    let line = "あいうえおかきくけこ";
    assert!(
        chars
            .line_widths(line, &monospace, px(TAB_STOP), &mut off, &mut starts)
            .is_none()
    );
    let first = calls.get();
    assert!(first > 0);
    assert!(
        chars
            .line_widths(line, &monospace, px(TAB_STOP), &mut off, &mut starts)
            .is_none()
    );
    assert_eq!(calls.get(), first);
}

#[test]
fn a_passing_line_shapes_the_line_once_and_its_new_characters_in_one_call() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let calls = Cell::new(0);
    let mut counted = |text: &str| {
        calls.set(calls.get() + 1);
        shaper(text)
    };
    let batches = Cell::new(0);
    let mut counted_starts = |text: &str| {
        batches.set(batches.get() + 1);
        starts(text)
    };
    let line = "ab中文中\tx";
    assert!(
        chars
            .line_widths(
                line,
                &monospace,
                px(TAB_STOP),
                &mut counted,
                &mut counted_starts
            )
            .is_some()
    );
    assert_eq!((calls.get(), batches.get()), (1, 1));
    assert_eq!(chars.others.get(&'中'), Some(&Some(WIDE)));
    calls.set(0);
    assert!(
        chars
            .line_widths(line, &monospace, px(TAB_STOP), &mut counted, &mut starts)
            .is_some()
    );
    assert_eq!(calls.get(), 0);
    // Another line with known characters shapes only the line.
    assert!(
        chars
            .line_widths("文中a", &monospace, px(TAB_STOP), &mut counted, &mut starts)
            .is_some()
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn new_characters_are_measured_sixteen_to_a_shaping_call() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let batches = Cell::new(0);
    let mut counted_starts = |text: &str| {
        batches.set(batches.get() + 1);
        starts(text)
    };
    let line: String = ('一'..='鿿').take(2 * BATCH_CHARS + 1).collect();
    let widths = chars
        .line_widths(
            &line,
            &monospace,
            px(TAB_STOP),
            &mut shaper,
            &mut counted_starts,
        )
        .unwrap();
    assert_eq!(widths.width(0..line.len()), shaper(&line));
    assert_eq!(batches.get(), 3);
}

#[test]
fn a_batch_without_one_glyph_per_character_is_measured_character_by_character() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let calls = Cell::new(0);
    let mut counted = |text: &str| {
        calls.set(calls.get() + 1);
        shaper(text)
    };
    // A shaper that drops a glyph, as one that draws two characters with one glyph would.
    let mut short_starts = |text: &str| {
        let mut starts = starts(text);
        starts.pop();
        starts
    };
    let line = "中文";
    let widths = chars
        .line_widths(
            line,
            &monospace,
            px(TAB_STOP),
            &mut counted,
            &mut short_starts,
        )
        .unwrap();
    assert_eq!(widths.width(0..line.len()), shaper(line));
    assert_eq!(calls.get(), 2 * 2 + 1);
}

#[test]
fn a_character_rejected_in_a_batch_leaves_the_others_measured() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let kerned = |text: &str| {
        let pairs = text.matches("ЖЖ").count();
        shaper(text) + px(pairs as f32)
    };
    let mut kerned_starts = |text: &str| starts_from(kerned, text);
    let mut kerned = kerned;
    assert!(
        chars
            .line_widths(
                "中Ж文",
                &monospace,
                px(TAB_STOP),
                &mut kerned,
                &mut kerned_starts
            )
            .is_none()
    );
    assert_eq!(chars.others.get(&'Ж'), Some(&None));
    assert_eq!(chars.others.get(&'中'), Some(&Some(WIDE)));
    assert_eq!(chars.others.get(&'文'), Some(&Some(WIDE)));
}

#[test]
fn lines_of_ascii_and_tabs_share_one_comparison_with_the_shaper() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let calls = Cell::new(0);
    let mut counted = |text: &str| {
        calls.set(calls.get() + 1);
        shaper(text)
    };
    for (line, expected_calls) in [("\tfirst line", 1), ("\t\tsecond\tline", 1), ("x\ty", 1)] {
        assert!(
            chars
                .line_widths(line, &monospace, px(TAB_STOP), &mut counted, &mut starts)
                .is_some()
        );
        assert_eq!(calls.get(), expected_calls, "{line:?}");
    }
    // A line with a wide character is still compared on its own.
    assert!(
        chars
            .line_widths("\t中", &monospace, px(TAB_STOP), &mut counted, &mut starts)
            .is_some()
    );
    assert_eq!(calls.get(), 1 + 1);
}

#[test]
fn a_failed_comparison_keeps_every_line_of_ascii_and_tabs_on_the_shaped_path() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let calls = Cell::new(0);
    // A shaper whose tabs are wider than the tab stop arithmetic says.
    let mut wide_tabs = |text: &str| {
        calls.set(calls.get() + 1);
        shaper(text) + px(text.matches('\t').count() as f32)
    };
    for line in ["\tfirst", "\tsecond"] {
        assert!(
            chars
                .line_widths(line, &monospace, px(TAB_STOP), &mut wide_tabs, &mut starts)
                .is_none()
        );
    }
    assert_eq!(calls.get(), 1);
    chars.clear();
    assert!(
        chars
            .line_widths(
                "\tfirst",
                &monospace,
                px(TAB_STOP),
                &mut shaper,
                &mut starts
            )
            .is_some(),
        "clear forgets the failed comparison"
    );
}

#[test]
fn lines_for_the_shaped_path_return_none() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    assert!(widths_of(&mut chars, "").is_none());
    assert!(widths_of(&mut chars, "plain ascii").is_none());
    assert!(widths_of(&mut chars, "中😀").is_none());
    assert!(widths_of(&mut chars, "中\r").is_none());
    assert!(
        chars
            .line_widths("中\tx", &monospace, px(0.), &mut shaper, &mut starts)
            .is_none()
    );
    assert!(
        chars
            .line_widths("中x", &monospace, px(0.), &mut shaper, &mut starts)
            .is_some()
    );
}

#[test]
fn clear_forgets_characters_and_checked_lines() {
    let monospace = monospace();
    let mut chars = CharWidths::default();
    let line = "中a";
    assert!(
        chars
            .line_widths(line, &monospace, px(TAB_STOP), &mut shaper, &mut starts)
            .is_some()
    );
    chars.clear();
    assert!(chars.others.is_empty() && chars.checks.results.is_empty() && chars.ascii.is_none());
    let calls = Cell::new(0);
    let mut counted = |text: &str| {
        calls.set(calls.get() + 1);
        shaper(text)
    };
    assert!(
        chars
            .line_widths(line, &monospace, px(TAB_STOP), &mut counted, &mut starts)
            .is_some()
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn release_excess_drops_only_oversized_buffers() {
    let mut chars = CharWidths::default();
    chars.prefix = Vec::with_capacity(MAX_RETAINED_ENTRIES + 1);
    chars.tabs = Vec::with_capacity(8);
    chars.release_excess();
    assert_eq!(chars.prefix.capacity(), 0);
    assert!(chars.tabs.capacity() >= 8);
}

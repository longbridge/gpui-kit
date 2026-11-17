//! Font eligibility for table measurement, with synthetic shapers standing in for fonts.

use super::*;

/// CoreText's Menlo advance at 13px, which is not a short binary fraction.
const ADVANCE: f64 = 7.826_660_633_087_158;

/// Positions accumulate in `f64` and are rounded per glyph, as CoreText reports them.
fn starts_with(advance: impl Fn(char) -> f64) -> impl Fn(&str) -> Vec<Pixels> {
    move |text| {
        let mut x = 0.;
        let mut starts = vec![px(0.)];
        for c in text.chars() {
            x += advance(c);
            starts.push(px(x as f32));
        }
        starts
    }
}

fn monospace_starts(text: &str) -> Vec<Pixels> {
    starts_with(|_| ADVANCE)(text)
}

fn monospace(text: &str) -> Pixels {
    *monospace_starts(text).last().unwrap()
}

#[test]
fn a_monospace_font_is_detected_with_its_shaped_widths() {
    let detected = MonospaceWidths::detect(monospace_starts, monospace).unwrap();
    assert_eq!(detected.width(0), Some(px(0.)));
    for len in [1, 2, 7, 100, 3000, TABLE_CHARS] {
        assert_eq!(detected.width(len), Some(monospace(&"x".repeat(len))));
    }
}

#[test]
fn widths_past_the_table_are_left_to_the_shaper() {
    let detected = MonospaceWidths::detect(monospace_starts, monospace).unwrap();
    assert_eq!(detected.width(TABLE_CHARS + 1), None);
}

#[test]
fn proportional_fonts_are_rejected() {
    let advance = |c| if c == 'i' || c == 'l' { 3. } else { ADVANCE };
    let starts = starts_with(advance);
    let measure = |text: &str| *starts_with(advance)(text).last().unwrap();
    assert_eq!(MonospaceWidths::detect(starts, measure), None);
}

#[test]
fn kerned_pairs_are_rejected() {
    let kerned = |text: &str| monospace(text) - px(text.matches("AV").count() as f32 * 0.5);
    assert_eq!(MonospaceWidths::detect(monospace_starts, kerned), None);
}

#[test]
fn width_changing_ligatures_are_rejected() {
    let ligated = |text: &str| monospace(text) - px(text.matches("===").count() as f32);
    assert_eq!(MonospaceWidths::detect(monospace_starts, ligated), None);
}

#[test]
fn line_widths_that_differ_from_glyph_positions_are_rejected() {
    // A shaper that snaps a line's width to its own grid disagrees with the positions in a run.
    let snapped = |text: &str| px((f32::from(monospace(text)) * 64.).round() / 64.);
    assert_eq!(MonospaceWidths::detect(monospace_starts, snapped), None);
}

#[test]
fn unusable_runs_are_rejected() {
    assert_eq!(MonospaceWidths::detect(|_| vec![px(0.)], monospace), None);
    let flat = |text: &str| vec![px(0.); text.len() + 1];
    assert_eq!(MonospaceWidths::detect(flat, |_| px(0.)), None);
}

#[test]
fn only_printable_ascii_is_measured_by_lookup() {
    assert!(is_printable_ascii(""));
    assert!(is_printable_ascii("  \"key\": [1, 2] ~"));
    for text in ["a\tb", "a\rb", "café", "中文", "\u{7f}", "a\u{301}"] {
        assert!(!is_printable_ascii(text), "{text:?}");
    }
}

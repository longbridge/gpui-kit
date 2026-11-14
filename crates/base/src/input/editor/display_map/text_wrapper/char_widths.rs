//! Wrapping lines with tabs and non-ASCII text by summing per-character widths.
//!
//! In a monospace font the width of a line is the sum of the advances of its characters, so a
//! line made only of characters that never combine with a neighbor can be measured without
//! shaping every prefix of it. Each character is shaped on its own once, and each distinct line is
//! compared with the shaper once, so a wrong sum is caught before a break could move. Lines of
//! printable ASCII and tabs share one such comparison, because their widths come from the checked
//! monospace table.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
};

use gpui::{Pixels, px};

use super::monospace::{MonospaceWidths, TABLE_CHARS, is_printable_ascii};

#[cfg(test)]
mod tests;

/// Capacity, in entries, above which a wrap pass releases the line buffers.
const MAX_RETAINED_ENTRIES: usize = 64 * 1024;

/// Checked lines remembered before the record is cleared.
const MAX_CHECKED_LINES: usize = 1 << 20;

/// How far a character's eight-fold run may differ from eight times its single advance, in pixels.
const ADVANCE_TOLERANCE: f64 = 0.01;

/// Smallest difference, in pixels, between a summed and a shaped line width that fails the check.
const LINE_TOLERANCE: f32 = 0.01;

/// Advances of single characters in one font, and the lines already checked against the shaper.
#[derive(Default)]
pub(super) struct CharWidths {
    /// Advance of one printable ASCII character, set from the monospace table.
    ascii: Option<f64>,
    /// Advance of each other eligible character. `None` marks a character that failed the check.
    others: HashMap<char, Option<f64>>,
    checks: LineChecks,
    /// Result of the one check that covers every line of printable ASCII and tabs. `None` until
    /// the first such line.
    ascii_tabs_check: Option<bool>,
    /// `prefix[i]` is the summed advance of the characters before byte `i`, tabs counted as 0.
    prefix: Vec<f64>,
    /// Byte offsets of the tabs in the line, in order.
    tabs: Vec<usize>,
}

impl CharWidths {
    /// Forget everything. Call when the font, the font size, or the tab size changes.
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Drop the line buffers when a very long line has grown them past the retained limit.
    pub(super) fn release_excess(&mut self) {
        if self.prefix.capacity() > MAX_RETAINED_ENTRIES {
            self.prefix = Vec::new();
        }
        if self.tabs.capacity() > MAX_RETAINED_ENTRIES {
            self.tabs = Vec::new();
        }
    }

    /// Widths for `line`, or `None` when the line must be shaped.
    ///
    /// `shaped` returns the shaped width of a string after tab expansion.
    pub(super) fn line_widths(
        &mut self,
        line: &str,
        monospace: &MonospaceWidths,
        tab_stop: Pixels,
        shaped: &mut impl FnMut(&str) -> Pixels,
    ) -> Option<LineWidths<'_>> {
        if line.is_empty()
            || (tab_stop <= px(0.) && line.contains('\t'))
            || is_printable_ascii(line)
        {
            return None;
        }
        if self.ascii.is_none() {
            let table = monospace.width(TABLE_CHARS)?;
            self.ascii = Some(f64::from(f32::from(table)) / TABLE_CHARS as f64);
        }
        let ascii_and_tabs = self.fill_prefix(line, shaped)?;
        let passed = if ascii_and_tabs {
            self.passes_ascii_tabs_check(line, tab_stop, shaped)
        } else {
            self.passes_line_check(line, tab_stop, shaped)
        };
        passed.then(|| self.view(tab_stop))
    }

    /// Fill `prefix` and `tabs` for `line`, measuring characters not seen before.
    ///
    /// Returns whether the line is only printable ASCII and tabs.
    fn fill_prefix(&mut self, line: &str, shaped: &mut impl FnMut(&str) -> Pixels) -> Option<bool> {
        let ascii = self.ascii?;
        self.prefix.clear();
        self.tabs.clear();
        self.prefix.push(0.0);
        let mut sum = 0.0;
        let mut ascii_and_tabs = true;
        for (offset, c) in line.char_indices() {
            if c == '\t' {
                self.tabs.push(offset);
            } else if matches!(c, ' '..='~') {
                sum += ascii;
            } else if is_fast_char(c) {
                ascii_and_tabs = false;
                sum += self.advance_of(c, shaped)?;
            } else {
                return None;
            }
            for _ in 0..c.len_utf8() {
                self.prefix.push(sum);
            }
        }
        Some(ascii_and_tabs)
    }

    /// Whether lines of printable ASCII and tabs are measured correctly.
    ///
    /// Their widths come from the monospace table, which is already checked against the shaper,
    /// and from tab stop arithmetic. Only the arithmetic is new, so the first such line is
    /// compared with the shaper and its result stands for the rest.
    fn passes_ascii_tabs_check(
        &mut self,
        line: &str,
        tab_stop: Pixels,
        shaped: &mut impl FnMut(&str) -> Pixels,
    ) -> bool {
        if let Some(passed) = self.ascii_tabs_check {
            return passed;
        }
        let passed = self.matches_shaper(line, tab_stop, shaped);
        self.ascii_tabs_check = Some(passed);
        passed
    }

    /// Whether the summed width of `line` matches its shaped width, shaping it once per distinct
    /// text. The result does not depend on the wrap width.
    fn passes_line_check(
        &mut self,
        line: &str,
        tab_stop: Pixels,
        shaped: &mut impl FnMut(&str) -> Pixels,
    ) -> bool {
        let mut hasher = DefaultHasher::new();
        line.hash(&mut hasher);
        let key = hasher.finish();
        if !self.checks.needs_check(key) {
            return self.checks.results[&key];
        }
        let passed = self.matches_shaper(line, tab_stop, shaped);
        self.checks.record(key, passed);
        passed
    }

    /// Whether the summed width of `line`, whose prefix sums are filled, equals its shaped width.
    fn matches_shaper(
        &self,
        line: &str,
        tab_stop: Pixels,
        shaped: &mut impl FnMut(&str) -> Pixels,
    ) -> bool {
        let summed = f32::from(self.view(tab_stop).width(0..line.len()));
        let actual = f32::from(shaped(line));
        (summed - actual).abs() <= LINE_TOLERANCE.max(actual * 1e-5)
    }

    /// The advance of `c`, shaping it on a miss. `None` when `c` failed the check.
    fn advance_of(&mut self, c: char, shaped: &mut impl FnMut(&str) -> Pixels) -> Option<f64> {
        if let Some(known) = self.others.get(&c) {
            return *known;
        }
        let one = f64::from(f32::from(shaped(&c.to_string())));
        let eight = f64::from(f32::from(shaped(&c.to_string().repeat(8))));
        let passed = one > 0.0 && (eight - 8.0 * one).abs() <= ADVANCE_TOLERANCE;
        let advance = passed.then_some(eight / 8.0);
        self.others.insert(c, advance);
        advance
    }

    fn view(&self, tab_stop: Pixels) -> LineWidths<'_> {
        LineWidths {
            prefix: &self.prefix,
            tabs: &self.tabs,
            tab_stop: f64::from(f32::from(tab_stop)),
        }
    }
}

/// Widths of the substrings of one line.
pub(super) struct LineWidths<'a> {
    prefix: &'a [f64],
    tabs: &'a [usize],
    tab_stop: f64,
}

impl LineWidths<'_> {
    /// Width of `line[range]` measured as a string of its own, so tab stops count from `range.start`.
    pub(super) fn width(&self, range: Range<usize>) -> Pixels {
        let first = self.tabs.partition_point(|&tab| tab < range.start);
        let last = self.tabs.partition_point(|&tab| tab < range.end);
        let mut x = 0.0;
        let mut pos = range.start;
        for &tab in &self.tabs[first..last] {
            x += self.prefix[tab] - self.prefix[pos];
            x = self.tab_stop * ((x / self.tab_stop).floor() + 1.0);
            pos = tab + 1;
        }
        x += self.prefix[range.end] - self.prefix[pos];
        px(x as f32)
    }
}

/// Lines already compared with the shaper, keyed by a hash of their text.
#[derive(Default)]
struct LineChecks {
    results: HashMap<u64, bool>,
}

impl LineChecks {
    /// Whether `line` must be compared with the shaper now. Always true for a line not seen before.
    fn needs_check(&self, key: u64) -> bool {
        !self.results.contains_key(&key)
    }

    fn record(&mut self, key: u64, passed: bool) {
        if self.results.len() >= MAX_CHECKED_LINES {
            self.results.clear();
        }
        self.results.insert(key, passed);
    }
}

/// Whether `c` is one grapheme cluster on its own that never combines with a neighbor.
///
/// A line made only of these characters is wrapped by summed widths. Everything else, such as
/// emoji, combining marks, and joining scripts, stays on the shaped path.
pub(super) fn is_fast_char(c: char) -> bool {
    matches!(
        c,
        '\u{0020}'..='\u{007E}'
            | '\u{0009}'
            | '\u{00A1}'..='\u{00AC}'
            | '\u{00AE}'..='\u{00FF}'
            | '\u{0100}'..='\u{017F}'
            | '\u{0391}'..='\u{03A1}'
            | '\u{03A3}'..='\u{03C9}'
            | '\u{0401}'
            | '\u{0410}'..='\u{044F}'
            | '\u{0451}'
            | '\u{2010}'..='\u{2027}'
            | '\u{2030}'..='\u{203A}'
            | '\u{3000}'..='\u{3029}'
            | '\u{3030}'..='\u{303F}'
            | '\u{3041}'..='\u{3096}'
            | '\u{30A1}'..='\u{30FA}'
            | '\u{30FC}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{AC00}'..='\u{D7A3}'
            | '\u{FF01}'..='\u{FF60}'
            | '\u{FFE0}'..='\u{FFE6}'
    )
}

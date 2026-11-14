//! Wrapping lines with tabs and non-ASCII text by summing per-character widths.
//!
//! In a monospace font the width of a line is the sum of the advances of its characters, so a
//! line made only of characters that never combine with a neighbor can be measured without
//! shaping every prefix of it. New characters are shaped once, many to a shaping call, and each
//! distinct line is compared with the shaper once, so a wrong sum is caught before a break could
//! move. Lines of printable ASCII and tabs share one such comparison, because their widths come
//! from the checked monospace table.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
};

use gpui::{Pixels, px};

use super::monospace::{MonospaceWidths, TABLE_CHARS, is_printable_ascii};
use crate::input::tab_stops::next_tab_stop;

#[cfg(test)]
mod tests;

/// Capacity, in entries, above which a wrap pass releases the line buffers.
const MAX_RETAINED_ENTRIES: usize = 64 * 1024;

/// Checked lines remembered before the record is cleared.
const MAX_CHECKED_LINES: usize = 1 << 20;

/// How far a character's eight-fold run may differ from eight times its single advance, in pixels.
const ADVANCE_TOLERANCE: f64 = 0.01;

/// Copies of a new character shaped in a row to measure it.
const RUN_CHARS: usize = 8;

/// New characters measured by one shaping call. Glyph positions are `f32`, so a longer batch
/// would round the positions near its end by more than `ADVANCE_TOLERANCE` allows.
const BATCH_CHARS: usize = 16;

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
    /// Characters of the line that have no entry in `others` yet.
    unmeasured: Vec<char>,
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
        if self.unmeasured.capacity() > MAX_RETAINED_ENTRIES {
            self.unmeasured = Vec::new();
        }
    }

    /// Widths for `line`, or `None` when the line must be shaped.
    ///
    /// `shaped` returns the shaped width of a string after tab expansion. `glyph_starts` returns
    /// the x position of each glyph of a string, followed by the width of the string.
    pub(super) fn line_widths(
        &mut self,
        line: &str,
        monospace: &MonospaceWidths,
        tab_stop: Pixels,
        shaped: &mut impl FnMut(&str) -> Pixels,
        glyph_starts: &mut impl FnMut(&str) -> Vec<Pixels>,
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
        let ascii_and_tabs = self.fill_prefix(line, shaped, glyph_starts)?;
        let passed = if ascii_and_tabs {
            self.passes_ascii_tabs_check(line, tab_stop, shaped)
        } else {
            self.passes_line_check(line, tab_stop, shaped)
        };
        passed.then(|| self.view(tab_stop))
    }

    /// Fill `prefix` and `tabs` for `line`, measuring characters not seen before.
    ///
    /// Returns whether the line is only printable ASCII and tabs, or `None` when the line holds a
    /// character that must be shaped.
    fn fill_prefix(
        &mut self,
        line: &str,
        shaped: &mut impl FnMut(&str) -> Pixels,
        glyph_starts: &mut impl FnMut(&str) -> Vec<Pixels>,
    ) -> Option<bool> {
        match self.sum_known(line) {
            Summed::Done { ascii_and_tabs } => return Some(ascii_and_tabs),
            Summed::Ineligible => return None,
            Summed::Unmeasured => {}
        }
        self.measure_new(shaped, glyph_starts);
        match self.sum_known(line) {
            Summed::Done { ascii_and_tabs } => Some(ascii_and_tabs),
            Summed::Ineligible | Summed::Unmeasured => None,
        }
    }

    /// Fill `prefix` and `tabs` from the advances already known.
    ///
    /// When the line has characters with no advance yet, they are left in `unmeasured` and the
    /// buffers are not complete.
    fn sum_known(&mut self, line: &str) -> Summed {
        let Some(ascii) = self.ascii else {
            return Summed::Ineligible;
        };
        self.prefix.clear();
        self.tabs.clear();
        self.unmeasured.clear();
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
                match self.others.get(&c) {
                    Some(Some(advance)) => sum += advance,
                    Some(None) => return Summed::Ineligible,
                    None => self.unmeasured.push(c),
                }
            } else {
                return Summed::Ineligible;
            }
            if self.unmeasured.is_empty() {
                for _ in 0..c.len_utf8() {
                    self.prefix.push(sum);
                }
            }
        }
        if self.unmeasured.is_empty() {
            Summed::Done { ascii_and_tabs }
        } else {
            Summed::Unmeasured
        }
    }

    /// Measure the characters in `unmeasured`, several to a shaping call.
    fn measure_new(
        &mut self,
        shaped: &mut impl FnMut(&str) -> Pixels,
        glyph_starts: &mut impl FnMut(&str) -> Vec<Pixels>,
    ) {
        let mut new = std::mem::take(&mut self.unmeasured);
        new.sort_unstable();
        new.dedup();
        for batch in new.chunks(BATCH_CHARS) {
            if !self.measure_batch(batch, glyph_starts) {
                for &c in batch {
                    self.measure_alone(c, shaped);
                }
            }
        }
        new.clear();
        self.unmeasured = new;
    }

    /// Measure every character of `batch` with one shaping call.
    ///
    /// The shaped text is a run of each character, with a space between runs. The last glyph of a
    /// run stands next to a space or the end of the text, so its advance is the advance of the
    /// character on its own. A character whose run is not `RUN_CHARS` times that advance changes
    /// width next to itself and is rejected.
    ///
    /// Returns false, having recorded nothing, when the shaper did not return one glyph per
    /// character, because the glyphs cannot be matched to characters then.
    fn measure_batch(
        &mut self,
        batch: &[char],
        glyph_starts: &mut impl FnMut(&str) -> Vec<Pixels>,
    ) -> bool {
        let stride = RUN_CHARS + 1;
        let mut text = String::with_capacity(batch.len() * stride * 3);
        for (i, &c) in batch.iter().enumerate() {
            if i > 0 {
                text.push(' ');
            }
            text.extend(std::iter::repeat_n(c, RUN_CHARS));
        }
        let starts = glyph_starts(&text);
        let in_order = starts.windows(2).all(|pair| pair[0] <= pair[1]);
        if starts.len() != batch.len() * stride || !in_order {
            return false;
        }
        let at = |glyph: usize| f64::from(f32::from(starts[glyph]));
        for (i, &c) in batch.iter().enumerate() {
            let first = i * stride;
            let one = at(first + RUN_CHARS) - at(first + RUN_CHARS - 1);
            let run = at(first + RUN_CHARS) - at(first);
            self.record(c, one, run);
        }
        true
    }

    /// Measure `c` with two shaping calls, on its own and as a run.
    fn measure_alone(&mut self, c: char, shaped: &mut impl FnMut(&str) -> Pixels) {
        let mut text = c.to_string();
        let one = f64::from(f32::from(shaped(&text)));
        text.extend(std::iter::repeat_n(c, RUN_CHARS - 1));
        let run = f64::from(f32::from(shaped(&text)));
        self.record(c, one, run);
    }

    /// Record the advance of `c` from its width alone and as a run of `RUN_CHARS`.
    fn record(&mut self, c: char, one: f64, run: f64) {
        let count = RUN_CHARS as f64;
        let passed = one > 0.0 && (run - count * one).abs() <= ADVANCE_TOLERANCE;
        self.others.insert(c, passed.then_some(run / count));
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

    fn view(&self, tab_stop: Pixels) -> LineWidths<'_> {
        LineWidths {
            prefix: &self.prefix,
            tabs: &self.tabs,
            tab_stop: f64::from(f32::from(tab_stop)),
        }
    }
}

/// What `CharWidths::sum_known` found in a line.
enum Summed {
    /// Every character has an advance, and the buffers are filled.
    Done { ascii_and_tabs: bool },
    /// The line holds a character that must be shaped.
    Ineligible,
    /// The line holds eligible characters that are not measured yet.
    Unmeasured,
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
            x = next_tab_stop(x, self.tab_stop);
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

//! A single-line input's text as AccessKit text runs.
//!
//! AccessKit gives an assistive client a field's text, caret and selection only when the
//! field's node has `Role::TextRun` children (`accesskit_consumer` `supports_text_ranges`):
//! the macOS adapter then answers `AXNumberOfCharacters`, `AXSelectedTextRange` and the text
//! range calls, the AT-SPI adapter offers the Text interface, and the UIA adapter offers the
//! Text pattern. Without runs a screen reader cannot read what was typed.
//!
//! The runs are captured at render time from the editor's text and active selection and
//! pushed as synthetic children with `a11y_synthetic_children`. A character is a grapheme
//! cluster, as the editor moves its caret. Character positions come from the previous frame's
//! layout and are given only while that layout holds exactly the current text, so a run is
//! never described with another text's geometry; the next frame brings them.
use super::{InputBaseState, kind::InputModeKind};
use gpui::{Bounds, Pixels, Window, accesskit};
use unicode_segmentation::UnicodeSegmentation as _;

impl<M: InputModeKind> InputBaseState<M> {
    /// The runs for this frame, or None where the field gives no text: no accessibility
    /// client, a masked or unexposed value, or a multi-line editor (not supported yet).
    pub(super) fn accessibility_text_runs(&self, window: &Window) -> Option<A11yTextRuns> {
        let accessibility = self.accessibility.as_ref()?;
        if !window.is_a11y_active()
            || !accessibility.value_exposed()
            || self.masked
            || self.is_multi_line()
        {
            return None;
        }
        let text = self.text.to_string();
        let range = self.selected_range();
        let selection = if self.is_selection_reversed() {
            (range.end, range.start)
        } else {
            (range.start, range.end)
        };
        // The previous frame's layout describes this text only while its one line has
        // exactly this text's length.
        let has_layout = self.last_layout.as_ref().is_some_and(|layout| {
            layout.lines.len() == 1
                && layout.visible_line_byte_offsets.first() == Some(&0)
                && layout.lines[0].len() == text.len()
        });
        A11yTextRuns::capture(&text, selection, window.scale_factor(), |bytes| {
            has_layout.then(|| self.range_to_bounds(&bytes)).flatten()
        })
    }
}

/// AccessKit's per-run lists hold `u8`s, so a run holds at most this many characters.
const RUN_CHARACTERS: usize = u8::MAX as usize;

/// One run's share of the field's text.
struct Run {
    text: String,
    character_lengths: Vec<u8>,
    word_starts: Vec<u8>,
    /// Each character's left edge and advance, in physical pixels, relative to `bounds.x0`.
    geometry: Option<(Vec<f32>, Vec<f32>)>,
    bounds: Option<accesskit::Rect>,
}

/// The field's text runs and its selection, as character positions in those runs.
pub(crate) struct A11yTextRuns {
    runs: Vec<Run>,
    anchor: usize,
    focus: usize,
}

/// One grapheme of the field: its bytes in the text and, when known, its laid-out bounds.
pub(crate) struct Grapheme<'a> {
    pub(crate) text: &'a str,
    pub(crate) is_word_start: bool,
    pub(crate) bounds: Option<Bounds<Pixels>>,
}

impl A11yTextRuns {
    /// `selection` is the active selection as byte offsets (anchor, focus); `bounds_of` gives a
    /// byte range's laid-out bounds in window coordinates, or None.
    pub(crate) fn capture(
        text: &str,
        selection: (usize, usize),
        scale: f32,
        bounds_of: impl Fn(std::ops::Range<usize>) -> Option<Bounds<Pixels>>,
    ) -> Option<Self> {
        let graphemes = graphemes(text, &bounds_of);
        // A grapheme longer than a u8 of bytes cannot be described; expose nothing rather
        // than a run whose lengths do not add up to its text.
        if graphemes
            .iter()
            .any(|g| g.text.len() > usize::from(u8::MAX))
        {
            return None;
        }
        let has_geometry = graphemes.iter().all(|g| g.bounds.is_some());
        let mut runs: Vec<Run> = graphemes
            .chunks(RUN_CHARACTERS)
            .map(|chunk| Run::new(chunk, has_geometry, scale))
            .collect();
        if runs.is_empty() {
            // An empty field still has a caret, and a caret needs a run to sit in.
            runs.push(Run::new(&[], false, scale));
        }
        Some(Self {
            runs,
            anchor: character_index(text, selection.0),
            focus: character_index(text, selection.1),
        })
    }

    pub(crate) fn push(self, builder: &mut gpui::A11ySubtreeBuilder) {
        let mut ids = Vec::with_capacity(self.runs.len());
        for (index, run) in self.runs.into_iter().enumerate() {
            let id = builder.synthetic_node_id(("text-run", index));
            let mut node = accesskit::Node::new(accesskit::Role::TextRun);
            node.set_value(run.text);
            node.set_character_lengths(run.character_lengths);
            node.set_word_starts(run.word_starts);
            if let Some((positions, widths)) = run.geometry {
                node.set_character_positions(positions);
                node.set_character_widths(widths);
            }
            if let Some(bounds) = run.bounds {
                node.set_bounds(bounds);
            }
            builder.push_child(id, node);
            ids.push(id);
        }
        let anchor = position(&ids, self.anchor);
        let focus = position(&ids, self.focus);
        builder
            .parent_node()
            .set_text_selection(accesskit::TextSelection { anchor, focus });
    }
}

impl Run {
    fn new(chunk: &[Grapheme<'_>], has_geometry: bool, scale: f32) -> Self {
        let text: String = chunk.iter().map(|g| g.text).collect();
        let character_lengths = chunk.iter().map(|g| g.text.len() as u8).collect();
        let word_starts = chunk
            .iter()
            .enumerate()
            .filter(|(_, g)| g.is_word_start)
            .map(|(index, _)| index as u8)
            .collect();
        let bounds: Option<Vec<Bounds<Pixels>>> =
            has_geometry.then(|| chunk.iter().filter_map(|g| g.bounds).collect());
        let run_bounds = bounds.as_ref().and_then(|all| union(all, scale));
        let geometry = bounds.zip(run_bounds).map(|(all, run)| {
            let positions = all
                .iter()
                .map(|b| f32::from(b.origin.x) * scale - run.x0 as f32)
                .collect();
            let widths = all
                .iter()
                .map(|b| f32::from(b.size.width) * scale)
                .collect();
            (positions, widths)
        });
        Self {
            text,
            character_lengths,
            word_starts,
            geometry,
            bounds: run_bounds,
        }
    }
}

fn graphemes<'a>(
    text: &'a str,
    bounds_of: &impl Fn(std::ops::Range<usize>) -> Option<Bounds<Pixels>>,
) -> Vec<Grapheme<'a>> {
    // A word starts at the first character after whitespace, so trailing whitespace stays
    // with the word before it (AccessKit's `word_starts` convention).
    let mut was_space = true;
    text.grapheme_indices(true)
        .map(|(start, grapheme)| {
            let is_space = grapheme.chars().all(char::is_whitespace);
            let is_word_start = was_space && !is_space && start != 0;
            was_space = is_space;
            Grapheme {
                text: grapheme,
                is_word_start,
                bounds: bounds_of(start..start + grapheme.len()),
            }
        })
        .collect()
}

/// The grapheme index at byte `offset`, counting a grapheme the offset falls inside as before it.
fn character_index(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .take_while(|(start, _)| *start < offset)
        .count()
}

/// The position of the `index`-th character of the whole text, in its run. An index at the
/// end of a full run sits at that run's end, never at the start of an absent next run.
fn position(ids: &[accesskit::NodeId], index: usize) -> accesskit::TextPosition {
    let run = (index / RUN_CHARACTERS).min(ids.len() - 1);
    accesskit::TextPosition {
        node: ids[run],
        character_index: index - run * RUN_CHARACTERS,
    }
}

/// The physical-pixel rectangle around every bounds in `all`.
fn union(all: &[Bounds<Pixels>], scale: f32) -> Option<accesskit::Rect> {
    let first = all.first()?;
    let mut rect = *first;
    for bounds in &all[1..] {
        rect = rect.union(bounds);
    }
    Some(accesskit::Rect {
        x0: f64::from(f32::from(rect.origin.x) * scale),
        y0: f64::from(f32::from(rect.origin.y) * scale),
        x1: f64::from(f32::from(rect.origin.x + rect.size.width) * scale),
        y1: f64::from(f32::from(rect.origin.y + rect.size.height) * scale),
    })
}

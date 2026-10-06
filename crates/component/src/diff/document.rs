use std::{ops::Range, sync::Arc, time::Duration};

use gpui::SharedString;
#[cfg(test)]
use similar::TextDiff;
use smallvec::{SmallVec, smallvec};
use unicode_segmentation::UnicodeSegmentation as _;

use crate::{Rope, highlighter::SyntaxHighlighter};

/// One parsed file side containing only the source fragments in the patch.
/// An empty side is distinct from a missing side such as `/dev/null`.
#[derive(Clone, Debug)]
pub struct DiffFile {
    name: SharedString,
    text: SharedString,
}

impl DiffFile {
    pub(crate) fn from_patch(name: SharedString, text: SharedString) -> Self {
        Self { name, text }
    }
    /// Retains a file's name and exact source, including line endings.
    #[cfg(test)]
    pub(crate) fn new(name: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
        }
    }

    pub fn name(&self) -> &SharedString {
        &self.name
    }
    /// Concatenated patch-provided fragments, not the complete file contents.
    pub fn text(&self) -> &SharedString {
        &self.text
    }
    fn detected_language(&self) -> SharedString {
        // File paths may come from another platform. Only the final component
        // determines the language; dots in a directory are not extensions.
        let name = self
            .name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .to_lowercase();
        let language = match name.as_str() {
            "makefile" | "gnumakefile" => "make",
            "cmakelists.txt" => "cmake",
            _ => match name.rsplit_once('.').map(|(_, extension)| extension) {
                Some("jsx" | "mjs" | "cjs") => "javascript",
                Some("mts" | "cts") => "typescript",
                Some("cc" | "cxx" | "hpp" | "hxx") => "cpp",
                Some("h") => "c",
                Some("htm") => "html",
                Some("gql") => "graphql",
                Some(
                    extension @ ("astro" | "sh" | "bash" | "c" | "cmake" | "cs" | "cpp" | "css"
                    | "scss" | "diff" | "ejs" | "ex" | "exs" | "erb" | "go"
                    | "graphql" | "html" | "java" | "js" | "json" | "jsonc" | "kt"
                    | "kts" | "lua" | "md" | "mdx" | "php" | "php3" | "php4" | "php5"
                    | "phtml" | "proto" | "py" | "pyi" | "rb" | "rs" | "scala" | "sql"
                    | "svelte" | "swift" | "toml" | "tsx" | "ts" | "yaml" | "yml"
                    | "zig"),
                ) => extension,
                _ => "text",
            },
        };
        crate::highlighter::language_name(if language == "exs" {
            "elixir"
        } else {
            language
        })
    }
}

/// Which source version a position belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiffSide {
    Original,
    Modified,
}

/// A one-based line in a source file, independent of the visual layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiffLinePosition {
    side: DiffSide,
    line: usize,
}

impl DiffLinePosition {
    /// Creates a position, clamping the line to at least one.
    pub fn new(side: DiffSide, line: usize) -> Self {
        Self {
            side,
            line: line.max(1),
        }
    }
    pub fn side(&self) -> DiffSide {
        self.side
    }
    pub fn line(&self) -> usize {
        self.line
    }
}

/// A selected inclusive range of source lines on one file side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffLineRange {
    side: DiffSide,
    start: usize,
    end: usize,
}

impl DiffLineRange {
    /// Normalizes the endpoints and clamps them to one-based line numbers.
    pub fn new(side: DiffSide, start: usize, end: usize) -> Self {
        Self {
            side,
            start: start.min(end).max(1),
            end: start.max(end).max(1),
        }
    }
    pub fn side(&self) -> DiffSide {
        self.side
    }
    pub fn start(&self) -> usize {
        self.start
    }
    pub fn end(&self) -> usize {
        self.end
    }
    pub(crate) fn contains(&self, position: DiffLinePosition) -> bool {
        self.side == position.side && (self.start..=self.end).contains(&position.line)
    }
}

#[derive(Debug)]
pub(crate) struct SourceLine {
    pub line_number: usize,
    pub text: SharedString,
    pub display: SharedString,
    pub chunks: SmallVec<[DisplayChunk; 1]>,
    pub counterpart: Option<usize>,
    // Only expanded tabs need an entry; ordinary UTF-8 bytes map by identity
    // plus the cumulative expansion of earlier tabs.
    display_tabs: Vec<DisplayTab>,
    pub source: Range<usize>,
    pub content_end: usize,
}

#[derive(Debug)]
pub(crate) struct DisplayChunk {
    pub range: Range<usize>,
    pub text: SharedString,
}

#[derive(Debug)]
struct DisplayTab {
    source_offset: usize,
    display_offset: usize,
    width: usize,
}

impl SourceLine {
    /// Reuses the retained text for a projected chunk. The fallback supports
    /// valid partial ranges without making ordinary rendering allocate.
    pub(crate) fn chunk_text(&self, range: &Range<usize>) -> SharedString {
        if range.start == 0 && range.end == self.display.len() {
            return self.display.clone();
        }
        let ix = self
            .chunks
            .partition_point(|chunk| chunk.range.start < range.start);
        if let Some(chunk) = self.chunks.get(ix)
            && chunk.range == *range
        {
            return chunk.text.clone();
        }
        self.display
            .get(range.clone())
            .map_or_else(SharedString::default, SharedString::from)
    }

    pub fn source_offset(&self, display_offset: usize) -> usize {
        let display_offset = display_offset.min(self.display.len());
        let preceding = self
            .display_tabs
            .partition_point(|tab| tab.display_offset <= display_offset);
        let Some(tab) = preceding.checked_sub(1).map(|ix| &self.display_tabs[ix]) else {
            return display_offset;
        };
        if display_offset < tab.display_offset + tab.width {
            tab.source_offset
        } else {
            display_offset - (tab.display_offset + tab.width - tab.source_offset - 1)
        }
    }
    pub fn display_range(&self, source: Range<usize>) -> Range<usize> {
        self.display_offset(source.start)..self.display_offset(source.end)
    }

    fn display_offset(&self, source_offset: usize) -> usize {
        let source_offset = source_offset.min(self.text.len());
        let preceding = self
            .display_tabs
            .partition_point(|tab| tab.source_offset < source_offset);
        let expansion = preceding.checked_sub(1).map_or(0, |ix| {
            let tab = &self.display_tabs[ix];
            tab.display_offset + tab.width - tab.source_offset - 1
        });
        source_offset + expansion
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinePair {
    pub original: Option<usize>,
    pub modified: Option<usize>,
    pub changed: bool,
}

pub(crate) struct DocumentInner {
    pub original: Option<DiffFile>,
    pub modified: Option<DiffFile>,
    pub original_lines: Vec<SourceLine>,
    pub modified_lines: Vec<SourceLine>,
    pub pairs: Vec<LinePair>,
    pub additions: usize,
    pub deletions: usize,
    pub hunk_boundaries: Vec<(usize, SharedString)>,
    pub metadata: Vec<SharedString>,
    pub binary: bool,
    pub original_highlighter: Option<SyntaxHighlighter>,
    pub modified_highlighter: Option<SyntaxHighlighter>,
}

/// Immutable display data parsed from a unified or Git diff.
/// Only source fragments present in the patch are retained; unavailable context
/// is never reconstructed or compared.
#[derive(Clone)]
pub struct DiffDocument(pub(crate) Arc<DocumentInner>);

impl DiffDocument {
    /// Compares two existing file versions, including empty files.
    #[cfg(test)]
    pub(crate) fn new(original: DiffFile, modified: DiffFile) -> Self {
        Self::build(Some(original), Some(modified))
    }
    /// Compares a newly added file against a deliberately missing original.
    #[cfg(test)]
    pub(crate) fn added(modified: DiffFile) -> Self {
        Self::build(None, Some(modified))
    }
    /// Compares a deleted file against a deliberately missing modified version.
    #[cfg(test)]
    pub(crate) fn deleted(original: DiffFile) -> Self {
        Self::build(Some(original), None)
    }

    /// Parses all files in a unified or Git diff without computing differences.
    pub fn parse(
        patch: impl Into<SharedString>,
    ) -> Result<Vec<Self>, super::parser::DiffParseError> {
        super::parser::parse(patch.into())
    }

    /// Git metadata such as file modes, similarity and rename headers.
    pub fn metadata(&self) -> &[SharedString] {
        &self.0.metadata
    }

    /// Whether the patch reports binary contents rather than textual hunks.
    pub fn is_binary(&self) -> bool {
        self.0.binary
    }

    pub fn original(&self) -> Option<&DiffFile> {
        self.0.original.as_ref()
    }
    pub fn modified(&self) -> Option<&DiffFile> {
        self.0.modified.as_ref()
    }
    pub fn additions(&self) -> usize {
        self.0.additions
    }
    pub fn deletions(&self) -> usize {
        self.0.deletions
    }
    pub fn has_changes(&self) -> bool {
        self.additions() > 0
            || self.deletions() > 0
            || !self.0.metadata.is_empty()
            || self.0.binary
            || self.original().is_none()
            || self.modified().is_none()
    }
    /// Number of retained source lines, excluding unavailable context.
    pub fn line_count(&self, side: DiffSide) -> usize {
        self.lines(side).len()
    }

    /// Copies patch-provided lines. Unavailable context is omitted, without inventing source lines.
    pub fn text_for_range(&self, range: DiffLineRange) -> String {
        let lines = self.lines(range.side);
        let start_ix = lines.partition_point(|line| line.line_number < range.start);
        let end_ix = lines.partition_point(|line| line.line_number <= range.end);
        let selected = &lines[start_ix..end_ix];
        if selected.is_empty() {
            return String::new();
        }
        self.source(range.side)[selected[0].source.start..selected.last().unwrap().source.end]
            .to_owned()
    }

    pub(crate) fn position(&self, side: DiffSide, index: usize) -> DiffLinePosition {
        DiffLinePosition::new(side, self.lines(side)[index].line_number)
    }
    pub(crate) fn line_index(&self, position: DiffLinePosition) -> Option<usize> {
        self.lines(position.side)
            .binary_search_by_key(&position.line, |line| line.line_number)
            .ok()
    }

    pub(crate) fn lines(&self, side: DiffSide) -> &[SourceLine] {
        match side {
            DiffSide::Original => &self.0.original_lines,
            DiffSide::Modified => &self.0.modified_lines,
        }
    }
    pub(crate) fn source(&self, side: DiffSide) -> &str {
        match side {
            DiffSide::Original => self.original(),
            DiffSide::Modified => self.modified(),
        }
        .map_or("", |file| file.text.as_str())
    }
    pub(crate) fn highlighter(&self, side: DiffSide) -> Option<&SyntaxHighlighter> {
        match side {
            DiffSide::Original => self.0.original_highlighter.as_ref(),
            DiffSide::Modified => self.0.modified_highlighter.as_ref(),
        }
    }

    // Existing interaction fixtures use complete strings to generate an input
    // patch, then exercise the production parser. No comparison ships in UI.
    #[cfg(test)]
    fn build(original: Option<DiffFile>, modified: Option<DiffFile>) -> Self {
        fn path(name: &str) -> String {
            format!(
                "\"{}\"",
                name.replace('\\', "\\\\")
                    .replace('\"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t")
            )
        }
        let old = original.as_ref().map_or("", |file| file.text.as_str());
        let new = modified.as_ref().map_or("", |file| file.text.as_str());
        let old_lines: Vec<&str> = old.split_inclusive('\n').collect();
        let new_lines: Vec<&str> = new.split_inclusive('\n').collect();
        let old_name = original
            .as_ref()
            .map_or_else(|| "/dev/null".to_owned(), |file| path(&file.name));
        let new_name = modified
            .as_ref()
            .map_or_else(|| "/dev/null".to_owned(), |file| path(&file.name));
        let diff = TextDiff::from_slices(&old_lines, &new_lines);
        let mut patch = diff
            .unified_diff()
            .context_radius(old_lines.len().max(new_lines.len()))
            .header(&old_name, &new_name)
            .to_string();
        if patch.is_empty() {
            patch = format!("--- {old_name}\n+++ {new_name}\n");
            if !old_lines.is_empty() {
                patch.push_str(&format!(
                    "@@ -1,{} +1,{} @@\n",
                    old_lines.len(),
                    new_lines.len()
                ));
                for line in &old_lines {
                    patch.push(' ');
                    patch.push_str(line);
                    if !line.ends_with('\n') {
                        patch.push_str("\n\\ No newline at end of file\n");
                    }
                }
            }
        }
        if original.is_none() {
            patch.insert_str(0, "diff --git a/fixture b/fixture\nnew file mode 100644\n");
        } else if modified.is_none() {
            patch.insert_str(
                0,
                "diff --git a/fixture b/fixture\ndeleted file mode 100644\n",
            );
        }
        let mut document = Self::parse(patch)
            .expect("valid generated fixture patch")
            .remove(0);
        let inner = Arc::get_mut(&mut document.0).unwrap();
        // Single full-context fixtures predate hunk navigation; explicit patch
        // tests retain boundaries and verify their actual UI projection.
        inner.hunk_boundaries.clear();
        document
    }
}

pub(crate) fn prepare_highlighter(file: Option<&DiffFile>) -> Option<SyntaxHighlighter> {
    let file = file?;
    let mut highlighter = SyntaxHighlighter::new(&file.detected_language());
    // This is a host-parser budget, not a bound on highlighter construction:
    // query compilation and injected-language parsing have their own costs.
    highlighter.update(
        None,
        &Rope::from_str(&file.text),
        Some(Duration::from_millis(100)),
    );
    Some(highlighter)
}

pub(crate) fn source_lines(text: &str) -> Vec<SourceLine> {
    let mut offset = 0;
    text.split_inclusive('\n')
        .enumerate()
        .map(|(index, source)| {
            let content = source.strip_suffix('\n').unwrap_or(source);
            let content = if source.ends_with('\n') {
                content.strip_suffix('\r').unwrap_or(content)
            } else {
                content
            };
            let text = SharedString::from(content);
            let (display, display_tabs): (SharedString, Vec<DisplayTab>) = if content.contains('\t')
            {
                let mut display = String::with_capacity(content.len());
                let mut display_tabs = Vec::new();
                let mut column = 0;
                for (ix, ch) in content.char_indices() {
                    if ch == '\t' {
                        let spaces = 4 - column % 4;
                        display_tabs.push(DisplayTab {
                            source_offset: ix,
                            display_offset: display.len(),
                            width: spaces,
                        });
                        for _ in 0..spaces {
                            display.push(' ');
                        }
                        column += spaces;
                    } else {
                        display.push(ch);
                        column += 1;
                    }
                }
                (display.into(), display_tabs)
            } else {
                (text.clone(), Vec::new())
            };
            let chunks = display_chunks(&display)
                .into_iter()
                .map(|range| {
                    let text = if range.start == 0 && range.end == display.len() {
                        display.clone()
                    } else {
                        SharedString::from(&display[range.clone()])
                    };
                    DisplayChunk { range, text }
                })
                .collect();
            let line = SourceLine {
                line_number: index + 1,
                text,
                display,
                chunks,
                counterpart: None,
                display_tabs,
                source: offset..offset + source.len(),
                content_end: offset + content.len(),
            };
            offset += source.len();
            line
        })
        .collect()
}

/// Bound native selection projection while preserving grapheme boundaries.
/// Exceptionally large clusters split only at UTF-8 boundaries.
pub(crate) fn display_chunks(text: &str) -> SmallVec<[Range<usize>; 1]> {
    const MAX_BYTES: usize = 512;
    if text.len() <= MAX_BYTES {
        return smallvec![0..text.len()];
    }
    let mut chunks = SmallVec::new();
    let mut start = 0;
    let mut end = 0;
    for (ix, grapheme) in text.grapheme_indices(true) {
        if ix + grapheme.len() - start > MAX_BYTES && start < ix {
            chunks.push(start..ix);
            start = ix;
        }
        if grapheme.len() > MAX_BYTES {
            for (relative, character) in grapheme.char_indices() {
                let offset = ix + relative;
                if offset + character.len_utf8() - start > MAX_BYTES {
                    chunks.push(start..offset);
                    start = offset;
                }
            }
        }
        end = ix + grapheme.len();
    }
    if start < end {
        chunks.push(start..end);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_preserves_source_and_alignment() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DiffFile>();
        assert_send_sync::<DiffDocument>();
        let doc = DiffDocument::new(
            DiffFile::new("a.rs", "same\r\n旧值\r\nremoved\r\ntail"),
            DiffFile::new("a.rs", "same\r\n新值\r\ntail\r\n"),
        );
        assert_eq!((doc.additions(), doc.deletions()), (2, 3));
        assert_eq!(
            doc.0.pairs.first().unwrap(),
            &LinePair {
                original: Some(0),
                modified: Some(0),
                changed: false
            }
        );
        for side in [DiffSide::Original, DiffSide::Modified] {
            assert_eq!(
                doc.text_for_range(DiffLineRange::new(side, 1, usize::MAX)),
                doc.source(side)
            );
        }
        let same = DiffDocument::new(DiffFile::new("empty", ""), DiffFile::new("empty", ""));
        assert!(!same.has_changes());
        assert_eq!(same.line_count(DiffSide::Original), 0);
        let added = DiffDocument::added(DiffFile::new("new", "one\n\n"));
        assert_eq!((added.additions(), added.deletions()), (2, 0));
        assert!(added.0.pairs.iter().all(|p| p.original.is_none()));
        let deleted = DiffDocument::deleted(DiffFile::new("old", "one\n"));
        assert_eq!((deleted.additions(), deleted.deletions()), (0, 1));
        assert!(DiffDocument::added(DiffFile::new("empty", "")).has_changes());
    }

    #[test]
    fn alignment_covers_each_source_line_once() {
        for (old, new) in [
            ("one\ntwo\n", "one\ninserted\ntwo\n"),
            ("one\nremoved\ntwo\n", "one\ntwo\n"),
            ("old one\nold two\n", "new one\n"),
            (
                "repeat\nold\nrepeat\nsame\nold again\n",
                "repeat\nnew\nrepeat\nsame\nnew again\n",
            ),
            ("same\n", "same\r\n"),
            ("same\n", "same"),
            ("a\rb", "a\rc"),
            ("a\rb\nlast\r", "a\rc\nlast\r\n"),
            ("", "\n"),
            ("\n", ""),
        ] {
            let doc = DiffDocument::new(DiffFile::new("a.txt", old), DiffFile::new("a.txt", new));
            assert_eq!(
                doc.0
                    .pairs
                    .iter()
                    .filter_map(|pair| pair.original)
                    .collect::<Vec<_>>(),
                (0..doc.line_count(DiffSide::Original)).collect::<Vec<_>>()
            );
            assert_eq!(
                doc.0
                    .pairs
                    .iter()
                    .filter_map(|pair| pair.modified)
                    .collect::<Vec<_>>(),
                (0..doc.line_count(DiffSide::Modified)).collect::<Vec<_>>()
            );
            assert_eq!(
                doc.deletions(),
                doc.0
                    .pairs
                    .iter()
                    .filter(|pair| pair.changed && pair.original.is_some())
                    .count()
            );
            assert_eq!(
                doc.additions(),
                doc.0
                    .pairs
                    .iter()
                    .filter(|pair| pair.changed && pair.modified.is_some())
                    .count()
            );
            assert!(doc.has_changes());
            for pair in &doc.0.pairs {
                if !pair.changed {
                    let old_line = &doc.lines(DiffSide::Original)[pair.original.unwrap()];
                    let new_line = &doc.lines(DiffSide::Modified)[pair.modified.unwrap()];
                    assert_eq!(&old[old_line.source.clone()], &new[new_line.source.clone()]);
                }
            }
        }
        let doc = DiffDocument::new(
            DiffFile::new("a.txt", "same\nold\ntail\n"),
            DiffFile::new("a.txt", "same\nnew\nextra\ntail\n"),
        );
        assert_eq!(
            doc.0.pairs[1],
            LinePair {
                original: Some(1),
                modified: Some(1),
                changed: true
            }
        );
        assert_eq!(
            doc.0.pairs[2],
            LinePair {
                original: None,
                modified: Some(2),
                changed: true
            }
        );
        assert_eq!(
            doc.text_for_range(DiffLineRange::new(DiffSide::Modified, 3, 2)),
            "new\nextra\n"
        );
        assert_eq!(
            doc.text_for_range(DiffLineRange::new(DiffSide::Original, 99, 100)),
            ""
        );
        let bare_cr = DiffDocument::new(
            DiffFile::new("a.txt", "a\rb"),
            DiffFile::new("a.txt", "a\rb"),
        );
        assert!(!bare_cr.has_changes());
        assert_eq!(bare_cr.line_count(DiffSide::Original), 1);
        assert_eq!(bare_cr.0.pairs.len(), 1);
    }

    #[test]
    fn source_display_mapping_preserves_tabs_and_unicode() {
        let lines = source_lines("é\t中\t\r\nlast\r");
        let first = &lines[0];
        assert_eq!(first.text.as_str(), "é\t中\t");
        assert_eq!(first.display.as_str(), "é   中   ");
        assert_eq!(first.source, 0..9);
        assert_eq!(first.content_end, 7);
        assert_eq!(first.display_tabs.len(), 2);
        assert_eq!(first.display_range(2..3), 2..5);
        assert_eq!(first.display_range(6..7), 8..11);
        assert_eq!(
            first.display_range(0..first.text.len()),
            0..first.display.len()
        );
        for offset in 2..5 {
            assert_eq!(first.source_offset(offset), 2);
        }
        assert_eq!(first.source_offset(5), 3);
        assert_eq!(first.source_offset(first.display.len()), first.text.len());
        assert_eq!(first.source_offset(usize::MAX), first.text.len());
        assert_eq!(lines[1].text.as_str(), "last\r");
        assert!(lines[1].display_tabs.is_empty());
        assert_eq!(lines[1].source_offset(2), 2);
        assert_eq!(lines[1].source_offset(usize::MAX), lines[1].text.len());
        assert_eq!(lines[1].display_range(1..4), 1..4);
        let plain_unicode = source_lines("中é");
        assert!(plain_unicode[0].display_tabs.is_empty());
        assert_eq!(plain_unicode[0].display_range(3..5), 3..5);
        assert_eq!(plain_unicode[0].source_offset(3), 3);
        assert!(source_lines("").is_empty());
        assert_eq!(source_lines("\n").len(), 1);
        // Compare every byte against the former dense mapping, including
        // UTF-8 interiors and all spaces belonging to each expanded tab.
        for (display_offset, source_offset) in
            [0, 1, 2, 2, 2, 3, 4, 5, 6, 6, 6, 7].into_iter().enumerate()
        {
            assert_eq!(first.source_offset(display_offset), source_offset);
        }
        for (source_offset, display_offset) in [0, 1, 2, 5, 6, 7, 8, 11].into_iter().enumerate() {
            assert_eq!(
                first.display_range(source_offset..source_offset),
                display_offset..display_offset
            );
        }
        let adjacent_tabs = source_lines("\t\té\t");
        assert_eq!(adjacent_tabs[0].display.as_str(), "        é   ");
        assert_eq!(adjacent_tabs[0].display_tabs.len(), 3);
        for (source_offset, display_offset) in [0, 4, 8, 9, 10, 13].into_iter().enumerate() {
            assert_eq!(
                adjacent_tabs[0].display_range(source_offset..source_offset),
                display_offset..display_offset
            );
        }
        let long_line = source_lines("a long source line without tabs uses shared heap storage");
        assert_eq!(long_line[0].text.as_ptr(), long_line[0].display.as_ptr());
        let long_tab_line = source_lines(&format!("{}\t", "a".repeat(100_000)));
        assert_eq!(long_tab_line[0].display_tabs.len(), 1);
    }

    #[test]
    fn display_chunks_retain_complete_utf8_source() {
        let ordinary = "an ordinary line uses the existing shared display string";
        let long_grapheme = format!("a{}", "\u{301}".repeat(600));
        for text in [
            String::new(),
            ordinary.to_owned(),
            "é👩‍💻中".repeat(200),
            long_grapheme,
        ] {
            let chunks = display_chunks(&text);
            let mut previous_end = 0;
            for range in &chunks {
                assert_eq!(range.start, previous_end);
                assert!(range.len() <= 512);
                assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
                previous_end = range.end;
            }
            assert_eq!(previous_end, text.len());
            assert_eq!(
                chunks
                    .iter()
                    .map(|range| &text[range.clone()])
                    .collect::<String>(),
                text
            );
            if text.is_empty() {
                continue;
            }
            let lines = source_lines(&text);
            let line = &lines[0];
            assert_eq!(line.chunks.len(), chunks.len());
            assert_eq!(line.chunk_text(&(0..line.display.len())), line.display);
            for chunk in &line.chunks {
                let retained = line.chunk_text(&chunk.range);
                assert_eq!(retained.as_str(), &text[chunk.range.clone()]);
                if retained.len() > 23 {
                    assert_eq!(retained.as_ptr(), chunk.text.as_ptr());
                }
            }
        }
        let normal_graphemes = "é👩‍💻中".repeat(200);
        let boundaries = normal_graphemes
            .grapheme_indices(true)
            .map(|(ix, _)| ix)
            .chain([normal_graphemes.len()])
            .collect::<Vec<_>>();
        for range in display_chunks(&normal_graphemes) {
            assert!(boundaries.contains(&range.start) && boundaries.contains(&range.end));
        }
        let line = &source_lines(ordinary)[0];
        assert_eq!(line.display.as_ptr(), line.chunks[0].text.as_ptr());
    }

    #[test]
    fn language_detection_uses_filename() {
        for (name, language) in [
            ("src/main.RS", "rust"),
            ("src/components/App.jsx", "javascript"),
            ("C:\\src\\lib.hpp", "cpp"),
            ("build/CMakeLists.txt", "cmake"),
            ("build/Makefile", "make"),
            ("directory.rs/README", "text"),
            ("ts", "text"),
            ("notes.unknown", "text"),
        ] {
            assert_eq!(
                DiffFile::new(name, "").detected_language().as_str(),
                language
            );
        }
    }
}

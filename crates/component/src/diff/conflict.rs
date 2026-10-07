//! Files outside a patch: a whole source shown for reference, and a working
//! file containing Git conflict markers.
use std::ops::Range;

use gpui::SharedString;

use super::{
    DiffFile, DiffFileStatus, DiffParseError,
    document::{FileSide, LinePair},
};

/// How the user resolved a conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiffConflictResolution {
    /// Keep the current change, between `<<<<<<<` and `=======`.
    Current,
    /// Keep the incoming change, between `=======` and `>>>>>>>`.
    Incoming,
    /// Keep the current change followed by the incoming one.
    Both,
}

/// Which part of a conflict a row introduces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConflictPart {
    Current,
    Base,
    Incoming,
}

/// One conflict region. Ranges index the file's modified-side lines, which
/// exclude the marker lines.
#[derive(Clone, Debug)]
pub(crate) struct Conflict {
    current: Range<usize>,
    base: Option<Range<usize>>,
    incoming: Range<usize>,
    current_label: SharedString,
    base_label: SharedString,
    incoming_label: SharedString,
}

impl Conflict {
    /// Every line of the region, in file order.
    pub(crate) fn lines(&self) -> Range<usize> {
        self.current.start..self.incoming.end
    }
    pub(crate) fn part(&self, part: ConflictPart) -> Option<Range<usize>> {
        match part {
            ConflictPart::Current => Some(self.current.clone()),
            ConflictPart::Base => self.base.clone(),
            ConflictPart::Incoming => Some(self.incoming.clone()),
        }
    }
    pub(crate) fn label(&self, part: ConflictPart) -> &SharedString {
        match part {
            ConflictPart::Current => &self.current_label,
            ConflictPart::Base => &self.base_label,
            ConflictPart::Incoming => &self.incoming_label,
        }
    }
    /// The part a line of the region belongs to.
    pub(crate) fn part_of(&self, ix: usize) -> Option<ConflictPart> {
        [
            ConflictPart::Current,
            ConflictPart::Base,
            ConflictPart::Incoming,
        ]
        .into_iter()
        .find(|part| self.part(*part).is_some_and(|range| range.contains(&ix)))
    }
    /// The lines a resolution keeps.
    pub(crate) fn kept(&self, resolution: DiffConflictResolution) -> Vec<Range<usize>> {
        match resolution {
            DiffConflictResolution::Current => vec![self.current.clone()],
            DiffConflictResolution::Incoming => vec![self.incoming.clone()],
            DiffConflictResolution::Both => vec![self.current.clone(), self.incoming.clone()],
        }
    }
}

/// Splits `text` into lines and their byte ranges, keeping line endings.
fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut offset = 0;
    text.split_inclusive('\n')
        .map(|line| {
            let range = offset..offset + line.len();
            offset = range.end;
            range
        })
        .collect()
}

impl DiffFile {
    /// A file shown in full, for reference, without changes. It appears as a
    /// single column in every display mode.
    pub fn unchanged(path: impl Into<SharedString>, text: &str) -> Self {
        let path = path.into();
        let ranges = line_ranges(text);
        let pairs = (0..ranges.len())
            .map(|ix| LinePair::new(None, Some(ix), false))
            .collect();
        let lines = ranges
            .into_iter()
            .enumerate()
            .map(|(ix, range)| (ix + 1, range));
        DiffFile::new(
            DiffFileStatus::Unchanged,
            FileSide::new(Some(path.clone()), String::new(), []),
            FileSide::new(Some(path), text.to_owned(), lines),
            pairs,
            Vec::new(),
            Vec::new(),
            false,
        )
    }

    /// Parses a working file containing Git conflict markers, including the
    /// `|||||||` base section of `diff3` style. Lines keep their numbers in
    /// the working file; marker lines are shown as conflict headings instead.
    pub fn parse_conflicts(
        path: impl Into<SharedString>,
        text: &str,
    ) -> Result<Self, DiffParseError> {
        let path = path.into();
        let mut source = String::new();
        let mut lines = Vec::new();
        let mut pairs = Vec::new();
        let mut conflicts = Vec::new();
        // The marker-delimited part being read, with the open region's ranges.
        let mut open: Option<(ConflictPart, Conflict)> = None;
        for (ix, range) in line_ranges(text).into_iter().enumerate() {
            let line = &text[range.clone()];
            let content = line.trim_end_matches(['\n', '\r']);
            let marker = |prefix: &str| {
                content
                    .strip_prefix(prefix)
                    .filter(|rest| rest.is_empty() || rest.starts_with(' '))
                    .map(|rest| SharedString::from(rest.trim().to_owned()))
            };
            let next = lines.len();
            let (opening, base, separator, closing) = (
                marker("<<<<<<<"),
                marker("|||||||"),
                marker("======="),
                marker(">>>>>>>"),
            );
            match open.as_mut() {
                None => {
                    if let Some(label) = opening {
                        open = Some((
                            ConflictPart::Current,
                            Conflict {
                                current: next..next,
                                base: None,
                                incoming: next..next,
                                current_label: label,
                                base_label: SharedString::default(),
                                incoming_label: SharedString::default(),
                            },
                        ));
                        continue;
                    }
                    if base.is_some() || closing.is_some() {
                        return Err(DiffParseError::new(
                            ix + 1,
                            "conflict marker outside a conflict",
                        ));
                    }
                }
                Some((part, conflict)) => {
                    if opening.is_some() {
                        return Err(DiffParseError::new(ix + 1, "nested conflict marker"));
                    }
                    match (*part, base, separator, closing) {
                        (ConflictPart::Current, Some(label), _, _) => {
                            conflict.current.end = next;
                            conflict.base = Some(next..next);
                            conflict.base_label = label;
                            *part = ConflictPart::Base;
                            continue;
                        }
                        (ConflictPart::Current | ConflictPart::Base, _, Some(_), _) => {
                            if *part == ConflictPart::Current {
                                conflict.current.end = next;
                            } else if let Some(base) = &mut conflict.base {
                                base.end = next;
                            }
                            conflict.incoming = next..next;
                            *part = ConflictPart::Incoming;
                            continue;
                        }
                        (ConflictPart::Incoming, _, _, Some(label)) => {
                            conflict.incoming.end = next;
                            conflict.incoming_label = label;
                            if let Some((_, conflict)) = open.take() {
                                conflicts.push(conflict);
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            let start = source.len();
            source.push_str(line);
            lines.push((ix + 1, start..source.len()));
            pairs.push(LinePair::new(None, Some(next), open.is_some()));
        }
        if open.is_some() {
            return Err(DiffParseError::new(
                text.lines().count(),
                "unterminated conflict",
            ));
        }
        Ok(DiffFile::new(
            DiffFileStatus::Conflicted,
            FileSide::new(Some(path.clone()), String::new(), []),
            FileSide::new(Some(path), source, lines),
            pairs,
            Vec::new(),
            Vec::new(),
            false,
        )
        .with_conflicts(conflicts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::DiffSide;

    const MERGE: &str = "fn retry() {\n<<<<<<< HEAD\n    3\n=======\n    5\n>>>>>>> feature\n}\n";

    #[test]
    fn unchanged_files_show_every_line_once() {
        let file = DiffFile::unchanged("src/lib.rs", "one\r\ntwo\nlast");
        assert_eq!(file.status(), DiffFileStatus::Unchanged);
        assert!(!file.has_changes());
        assert_eq!(file.lines_count(DiffSide::Modified), 3);
        assert_eq!(file.lines_count(DiffSide::Original), 0);
        assert_eq!(
            file.text_for_lines(DiffSide::Modified, 1, 3),
            "one\r\ntwo\nlast"
        );
        assert!(DiffFile::unchanged("empty", "").pairs().is_empty());
    }

    #[test]
    fn conflicts_keep_working_file_line_numbers() {
        let file = DiffFile::parse_conflicts("src/retry.rs", MERGE).unwrap();
        assert_eq!(file.status(), DiffFileStatus::Conflicted);
        let numbers = file
            .lines(DiffSide::Modified)
            .iter()
            .map(|line| line.line_number())
            .collect::<Vec<_>>();
        assert_eq!(numbers, [1, 3, 5, 7]);
        let conflict = &file.conflicts()[0];
        assert_eq!(conflict.part(ConflictPart::Current), Some(1..2));
        assert_eq!(conflict.part(ConflictPart::Incoming), Some(2..3));
        assert_eq!(conflict.label(ConflictPart::Current).as_str(), "HEAD");
        assert_eq!(conflict.label(ConflictPart::Incoming).as_str(), "feature");
        assert_eq!(conflict.part_of(2), Some(ConflictPart::Incoming));
        assert!(file.pairs()[1].is_changed() && !file.pairs()[0].is_changed());
    }

    #[test]
    fn diff3_bases_and_malformed_markers() {
        let file = DiffFile::parse_conflicts(
            "a.txt",
            "<<<<<<< ours\na\n||||||| base\nb\n=======\nc\n>>>>>>> theirs\n",
        )
        .unwrap();
        let conflict = &file.conflicts()[0];
        assert_eq!(conflict.part(ConflictPart::Base), Some(1..2));
        assert_eq!(conflict.label(ConflictPart::Base).as_str(), "base");
        assert_eq!(conflict.kept(DiffConflictResolution::Both), [0..1, 2..3]);
        for text in [
            "<<<<<<< a\nx\n",
            "<<<<<<< a\n<<<<<<< b\n",
            ">>>>>>> stray\n",
        ] {
            let error = DiffFile::parse_conflicts("a.txt", text).err().unwrap();
            assert!(error.line() > 0);
        }
        // A separator outside a conflict is ordinary text.
        assert!(DiffFile::parse_conflicts("a.md", "title\n=======\n").is_ok());
    }
}

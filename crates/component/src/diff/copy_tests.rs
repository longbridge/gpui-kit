use super::{DiffDocument, DiffFile, DiffLineRange, DiffSide};

#[test]
fn copying_source_line_ranges_preserves_whitespace_unicode_and_eof() {
    let original = "\t旧值 🦀\r\n\r\n尾部\t  ";
    let modified = "\t新值 👩‍💻\r\n\r\n尾部\t  \r\n";
    let document = DiffDocument::new(
        DiffFile::new("review.txt", original),
        DiffFile::new("review.txt", modified),
    );

    for (side, source, first_line, last_line) in [
        (DiffSide::Original, original, "\t旧值 🦀\r\n", "尾部\t  "),
        (
            DiffSide::Modified,
            modified,
            "\t新值 👩‍💻\r\n",
            "尾部\t  \r\n",
        ),
    ] {
        // Reversed endpoints and a zero endpoint still copy complete source lines.
        assert_eq!(
            document.text_for_range(DiffLineRange::new(side, usize::MAX, 0)),
            source
        );
        assert_eq!(
            document.text_for_range(DiffLineRange::new(side, 0, 0)),
            first_line
        );
        assert_eq!(
            document.text_for_range(DiffLineRange::new(side, 2, 2)),
            "\r\n"
        );
        assert_eq!(
            document.text_for_range(DiffLineRange::new(side, 3, usize::MAX)),
            last_line
        );
        assert_eq!(
            document.text_for_range(DiffLineRange::new(side, 4, usize::MAX)),
            ""
        );
    }
}

#[test]
fn copying_added_deleted_and_empty_files_respects_the_requested_side() {
    let source = "\t中文 🦀\r\n末行";
    for (document, present, missing) in [
        (
            DiffDocument::added(DiffFile::new("new.txt", source)),
            DiffSide::Modified,
            DiffSide::Original,
        ),
        (
            DiffDocument::deleted(DiffFile::new("old.txt", source)),
            DiffSide::Original,
            DiffSide::Modified,
        ),
    ] {
        assert_eq!(
            document.text_for_range(DiffLineRange::new(present, 1, usize::MAX)),
            source
        );
        assert_eq!(
            document.text_for_range(DiffLineRange::new(missing, 1, usize::MAX)),
            ""
        );
        assert_eq!(
            document.text_for_range(DiffLineRange::new(present, usize::MAX, usize::MAX)),
            ""
        );
    }

    let empty = DiffDocument::new(
        DiffFile::new("empty.txt", ""),
        DiffFile::new("empty.txt", ""),
    );
    for side in [DiffSide::Original, DiffSide::Modified] {
        assert_eq!(
            empty.text_for_range(DiffLineRange::new(side, 0, usize::MAX)),
            ""
        );
    }
}

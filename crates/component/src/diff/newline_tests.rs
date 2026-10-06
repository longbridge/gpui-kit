use super::{DiffDocument, DiffFile, DiffLineRange, DiffSide, has_line_ending_change};

#[test]
fn ending_changes_retain_counterparts_and_exact_source() {
    for (original, modified, ending_only) in [
        ("same\n", "same", true),
        ("same\r\n", "same\n", true),
        ("same", "same\r\n", true),
        ("old\r\n", "new\n", false),
        ("\t\n", "    \n", false),
    ] {
        let document = DiffDocument::new(
            DiffFile::new("a.txt", original),
            DiffFile::new("a.txt", modified),
        );
        assert!(document.has_changes());
        for side in [DiffSide::Original, DiffSide::Modified] {
            let line = &document.lines(side)[0];
            assert_eq!(line.counterpart, Some(0));
            assert_eq!(
                has_line_ending_change(&document, side, 0, line.counterpart),
                ending_only
            );
            assert_eq!(
                document.text_for_range(DiffLineRange::new(side, 1, 1)),
                document.source(side)
            );
        }
    }
}

#[test]
fn missing_files_and_blank_lines_have_no_false_counterparts() {
    for document in [
        DiffDocument::added(DiffFile::new("empty.txt", "")),
        DiffDocument::deleted(DiffFile::new("empty.txt", "")),
    ] {
        assert!(document.has_changes());
        assert_eq!((document.additions(), document.deletions()), (0, 0));
        assert_eq!(document.line_count(DiffSide::Original), 0);
        assert_eq!(document.line_count(DiffSide::Modified), 0);
    }
    for (original, modified, side) in [
        ("", "\n", DiffSide::Modified),
        ("\n", "", DiffSide::Original),
    ] {
        let document = DiffDocument::new(
            DiffFile::new("a.txt", original),
            DiffFile::new("a.txt", modified),
        );
        let line = &document.lines(side)[0];
        assert!(line.text.is_empty());
        assert_eq!(line.counterpart, None);
        assert!(!has_line_ending_change(
            &document,
            side,
            0,
            line.counterpart
        ));
        assert_eq!(
            document.text_for_range(DiffLineRange::new(side, 1, 1)),
            "\n"
        );
    }
}

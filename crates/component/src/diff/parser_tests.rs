use super::{DiffDocument, DiffLinePosition, DiffLineRange, DiffSide};

#[test]
fn sparse_hunks_preserve_source_numbers_and_available_copy() {
    let patch = "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -10,2 +20,2 @@ fn first\n keep\n-old\n+new\n@@ -100 +110 @@\n-last\n+next\n";
    let files = DiffDocument::parse(patch).unwrap();
    let doc = &files[0];
    assert_eq!((doc.additions(), doc.deletions()), (2, 2));
    assert_eq!(doc.line_count(DiffSide::Original), 3);
    assert_eq!(doc.position(DiffSide::Modified, 1).line(), 21);
    assert_eq!(
        doc.line_index(DiffLinePosition::new(DiffSide::Original, 100)),
        Some(2)
    );
    assert_eq!(
        doc.line_index(DiffLinePosition::new(DiffSide::Original, 50)),
        None
    );
    assert_eq!(
        doc.text_for_range(DiffLineRange::new(DiffSide::Original, 1, 200)),
        "keep\nold\nlast\n"
    );
    assert_eq!(doc.0.hunk_boundaries[1].0, 2);
}

#[test]
fn multi_file_missing_sides_metadata_and_binary() {
    let patch = "diff --git a/new b/new\nnew file mode 100644\n--- /dev/null\n+++ b/new\n@@ -0,0 +1 @@\n+added\ndiff --git a/old b/old\ndeleted file mode 100644\n--- a/old\n+++ /dev/null\n@@ -1 +0,0 @@\n-deleted\ndiff --git a/from b/to\nsimilarity index 100%\nrename from from\nrename to to\ndiff --git a/pic b/pic\nBinary files a/pic and b/pic differ\n";
    let files = DiffDocument::parse(patch).unwrap();
    assert_eq!(files.len(), 4);
    assert!(files[0].original().is_none());
    assert!(files[1].modified().is_none());
    assert_eq!(files[2].modified().unwrap().name().as_str(), "to");
    assert!(files[2].has_changes());
    assert!(files[3].is_binary());
    assert_eq!(files[3].line_count(DiffSide::Original), 0);
}

#[test]
fn newline_markers_and_transport_are_distinct() {
    let files = DiffDocument::parse("--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n").unwrap();
    assert_eq!(files[0].original().unwrap().text().as_str(), "old");
    assert_eq!(files[0].modified().unwrap().text().as_str(), "new");
    let files = DiffDocument::parse("--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\r\n+new\r\n").unwrap();
    assert_eq!(files[0].original().unwrap().text().as_str(), "old\r\n");
    let files =
        DiffDocument::parse("--- a/a\r\n+++ b/a\r\n@@ -1 +1 @@\r\n-old\r\n+new\r\n").unwrap();
    assert_eq!(files[0].original().unwrap().text().as_str(), "old\n");
}

#[test]
fn quoted_git_paths_and_context_prefixes() {
    let files = DiffDocument::parse("diff --git \"a/\\344\\270\\255.rs\" \"b/\\344\\270\\255.rs\"\n--- \"a/\\344\\270\\255.rs\"\n+++ \"b/\\344\\270\\255.rs\"\n@@ -1,2 +1,2 @@\n --- source text\n-old\n+new\n").unwrap();
    assert_eq!(files[0].original().unwrap().name().as_str(), "中.rs");
    assert!(
        files[0]
            .original()
            .unwrap()
            .text()
            .starts_with("--- source text\n")
    );
}

#[test]
fn malformed_hunks_fail_without_partial_documents() {
    for patch in [
        "--- a/a\n+++ b/a\n@@ -1,2 +1,2 @@\n-old\n+new\n",
        "--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n+extra\n",
        "--- a/a\n+++ b/a\n@@ -1 +1 @@\n same\n@@ -1 +2 @@\n again\n",
        "--- a/a\n+++ b/a\n@@ -18446744073709551615 +1 @@\n-old\n+new\n",
        "diff --cc a.rs\n@@@ -1 -1 +1 @@@\n",
        "--- /dev/null\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n",
    ] {
        let error = DiffDocument::parse(patch)
            .err()
            .expect("reject malformed patch");
        assert!(error.line() > 0);
        assert!(!error.message().is_empty());
    }
    assert!(DiffDocument::parse("").unwrap().is_empty());
}

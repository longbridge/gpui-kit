use super::{DiffFile, DiffSide};

#[test]
fn sparse_hunks_preserve_source_numbers_and_available_copy() {
    let patch = "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -10,2 +20,2 @@ fn first\n keep\n-old\n+new\n@@ -100 +110 @@\n-last\n+next\n";
    let files = DiffFile::parse(patch).unwrap();
    let doc = &files[0];
    assert_eq!((doc.additions(), doc.deletions()), (2, 2));
    assert_eq!(doc.lines_count(DiffSide::Original), 3);
    assert_eq!(doc.line_number(DiffSide::Modified, 1), 21);
    assert_eq!(doc.line_index(DiffSide::Original, 100), Some(2));
    assert_eq!(doc.line_index(DiffSide::Original, 50), None);
    assert_eq!(
        doc.text_for_lines(DiffSide::Original, 1, 200),
        "keep\nold\nlast\n"
    );
    assert_eq!(doc.hunks()[1].pairs().start, 2);
    assert_eq!(doc.hunks()[1].lines(DiffSide::Original), 2..3);
    assert_eq!(doc.line_number_digits(), 3);
}

#[test]
fn multi_file_missing_sides_metadata_and_binary() {
    let patch = "diff --git a/new b/new\nnew file mode 100644\n--- /dev/null\n+++ b/new\n@@ -0,0 +1 @@\n+added\ndiff --git a/old b/old\ndeleted file mode 100644\n--- a/old\n+++ /dev/null\n@@ -1 +0,0 @@\n-deleted\ndiff --git a/from b/to\nsimilarity index 100%\nrename from from\nrename to to\ndiff --git a/pic b/pic\nBinary files a/pic and b/pic differ\n";
    let files = DiffFile::parse(patch).unwrap();
    assert_eq!(files.len(), 4);
    assert!(files[0].original_path().is_none());
    assert_eq!(files[0].path().as_str(), "new");
    assert!(files[1].modified_path().is_none());
    assert_eq!(files[1].path().as_str(), "old");
    assert_eq!(files[2].original_path().unwrap().as_str(), "from");
    assert_eq!(files[2].modified_path().unwrap().as_str(), "to");
    assert!(files[2].has_changes());
    assert!(files[3].is_binary());
    assert_eq!(files[3].lines_count(DiffSide::Original), 0);
}

#[test]
fn newline_markers_and_transport_are_distinct() {
    let files = DiffFile::parse("--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n").unwrap();
    assert_eq!(files[0].source(DiffSide::Original), "old");
    assert_eq!(files[0].source(DiffSide::Modified), "new");
    let files = DiffFile::parse("--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\r\n+new\r\n").unwrap();
    assert_eq!(files[0].source(DiffSide::Original), "old\r\n");
    let files = DiffFile::parse("--- a/a\r\n+++ b/a\r\n@@ -1 +1 @@\r\n-old\r\n+new\r\n").unwrap();
    assert_eq!(files[0].source(DiffSide::Original), "old\n");
}

#[test]
fn quoted_git_paths_and_context_prefixes() {
    let files = DiffFile::parse("diff --git \"a/\\344\\270\\255.rs\" \"b/\\344\\270\\255.rs\"\n--- \"a/\\344\\270\\255.rs\"\n+++ \"b/\\344\\270\\255.rs\"\n@@ -1,2 +1,2 @@\n --- source text\n-old\n+new\n").unwrap();
    assert_eq!(files[0].original_path().unwrap().as_str(), "中.rs");
    assert!(
        files[0]
            .source(DiffSide::Original)
            .starts_with("--- source text\n")
    );
}

#[test]
fn format_patch_signature_ends_the_message() {
    let patch = "From 1a2b Mon Sep 17 00:00:00 2001\nSubject: [PATCH 1/2] Change\n\n---\n a.rs | 2 +-\n 1 file changed\n\ndiff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+new\n-- \n2.47.0\n\nFrom 3c4d Mon Sep 17 00:00:00 2001\nSubject: [PATCH 2/2] Again\n\n---\n b.rs | 2 +-\n\ndiff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -1 +1 @@\n-before\n+after\n--\n2.47.0\n";
    let files = DiffFile::parse(patch).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path().as_str(), "a.rs");
    assert_eq!(files[1].path().as_str(), "b.rs");
    assert_eq!(files[1].source(DiffSide::Modified), "after\n");
}

#[test]
fn git_prefix_configurations_resolve_paths() {
    for (header, old_header, new_header) in [
        ("a/src/a.rs b/src/a.rs", "a/src/a.rs", "b/src/a.rs"),
        ("i/src/a.rs w/src/a.rs", "i/src/a.rs", "w/src/a.rs"),
        ("c/src/a.rs i/src/a.rs", "c/src/a.rs", "i/src/a.rs"),
        ("src/a.rs src/a.rs", "src/a.rs", "src/a.rs"),
        (
            "a/dir with space/a.rs b/dir with space/a.rs",
            "a/dir with space/a.rs",
            "b/dir with space/a.rs",
        ),
    ] {
        let patch = format!(
            "diff --git {header}\nindex 1..2 100644\n--- {old_header}\n+++ {new_header}\n@@ -1 +1 @@\n-old\n+new\n"
        );
        let files = DiffFile::parse(&patch).unwrap();
        let expected = if header.contains("space") {
            "dir with space/a.rs"
        } else {
            "src/a.rs"
        };
        assert_eq!(
            files[0].original_path().unwrap().as_str(),
            expected,
            "{header}"
        );
        assert_eq!(
            files[0].modified_path().unwrap().as_str(),
            expected,
            "{header}"
        );
    }
    // A mode change has no `---`/`+++` records; the Git header names the file.
    let files =
        DiffFile::parse("diff --git src/run.sh src/run.sh\nold mode 100644\nnew mode 100755\n")
            .unwrap();
    assert_eq!(files[0].path().as_str(), "src/run.sh");
}

#[test]
fn plain_unified_paths_keep_directories_named_like_prefixes() {
    let files =
        DiffFile::parse("--- a/notes.txt\n+++ a/notes.txt\n@@ -1 +1 @@\n-old\n+new\n").unwrap();
    assert_eq!(files[0].path().as_str(), "a/notes.txt");
    let files =
        DiffFile::parse("--- a/notes.txt\n+++ b/notes.txt\n@@ -1 +1 @@\n-old\n+new\n").unwrap();
    assert_eq!(files[0].path().as_str(), "notes.txt");
}

#[test]
fn whitespace_stripped_context_lines_are_empty_context() {
    let files =
        DiffFile::parse("--- a/a\n+++ b/a\n@@ -1,3 +1,3 @@\n first\n\n-old\n+new\n").unwrap();
    assert_eq!(files[0].source(DiffSide::Original), "first\n\nold\n");
    assert_eq!(files[0].source(DiffSide::Modified), "first\n\nnew\n");
    // A blank line after a complete hunk separates files instead.
    let files = DiffFile::parse(
        "--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-x\n+y\n",
    )
    .unwrap();
    assert_eq!(files.len(), 2);
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
        let error = DiffFile::parse(patch)
            .err()
            .expect("reject malformed patch");
        assert!(error.line() > 0);
        assert!(!error.message().is_empty());
    }
    assert!(DiffFile::parse("").unwrap().is_empty());
}

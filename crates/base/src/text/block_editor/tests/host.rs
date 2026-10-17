//! Data-integrity and host-trust-boundary coverage; no GPUI platform services.
use super::*;

#[test]
fn cumulative_native_ranges_preserve_surroundings_and_undo() {
    let mut editor = model("before 雪 after");
    select(&mut editor, 0, 0..10); // Insert at head, never replace the selection.
    let mut range = editor.capture_caret().unwrap();
    editor.replace_captured(&mut range, "one").unwrap();
    editor.replace_captured(&mut range, "two\nthree🦀").unwrap();
    editor.replace_captured(&mut range, "final").unwrap();
    assert_eq!(editor.source(), "before 雪final after");
    assert!(editor.undo());
    assert_eq!(editor.source(), "before 雪two\n\nthree🦀 after");
    assert!(editor.redo());
    assert_eq!(editor.source(), "before 雪final after");
    assert_invariants(&editor);
}

#[test]
fn captures_reject_other_owners_navigation_loads_edits_and_undo() {
    let mut editor = model("draft");
    editor.select_end().unwrap();
    let capture = editor.capture_caret().unwrap();
    let mut other = model("draft");
    assert_eq!(
        other.replace_captured(&mut capture.clone(), "bad"),
        Err(CapturedRangeError::Stale)
    );
    caret_in(&mut editor, 0, 0);
    editor.select_end().unwrap(); // Returning to the same caret is still navigation.
    assert_eq!(
        editor.replace_captured(&mut capture.clone(), "bad"),
        Err(CapturedRangeError::Stale)
    );
    let capture = editor.capture_caret().unwrap();
    editor.invalidate_captures(); // External session/window changed, text stayed equal.
    assert!(!editor.is_capture_current(&capture));
    let capture = editor.capture_caret().unwrap();
    editor.set_source("draft").unwrap();
    assert!(!editor.is_capture_current(&capture));
    editor.select_end().unwrap();
    let mut capture = editor.capture_caret().unwrap();
    editor.replace_captured(&mut capture, " voice").unwrap();
    editor.insert_text("human", EditOrigin::Typing).unwrap();
    assert_eq!(
        editor.replace_captured(&mut capture, "bad"),
        Err(CapturedRangeError::Stale)
    );
    assert!(editor.undo());
    assert!(!editor.is_capture_current(&capture));
    assert_eq!(editor.source(), "draft voice");
}

#[test]
fn completion_edits_leaf_ranges_not_escaped_source_offsets() {
    let mut editor = model("**prefix** !query");
    editor.select_end().unwrap();
    assert_eq!(editor.caret_prefix(), Some("prefix !query"));
    let mut capture = editor.capture_suffix("!query".len()).unwrap();
    editor.replace_captured(&mut capture, "@accepted ").unwrap();
    assert_eq!(editor.caret_prefix(), Some("prefix @accepted "));
    assert_eq!(editor.source(), "**prefix** @accepted ");
    assert!(editor.undo());
    assert_eq!(editor.source(), "**prefix** !query");
}

#[test]
fn long_nested_source_is_shared_per_revision_not_per_selection() {
    let source = (0..1500)
        .map(|ix| format!("- **item {ix}**\n  - nested `code`\n"))
        .collect::<String>();
    let mut editor = model(&source);
    let start = std::time::Instant::now();
    let first = editor.source_snapshot();
    let first_elapsed = start.elapsed();
    let start = std::time::Instant::now();
    for _ in 0..500 {
        assert_eq!(
            editor.source_snapshot().text().as_ptr(),
            first.text().as_ptr()
        );
    }
    let repeated_elapsed = start.elapsed();
    editor.select_end().unwrap();
    assert_eq!(
        editor.source_snapshot().text().as_ptr(),
        first.text().as_ptr()
    );
    editor.insert_text(" changed", EditOrigin::Typing).unwrap();
    let next = editor.source_snapshot();
    assert_ne!(next.revision(), first.revision());
    assert!(next.text().ends_with("changed"));
    eprintln!(
        "nested source: {} input bytes, {} exported bytes; first {:?}, 500 shared snapshots {:?}",
        source.len(),
        first.text().len(),
        first_elapsed,
        repeated_elapsed
    );
    // The second hot reader is a large single code leaf, not an app cache.
    let mut code = model(&format!("```rs\n{source}\n```"));
    code.select_end().unwrap();
    let cold = std::time::Instant::now();
    let prefix = code.caret_prefix_snapshot().unwrap();
    let cold_time = cold.elapsed();
    let warm = std::time::Instant::now();
    for _ in 0..500 {
        assert_eq!(
            code.caret_prefix_snapshot().unwrap().as_ptr(),
            prefix.as_ptr()
        );
    }
    let warm_time = warm.elapsed();
    caret_in(&mut code, 0, 1);
    assert_eq!(code.caret_prefix_snapshot().as_deref(), Some("-"));
    code.insert_text("new", EditOrigin::Typing).unwrap();
    assert_eq!(code.caret_prefix_snapshot().as_deref(), Some("-new"));
    eprintln!(
        "code prefix: {} bytes; first {:?}, 500 shared reads {:?}",
        prefix.len(),
        cold_time,
        warm_time
    );
}

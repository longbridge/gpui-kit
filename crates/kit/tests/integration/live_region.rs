use crate::common;
use gpui_kit::{
    AppContext, Context, IntoElement, Render, SharedString, TestAppContext, Window,
    component::live_region::LiveRegion, div, prelude::*,
};

struct SaveStatus {
    status: SharedString,
}

impl Render for SaveStatus {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(LiveRegion::new("save-status", self.status.clone()))
    }
}

/// Platforms announce a live node when its text changes, so the region must
/// stay one node with one identity from frame to frame. It also draws the text,
/// which must not reach the tree a second time: a reader browsing the view
/// would hear it twice. Its `live` property is
/// not in the debug tree; `gpui-component`'s unit tests check it.
#[gpui_kit::test]
fn live_region_stays_one_node_as_its_text_changes(cx: &mut TestAppContext) {
    // See `a11y_text::quick_start_text_is_in_the_accessibility_tree` for why this re-runs.
    if std::env::var("GPUI_A11Y_ACTIVE").as_deref() != Ok("1") {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "live_region::live_region_stays_one_node_as_its_text_changes",
                "--nocapture",
            ])
            .env("GPUI_A11Y_ACTIVE", "1")
            .status()
            .unwrap();
        assert!(
            status.success(),
            "accessibility tree check failed: {status}"
        );
        return;
    }

    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, None, |_, cx| {
        cx.new(|_| SaveStatus {
            status: "Not saved".into(),
        })
    });
    let tree = |cx: &mut TestAppContext| {
        cx.update_window(handle.into(), |_, window, cx| {
            window.draw(cx).clear(cx);
            window.debug_a11y_tree_json().expect("accessibility tree")
        })
        .unwrap()
    };
    let first = tree(cx);
    let id = region_node(&first);
    assert!(first.contains("\"label\": \"Not saved\""), "{first}");
    assert!(first.contains("\"value\": \"Not saved\""), "{first}");
    assert_eq!(
        first.matches("Not saved").count(),
        2,
        "only the region's label and value: {first}"
    );

    view.update(cx, |view, cx| {
        view.status = "Saved".into();
        cx.notify();
    });
    let saved = tree(cx);
    assert_eq!(
        region_node(&saved),
        id,
        "a new node is not announced as a change"
    );
    assert!(saved.contains("\"label\": \"Saved\""), "{saved}");
    assert!(saved.contains("\"value\": \"Saved\""), "{saved}");
    assert!(!saved.contains("Not saved"), "{saved}");
    assert_eq!(saved.matches("\"Saved\"").count(), 2, "{saved}");
}

/// The AccessKit ID of the one Status node rendered for the region.
fn region_node(tree: &str) -> String {
    let lines: Vec<&str> = tree.lines().collect();
    let nodes: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(r#""element_id": "Name(\"save-status\")""#))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(nodes.len(), 1, "{tree}");
    let accesskit_id = lines[nodes[0] - 1].trim();
    assert!(accesskit_id.starts_with(r#""accesskit_id""#), "{tree}");
    let role = lines[nodes[0]..]
        .iter()
        .find(|line| line.contains(r#""role""#));
    assert_eq!(
        role.map(|line| line.trim()),
        Some(r#""role": "Status","#),
        "{tree}"
    );
    accesskit_id.to_string()
}

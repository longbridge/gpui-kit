//! What a screen reader learns about a `PopupMenu` (#4130).

use crate::common;
use gpui_kit::component::{button::Button, menu::DropdownMenu};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext as _, Context, TestAppContext, Window, actions, div, prelude::*, px, size,
};

actions!(menu_a11y_test, [Choose]);

struct Sort;
impl Render for Sort {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                Button::new("sort")
                    .label("Sort")
                    .dropdown_menu(|menu, window, cx| {
                        menu.accessibility_label("Sort")
                            .label("Order")
                            .menu_with_check("Name", true, Box::new(Choose))
                            .menu_with_check("Date", false, Box::new(Choose))
                            .separator()
                            .submenu("Group by", window, cx, |menu, _, _| {
                                menu.menu("Kind", Box::new(Choose))
                            })
                            .separator()
                            .menu("Reset", Box::new(Choose))
                    }),
            )
    }
}

/// Each menu item in the last frame's accessibility tree, as its label,
/// position and set size, in position order.
fn accessible_menu_items(window: &Window) -> Vec<(String, Option<u64>, Option<u64>)> {
    let json = window.debug_a11y_tree_json().expect("accessibility tree");
    let tree: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut items: Vec<_> = tree["nodes"]
        .as_object()
        .unwrap()
        .values()
        .map(|node| &node["aria"])
        .filter(|aria| aria["role"] == "MenuItem")
        .map(|aria| {
            (
                aria["label"].as_str().unwrap_or_default().to_owned(),
                aria["position_in_set"].as_u64(),
                aria["size_of_set"].as_u64(),
            )
        })
        .collect();
    items.sort_by_key(|(_, position, _)| *position);
    items
}

/// A screen reader names the menu and says where each item is, "2 of 5".
/// Separators only draw a line, so they are neither announced nor counted,
/// and a submenu is named after the item that opens it.
#[gpui_kit::test]
fn a_menu_is_named_and_numbers_its_items_without_separators(cx: &mut TestAppContext) {
    // GPUI Fast's test window builds accessibility data only when asked to.
    // Re-run in a child so the flag is set before any frame, without changing
    // the process environment during parallel tests.
    if std::env::var("GPUI_A11Y_ACTIVE").as_deref() != Ok("1") {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "menu_a11y::a_menu_is_named_and_numbers_its_items_without_separators",
                "--nocapture",
            ])
            .env("GPUI_A11Y_ACTIVE", "1")
            .status()
            .unwrap();
        assert!(status.success(), "accessibility check failed: {status}");
        return;
    }

    cx.update(gpui_kit::init);
    let (handle, _) =
        common::open_window(cx, Some(size(px(640.), px(480.))), |_, cx| cx.new(|_| Sort));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("sort", cx);
        window.render_frame(cx);

        assert_eq!(window.find("popup-menu").label(), Some("Sort"));
        let menu = window.within("popup-menu");
        assert_eq!(menu.find(3usize).role(), None, "separator");
        assert_eq!(menu.find(5usize).role(), None, "separator");
        let item = |label: &str, position| (label.to_owned(), Some(position), Some(5));
        assert_eq!(
            accessible_menu_items(window),
            [
                item("Order", 1),
                item("Name", 2),
                item("Date", 3),
                item("Group by", 4),
                item("Reset", 5),
            ]
        );

        window.within("popup-menu").click(4usize, cx);
        window.render_frame(cx);
        assert_eq!(
            window.within("submenu").find("popup-menu").label(),
            Some("Group by")
        );
    })
    .unwrap();
}

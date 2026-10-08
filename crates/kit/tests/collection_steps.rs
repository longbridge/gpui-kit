mod common;

use gpui_kit::component::{
    IndexPath,
    list::{List, ListDelegate, ListItem, ListState},
};
use gpui_kit::test::{TestEventExt, TestQueryExt, TestSupportExt, TestWindowExt};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, Modifiers, MouseButton, TestAppContext, Window,
    div, prelude::*, px, size,
};

const DOCUMENTS: [&str; 3] = ["Roadmap", "Release notes", "Meeting notes"];

#[derive(Default)]
struct Documents {
    selected: Option<IndexPath>,
    confirmed: Vec<(IndexPath, bool)>,
    canceled: usize,
}

impl ListDelegate for Documents {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        DOCUMENTS.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let name = DOCUMENTS[ix.row];
        Some(ListItem::new(name).accessibility_label(name).child(name))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix;
    }

    fn confirm(&mut self, secondary: bool, _: &mut Window, _: &mut Context<ListState<Self>>) {
        self.confirmed.push((
            self.selected.expect("selection precedes confirm"),
            secondary,
        ));
    }

    fn cancel(&mut self, _: &mut Window, _: &mut Context<ListState<Self>>) {
        self.canceled += 1;
    }
}

struct DocumentPicker {
    list: Entity<ListState<Documents>>,
}

impl Render for DocumentPicker {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().h(px(160.)).child(List::new(&self.list)))
            .child(div().id("outside-list").test_support().h(px(80.)).w_full())
    }
}

fn open_picker(
    cx: &mut TestAppContext,
) -> (
    gpui_kit::WindowHandle<gpui_kit::base::Root>,
    Entity<DocumentPicker>,
) {
    cx.update(gpui_kit::init);
    common::open_window(cx, Some(size(px(320.), px(240.))), |window, cx| {
        cx.new(|cx| {
            let list = cx.new(|cx| ListState::new(Documents::default(), window, cx));
            list.focus_handle(cx).focus(window, cx);
            DocumentPicker { list }
        })
    })
}

#[gpui_kit::test]
fn list_activation_uses_modifiers_at_release_and_not_at_pointer_down(cx: &mut TestAppContext) {
    let (handle, picker) = open_picker(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let list = picker.read(cx).list.clone();
        window.render_frame(cx);
        let path = window.find_by_label("Release notes").path().to_vec();
        let position = window.find_by_label("Release notes").bounds().center();

        for (held_at_down, held_at_up, secondary) in [
            (Modifiers::default(), Modifiers::secondary_key(), true),
            (Modifiers::secondary_key(), Modifiers::default(), false),
        ] {
            let count = list.read(cx).delegate().confirmed.len();
            window.change_modifiers(held_at_down, cx);
            window.pointer_down(position, MouseButton::Left, cx);
            assert_eq!(list.read(cx).delegate().confirmed.len(), count);
            window.change_modifiers(held_at_up, cx);
            assert_eq!(window.find_by_label("Release notes").path(), path);
            window.pointer_up(position, MouseButton::Left, cx);
            assert_eq!(list.read(cx).selected_index(), Some(IndexPath::new(1)));
            assert_eq!(
                list.read(cx).delegate().confirmed[count],
                (IndexPath::new(1), secondary)
            );
            assert_eq!(list.read(cx).delegate().confirmed.len(), count + 1);
            assert!(list.focus_handle(cx).is_focused(window));
        }
        window.change_modifiers(Modifiers::default(), cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn list_drag_out_release_does_not_activate_or_replace_selection(cx: &mut TestAppContext) {
    let (handle, picker) = open_picker(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let list = picker.read(cx).list.clone();
        window.render_frame(cx);
        window.click("Roadmap", cx);
        let position = window.find_by_label("Meeting notes").bounds().center();
        let outside = window.find("outside-list").bounds().center();
        window.change_modifiers(Modifiers::secondary_key(), cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_move(outside, Some(MouseButton::Left), cx);
        window.pointer_up(outside, MouseButton::Left, cx);
        assert_eq!(list.read(cx).selected_index(), Some(IndexPath::new(0)));
        assert_eq!(
            list.read(cx).delegate().confirmed,
            [(IndexPath::new(0), false)]
        );
        assert!(list.focus_handle(cx).is_focused(window));

        window.change_modifiers(Modifiers::default(), cx);
        window.click("Meeting notes", cx);
        assert_eq!(list.read(cx).selected_index(), Some(IndexPath::new(2)));
        assert_eq!(
            list.read(cx).delegate().confirmed,
            [(IndexPath::new(0), false), (IndexPath::new(2), false)]
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn list_repeated_navigation_confirms_current_item_and_cancel_clears_selection(
    cx: &mut TestAppContext,
) {
    let (handle, picker) = open_picker(cx);
    cx.update_window(handle.into(), |_, window, cx| {
        let list = picker.read(cx).list.clone();
        window.key_down("down", false, cx);
        assert_eq!(list.read(cx).selected_index(), Some(IndexPath::new(0)));
        window.key_down("down", true, cx);
        assert_eq!(list.read(cx).selected_index(), Some(IndexPath::new(1)));
        window.key_up("down", cx);
        assert_eq!(list.read(cx).selected_index(), Some(IndexPath::new(1)));
        assert!(list.read(cx).delegate().confirmed.is_empty());
        assert!(window.find_by_label("Release notes").visible());

        window.change_modifiers(Modifiers::secondary_key(), cx);
        window.key_down("enter", false, cx);
        assert_eq!(
            list.read(cx).delegate().confirmed,
            [(IndexPath::new(1), true)]
        );
        window.key_up("enter", cx);
        assert_eq!(list.read(cx).delegate().confirmed.len(), 1);
        window.change_modifiers(Modifiers::default(), cx);
        window.key_down("escape", false, cx);
        assert_eq!(list.read(cx).selected_index(), None);
        assert_eq!(list.read(cx).delegate().selected, None);
        assert_eq!(list.read(cx).delegate().canceled, 1);
        window.key_up("escape", cx);
        window.press("enter", cx);
        assert_eq!(list.read(cx).delegate().confirmed.len(), 1);
        assert!(list.focus_handle(cx).is_focused(window));
    })
    .unwrap();
}

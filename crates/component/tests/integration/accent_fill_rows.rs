//! Content drawn on the theme's `accent` fill takes `accent_foreground`, the
//! color the theme pairs with that fill, rather than the page `foreground`.

#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Context, Hsla, InteractiveElement as _, IntoElement, Modifiers,
    ParentElement as _, Render, RenderOnce, Styled as _, TestAppContext, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Selectable as _, Theme, ThemeConfig, list::ListItem,
    searchable_list::SearchableListItemElement,
};

/// Records the text color its parent hands down to its content.
#[derive(IntoElement)]
struct TextColor(Rc<Cell<Option<Hsla>>>);

impl RenderOnce for TextColor {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        self.0.set(Some(window.text_style().color));
        div()
            .debug_selector(|| "content".into())
            .w(px(80.))
            .h(px(16.))
    }
}

struct Harness {
    build: Box<dyn Fn(TextColor) -> AnyElement>,
    color: Rc<Cell<Option<Hsla>>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        (self.build)(TextColor(self.color.clone()))
    }
}

/// A light theme whose accent is a saturated blue, so the foreground it pairs
/// with that fill (white) differs from the page foreground (near black).
fn init_blue_accent_theme(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        let config = serde_json::from_value::<ThemeConfig>(serde_json::json!({
            "name": "Blue Accent",
            "mode": "light",
            "colors": {
                "foreground": "#0a0a0a",
                "accent.background": "#2563eb",
                "accent.foreground": "#ffffff"
            }
        }))
        .unwrap();
        Theme::global_mut(cx).apply_config(&Rc::new(config));
    });
}

/// The text color that `build`'s element passes to the content it wraps,
/// with the pointer resting on that content when `hovered`.
fn content_color(
    cx: &mut TestAppContext,
    hovered: bool,
    build: impl Fn(TextColor) -> AnyElement + 'static,
) -> Hsla {
    let color = Rc::new(Cell::new(None));
    let (_, cx) = cx.add_window_view({
        let color = color.clone();
        move |_, _| Harness {
            build: Box::new(build),
            color,
        }
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    if hovered {
        let content = cx
            .debug_bounds("content")
            .expect("the content must lay out");
        cx.simulate_mouse_move(content.center(), None, Modifiers::none());
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    color.get().expect("the content must render")
}

#[gpui::test]
fn searchable_list_cursor_and_hover_rows_use_accent_foreground(cx: &mut TestAppContext) {
    init_blue_accent_theme(cx);
    let (foreground, accent_foreground) =
        cx.update(|cx| (cx.theme().foreground, cx.theme().accent_foreground));

    let row = |selected| {
        move |content: TextColor| {
            SearchableListItemElement::new(0)
                .selected(selected)
                .child(content)
                .into_any_element()
        }
    };
    assert_eq!(content_color(cx, false, row(true)), accent_foreground);
    assert_eq!(content_color(cx, false, row(false)), foreground);
    // A hovered row takes a translucent accent fill.
    assert_eq!(content_color(cx, true, row(false)), accent_foreground);
}

#[gpui::test]
fn list_item_accent_selection_uses_accent_foreground(cx: &mut TestAppContext) {
    init_blue_accent_theme(cx);
    let (foreground, accent_foreground) =
        cx.update(|cx| (cx.theme().foreground, cx.theme().accent_foreground));
    let row = |content: TextColor| {
        ListItem::new("row")
            .selected(true)
            .child(content)
            .into_any_element()
    };

    // The default active highlight is a faint tint, which keeps `foreground`.
    assert_eq!(content_color(cx, false, row), foreground);

    cx.update(|cx| Theme::global_mut(cx).list.active_highlight = false);
    assert_eq!(content_color(cx, false, row), accent_foreground);
}

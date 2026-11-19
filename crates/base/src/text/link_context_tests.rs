use std::sync::{Arc, Mutex};

use gpui::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
    ParentElement as _, Pixels, Point, Render, SharedString, Styled as _, TestAppContext, Window,
    div, point, px,
};

use super::{TextView, TextViewState};
use crate::{TextSelection, TextSelectionLayer};

#[derive(Default)]
struct Events {
    link: Option<SharedString>,
    ancestor: Option<SharedString>,
    activations: Vec<MouseButton>,
}

struct LinkView {
    state: Entity<TextViewState>,
    events: Arc<Mutex<Events>>,
}

impl Render for LinkView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let context_events = self.events.clone();
        let ancestor_events = self.events.clone();
        let click_events = self.events.clone();
        div()
            .w(px(400.))
            .child(TextSelectionLayer)
            .on_mouse_down(MouseButton::Right, move |_, _, _| {
                let mut events = ancestor_events.lock().unwrap();
                events.ancestor = events.link.clone();
            })
            .child(
                TextView::new(&self.state)
                    .selectable(true)
                    .on_link_context_menu(move |url, _, _, _| {
                        context_events.lock().unwrap().link = Some(url.clone());
                    })
                    .on_link_click(move |_, event, _, _| {
                        if let gpui::ClickEvent::Mouse(click) = event {
                            click_events
                                .lock()
                                .unwrap()
                                .activations
                                .push(click.up.button);
                        }
                    }),
            )
    }
}

const URL: &str = "https://example.org/path";
const IMAGE: &str = concat!(
    "data:image/png;base64,",
    "iVBORw0KGgoAAAANSUhEUgAAACgAAAAYCAYAAACIhL/AAAAANUlEQVR4nO3OMREAIAwAsfpXVleggnuGDNkzs3u+lgcEBeuAoGAdEBSsA4KCdUBQsA4IPnYBtcGYyGMVk+kAAAAASUVORK5CYII=",
);

fn check_secondary_press(
    source: &str,
    at: Point<Pixels>,
    expected_text: Option<&str>,
    cx: &mut TestAppContext,
) {
    cx.update(crate::init);
    let events = Arc::new(Mutex::new(Events::default()));
    let (view, cx) = cx.add_window_view(|_, cx| LinkView {
        state: cx.new(|cx| TextViewState::markdown(source, cx)),
        events: events.clone(),
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        view.read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.select_all(cx));
        window.draw(cx).clear(cx);
    });
    let selected = cx.update(|window, cx| TextSelection::selected_text(window, cx));
    if let Some(text) = expected_text {
        assert!(selected.contains(text), "selected text: {selected:?}");
    }

    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    {
        let events = events.lock().unwrap();
        assert_eq!(events.link.as_deref(), Some(URL));
        assert_eq!(events.ancestor.as_deref(), Some(URL));
        assert!(events.activations.is_empty());
    }
    assert_eq!(
        cx.update(|window, cx| TextSelection::selected_text(window, cx)),
        selected
    );

    cx.update(|window, cx| {
        TextSelection::clear(window, cx);
        window.draw(cx).clear(cx);
    });
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    assert!(events.lock().unwrap().activations.is_empty());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_down(at, MouseButton::Middle, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Middle, Modifiers::default());
    assert_eq!(
        events.lock().unwrap().activations,
        [MouseButton::Left, MouseButton::Middle]
    );
}

#[gpui::test]
fn a_formatted_link_reports_the_context_press_with_a_selection(cx: &mut TestAppContext) {
    check_secondary_press(
        &format!("[alpha **bravo**]({URL}) tail"),
        point(px(20.), px(12.)),
        Some("alpha bravo"),
        cx,
    );
}

#[gpui::test]
fn a_standalone_linked_image_reports_the_context_press(cx: &mut TestAppContext) {
    check_secondary_press(
        &format!("[![dot]({IMAGE})]({URL})"),
        point(px(12.), px(12.)),
        None,
        cx,
    );
}

#[gpui::test]
fn an_inline_linked_image_reports_the_context_press(cx: &mut TestAppContext) {
    check_secondary_press(
        &format!("head [![dot]({IMAGE})]({URL}) tail"),
        point(px(60.), px(12.)),
        Some("head"),
        cx,
    );
}

use crate::common;
use gpui_kit::{
    AppContext, Context, FocusHandle, IntoElement, Render, TestAppContext, Window,
    base::{Tab, Tabs},
    component::tab::{Tab as StyledTab, TabBar},
    div,
    prelude::*,
    px, size,
    test::{TestSupportExt, TestWindowExt},
};

struct TabsProbe {
    styled: bool,
    selected: usize,
    activations: Vec<usize>,
    outside: FocusHandle,
    nested: FocusHandle,
}

impl Render for TabsProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let nested_focus = self.nested.clone();
        let nested = div()
            .id("nested")
            .test_support()
            .size(px(16.))
            .track_focus(&self.nested)
            .on_mouse_down(gpui_kit::MouseButton::Left, move |_, window, cx| {
                window.focus(&nested_focus, cx);
                window.prevent_default();
                // Deliberately bubble: the enclosing tab must honor default prevention.
            })
            .on_click(|_, _, cx| cx.stop_propagation());
        let tabs = if self.styled {
            TabBar::new("tabs")
                .selected_index(self.selected)
                .child(StyledTab::new().label("First").suffix(nested))
                .child(StyledTab::new().label("Second"))
                .child(StyledTab::new().label("Disabled").disabled(true))
                .on_click(cx.listener(|this, ix, _, cx| {
                    this.selected = *ix;
                    this.activations.push(*ix);
                    cx.notify();
                }))
                .into_any_element()
        } else {
            Tabs::new("tabs")
                .flex()
                .child(
                    Tab::new(0usize)
                        .w(px(120.))
                        .h(px(32.))
                        .selected(self.selected == 0)
                        .child("First")
                        .child(nested)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.selected = 0;
                            this.activations.push(0);
                            cx.notify();
                        })),
                )
                .children((1..3).map(|ix| {
                    Tab::new(ix)
                        .w(px(120.))
                        .h(px(32.))
                        .selected(self.selected == ix)
                        .disabled(ix == 2)
                        .child("Tab")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected = ix;
                            this.activations.push(ix);
                            cx.notify();
                        }))
                }))
                .into_any_element()
        };
        div()
            .child(div().id("outside").size(px(20.)).track_focus(&self.outside))
            .child(tabs)
    }
}

fn pointer_focus(cx: &mut TestAppContext, styled: bool) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(640.), px(240.))), |window, cx| {
        cx.new(|cx| {
            let outside = cx.focus_handle().tab_stop(true);
            window.focus(&outside, cx);
            TabsProbe {
                styled,
                selected: 0,
                activations: Vec::new(),
                outside,
                nested: cx.focus_handle(),
            }
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(1usize, cx);
        assert_eq!(view.read(cx).activations, vec![1]);
        assert_eq!(window.find(1usize).focused(), Some(true));
        window.press("left", cx);
        assert_eq!(view.read(cx).selected, 1, "arrows only move focus");
        assert_eq!(window.find(0usize).focused(), Some(true));
        window.press("space", cx);
        assert_eq!(view.read(cx).activations, vec![1, 0]);
        window.press("right", cx);
        assert_eq!(view.read(cx).selected, 0);
        window.press("enter", cx);
        assert_eq!(view.read(cx).activations, vec![1, 0, 1]);
        window.press("right", cx);
        assert_eq!(
            window.find(0usize).focused(),
            Some(true),
            "skip disabled and wrap"
        );

        window.focus(&view.read(cx).outside.clone(), cx);
        window.click(2usize, cx);
        assert!(view.read(cx).outside.is_focused(window));
        assert_eq!(view.read(cx).activations, vec![1, 0, 1]);

        window.click("nested", cx);
        assert!(view.read(cx).nested.is_focused(window));
        window.press("right", cx);
        window.press("space", cx);
        assert!(view.read(cx).nested.is_focused(window));
        assert_eq!(view.read(cx).activations, vec![1, 0, 1]);
    })
    .unwrap();
}

#[gpui_kit::test]
fn base_tabs_pointer_focus(cx: &mut TestAppContext) {
    pointer_focus(cx, false);
}

#[gpui_kit::test]
fn component_tabs_pointer_focus(cx: &mut TestAppContext) {
    pointer_focus(cx, true);
}

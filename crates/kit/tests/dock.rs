mod common;
use gpui_kit::component::dock::{
    BasePanel, DockArea, DockLayout, DockSkin, Panel, PanelControl, PanelEvent, PanelStyle,
    panel_handle,
};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::test::{TestAppContextExt, TestSupportExt, TestWindowExt};
use gpui_kit::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, TestAppContext, Window,
    div, prelude::*, px, size,
};
use std::time::Duration;
struct Document {
    name: &'static str,
    focus: FocusHandle,
    input: Entity<InputState>,
    removed_focus: Option<FocusHandle>,
    visible: bool,
}
impl Document {
    fn new(name: &'static str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            name,
            focus: cx.focus_handle(),
            input: cx.new(|cx| InputState::new(window, cx)),
            removed_focus: None,
            visible: true,
        }
    }
}
impl BasePanel for Document {
    fn panel_name(&self) -> &'static str {
        self.name
    }
    fn visible(&self, _: &App) -> bool {
        self.visible
    }
    fn on_removed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(focus) = &self.removed_focus {
            focus.focus(window, cx);
        }
    }
}
impl Panel for Document {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.name
    }
    fn zoom_control(&self, _: &App) -> Option<PanelControl> {
        Some(PanelControl::Toolbar)
    }
}
impl EventEmitter<PanelEvent> for Document {}
impl Focusable for Document {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for Document {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(self.name)
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            .child(self.name)
            .child(Input::new(&self.input).id("editor"))
    }
}
struct Editor {
    area: Entity<DockArea>,
}
impl Render for Editor {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.area.clone())
    }
}
#[gpui_kit::test]
async fn dock_switches_and_reorders_real_tabs(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(900.), px(600.))), |window, cx| {
        let (area, skin) = DockSkin::dock_area("editor", None, window, cx);
        skin.set_panel_style(PanelStyle::TabBar, cx);
        let a = cx.new(|cx| Document::new("alpha", window, cx));
        let b = cx.new(|cx| Document::new("beta", window, cx));
        let layout = DockLayout::tabs()
            .panel_view(panel_handle(a), cx)
            .panel_view(panel_handle(b), cx);
        area.update(cx, |area, cx| area.set_center(layout, window, cx));
        let editor = cx.new(|_| Editor { area });
        editor
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("alpha").visible());
        window.within("tab-bar").click(1usize, cx);
        assert!(window.find("beta").visible());
        assert!(window.try_find("alpha").is_none());
        window.within("tab-bar").drag_to(1usize, 0usize, cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.within("tab-bar").find(0usize).selected() == Some(true)
            && window.try_find("beta").is_some()
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        window.within("tab-bar").click(1usize, cx);
        assert!(
            window.find("alpha").visible(),
            "dragging beta to slot zero must move alpha to slot one"
        );
        assert!(window.try_find("beta").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
async fn dock_moves_a_tab_between_groups_and_zooms_the_result(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = common::open_window(cx, Some(size(px(900.), px(600.))), |window, cx| {
        let (area, _) = DockSkin::dock_area("editor", None, window, cx);
        let a = cx.new(|cx| Document::new("alpha", window, cx));
        let b = cx.new(|cx| Document::new("beta", window, cx));
        let c = cx.new(|cx| Document::new("gamma", window, cx));
        let layout = DockLayout::h_split()
            .child(
                DockLayout::tabs()
                    .panel_view(panel_handle(a), cx)
                    .panel_view(panel_handle(b), cx),
                None,
            )
            .child(DockLayout::tabs().panel_view(panel_handle(c), cx), None);
        area.update(cx, |area, cx| area.set_center(layout, window, cx));
        let editor = cx.new(|_| Editor { area });
        editor
    });
    let mut right_bounds = None;
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let right = window.find("gamma").bounds();
        right_bounds = Some(right);
        assert!(window.find("alpha").bounds().right() <= right.left());
        let from = window.within("tab-bar").find(1usize).bounds().center();
        window.drag(from, right.center(), cx);
    })
    .unwrap();
    let right_bounds = right_bounds.unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window
            .try_find("beta")
            .is_some_and(|panel| panel.bounds().left() >= right_bounds.left())
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(window.find("alpha").visible());
        assert!(window.try_find("gamma").is_none());
        // The panel path contains its owning group's native entity scope. No
        // new global IDs or layout wrappers are needed for repeated toolbars.
        let snapshot = window.find("beta");
        let group_ix = snapshot
            .path()
            .iter()
            .position(|id| *id == "tab-panel".into())
            .unwrap();
        let group = snapshot.path()[group_ix - 1].clone();
        window.within(group).click("zoom-in", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.find("beta").bounds().size.width > right_bounds.size.width
    })
    .await;
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(window.try_find("alpha").is_none());
        window.click("zoom-out", cx);
    })
    .unwrap();
    cx.wait_for(handle.into(), Duration::from_secs(1), |window, _| {
        window.try_find("alpha").is_some()
            && window.find("beta").bounds().size.width <= right_bounds.size.width
    })
    .await;
}

struct TwoTabs {
    area: Entity<DockArea>,
    alpha: Entity<Document>,
    beta: Entity<Document>,
    elsewhere: FocusHandle,
}
impl Render for TwoTabs {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().track_focus(&self.elsewhere).h(px(20.)))
            .child(self.area.clone())
    }
}

fn two_tabs(cx: &mut TestAppContext) -> (gpui_kit::AnyWindowHandle, Entity<TwoTabs>) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(900.), px(600.))), |window, cx| {
        let (area, skin) = DockSkin::dock_area("editor", None, window, cx);
        skin.set_panel_style(PanelStyle::TabBar, cx);
        skin.set_close_button_visible(true, cx);
        let alpha = cx.new(|cx| Document::new("alpha", window, cx));
        let beta = cx.new(|cx| Document::new("beta", window, cx));
        let layout = DockLayout::tabs()
            .panel_view(panel_handle(alpha.clone()), cx)
            .panel_view(panel_handle(beta.clone()), cx);
        area.update(cx, |area, cx| area.set_center(layout, window, cx));
        cx.new(|cx| TwoTabs {
            area,
            alpha,
            beta,
            elsewhere: cx.focus_handle(),
        })
    });
    (handle.into(), view)
}

#[gpui_kit::test]
fn dock_hands_the_focus_to_the_tab_that_replaces_a_closed_one(cx: &mut TestAppContext) {
    let (handle, view) = two_tabs(cx);
    cx.update_window(handle, |_, window, cx| {
        let view = view.read(cx);
        let (area, alpha, beta) = (view.area.clone(), view.alpha.clone(), view.beta.clone());
        window.render_frame(cx);
        let alpha_focus = alpha.read(cx).focus.clone();
        alpha_focus.focus(window, cx);
        window.render_frame(cx);
        assert!(alpha.read(cx).focus.is_focused(window));

        area.update(cx, |area, cx| area.remove_panel(alpha, window, cx));
        window.render_frame(cx);
        assert!(window.find("beta").visible());
        assert!(
            beta.read(cx).focus.is_focused(window),
            "closing the focused tab must leave the keyboard on the one shown in its place"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn dock_leaves_the_focus_alone_when_a_tab_without_it_closes(cx: &mut TestAppContext) {
    let (handle, view) = two_tabs(cx);
    cx.update_window(handle, |_, window, cx| {
        let view = view.read(cx);
        let (area, alpha, elsewhere) = (
            view.area.clone(),
            view.alpha.clone(),
            view.elsewhere.clone(),
        );
        window.render_frame(cx);
        elsewhere.focus(window, cx);
        window.render_frame(cx);

        area.update(cx, |area, cx| area.remove_panel(alpha, window, cx));
        window.render_frame(cx);
        assert!(elsewhere.is_focused(window));
    })
    .unwrap();
}

fn close_visible_tab(window: &mut Window, name: &str, ix: usize, cx: &mut App) {
    let panel = window.find(name.to_owned());
    let group_ix = panel
        .path()
        .iter()
        .position(|id| *id == "tab-panel".into())
        .unwrap();
    let group = panel.path()[group_ix - 1].clone();
    window.within(group).click(("close-tab", ix), cx);
}

#[gpui_kit::test]
async fn dock_close_button_transfers_focus_from_a_child_input(cx: &mut TestAppContext) {
    let (handle, view) = two_tabs(cx);
    let beta = cx
        .update_window(handle, |_, window, cx| {
            let beta = view.read(cx).beta.clone();
            window.render_frame(cx);
            window.within("alpha").click("editor", cx);
            assert_eq!(window.within("alpha").find("editor").focused(), Some(true));
            close_visible_tab(window, "alpha", 0, cx);
            beta
        })
        .unwrap();
    cx.wait_for(handle, Duration::from_secs(1), |window, cx| {
        window.try_find("alpha").is_none()
            && window.try_find("beta").is_some_and(|panel| panel.visible())
            && beta.read(cx).focus.is_focused(window)
    })
    .await;
}

#[gpui_kit::test]
async fn dock_close_button_preserves_focus_chosen_by_on_removed(cx: &mut TestAppContext) {
    let (handle, view) = two_tabs(cx);
    let elsewhere = cx
        .update_window(handle, |_, window, cx| {
            let view = view.read(cx);
            let (alpha, elsewhere) = (view.alpha.clone(), view.elsewhere.clone());
            alpha.update(cx, |panel, _| panel.removed_focus = Some(elsewhere.clone()));
            window.render_frame(cx);
            window.within("alpha").click("editor", cx);
            close_visible_tab(window, "alpha", 0, cx);
            elsewhere
        })
        .unwrap();
    cx.wait_for(handle, Duration::from_secs(1), |window, _| {
        window.try_find("alpha").is_none() && elsewhere.is_focused(window)
    })
    .await;
}

#[gpui_kit::test]
async fn dock_close_button_focuses_the_remaining_split_when_a_group_disappears(
    cx: &mut TestAppContext,
) {
    for close_alpha in [true, false] {
        let (handle, view) = two_tabs(cx);
        let next = cx
            .update_window(handle, |_, window, cx| {
                let view = view.read(cx);
                let (area, alpha, beta) =
                    (view.area.clone(), view.alpha.clone(), view.beta.clone());
                let layout = DockLayout::h_split()
                    .child(
                        DockLayout::tabs().panel_view(panel_handle(alpha.clone()), cx),
                        None,
                    )
                    .child(
                        DockLayout::tabs().panel_view(panel_handle(beta.clone()), cx),
                        None,
                    );
                area.update(cx, |area, cx| area.set_center(layout, window, cx));
                window.render_frame(cx);
                let (name, next) = if close_alpha {
                    ("alpha", beta)
                } else {
                    ("beta", alpha)
                };
                window.within(name).click("editor", cx);
                close_visible_tab(window, name, 0, cx);
                next
            })
            .unwrap();
        cx.wait_for(handle, Duration::from_secs(1), |window, cx| {
            let closed = if close_alpha { "alpha" } else { "beta" };
            window.try_find(closed).is_none()
                && next.read(cx).focus.is_focused(window)
                && window.find(next.read(cx).name).visible()
        })
        .await;
    }
}

#[gpui_kit::test]
async fn dock_close_button_preserves_focus_when_closing_an_inactive_tab(cx: &mut TestAppContext) {
    let (handle, view) = two_tabs(cx);
    let input = cx
        .update_window(handle, |_, window, cx| {
            let input = view.read(cx).alpha.read(cx).input.clone();
            window.render_frame(cx);
            window.within("alpha").click("editor", cx);
            close_visible_tab(window, "alpha", 1, cx);
            input
        })
        .unwrap();
    cx.wait_for(handle, Duration::from_secs(1), |window, cx| {
        let beta = view.read(cx).beta.clone();
        let area = view.read(cx).area.clone();
        area.read(cx).panel(beta.entity_id().into()).is_none()
            && input.focus_handle(cx).is_focused(window)
    })
    .await;
}

#[gpui_kit::test]
async fn dock_close_button_skips_a_hidden_neighbor_when_handing_off_focus(cx: &mut TestAppContext) {
    let (handle, view) = two_tabs(cx);
    let beta = cx
        .update_window(handle, |_, window, cx| {
            let view = view.read(cx);
            let (area, alpha, beta) = (view.area.clone(), view.alpha.clone(), view.beta.clone());
            let hidden = cx.new(|cx| Document {
                visible: false,
                ..Document::new("hidden", window, cx)
            });
            let layout = DockLayout::h_split()
                .child(DockLayout::tabs().panel_view(panel_handle(alpha), cx), None)
                .child(
                    DockLayout::tabs().panel_view(panel_handle(hidden), cx),
                    None,
                )
                .child(
                    DockLayout::tabs().panel_view(panel_handle(beta.clone()), cx),
                    None,
                );
            area.update(cx, |area, cx| area.set_center(layout, window, cx));
            window.render_frame(cx);
            assert!(window.try_find("hidden").is_none());
            window.within("alpha").click("editor", cx);
            close_visible_tab(window, "alpha", 0, cx);
            beta
        })
        .unwrap();
    cx.wait_for(handle, Duration::from_secs(1), |window, cx| {
        window.try_find("alpha").is_none()
            && window.find("beta").visible()
            && beta.read(cx).focus.is_focused(window)
    })
    .await;
}

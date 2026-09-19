use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    Anchor, AnyElement, App, Context, DismissEvent, Element, ElementId, Entity, FocusHandle,
    Focusable, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement,
    IntoElement, LayoutId, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    StyleRefinement, Styled, Subscription, Window, anchored, deferred, div, prelude::FluentBuilder,
    px,
};

use crate::menu::PopupMenu;

/// A extension trait for adding a context menu to an element.
pub trait ContextMenuExt: InteractiveElement + ParentElement + Styled {
    /// Add a context menu to the element.
    ///
    /// This will changed the element to be `relative` positioned, and add a child `ContextMenu` element.
    /// Because the `ContextMenu` element is positioned `absolute`, it will not affect the layout of the parent element.
    #[track_caller]
    fn context_menu(
        mut self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> ContextMenu<Self>
    where
        Self: Sized,
    {
        // The ID must be stable across renders, otherwise the element state
        // (open menu) is lost on every re-render.
        let caller = std::panic::Location::caller();
        let id = self
            .interactivity()
            .element_id
            .clone()
            .map(|id| ElementId::Name(format!("context-menu-{:?}", id).into()))
            .unwrap_or_else(|| ElementId::CodeLocation(*caller));
        ContextMenu::new(id, self).menu(f)
    }
}

impl<E: InteractiveElement + ParentElement + Styled> ContextMenuExt for E {}

/// A context menu that can be shown on right-click.
pub struct ContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu: Option<Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>>,
    // This is not in use, just for style refinement forwarding.
    _ignore_style: StyleRefinement,
    anchor: Anchor,
}

impl<E: ParentElement + Styled> ContextMenu<E> {
    /// Create a new context menu with the given ID.
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            anchor: Anchor::TopLeft,
            _ignore_style: StyleRefinement::default(),
        }
    }

    /// Build the context menu using the given builder function.
    #[must_use]
    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    {
        self.menu = Some(Rc::new(builder));
        self
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut ContextMenuState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<ContextMenuState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl<E: ParentElement + Styled> ParentElement for ContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: ParentElement + Styled> Styled for ContextMenu<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            &mut self._ignore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for ContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct ContextMenuSharedState {
    menu_view: Option<Entity<PopupMenu>>,
    open: bool,
    position: Point<Pixels>,
    /// Registered on this element's dispatch node every frame and never
    /// focused, so the menu can resolve its shortcut hints against the
    /// trigger's key contexts on the frame it opens: GPUI looks a handle up in
    /// the previously rendered frame, where the menu's own element is not yet.
    trigger_focus_handle: Option<FocusHandle>,
    _subscription: Option<Subscription>,
}

pub struct ContextMenuState {
    element: Option<AnyElement>,
    /// Whether this trigger draws the open menu this frame.
    ///
    /// Triggers without an `ElementId` fall back to their code location, so
    /// rows rendered from one call site share the element state and all see
    /// the menu as open. Only the trigger that was pressed draws it: stacked
    /// copies of one `PopupMenu` share an item's pending-click state, and the
    /// covered copies clear it on mouse up before the visible one fires.
    draws_menu: Rc<Cell<bool>>,
    shared_state: Rc<RefCell<ContextMenuSharedState>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            element: None,
            draws_menu: Rc::default(),
            shared_state: Rc::new(RefCell::new(ContextMenuSharedState {
                menu_view: None,
                open: false,
                position: Default::default(),
                trigger_focus_handle: None,
                _subscription: None,
            })),
        }
    }
}

/// The deferred menu layer, laid out by every trigger that shares the open
/// state but drawn only by the one whose bounds contain the press.
struct DeferredMenu {
    draws: Rc<Cell<bool>>,
    menu: Option<AnyElement>,
}

impl IntoElement for DeferredMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DeferredMenu {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let menu = self.menu.as_mut().expect("menu should exist");
        (menu.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.draws.get() {
            return;
        }
        if let Some(menu) = &mut self.menu {
            menu.prepaint(window, cx);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.draws.get() {
            return;
        }
        if let Some(menu) = &mut self.menu {
            menu.paint(window, cx);
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for ContextMenu<E> {
    type RequestLayoutState = ContextMenuState;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let anchor = self.anchor;

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, state: &mut ContextMenuState, window, cx| {
                let (position, open) = {
                    let shared_state = state.shared_state.borrow();
                    (shared_state.position, shared_state.open)
                };
                state
                    .shared_state
                    .borrow_mut()
                    .trigger_focus_handle
                    .get_or_insert_with(|| cx.focus_handle());
                let menu_view = state.shared_state.borrow().menu_view.clone();
                let draws_menu = Rc::new(Cell::new(false));
                let mut menu_element = None;
                if open {
                    let has_menu_item = menu_view
                        .as_ref()
                        .map(|menu| !menu.read(cx).is_empty())
                        .unwrap_or(false);

                    if has_menu_item {
                        menu_element = Some(
                            deferred(
                                anchored().child(
                                    div()
                                        .w(window.bounds().size.width)
                                        .h(window.bounds().size.height)
                                        .on_scroll_wheel(|_, _, cx| {
                                            cx.stop_propagation();
                                        })
                                        .child(
                                            anchored()
                                                .position(position)
                                                .snap_to_window_with_margin(px(8.))
                                                .anchor(anchor)
                                                .when_some(menu_view, |this, menu| {
                                                    // Focus the menu, so that can be handle the action.
                                                    if !menu
                                                        .focus_handle(cx)
                                                        .contains_focused(window, cx)
                                                    {
                                                        menu.focus_handle(cx).focus(window, cx);
                                                    }

                                                    this.child(menu.clone())
                                                }),
                                        ),
                                ),
                            )
                            .with_priority(gpui_base::POPUP_PRIORITY)
                            .into_any(),
                        );
                    }
                }
                let menu_element = menu_element.map(|menu| DeferredMenu {
                    draws: draws_menu.clone(),
                    menu: Some(menu),
                });

                let mut element = this
                    .element
                    .take()
                    .expect("Element should exists.")
                    .children(menu_element)
                    .into_any_element();

                let layout_id = element.request_layout(window, cx);

                (
                    layout_id,
                    ContextMenuState {
                        element: Some(element),
                        draws_menu,
                        shared_state: state.shared_state.clone(),
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(trigger_focus) = request_layout
            .shared_state
            .borrow()
            .trigger_focus_handle
            .as_ref()
        {
            window.set_focus_handle(trigger_focus, cx);
        }
        let position = request_layout.shared_state.borrow().position;
        request_layout.draws_menu.set(bounds.contains(&position));
        if let Some(element) = &mut request_layout.element {
            element.prepaint(window, cx);
        }
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = &mut request_layout.element {
            element.paint(window, cx);
        }

        // Take the builder before setting up element state to avoid borrow issues
        let builder = self.menu.clone();

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |_view, state: &mut ContextMenuState, window, _| {
                let shared_state = state.shared_state.clone();

                let hitbox = hitbox.clone();
                // When right mouse click, to build content menu, and show it at the mouse position.
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase.bubble()
                        && event.button == MouseButton::Right
                        && hitbox.is_hovered(window)
                    {
                        // Capture the focused element to restore focus to on dismiss.
                        // If focus is still on the previous menu, keep its captured focus.
                        let previous_focus_handle = window.focused(cx).and_then(|focused| {
                            let shared_state = shared_state.borrow();
                            match shared_state.menu_view.as_ref() {
                                Some(menu) if menu.read(cx).focus_handle == focused => {
                                    menu.read(cx).previous_focus_handle.clone()
                                }
                                _ => Some(focused),
                            }
                        });

                        {
                            let mut shared_state = shared_state.borrow_mut();
                            // Clear any existing menu view to allow immediate replacement
                            // Set the new position and open the menu
                            shared_state.menu_view = None;
                            shared_state._subscription = None;
                            shared_state.position = event.position;
                            shared_state.open = true;
                        }

                        // Use defer to build the menu in the next frame, avoiding race conditions
                        window.defer(cx, {
                            let shared_state = shared_state.clone();
                            let builder = builder.clone();
                            move |window, cx| {
                                let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                                    let Some(build) = &builder else {
                                        return menu;
                                    };
                                    build(menu, window, cx)
                                });
                                let trigger_focus_handle =
                                    shared_state.borrow().trigger_focus_handle.clone();
                                menu.update(cx, |menu, cx| {
                                    menu.set_trigger_focus(trigger_focus_handle, cx);
                                    menu.set_previous_focus(previous_focus_handle, cx);
                                });

                                // Set up the subscription for dismiss handling
                                let _subscription = window.subscribe(&menu, cx, {
                                    let shared_state = shared_state.clone();
                                    move |_, _: &DismissEvent, window, _cx| {
                                        shared_state.borrow_mut().open = false;
                                        window.refresh();
                                    }
                                });

                                // Update the shared state with the built menu and subscription
                                {
                                    let mut state = shared_state.borrow_mut();
                                    state.menu_view = Some(menu.clone());
                                    state._subscription = Some(_subscription);
                                    window.refresh();
                                }
                            }
                        });
                    }
                });
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::PopupMenuItem;
    use crate::theme::Theme;
    use gpui::{
        Context, FocusHandle, IntoElement, KeyBinding, Render, TestAppContext, VisualTestContext,
        actions, point, px,
    };
    use std::cell::Cell;

    actions!(context_menu_test, [RemoveTab, CopyText]);

    /// The regression shape: the action handler lives on the trigger's
    /// ancestor (like an action bar), which is NOT on the focus path while
    /// focus is in the content area.
    struct TestRoot {
        content_focus: FocusHandle,
        received: Rc<Cell<bool>>,
    }

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let received = self.received.clone();
            div()
                .size_full()
                .child(
                    div()
                        .id("content")
                        .h(px(40.))
                        .track_focus(&self.content_focus),
                )
                .child(
                    div()
                        .id("action-bar")
                        .h(px(60.))
                        .on_action(move |_: &RemoveTab, _, _| received.set(true))
                        .child(
                            div()
                                .id("tab")
                                .size_full()
                                .context_menu(|menu, _, _| menu.menu("Close", Box::new(RemoveTab))),
                        ),
                )
        }
    }

    #[gpui::test]
    fn action_bubbles_from_trigger_and_focus_restores_on_dismiss(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
            super::super::popup_menu::init(cx);
        });

        let received = Rc::new(Cell::new(false));
        let (root, cx) = cx.add_window_view({
            let received = received.clone();
            move |window, cx| {
                let content_focus = cx.focus_handle();
                content_focus.focus(window, cx);
                TestRoot {
                    content_focus,
                    received,
                }
            }
        });
        let content_focus = root.read_with(cx, |root, _| root.content_focus.clone());
        let cx: &mut VisualTestContext = cx;
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Right-click inside the tab to open the context menu.
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Right,
            position: point(px(50.), px(70.)),
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        // The menu entity is built in a deferred callback, then rendered
        // (which also focuses it) on the next draw.
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });

        // Select "Close" and confirm. Keyboard confirm and mouse click share
        // the same `confirm` path in `PopupMenu`.
        cx.simulate_keystrokes("down enter");
        cx.run_until_parked();

        // The action must reach the handler on the trigger's ancestor chain,
        // even though the action bar was never on the focus path.
        assert!(received.get());
        // And dismiss must restore focus to where it was before the menu
        // opened, keeping the dangling-focus fix (#2614).
        cx.update(|window, cx| {
            assert_eq!(window.focused(cx).as_ref(), Some(&content_focus));
        });
    }

    const CONTEXT: &str = "context_menu_test";

    /// The story shape: nothing is focused, the key binding lives in the key
    /// context of the trigger's ancestor, and other content outside that
    /// context paints after the trigger.
    struct UnfocusedRoot {
        frames: Rc<Cell<usize>>,
    }

    impl Render for UnfocusedRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.frames.set(self.frames.get() + 1);
            div()
                .size_full()
                .child(
                    div()
                        .key_context(CONTEXT)
                        .on_action(|_: &CopyText, _, _| {})
                        .child(
                            div()
                                .id("tab")
                                .w(px(100.))
                                .h(px(30.))
                                .context_menu(|menu, _, _| menu.menu("Copy", Box::new(CopyText))),
                        ),
                )
                .child(div().child("Status"))
        }
    }

    /// The issue shape (#3134): rows rendered from one call site without an
    /// `ElementId` share the context menu's element state.
    struct RowsRoot {
        clicked: Rc<Cell<usize>>,
    }

    impl Render for RowsRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().children((0..3).map(|_| {
                let clicked = self.clicked.clone();
                div()
                    .w(px(100.))
                    .h(px(30.))
                    .context_menu(move |menu, _, _| {
                        let clicked = clicked.clone();
                        menu.item(
                            PopupMenuItem::new("Favorite")
                                .on_click(move |_, _, _| clicked.set(clicked.get() + 1)),
                        )
                    })
            }))
        }
    }

    #[gpui::test]
    fn item_click_fires_once_from_rows_without_an_id(cx: &mut TestAppContext) {
        cx.update(|cx| crate::init(cx));
        let clicked = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let clicked = clicked.clone();
            move |_, _| RowsRoot { clicked }
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        // Right-click the second row; the menu opens at the press position.
        let press = point(px(10.), px(40.));
        cx.simulate_mouse_down(press, MouseButton::Right, Default::default());
        cx.simulate_mouse_up(press, MouseButton::Right, Default::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        // Click the first item, which sits inside the menu's content padding.
        let item = point(press.x + px(30.), press.y + px(17.));
        cx.simulate_mouse_move(item, None, Default::default());
        cx.simulate_click(item, Default::default());
        cx.run_until_parked();

        assert_eq!(
            clicked.get(),
            1,
            "the item's on_click must fire exactly once"
        );
    }

    #[gpui::test]
    fn shortcut_hint_is_painted_on_the_frame_the_menu_opens(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            cx.bind_keys([KeyBinding::new("ctrl-c", CopyText, Some(CONTEXT))]);
        });
        let frames = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let frames = frames.clone();
            move |_, _| UnfocusedRoot { frames }
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.focused(cx).is_none());
        });
        let frames_before_open = frames.get();

        // Right-click inside the tab; the menu is built in a deferred callback
        // and drawn on the frame that follows.
        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Right,
            Default::default(),
        );

        assert_eq!(
            frames.get(),
            frames_before_open + 1,
            "the press must be followed by exactly one frame for this to test the first one"
        );
        assert!(
            cx.debug_bounds("kbd:ctrl-c").is_some(),
            "the shortcut hint must be painted on the same frame as its item"
        );
    }
}

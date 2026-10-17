use std::rc::Rc;

use gpui::{
    Anchor, AnyElement, App, Context, DismissEvent, Element, ElementId, Entity, FocusHandle,
    Focusable, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, LayoutId,
    RenderOnce, SharedString, Styled, Window, prelude::FluentBuilder,
};

use crate::{Selectable, button::Button, menu::PopupMenu, popover::Popover};

/// A dropdown menu trait for buttons and other interactive elements
pub trait DropdownMenu: Styled + Selectable + InteractiveElement + IntoElement + 'static {
    /// Create a dropdown menu with the given items, anchored to the TopLeft corner
    fn dropdown_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        self.dropdown_menu_with_anchor(Anchor::TopLeft, f)
    }

    /// Create a dropdown menu with the given items, anchored to the given corner
    fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Anchor>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        let id = self.interactivity().element_id.clone();

        DropdownMenuPopover::new(
            id.unwrap_or(0.into()),
            anchor,
            self,
            |trigger, handle| trigger.track_focus(handle),
            f,
        )
    }
}

impl DropdownMenu for Button {
    fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Anchor>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        let id = self.interactivity().element_id.clone();
        DropdownMenuPopover::new(
            id.unwrap_or(0.into()),
            anchor,
            self,
            |button, handle| button.menu_focus(handle),
            f,
        )
    }
}

#[derive(IntoElement)]
pub struct DropdownMenuPopover<T: Selectable + IntoElement + 'static> {
    id: ElementId,
    anchor: Anchor,
    trigger: T,
    track_trigger_focus: fn(T, &FocusHandle) -> T,
    builder: Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl<T> DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn new(
        id: ElementId,
        anchor: impl Into<Anchor>,
        trigger: T,
        track_trigger_focus: fn(T, &FocusHandle) -> T,
        builder: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        Self {
            id: SharedString::from(format!("dropdown-menu:{:?}", id)).into(),
            anchor: anchor.into(),
            trigger,
            track_trigger_focus,
            builder: Rc::new(builder),
            on_open_change: None,
        }
    }

    /// Set the anchor corner for the dropdown menu popover.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Add a callback to be called when the menu opens or closes.
    ///
    /// The `&bool` parameter is the **new open state**.
    pub fn on_open_change(
        mut self,
        callback: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }
}

#[derive(Default)]
struct DropdownMenuState {
    menu: Option<Entity<PopupMenu>>,
}

impl<T> RenderOnce for DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        TriggerFocus::new(self.id.clone(), move |trigger_focus, window, cx| {
            self.render_popover(trigger_focus, window, cx)
                .into_any_element()
        })
    }
}

impl<T> DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn render_popover(
        self,
        trigger_focus: FocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) -> Popover {
        let builder = self.builder.clone();
        let menu_state =
            window.use_keyed_state(self.id.clone(), cx, |_, _| DropdownMenuState::default());

        Popover::new(SharedString::from(format!("popover:{}", self.id)))
            .appearance(false)
            .overlay_closable(false)
            .trigger((self.track_trigger_focus)(self.trigger, &trigger_focus))
            .anchor(self.anchor)
            .when_some(self.on_open_change, |this, callback| {
                this.on_open_change(move |open, window, cx| callback(open, window, cx))
            })
            .content(move |_, window, cx| {
                // Here is special logic to only create the PopupMenu once and reuse it.
                // Because this `content` will called in every time render, so we need to store the menu
                // in state to avoid recreating at every render.
                //
                // And we also need to rebuild the menu when it is dismissed, to rebuild menu items
                // dynamically for support `dropdown_menu` method, so we listen for DismissEvent below.
                let menu = match menu_state.read(cx).menu.clone() {
                    Some(menu) => menu,
                    None => {
                        let builder = builder.clone();
                        let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                            builder(menu, window, cx)
                        });
                        menu.update(cx, |menu, cx| {
                            menu.set_trigger_focus(Some(trigger_focus.clone()), cx);
                            menu.set_previous_focus(Some(trigger_focus.clone()), cx);
                        });
                        menu_state.update(cx, |state, _| {
                            state.menu = Some(menu.clone());
                        });
                        menu.focus_handle(cx).focus(window, cx);

                        // Listen for dismiss events from the PopupMenu to close the popover.
                        //
                        // Hold Weak handles here, not strong clones: the listener
                        // lives as long as `menu`, which `menu_state` owns, so a
                        // strong capture would close the cycle
                        // `menu_state -> menu -> listener -> menu_state` and leak
                        // the `PopupMenu` and `DropdownMenuState` when the trigger
                        // stops being rendered while the menu is open.
                        let popover_state = cx.entity().downgrade();
                        window
                            .subscribe(&menu, cx, {
                                let menu_state = menu_state.downgrade();
                                move |_, _: &DismissEvent, window, cx| {
                                    if let Some(popover_state) = popover_state.upgrade() {
                                        popover_state.update(cx, |state, cx| {
                                            state.dismiss(window, cx);
                                        });
                                    }
                                    _ = menu_state.update(cx, |state, _| {
                                        state.menu = None;
                                    });
                                }
                            })
                            .detach();

                        menu.clone()
                    }
                };

                menu.clone()
            })
    }
}

type TriggerFocusBuild = Box<dyn FnOnce(FocusHandle, &mut Window, &mut App) -> AnyElement>;

/// Owns the actual trigger's stable handle for focus restoration and shortcut hints.
struct TriggerFocus {
    id: ElementId,
    build: Option<TriggerFocusBuild>,
}

#[derive(Default)]
struct TriggerFocusState {
    focus_handle: Option<FocusHandle>,
}

struct TriggerFocusFrame {
    child: AnyElement,
}

impl TriggerFocus {
    fn new(
        id: ElementId,
        build: impl FnOnce(FocusHandle, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id,
            build: Some(Box::new(build)),
        }
    }
}

impl IntoElement for TriggerFocus {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TriggerFocus {
    type RequestLayoutState = TriggerFocusFrame;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let focus_handle =
            window.with_optional_element_state::<TriggerFocusState, _>(id, |state, _| {
                let mut state = state.flatten().unwrap_or_default();
                let focus_handle = state
                    .focus_handle
                    .get_or_insert_with(|| cx.focus_handle())
                    .clone();
                (focus_handle, Some(state))
            });
        let build = self.build.take().expect("TriggerFocus is laid out once");
        let mut child = build(focus_handle.clone(), window, cx);
        let layout_id = child.request_layout(window, cx);

        (layout_id, TriggerFocusFrame { child })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        frame: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        frame: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        KeyBinding, MouseButton, ParentElement as _, Render, TestAppContext, WeakEntity, actions,
        div, point, px,
    };
    use std::cell::{Cell, RefCell};

    actions!(dropdown_menu_test, [CopyText]);

    const CONTEXT: &str = "dropdown_menu_test";

    /// The story shape: the key binding lives in the key context of the
    /// trigger's ancestor, the menu names no `action_context`, and other
    /// content outside that context paints after the trigger.
    struct TestRoot {
        frames: Rc<Cell<usize>>,
    }

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.frames.set(self.frames.get() + 1);
            div()
                .size_full()
                .child(
                    div()
                        .key_context(CONTEXT)
                        .on_action(|_: &CopyText, _, _| {})
                        .child(
                            Button::new("trigger")
                                .label("Edit")
                                .w(px(100.))
                                .h(px(30.))
                                .dropdown_menu(|menu, _, _| menu.menu("Copy", Box::new(CopyText))),
                        ),
                )
                .child(div().child("Status"))
        }
    }

    /// Records the `PopupMenu` the dropdown builds, so a test can observe
    /// its release through a weak handle.
    struct MenuProbeRoot {
        menu: Rc<RefCell<Option<WeakEntity<PopupMenu>>>>,
    }

    impl Render for MenuProbeRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let slot = self.menu.clone();
            div().size_full().child(
                Button::new("trigger")
                    .label("Edit")
                    .w(px(100.))
                    .h(px(30.))
                    .dropdown_menu(move |menu, _, cx| {
                        *slot.borrow_mut() = Some(cx.weak_entity());
                        menu.menu("Copy", Box::new(CopyText))
                    }),
            )
        }
    }

    /// Opening the menu and closing the window without dismissing it must
    /// release the `PopupMenu`: the dismiss subscription lives as long as the
    /// menu, which `DropdownMenuState` owns, so strong captures in it formed
    /// a cycle that no element-state collection could break. The state still
    /// holds the menu while it is open, so the menu's release also proves the
    /// `DropdownMenuState` was released.
    ///
    /// The popover must go too: it holds the deferred-popover registration
    /// while open, and a leaked one keeps every later right-click menu
    /// stepping aside as if a popup were still showing.
    #[gpui::test]
    fn open_without_dismiss_releases_the_menu(cx: &mut TestAppContext) {
        cx.update(|cx| crate::init(cx));
        let menu = Rc::new(RefCell::new(None::<WeakEntity<PopupMenu>>));

        {
            let (_, cx) = cx.add_window_view(|_, _| MenuProbeRoot { menu: menu.clone() });
            cx.update(|window, cx| window.draw(cx).clear(cx));

            // Click the trigger; the menu opens and is left open.
            cx.simulate_mouse_down(
                point(px(10.), px(10.)),
                MouseButton::Left,
                Default::default(),
            );
            cx.run_until_parked();
            assert!(
                menu.borrow()
                    .as_ref()
                    .and_then(|menu| menu.upgrade())
                    .is_some(),
                "the menu must be open before the window closes"
            );

            // Close the window without dismissing the menu.
            cx.update(|window, _| window.remove_window());
            cx.run_until_parked();
        }

        assert!(
            menu.borrow()
                .as_ref()
                .and_then(|menu| menu.upgrade())
                .is_none(),
            "the PopupMenu must be released with the window"
        );
        cx.update(|cx| {
            assert!(
                !gpui_base::GlobalState::is_in_deferred_context(cx),
                "the popover's deferred registration must be released with the window"
            )
        });
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
            move |_, _| TestRoot { frames }
        });
        // The popup host captures its trigger bounds on the first frame.
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let frames_before_open = frames.get();

        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Left,
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
    use crate::{
        WindowExt as _,
        input::{Input, InputState},
        menu::PopupMenuItem,
    };
    use gpui::AppContext as _;
    #[derive(Clone, Copy)]
    enum DialogActivation {
        Handler,
        Element,
        Submenu,
        Action,
    }

    struct DialogMenuRoot {
        input: Entity<InputState>,
        menu: Rc<RefCell<Option<WeakEntity<PopupMenu>>>>,
        activations: Rc<Cell<usize>>,
        activation: DialogActivation,
    }

    fn open_editor(
        input: &Entity<InputState>,
        activations: &Cell<usize>,
        window: &mut Window,
        cx: &mut App,
    ) {
        activations.set(activations.get() + 1);
        let dialog_input = input.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title("Edit forward")
                .child(Input::new(&dialog_input))
        });
        input.focus_handle(cx).focus(window, cx);
    }

    fn editor_item(
        input: Entity<InputState>,
        activations: Rc<Cell<usize>>,
        custom: bool,
    ) -> PopupMenuItem {
        let item = if custom {
            PopupMenuItem::element(|_, _| {
                div()
                    .debug_selector(|| "custom-editor-item".into())
                    .child("Edit forward…")
            })
        } else {
            PopupMenuItem::new("Edit forward…")
        };
        item.on_click(move |_, window, cx| open_editor(&input, &activations, window, cx))
    }

    impl Render for DialogMenuRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let input = self.input.clone();
            let action_input = input.clone();
            let menu_probe = self.menu.clone();
            let activations = self.activations.clone();
            let action_activations = activations.clone();
            let activation = self.activation;
            div()
                .size_full()
                .on_action(move |_: &OpenEditor, window, cx| {
                    open_editor(&action_input, &action_activations, window, cx);
                })
                .child(
                    Button::new("dialog-trigger")
                        .label("Actions")
                        .w(px(100.))
                        .h(px(30.))
                        .dropdown_menu(move |menu, window, cx| {
                            *menu_probe.borrow_mut() = Some(cx.weak_entity());
                            let input = input.clone();
                            let activations = activations.clone();
                            match activation {
                                DialogActivation::Handler => {
                                    menu.item(editor_item(input, activations, false))
                                }
                                DialogActivation::Element => {
                                    menu.item(editor_item(input, activations, true))
                                }
                                DialogActivation::Action => {
                                    menu.menu("Edit forward…", Box::new(OpenEditor))
                                }
                                DialogActivation::Submenu => {
                                    menu.submenu("More", window, cx, move |menu, _, _| {
                                        menu.item(editor_item(
                                            input.clone(),
                                            activations.clone(),
                                            false,
                                        ))
                                    })
                                }
                            }
                        }),
                )
                .child(Button::new("other-control").label("Other control"))
        }
    }

    #[gpui::test]
    fn menu_dialog_escape_restores_the_trigger_without_stealing_initial_focus(
        cx: &mut TestAppContext,
    ) {
        dialog_round_trip(cx, DialogActivation::Handler, false);
    }

    #[gpui::test]
    fn clicked_custom_menu_item_restores_the_trigger_after_dialog_escape(cx: &mut TestAppContext) {
        dialog_round_trip(cx, DialogActivation::Element, true);
    }

    #[gpui::test]
    fn submenu_dialog_escape_restores_the_top_level_trigger(cx: &mut TestAppContext) {
        dialog_round_trip(cx, DialogActivation::Submenu, false);
    }

    #[gpui::test]
    fn menu_action_keeps_its_dispatch_path_and_dialog_focus_chain(cx: &mut TestAppContext) {
        dialog_round_trip(cx, DialogActivation::Action, false);
    }

    fn dialog_round_trip(cx: &mut TestAppContext, activation: DialogActivation, pointer: bool) {
        cx.update(|cx| {
            crate::init(cx);
            cx.set_reduce_motion(true);
        });
        let menu = Rc::new(RefCell::new(None::<WeakEntity<PopupMenu>>));
        let activations = Rc::new(Cell::new(0));
        let mut input_focus = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
            input_focus = Some(input.focus_handle(cx));
            let view = cx.new(|_| DialogMenuRoot {
                input,
                menu: menu.clone(),
                activations: activations.clone(),
                activation,
            });
            crate::Root::new(view, window, cx)
        });
        let input_focus = input_focus.unwrap();
        let trigger_focus = cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus_next(cx);
            window.focused(cx).expect("Actions receives Tab focus")
        });

        // Exercise the real dropdown/popover and dialog layers, including the
        // menu's release, rather than keeping a dead menu alive in the test.
        if pointer {
            cx.update(|window, cx| {
                window.focus_next(cx);
                assert_ne!(window.focused(cx).as_ref(), Some(&trigger_focus));
            });
            cx.simulate_click(point(px(20.), px(15.)), Default::default());
        } else {
            cx.simulate_keystrokes("enter");
        }
        assert!(menu.borrow().as_ref().unwrap().upgrade().is_some());
        if pointer {
            let position = cx
                .debug_bounds("custom-editor-item")
                .expect("the custom menu item is painted")
                .center();
            cx.simulate_mouse_move(position, None, Default::default());
            cx.simulate_click(position, Default::default());
        } else if matches!(activation, DialogActivation::Submenu) {
            cx.simulate_keystrokes("down right enter");
        } else {
            cx.simulate_keystrokes("down enter");
        }
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(window.has_active_dialog(cx));
            assert_eq!(window.focused(cx).as_ref(), Some(&input_focus));
        });
        assert_eq!(activations.get(), 1);
        assert!(menu.borrow().as_ref().unwrap().upgrade().is_none());

        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert!(!window.has_active_dialog(cx));
            assert_eq!(window.focused(cx).as_ref(), Some(&trigger_focus));
        });
        // Restoration must also leave the trigger's keyboard path usable.
        cx.simulate_keystrokes("enter");
        assert!(menu.borrow().as_ref().unwrap().upgrade().is_some());
    }

    actions!(dialog_menu_test, [OpenEditor]);
}

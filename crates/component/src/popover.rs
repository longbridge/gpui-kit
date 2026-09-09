use gpui::{
    Anchor, Animation, AnimationExt as _, AnyElement, App, Bounds, Context, Div, ElementId,
    FocusHandle, InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels,
    RenderOnce, Stateful, StyleRefinement, Styled, Window, prelude::FluentBuilder as _, px,
};
use std::{rc::Rc, time::Duration};

use crate::ThemeStyled as _;
use crate::{
    Selectable, StyledExt as _,
    animation::ease_out_cubic,
    styled::{popover_ring, popover_shadow},
    v_flex,
};
use gpui_base::Popover as BasePopover;
pub use gpui_base::PopoverState;

pub(crate) fn init(_: &mut App) {}

/// How long a dropdown takes to settle into place after it opens.
///
/// This is shadcn/ui's figure: its popup surfaces carry `animate-in`, whose
/// duration is 150ms.
const DROPDOWN_ENTER_DURATION: Duration = Duration::from_millis(150);

/// Where a dropdown starts out, relative to where it comes to rest.
///
/// Negative is above, so the surface slides *down* out of the trigger's edge —
/// what shadcn/ui expresses as `data-[side=bottom]:slide-in-from-top-2`. Its
/// `2` is `0.5rem`, which is 8px at the default root size.
const DROPDOWN_ENTER_OFFSET: Pixels = px(-8.);

fn dropdown_positioner(bounds: Bounds<Pixels>) -> gpui_base::Positioner {
    gpui_base::Positioner::side(bounds)
        .placement(gpui_base::Placement::Bottom)
        .align(gpui_base::Align::Start)
        .offset(px(6.))
        .margin(px(8.))
}

/// Positions a dropdown surface under its trigger and animates it in.
///
/// This is the shared open motion for Select, Combobox and DatePicker, modelled
/// on shadcn/ui: over 150ms the surface fades up from nothing while sliding the
/// last 8px out of the trigger's edge, on an ease-out curve so it decelerates
/// into place.
///
/// `surface` must be the panel itself — the element carrying
/// [`ThemeStyled::popover_style`] — and not a wrapper around it. GPUI takes a
/// shadow's shape from the element it is set on, so a wrapper of a different
/// size would throw the shadow out of register with the panel.
///
/// # Why the shadow is animated too
///
/// GPUI has no group compositing: `opacity` multiplies into each primitive's
/// alpha separately rather than fading a composited subtree. A drop shadow is
/// painted as a full blurred rect *under* the element — the shader only cuts the
/// element out of `inset` shadows — so a translucent panel does not hide its own
/// shadow, and mid-fade the shadow shows straight through the panel as a dark
/// slab. Ramping the ink by the cube of the fade keeps it out of sight until the
/// panel is opaque enough to cover it, and still lands on the resting shadow
/// [`popover_shadow`] gives every other popup.
///
/// # Departures from shadcn
///
/// - shadcn also scales the surface up from 95% (`zoom-in-95`). GPUI has no
///   element transform — only images and SVGs take a `TransformationMatrix` —
///   so there is nothing to scale a subtree with, and the fade and slide carry
///   the motion on their own.
/// - There is no exit motion. A closing dropdown stops being rendered in the
///   same frame its state flips, so playing one would mean keeping the surface
///   mounted past the close, which is a change to how each of these components
///   tracks `open`.
/// - The slide always comes from above. [`gpui_base::Positioner`] resolves the
///   side the surface actually lands on during layout and does not report it
///   back, so a dropdown that flips above its trigger for want of room below
///   slides the opposite way — 8px over 150ms, in the rare case where it
///   happens.
///
/// Reduced motion needs no handling here: GPUI's animation element adopts the
/// final value on the first frame when the system asks for it.
pub(crate) fn dropdown_popup(
    id: impl Into<ElementId>,
    bounds: Bounds<Pixels>,
    surface: impl IntoElement + Styled + 'static,
    cx: &App,
) -> gpui_base::Positioner {
    let travel: f32 = DROPDOWN_ENTER_OFFSET.into();
    // Read out here: the animation runs long after `cx` is gone.
    let ring = popover_ring(cx);

    dropdown_positioner(bounds).child(surface.with_animation(
        id,
        Animation::new(DROPDOWN_ENTER_DURATION).with_easing(ease_out_cubic),
        move |surface, delta| {
            surface
                .top(px(travel * (1. - delta)))
                .opacity(delta)
                .shadow(popover_shadow(ring, delta * delta * delta))
        },
    ))
}

/// A popover element that can be triggered by a button or any other element.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    default_open: bool,
    open: Option<bool>,
    tracked_focus_handle: Option<FocusHandle>,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<
        Rc<
            dyn Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement
                + 'static,
        >,
    >,
    children: Vec<AnyElement>,
    /// Style for trigger element.
    /// This is used for hotfix the trigger element style to support w_full.
    trigger_style: Option<StyleRefinement>,
    mouse_button: MouseButton,
    appearance: bool,
    overlay_closable: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Popover {
    /// Create a new Popover with `view` mode.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopLeft,
            trigger: None,
            trigger_style: None,
            content: None,
            tracked_focus_handle: None,
            children: vec![],
            mouse_button: MouseButton::Left,
            appearance: true,
            overlay_closable: true,
            default_open: false,
            open: None,
            on_open_change: None,
        }
    }

    /// Set the anchor corner of the popover, default is [`Anchor::TopLeft`].
    ///
    /// Imagine the popover has a pointer tip (like a speech bubble's tail). The
    /// anchor is where that tip sits relative to the trigger: `Anchor::TopLeft`
    /// places it at the trigger's top-left corner, `Anchor::BottomRight` at the
    /// bottom-right, and so on. The popover then hangs off that point.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the mouse button to trigger the popover, default is `MouseButton::Left`.
    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    /// Set the trigger element of the popover.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Selectable + IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|is_open, _, _| {
            let selected = trigger.is_selected();
            trigger.selected(selected || is_open).into_any_element()
        }));
        self
    }

    /// Set the default open state of the popover, default is `false`.
    ///
    /// This is only used to initialize the open state of the popover.
    ///
    /// And please note that if you use the `open` method, this value will be ignored.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Force set the open state of the popover.
    ///
    /// If this is set, the popover will be controlled by this value.
    ///
    /// NOTE: You must be used in conjunction with `on_open_change` to handle state changes.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Add a callback to be called when the open state changes.
    ///
    /// The first `&bool` parameter is the **new open state**.
    ///
    /// This is useful when using the `open` method to control the popover state.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = Some(style);
        self
    }

    /// Set whether clicking outside the popover will dismiss it, default is `true`.
    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    /// Set the content builder for content of the Popover.
    ///
    /// This callback will called every time on render the popover.
    /// So, you should avoid creating new elements or entities in the content closure.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Set whether the popover no style, default is `false`.
    ///
    /// If no style:
    ///
    /// - The popover will not have a bg, border, shadow, or padding.
    /// - The click out of the popover will not dismiss it.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Bind the focus handle to receive focus when the popover is opened.
    /// If you not set this, a new focus handle will be created for the popover to
    ///
    /// If popover is opened, the focus will be moved to the focus handle.
    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Popover {
    pub(crate) fn render_popover_content(
        anchor: Anchor,
        appearance: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        v_flex()
            .id("content")
            .occlude()
            .tab_group()
            .when(appearance, |this| this.popover_style(cx).p_3())
            .map(|this| match anchor {
                Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => this.top_1(),
                Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => this.bottom_1(),
                Anchor::LeftCenter | Anchor::RightCenter => this.top_1(), // Fallback for centered
            })
    }
}

impl RenderOnce for Popover {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let anchor = self.anchor;
        let appearance = self.appearance;
        let style = self.style;
        let children = self.children;
        let content = self.content;

        BasePopover::new(self.id)
            .anchor(self.anchor)
            .mouse_button(self.mouse_button)
            .default_open(self.default_open)
            .overlay_closable(self.overlay_closable)
            .content(move |state, window, cx| {
                Self::render_popover_content(anchor, appearance, window, cx)
                    .when_some(content, |this, content| {
                        this.child((content)(state, window, cx))
                    })
                    .children(children)
                    .refine_style(&style)
            })
            .when_some(self.trigger, |this, trigger| this.trigger_with(trigger))
            .when_some(self.open, |this, open| this.open(open))
            .when_some(self.tracked_focus_handle, |this, handle| {
                this.track_focus(&handle)
            })
            .when_some(self.on_open_change, |this, callback| {
                this.on_open_change(move |open, window, cx| callback(open, window, cx))
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{button::Button, theme::Theme};
    use gpui::{Bounds, Context, MouseButton, Point, Render, div, point, px, size};
    use gpui_base::Popup as BasePopup;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn test_popover_builder_chaining() {
        let popover = Popover::new("test")
            .anchor(Anchor::BottomCenter)
            .mouse_button(MouseButton::Right)
            .default_open(true)
            .appearance(false)
            .overlay_closable(false);

        assert_eq!(popover.anchor, Anchor::BottomCenter);
        assert_eq!(popover.mouse_button, MouseButton::Right);
        assert!(popover.default_open);
        assert!(!popover.appearance);
        assert!(!popover.overlay_closable);
    }

    #[test]
    fn test_resolved_corner_top_positions() {
        use gpui::px;

        let bounds = Bounds {
            origin: Point {
                x: px(100.),
                y: px(100.),
            },
            size: gpui::Size {
                width: px(200.),
                height: px(50.),
            },
        };

        let pos = BasePopup::resolved_corner(Anchor::TopLeft, bounds);
        assert_eq!(pos.x, px(100.));
        assert_eq!(pos.y, px(100.));

        let pos = BasePopup::resolved_corner(Anchor::TopCenter, bounds);
        assert_eq!(pos.x, px(200.));
        assert_eq!(pos.y, px(100.));

        let pos = BasePopup::resolved_corner(Anchor::TopRight, bounds);
        assert_eq!(pos.x, px(300.));
        assert_eq!(pos.y, px(100.));

        let pos = BasePopup::resolved_corner(Anchor::BottomLeft, bounds);
        assert_eq!(pos.x, px(100.));
        assert_eq!(pos.y, px(50.));

        let pos = BasePopup::resolved_corner(Anchor::BottomCenter, bounds);
        assert_eq!(pos.x, px(200.));
        assert_eq!(pos.y, px(50.));

        let pos = BasePopup::resolved_corner(Anchor::BottomRight, bounds);
        assert_eq!(pos.x, px(300.));
        assert_eq!(pos.y, px(50.));
    }

    struct PopoverHarness {
        changes: Rc<RefCell<Vec<bool>>>,
    }

    impl Render for PopoverHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            Popover::new("runtime-popover")
                .trigger(Button::new("runtime-trigger").label("Open").size(px(100.)))
                .content(|_, _, _| {
                    div()
                        .debug_selector(|| "runtime-popover-content".into())
                        .size(px(40.))
                })
                .on_open_change(move |open, _, _| changes.borrow_mut().push(*open))
        }
    }

    #[gpui::test]
    fn pointer_open_and_outside_dismiss_use_the_base_popup_host(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::GlobalState::init(cx);
            cx.set_global(Theme::default());
            init(cx);
        });

        let changes = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let changes = changes.clone();
            move |_, _| PopoverHarness { changes }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.simulate_click(point(px(20.), px(20.)), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("runtime-popover-content").is_some());

        cx.simulate_click(point(px(300.), px(300.)), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("runtime-popover-content").is_none());
        // A change callback reports state transitions, not redundant dismissal
        // requests. The base host may see both paths, but only the first closes.
        assert_eq!(&*changes.borrow(), &[true, false]);
    }

    struct DefaultOpenHarness;

    impl Render for DefaultOpenHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Popover::new("default-open-popover")
                .default_open(true)
                .trigger(Button::new("default-open-trigger").label("Open"))
                .child(
                    div()
                        .debug_selector(|| "default-open-content".into())
                        .size(px(40.)),
                )
        }
    }

    #[gpui::test]
    fn default_open_is_forwarded_to_the_base_popover(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::GlobalState::init(cx);
            cx.set_global(Theme::default());
            init(cx);
        });
        let (_, cx) = cx.add_window_view(|_, _| DefaultOpenHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("default-open-content").is_some());
    }

    struct Harness {
        open: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().when(self.open, |this| {
                this.child(dropdown_popup(
                    "dropdown",
                    Bounds::new(point(px(0.), px(100.)), size(px(120.), px(30.))),
                    div().debug_selector(|| "surface".into()).size(px(50.)),
                    cx,
                ))
            })
        }
    }

    /// A dropdown that reused one animation key across opens would play its
    /// enter motion the first time and then appear already settled on every
    /// open after that. That is invisible in any single frame and easy to
    /// reintroduce by giving the animation a constant id, so it is pinned here.
    #[gpui::test]
    fn the_enter_motion_starts_over_every_time_the_dropdown_opens(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let (view, window) = cx.add_window_view(|_, _| Harness { open: true });

        window.update(|window, cx| window.draw(cx).clear(cx));
        let opening = window.debug_bounds("surface").unwrap().origin;

        // The animation runs off the wall clock, so settling is waited out
        // rather than stepped. Several times the duration leaves room for a
        // loaded machine.
        std::thread::sleep(DROPDOWN_ENTER_DURATION * 4);
        window.update(|window, cx| window.draw(cx).clear(cx));
        let settled = window.debug_bounds("surface").unwrap().origin;

        assert!(
            opening.y < settled.y,
            "the surface should slide down into place, from {opening:?} to {settled:?}",
        );

        for open in [false, true] {
            window.update(|window, cx| {
                view.update(cx, |this, cx| {
                    this.open = open;
                    cx.notify();
                });
                window.draw(cx).clear(cx);
            });
        }

        let reopening = window.debug_bounds("surface").unwrap().origin;
        assert!(
            reopening.y < settled.y,
            "reopening should start the motion over at {opening:?} rather than showing a \
             settled surface, but the first frame was already at {reopening:?}",
        );
    }
}

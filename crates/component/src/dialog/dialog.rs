use std::{rc::Rc, sync::LazyLock, time::Duration};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, BoxShadow, ClickEvent, Edges, FocusHandle, Hsla,
    InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, WindowControlArea, anchored, div, hsla, point,
    prelude::FluentBuilder, px,
};
use gpui_base::{ElementExt as _, TextSelectionScopeId};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, Root, Sizable as _, StyledExt, TITLE_BAR_HEIGHT, WindowExt as _,
    animation::cubic_bezier,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::{DialogContent, DialogTitle},
    scroll::ScrollableElement as _,
    v_flex,
};

pub static ANIMATION_DURATION: LazyLock<Duration> = LazyLock::new(|| Duration::from_secs_f64(0.25));
pub use gpui_base::actions::{Cancel, Confirm};

/// Dialog button props.
#[derive(Clone)]
pub struct DialogButtonProps {
    pub(crate) ok_text: Option<SharedString>,
    pub(crate) ok_variant: ButtonVariant,
    pub(crate) cancel_text: Option<SharedString>,
    pub(crate) cancel_variant: ButtonVariant,
    pub(crate) show_cancel: bool,
    pub(crate) on_ok: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    pub(crate) on_cancel: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    pub(crate) on_close: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
}

impl Default for DialogButtonProps {
    fn default() -> Self {
        Self {
            ok_text: None,
            ok_variant: ButtonVariant::Primary,
            cancel_text: None,
            cancel_variant: ButtonVariant::default(),
            show_cancel: false,
            on_ok: Rc::new(|_, _, _| true),
            on_cancel: Rc::new(|_, _, _| true),
            on_close: Rc::new(|_, _, _| {}),
        }
    }
}

impl DialogButtonProps {
    /// Sets the text of the OK button. Default is `OK`.
    pub fn ok_text(mut self, ok_text: impl Into<SharedString>) -> Self {
        self.ok_text = Some(ok_text.into());
        self
    }

    /// Sets the variant of the OK button. Default is `ButtonVariant::Primary`.
    pub fn ok_variant(mut self, ok_variant: ButtonVariant) -> Self {
        self.ok_variant = ok_variant;
        self
    }

    /// Sets the text of the Cancel button. Default is `Cancel`.
    pub fn cancel_text(mut self, cancel_text: impl Into<SharedString>) -> Self {
        self.cancel_text = Some(cancel_text.into());
        self
    }

    /// Sets the variant of the Cancel button. Default is `ButtonVariant::default()`.
    pub fn cancel_variant(mut self, cancel_variant: ButtonVariant) -> Self {
        self.cancel_variant = cancel_variant;
        self
    }

    /// Sets whether to show the Cancel button. Default is `false`.
    pub fn show_cancel(mut self, show_cancel: bool) -> Self {
        self.show_cancel = show_cancel;
        self
    }

    /// Sets the callback for when the dialog is has been confirmed.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_ok = Rc::new(on_ok);
        self
    }

    /// Sets the callback for when the dialog is has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_cancel = Rc::new(on_cancel);
        self
    }

    pub(crate) fn render_ok(&self, _: &mut Window, _: &mut App) -> AnyElement {
        let ok_text = self
            .ok_text
            .clone()
            .unwrap_or_else(|| t!("Dialog.ok").into());
        let ok_variant = self.ok_variant;

        Button::new("ok")
            .label(ok_text)
            .with_variant(ok_variant)
            .on_click(|_, window, cx| {
                window.dispatch_action(Box::new(Confirm { secondary: false }), cx)
            })
            .into_any_element()
    }

    pub(crate) fn render_cancel(&self, _: &mut Window, _: &mut App) -> AnyElement {
        let cancel_text = self
            .cancel_text
            .clone()
            .unwrap_or_else(|| t!("Dialog.cancel").into());
        let cancel_variant = self.cancel_variant;

        Button::new("cancel")
            .label(cancel_text)
            .with_variant(cancel_variant)
            .on_click(|_, window, cx| window.dispatch_action(Box::new(Cancel), cx))
            .into_any_element()
    }
}

type ContentBuilderFn = Rc<dyn Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static>;

#[derive(Clone)]
pub(crate) struct DialogProps {
    width: Pixels,
    max_width: Option<Pixels>,
    margin_top: Option<Pixels>,
    close_button: bool,

    overlay: bool,
    overlay_closable: bool,
    pub(crate) overlay_visible: bool,
    keyboard: bool,
}

impl Default for DialogProps {
    fn default() -> Self {
        Self {
            margin_top: None,
            width: px(448.),
            max_width: None,
            overlay: true,
            keyboard: true,
            overlay_visible: false,
            close_button: true,
            overlay_closable: true,
        }
    }
}

enum BaseDialogRoot {
    Dialog(gpui_base::Dialog),
    AlertDialog(gpui_base::AlertDialog),
}

macro_rules! map_base_root {
    ($self:expr, $method:ident($($arg:expr),* $(,)?)) => {
        match $self {
            BaseDialogRoot::Dialog(root) => BaseDialogRoot::Dialog(root.$method($($arg),*)),
            BaseDialogRoot::AlertDialog(root) => {
                BaseDialogRoot::AlertDialog(root.$method($($arg),*))
            }
        }
    };
}

impl BaseDialogRoot {
    fn layer(self, index: usize, topmost: bool) -> Self {
        map_base_root!(self, layer(index, topmost))
    }
    fn focus_handle(self, focus: FocusHandle) -> Self {
        map_base_root!(self, focus_handle(focus))
    }
    fn close_on_escape(self, value: bool) -> Self {
        map_base_root!(self, close_on_escape(value))
    }
    fn close_on_backdrop_press(self, value: bool) -> Self {
        match self {
            Self::Dialog(root) => Self::Dialog(root.close_on_backdrop_press(value)),
            Self::AlertDialog(root) => Self::AlertDialog(root),
        }
    }
    fn dismiss_below_y(self, value: Pixels) -> Self {
        map_base_root!(self, dismiss_below_y(value))
    }
    fn backdrop(self, element: impl IntoElement) -> Self {
        map_base_root!(self, backdrop(element))
    }
    fn popup(self, element: impl IntoElement) -> Self {
        map_base_root!(self, popup(element))
    }
    fn on_ok(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static) -> Self {
        map_base_root!(self, on_ok(handler))
    }
    fn on_cancel(
        self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        map_base_root!(self, on_cancel(handler))
    }
    fn on_close(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        map_base_root!(self, on_close(handler))
    }
    fn request_close(self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        map_base_root!(self, request_close(handler))
    }
}

impl IntoElement for BaseDialogRoot {
    type Element = <gpui_base::Dialog as IntoElement>::Element;
    fn into_element(self) -> Self::Element {
        match self {
            Self::Dialog(root) => root.into_element(),
            Self::AlertDialog(root) => root.into_element(),
        }
    }
}

/// A modal to display content in a dialog box.
#[derive(IntoElement)]
pub struct Dialog {
    base: Option<BaseDialogRoot>,
    pub(crate) style: StyleRefinement,
    children: Vec<AnyElement>,
    trigger: Option<AnyElement>,
    title: Option<AnyElement>,
    pub(crate) header: Option<AnyElement>,
    pub(crate) footer: Option<AnyElement>,
    pub(crate) content_builder: Option<ContentBuilderFn>,
    pub(crate) props: DialogProps,

    pub(super) button_props: DialogButtonProps,

    /// This will be change when open the dialog, the focus handle is create when open the dialog.
    pub(crate) focus_handle: FocusHandle,
    pub(crate) layer_ix: usize,
    pub(crate) selection_scope: TextSelectionScopeId,
}

pub(crate) fn overlay_color(overlay: bool, cx: &App) -> Hsla {
    if !overlay {
        return hsla(0., 0., 0., 0.);
    }

    cx.theme().overlay
}

impl Dialog {
    /// Create a new dialog.
    pub fn new(cx: &mut App) -> Self {
        Self {
            base: Some(BaseDialogRoot::Dialog(gpui_base::Dialog::new(cx))),
            focus_handle: cx.focus_handle(),
            style: StyleRefinement::default(),
            trigger: None,
            title: None,
            header: None,
            footer: None,
            content_builder: None,
            props: DialogProps::default(),
            children: Vec::new(),
            layer_ix: 0,
            selection_scope: TextSelectionScopeId::default(),
            button_props: DialogButtonProps::default(),
        }
    }

    /// Sets the trigger element for the dialog.
    ///
    /// When a trigger is set, the dialog will render as a trigger button that opens the dialog when clicked.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Sets the content of the dialog.
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static,
    {
        self.content_builder = Some(Rc::new(builder));
        self
    }

    /// Sets the title of the dialog.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Sets the footer of the dialog, the footer will render at the bottom of the dialog, usually for action buttons.
    ///
    /// When you set the footer, the `button_props` will be ignored, you need to render the action buttons by yourself.
    pub(crate) fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Sets the footer of the dialog, the footer will render at the bottom of the dialog, usually for action buttons.
    ///
    /// When you set the footer, the `button_props` will be ignored, you need to render the action buttons by yourself.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Set the button props of the dialog.
    pub fn button_props(mut self, button_props: DialogButtonProps) -> Self {
        self.button_props = button_props;
        self
    }
    pub(crate) fn with_base_alert_dialog(mut self, base: gpui_base::AlertDialog) -> Self {
        self.base = Some(BaseDialogRoot::AlertDialog(base));
        self.props.overlay_closable = false;
        self
    }

    /// Sets the callback for when the dialog is closed.
    ///
    /// Called after [`Self::on_ok`] or [`Self::on_cancel`] callback.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button_props.on_close = Rc::new(on_close);
        self
    }

    /// Sets the callback for when the dialog is has been confirmed.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_ok(on_ok);
        self
    }

    /// Sets the callback for when the dialog is has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_cancel(on_cancel);
        self
    }

    /// Sets the false to hide close icon, default: true
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.props.close_button = close_button;
        self
    }

    /// Set the top offset of the dialog, defaults to None, will use the 1/10 of the viewport height.
    pub fn margin_top(mut self, margin_top: impl Into<Pixels>) -> Self {
        self.props.margin_top = Some(margin_top.into());
        self
    }

    /// Sets the width of the dialog, defaults to 448px.
    ///
    /// See also [`Self::width`]
    pub fn w(mut self, width: impl Into<Pixels>) -> Self {
        self.props.width = width.into();
        self
    }

    /// Sets the width of the dialog, defaults to 448px.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.props.width = width.into();
        self
    }

    /// Set the maximum width of the dialog, defaults to `None`.
    pub fn max_w(mut self, max_width: impl Into<Pixels>) -> Self {
        self.props.max_width = Some(max_width.into());
        self
    }

    /// Set the overlay of the dialog, defaults to `true`.
    pub fn overlay(mut self, overlay: bool) -> Self {
        self.props.overlay = overlay;
        self
    }

    /// Set the overlay closable of the dialog, defaults to `true`.
    ///
    /// When the overlay is clicked, the dialog will be closed.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.props.overlay_closable = overlay_closable;
        self
    }

    /// Set whether to support keyboard esc to close the dialog, defaults to `true`.
    pub fn keyboard(mut self, keyboard: bool) -> Self {
        self.props.keyboard = keyboard;
        self
    }

    pub(crate) fn has_overlay(&self) -> bool {
        self.props.overlay
    }

    pub(crate) fn with_props(mut self, props: DialogProps) -> Self {
        self.props = props;
        self
    }

    fn defer_close_dialog(window: &mut Window, cx: &mut App) {
        Root::update(window, cx, |root, window, cx| {
            root.defer_close_dialog(window, cx);
        });
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Dialog {
    fn render_trigger(self, trigger: AnyElement, _: &mut Window, _: &mut App) -> AnyElement {
        let content_builder = self.content_builder.clone();
        let style = self.style.clone();
        let props = self.props.clone();
        let button_props = self.button_props.clone();

        gpui_base::DialogTrigger::new(trigger)
            .on_open(move |window, cx| {
                let content_builder = content_builder.clone();
                let style = style.clone();
                let props = props.clone();
                let button_props = button_props.clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .refine_style(&style)
                        .button_props(button_props.clone())
                        .with_props(props.clone())
                        .content({
                            let content_builder = content_builder.clone();
                            move |content, window, cx| {
                                if let Some(builder) = content_builder.clone() {
                                    builder(content, window, cx)
                                } else {
                                    content
                                }
                            }
                        })
                });
            })
            .into_any_element()
    }
}

impl RenderOnce for Dialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(trigger) = self.trigger.take() {
            return self.render_trigger(trigger, window, cx);
        }

        let layer_ix = self.layer_ix;
        let selection_scope = self.selection_scope;
        let on_close = self.button_props.on_close.clone();
        let on_ok = self.button_props.on_ok.clone();
        let on_cancel = self.button_props.on_cancel.clone();

        let window_paddings = crate::window_border::window_paddings(window);
        let view_size = window.viewport_size()
            - gpui::size(
                window_paddings.left + window_paddings.right,
                window_paddings.top + window_paddings.bottom,
            );
        let y = self.props.margin_top.unwrap_or(view_size.height / 10.) + px(layer_ix as f32 * 16.);
        let x = view_size.width / 2. - self.props.width / 2.;

        let base_size = window.text_style().font_size;
        let rem_size = window.rem_size();

        let mut paddings = Edges::all(px(16.));
        if let Some(pl) = self.style.padding.left {
            paddings.left = pl.to_pixels(base_size, rem_size);
        }
        if let Some(pr) = self.style.padding.right {
            paddings.right = pr.to_pixels(base_size, rem_size);
        }
        if let Some(pt) = self.style.padding.top {
            paddings.top = pt.to_pixels(base_size, rem_size);
        }
        if let Some(pb) = self.style.padding.bottom {
            paddings.bottom = pb.to_pixels(base_size, rem_size);
        }

        // x1 = 1/3, x2 = 2/3 make the bezier's time mapping the identity,
        // preserving the trajectory this dialog was tuned with before
        // `cubic_bezier` solved for x; vaul's (0.32, 0.72, 0., 1.) is far
        // more front-loaded under the CSS-correct solver.
        let animation = Animation::new(*ANIMATION_DURATION).with_easing(cubic_bezier(
            1. / 3.,
            0.72,
            2. / 3.,
            1.,
        ));

        anchored()
            .position(point(window_paddings.left, window_paddings.top))
            .snap_to_window()
            .child(
                div()
                    .id("dialog")
                    .occlude()
                    .w(view_size.width)
                    .h(view_size.height)
                    .child(
                        self.base
                            .take()
                            .expect("Dialog base host is always present")
                            .layer(
                                layer_ix,
                                (self.layer_ix + 1) == Root::read(window, cx).active_dialogs.len(),
                            )
                            .focus_handle(self.focus_handle.clone())
                            .close_on_escape(self.props.keyboard)
                            .close_on_backdrop_press(self.props.overlay_closable)
                            .dismiss_below_y(TITLE_BAR_HEIGHT)
                            .when(self.props.overlay, |this| {
                                this.backdrop(
                                    div()
                                        .absolute()
                                        .size_full()
                                        .window_control_area(WindowControlArea::Drag)
                                        .when(self.props.overlay_visible, |overlay| {
                                            overlay.bg(overlay_color(true, cx))
                                        }),
                                )
                            })
                            .on_ok(move |event, window, cx| on_ok(event, window, cx))
                            .on_cancel(move |event, window, cx| on_cancel(event, window, cx))
                            .on_close(move |event, window, cx| on_close(event, window, cx))
                            .request_close(move |deferred, window, cx| {
                                if deferred {
                                    Self::defer_close_dialog(window, cx);
                                } else {
                                    window.close_dialog(cx);
                                }
                            })
                            .popup(
                                v_flex()
                                    .id(layer_ix)
                                    .bg(cx.theme().tokens.background)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(cx.theme().radius_lg)
                                    .min_h_24()
                                    .pt(paddings.top)
                                    .pb(paddings.bottom)
                                    .gap(paddings.top.max(px(8.)))
                                    .refine_style(&self.style)
                                    .px_0()
                                    // There style is high priority, can't be overridden.
                                    .absolute()
                                    .occlude()
                                    .relative()
                                    .left(x)
                                    .top(y)
                                    .w(self.props.width)
                                    .when_some(self.props.max_width, |this, w| this.max_w(w))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .overflow_hidden()
                                            .gap_y_2()
                                            .when_some(self.header, |this, header| {
                                                this.child(
                                                    div()
                                                        .pl(paddings.left)
                                                        .pr(paddings.right)
                                                        .child(header),
                                                )
                                            })
                                            .when_some(self.title, |this, title| {
                                                this.child(
                                                    DialogTitle::new()
                                                        .pl(paddings.left)
                                                        .pr(paddings.right)
                                                        .child(title),
                                                )
                                            })
                                            .when_some(self.content_builder, |this, builder| {
                                                this.child(builder(
                                                    DialogContent::new()
                                                        .gap(paddings.bottom)
                                                        .pl(paddings.left)
                                                        .pr(paddings.right),
                                                    window,
                                                    cx,
                                                ))
                                            })
                                            .when(!self.children.is_empty(), |this| {
                                                this.child(
                                                    div().flex_1().overflow_hidden().child(
                                                        // Body
                                                        v_flex()
                                                            .size_full()
                                                            .overflow_y_scrollbar()
                                                            .pl(paddings.left)
                                                            .pr(paddings.right)
                                                            .children(self.children),
                                                    ),
                                                )
                                            }),
                                    )
                                    .when_some(self.footer, |this, footer| {
                                        this.child(
                                            div()
                                                .pl(paddings.left)
                                                .pr(paddings.right)
                                                .child(footer),
                                        )
                                    })
                                    .children(self.props.close_button.then(|| {
                                        let top = (paddings.top - px(10.)).max(px(8.));
                                        let right = (paddings.right - px(10.)).max(px(8.));

                                        gpui_base::DialogClose::new()
                                            .absolute()
                                            .top(top)
                                            .right(right)
                                            .trigger(|button| {
                                                Button::new("close")
                                                    .with_base(button)
                                                    .small()
                                                    .ghost()
                                                    .icon(IconName::Close)
                                            })
                                    }))
                                    .with_animation(
                                        "slide-down",
                                        animation.clone(),
                                        move |this, delta| {
                                            // This is equivalent to `shadow_xl` with an extra opacity.
                                            let shadow = vec![
                                                BoxShadow {
                                                    color: hsla(0., 0., 0., 0.1 * delta),
                                                    offset: point(px(0.), px(20.)),
                                                    blur_radius: px(25.),
                                                    spread_radius: px(-5.),
                                                    inset: false,
                                                },
                                                BoxShadow {
                                                    color: hsla(0., 0., 0., 0.1 * delta),
                                                    offset: point(px(0.), px(8.)),
                                                    blur_radius: px(10.),
                                                    spread_radius: px(-6.),
                                                    inset: false,
                                                },
                                            ];
                                            this.top(y * delta).shadow(shadow)
                                        },
                                    )
                                    .text_selection_scope(selection_scope),
                            ),
                    )
                    .with_animation("fade-in", animation, move |this, delta| this.opacity(delta)),
            )
            .into_any_element()
    }
}

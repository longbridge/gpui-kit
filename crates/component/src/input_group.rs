use gpui_base::TestSupportExt as _;
use std::{rc::Rc, time::Duration};

use gpui::{
    AccessibleAction, AnyElement, App, ClickEvent, DefiniteLength, Edges, ElementId, Entity,
    InteractiveElement, Interactivity, IntoElement, MouseButton, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, TextAlign, ViewElement,
    Window, canvas, div, prelude::FluentBuilder as _, px, rems,
};

use crate::{
    ActiveTheme as _, Disableable, FocusableExt as _, Icon, Selectable, Sizable, Size,
    StyleSized as _, StyledExt as _,
    button::{Button, ButtonCustomVariant, ButtonVariant, ButtonVariants},
    h_flex,
    input::{InputContentType, InputState, TextareaState, control::InputControl},
    native_menu::NativeMenu,
    v_flex,
};

/// A shared frame around one text control and any number of explicitly aligned addons.
///
/// The caller retains the `InputState` or `TextareaState`. The group owns only
/// composition and presentation; it does not wrap a styled `Input` or retain a
/// second editing state. `input` accepts either text control and replaces the slot.
#[derive(IntoElement)]
pub struct InputGroup {
    id: ElementId,
    style: StyleRefinement,
    focused_style: StyleRefinement,
    invalid_style: StyleRefinement,
    disabled_style: StyleRefinement,
    control: Option<GroupControl>,
    addons: Vec<InputGroupAddon>,
    size: Size,
    disabled: bool,
    readonly: bool,
    invalid: bool,
    focus_ring: bool,
    aria_label: Option<SharedString>,
}

impl InputGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            focused_style: StyleRefinement::default(),
            invalid_style: StyleRefinement::default(),
            disabled_style: StyleRefinement::default(),
            control: None,
            addons: Vec::new(),
            size: Size::default(),
            disabled: false,
            readonly: false,
            invalid: false,
            focus_ring: true,
            aria_label: None,
        }
    }

    /// Set a single-line input or textarea, replacing the previous control.
    pub fn input(mut self, input: impl Into<InputGroupControl>) -> Self {
        self.control = Some(input.into().0);
        self
    }

    /// Append an addon. Addons on the same side retain their insertion order.
    pub fn addon(mut self, addon: InputGroupAddon) -> Self {
        self.addons.push(addon);
        self
    }

    /// Prevent editing and interaction throughout this group.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Prevent editing while preserving selection, copying, and addon actions.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Display the caller's validation result. This does not reject text edits.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Set a name for the group. Name its text control separately with `aria_label`.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Refine the focused frame after its theme defaults. Invalid styling wins
    /// when both states apply. The closure runs immediately, once per call.
    pub fn focused_style(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.focused_style = build(self.focused_style);
        self
    }

    /// Refine the invalid frame after its error border and ring defaults.
    pub fn invalid_style(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.invalid_style = build(self.invalid_style);
        self
    }

    /// Refine the disabled frame without enabling editing or addon actions.
    /// Validation styling is applied afterwards when the group is also invalid.
    pub fn disabled_style(
        mut self,
        build: impl FnOnce(StyleRefinement) -> StyleRefinement,
    ) -> Self {
        self.disabled_style = build(self.disabled_style);
        self
    }
}

impl Sizable for InputGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl crate::FocusableExt for InputGroup {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_ring
    }
}

impl Styled for InputGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[derive(Clone, Copy, Default)]
struct GroupPresentation {
    size: Size,
    disabled: bool,
    readonly: bool,
    inline_start: bool,
    inline_end: bool,
    block_start: bool,
    block_end: bool,
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self
            .control
            .as_ref()
            .map(|control| control.input.state.clone());
        let disabled = self.disabled
            || self
                .control
                .as_ref()
                .is_some_and(|control| control.input.disabled);
        let focused = !disabled
            && state
                .as_ref()
                .is_some_and(|state| state.presentation(cx).focus_handle().is_focused(window));
        let multiline = state
            .as_ref()
            .is_some_and(|state| state.presentation(cx).is_multi_line());
        let has = |alignment| self.addons.iter().any(|addon| addon.alignment == alignment);
        let presentation = GroupPresentation {
            size: self.size,
            disabled,
            readonly: self.readonly,
            inline_start: has(InputGroupAddonAlignment::InlineStart),
            inline_end: has(InputGroupAddonAlignment::InlineEnd),
            block_start: has(InputGroupAddonAlignment::BlockStart),
            block_end: has(InputGroupAddonAlignment::BlockEnd),
        };
        let mut inline_start = Vec::new();
        let mut inline_end = Vec::new();
        let mut block_start = Vec::new();
        let mut block_end = Vec::new();
        for addon in self.addons {
            let target = match addon.alignment {
                InputGroupAddonAlignment::InlineStart => &mut inline_start,
                InputGroupAddonAlignment::InlineEnd => &mut inline_end,
                InputGroupAddonAlignment::BlockStart => &mut block_start,
                InputGroupAddonAlignment::BlockEnd => &mut block_end,
            };
            target.push(addon.render_in_group(presentation, window, cx));
        }
        let control = self
            .control
            .map(|control| control.render(presentation, window, cx));
        let theme = cx.theme();
        let appearance = GroupAppearance::new(theme, focused, disabled, self.invalid);
        let radius = theme.radius;
        let foreground = theme.foreground;
        let show_ring = theme.focus_ring && self.focus_ring;
        let state_border = if self.invalid {
            self.invalid_style.border_color
        } else if focused {
            self.focused_style.border_color
        } else {
            None
        };
        let ring = appearance
            .ring
            .map(|ring| state_border.map_or(ring, |border| border.opacity(ring.a)));
        // Nova transitions border and background colors, while its outside ring
        // changes immediately. Reduced motion is handled by the shared primitive.
        let (border, background) = window.with_id(self.id.clone(), |window| {
            let transition = || {
                gpui_base::Transition::new(Duration::from_millis(150))
                    .easing(gpui_base::Easing::cubic_bezier(0.4, 0., 0.2, 1.).unwrap())
            };
            (
                gpui_base::transition("border-color", appearance.border, transition(), window, cx),
                gpui_base::transition(
                    "background-color",
                    appearance.background,
                    transition(),
                    window,
                    cx,
                ),
            )
        });
        v_flex()
            .id(self.id)
            .test_support()
            .role(Role::Group)
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .relative()
            .w_full()
            .min_w_0()
            .when(
                !multiline && !presentation.block_start && !presentation.block_end,
                |this| this.input_h(self.size),
            )
            .rounded(radius)
            .border_1()
            .border_color(border)
            .bg(background)
            .text_color(foreground)
            .input_text_size(self.size)
            .shadow_none()
            .when(disabled, |this| {
                // Custom addon content must not bypass the group's disabled policy.
                this.capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
                    .capture_key_down(|event, _, cx| {
                        if event.keystroke.key != "tab" {
                            cx.stop_propagation();
                        }
                    })
            })
            .refine_style(&self.style)
            .when(disabled, |this| {
                this.bg(background)
                    .opacity(0.5)
                    .refine_style(&self.disabled_style)
            })
            .when(appearance.ring.is_some(), |this| this.border_color(border))
            .when(focused && !self.invalid, |this| {
                this.refine_style(&self.focused_style)
            })
            .when(self.invalid, |this| this.refine_style(&self.invalid_style))
            .when_some(ring.filter(|_| show_ring), |this, ring| {
                crate::styled::focus_ring(this, window, ring)
            })
            .when_some(state.filter(|_| !disabled), |this, state| {
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    // Native buttons prevent the default mouse-down focus action.
                    // Respect that before focusing the editor from an addon or inset.
                    if !window.default_prevented() {
                        state.focus(window, cx);
                        window.prevent_default();
                    }
                })
            })
            .children(block_start)
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .items_center()
                    .when(
                        !multiline && !presentation.block_start && !presentation.block_end,
                        |this| this.h_full(),
                    )
                    .children(inline_start)
                    .children(control)
                    .children(inline_end),
            )
            .children(block_end)
    }
}

struct GroupAppearance {
    background: gpui::Hsla,
    border: gpui::Hsla,
    ring: Option<gpui::Hsla>,
}

impl GroupAppearance {
    fn new(theme: &crate::Theme, focused: bool, disabled: bool, invalid: bool) -> Self {
        let background = if disabled {
            theme.input.opacity(if theme.is_dark() { 0.8 } else { 0.5 })
        } else if theme.is_dark() {
            theme.input.opacity(0.3)
        } else {
            theme.transparent
        };
        // Validation remains visible when editing is disabled. Focus alone never
        // reactivates a disabled control, and never replaces its validation color.
        let (border, ring) = if invalid {
            (
                theme.danger,
                Some(
                    theme
                        .danger
                        .opacity(if theme.is_dark() { 0.4 } else { 0.2 }),
                ),
            )
        } else if focused && !disabled {
            (theme.ring, Some(theme.ring.opacity(0.5)))
        } else {
            (theme.input, None)
        };
        Self {
            background,
            border,
            ring,
        }
    }
}

/// The logical side of an addon relative to the text control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputGroupAddonAlignment {
    #[default]
    InlineStart,
    InlineEnd,
    BlockStart,
    BlockEnd,
}

/// Text, icons, buttons, or custom content on one side of an input group.
///
/// Children retain their insertion order. Direct `InputGroupButton` children
/// inherit the group's disabled state. Custom children retain their own semantics.
#[derive(IntoElement)]
pub struct InputGroupAddon {
    id: ElementId,
    alignment: InputGroupAddonAlignment,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl InputGroupAddon {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            alignment: InputGroupAddonAlignment::default(),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn align(mut self, alignment: InputGroupAddonAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    fn render_in_group(
        mut self,
        presentation: GroupPresentation,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let mut has_button = false;
        let mut has_kbd = false;
        for child in &mut self.children {
            if let Some(button) = button_element::InputGroupButtonElement::from_element(child) {
                has_button = true;
                button.disable(presentation.disabled);
            }
            let child = button_element::unwrapped_element(child);
            has_button |= child.downcast_mut::<ViewElement<Button>>().is_some();
            has_kbd |= child
                .downcast_mut::<ViewElement<crate::kbd::Kbd>>()
                .is_some();
        }
        let border_top = self
            .style
            .border_widths
            .top
            .is_some_and(|width| width.to_pixels(window.rem_size()) > px(0.));
        let border_bottom = self
            .style
            .border_widths
            .bottom
            .is_some_and(|width| width.to_pixels(window.rem_size()) > px(0.));
        let inline_offset = if has_kbd {
            rems(-0.15)
        } else if has_button {
            rems(-0.3)
        } else {
            rems(0.)
        };
        h_flex()
            .id(self.id)
            .test_support()
            .flex_none()
            .gap_2()
            .py_1p5()
            .when(
                matches!(presentation.size, Size::XSmall | Size::Small),
                |this| this.py_0(),
            )
            .justify_center()
            .font_medium()
            .input_text_size(presentation.size)
            .text_color(cx.theme().muted_foreground)
            .cursor_text()
            .map(|this| match self.alignment {
                InputGroupAddonAlignment::InlineStart => this.pl_2().ml(inline_offset),
                InputGroupAddonAlignment::InlineEnd => this.pr_2().mr(inline_offset),
                InputGroupAddonAlignment::BlockStart => this
                    .w_full()
                    .justify_start()
                    .px_2p5()
                    .pt_2()
                    .when(border_bottom, |this| this.pb_2()),
                InputGroupAddonAlignment::BlockEnd => this
                    .w_full()
                    .justify_start()
                    .px_2p5()
                    .pb_2()
                    .when(border_top, |this| this.pt_2()),
            })
            .refine_style(&self.style)
            .children(self.children.into_iter().map(addon_child))
            .into_any_element()
    }
}

impl ParentElement for InputGroupAddon {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InputGroupAddon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupAddon {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_in_group(GroupPresentation::default(), window, cx)
    }
}

struct GroupControl {
    input: InputControl,
    style: StyleRefinement,
    editor_style: StyleRefinement,
    height: Option<DefiniteLength>,
}

/// A single-line input or textarea accepted by [`InputGroup::input`].
/// Constructed by converting either input part; it owns no editing state.
pub struct InputGroupControl(GroupControl);

impl From<InputGroupInput> for InputGroupControl {
    fn from(input: InputGroupInput) -> Self {
        Self(input.0)
    }
}

impl From<InputGroupTextarea> for InputGroupControl {
    fn from(textarea: InputGroupTextarea) -> Self {
        Self(textarea.0)
    }
}

impl GroupControl {
    fn new(input: InputControl) -> Self {
        Self {
            input,
            style: StyleRefinement::default(),
            editor_style: StyleRefinement::default(),
            height: None,
        }
    }

    fn render(
        mut self,
        presentation: GroupPresentation,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        self.input.disabled |= presentation.disabled;
        self.input.readonly |= presentation.readonly;
        let state = self.input.state.clone();
        let multiline = state.presentation(cx).is_multi_line();
        let mut editor = div()
            .relative()
            .flex()
            .flex_1()
            .min_w_0()
            .px_2p5()
            .when(multiline, |this| this.h_full().py_2())
            .when(!multiline, |this| {
                this.items_center()
                    .when(
                        !matches!(presentation.size, Size::XSmall | Size::Small),
                        |this| this.py_1(),
                    )
                    .when(
                        !presentation.block_start && !presentation.block_end,
                        |this| this.h_full(),
                    )
                    .when(presentation.inline_start, |this| this.pl_1p5())
                    .when(presentation.inline_end, |this| this.pr_1p5())
                    .when(presentation.block_start, |this| this.pb_3())
                    .when(presentation.block_end, |this| this.pt_3())
            })
            .refine_style(&self.editor_style);
        // The multiline engine owns its viewport insets, including scrollbar
        // placement. Transfer padding to it instead of adding a second inset.
        // Relative padding follows the measured viewport width and is recomputed
        // after resize; absolute padding needs no retained layout measurement.
        let padding = editor.style().padding.clone();
        let relative_padding = multiline
            && [padding.top, padding.right, padding.bottom, padding.left]
                .iter()
                .any(|value| matches!(value, Some(DefiniteLength::Fraction(_))));
        let viewport_width = relative_padding.then(|| {
            window.use_keyed_state(
                ("input-group-editor-width", state.entity_id()),
                cx,
                |_, _| px(0.),
            )
        });
        let width = viewport_width
            .as_ref()
            .map_or(px(0.), |width| *width.read(cx));
        let resolve = |value: Option<DefiniteLength>| {
            value
                .unwrap_or_default()
                .to_pixels(width.into(), window.rem_size())
        };
        let paddings = if multiline {
            Edges {
                top: resolve(padding.top),
                right: resolve(padding.right),
                bottom: resolve(padding.bottom),
                left: resolve(padding.left),
            }
        } else {
            Edges::default()
        };
        let overlays = self.input.prepare(
            paddings,
            self.editor_style
                .text
                .text_align
                .or(self.style.text.text_align)
                .unwrap_or(TextAlign::Left),
            window,
            cx,
        );
        // Frame and editor need distinct focus identities; the editor owns the
        // keyboard path, while this handle describes its accessible frame.
        let frame_focus = window
            .use_keyed_state(
                ("input-group-frame-focus", state.entity_id()),
                cx,
                |_, cx| cx.focus_handle(),
            )
            .read(cx)
            .clone();
        let accessibility_state = state.clone();
        let editor = editor
            .when(multiline, |this| this.p_0())
            .child(state.clone().into_any_element())
            .when_some(viewport_width, |this, width| {
                this.child(
                    canvas(
                        move |bounds, window, cx| {
                            width.update(cx, |width, cx| {
                                if *width != bounds.size.width {
                                    *width = bounds.size.width;
                                    cx.notify();
                                    window.refresh();
                                }
                            });
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
            })
            .map(|this| {
                if multiline {
                    v_flex()
                        .size_full()
                        .children(overlays.search)
                        .child(this)
                        .into_any_element()
                } else {
                    this.into_any_element()
                }
            });
        self.input
            .frame(("input-group-control", state.entity_id()), window, cx)
            .track_focus(&frame_focus)
            .when(!self.input.disabled, |this| {
                this.on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
                    accessibility_state.focus(window, cx);
                })
            })
            .relative()
            .flex()
            .flex_1()
            .min_w_0()
            .line_height(match presentation.size {
                Size::XSmall => rems(1.),
                Size::Large => rems(1.5),
                _ => rems(1.25),
            })
            .input_text_size(presentation.size)
            .text_color(cx.theme().foreground)
            .when(!multiline, |this| {
                this.when(
                    !presentation.block_start && !presentation.block_end,
                    |this| this.h_full(),
                )
            })
            .when(multiline, |this| this.h_auto().min_h_16())
            .when_some(self.height, |this, height| this.h(height))
            .refine_style(&self.style)
            .child(editor)
            .children(overlays.floating)
            .into_any_element()
    }
}

/// An unframed single-line input for the group's `input` slot.
#[derive(IntoElement)]
pub struct InputGroupInput(GroupControl);

impl InputGroupInput {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self(GroupControl::new(InputControl::new(state.clone().into())))
    }

    /// Supply the native content-type hint without changing the text or mask.
    pub fn content_type(mut self, content_type: InputContentType) -> Self {
        self.0.input.content_type = Some(content_type);
        self
    }
}

/// An unframed textarea for the group's `input` slot.
#[derive(IntoElement)]
pub struct InputGroupTextarea(GroupControl);

impl InputGroupTextarea {
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self(GroupControl::new(InputControl::new(state.clone().into())))
    }

    /// Set the viewport height. Rows and auto-grow remain owned by TextareaState.
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.0.height = Some(height.into());
        self
    }
}

macro_rules! impl_group_control {
    ($control:ident) => {
        impl $control {
            /// Refine the editing viewport after its defaults. Padding affects
            /// text layout, caret hit testing, selection, IME, and scrolling.
            /// The closure runs immediately; repeated calls accumulate overrides.
            pub fn editor_style(
                mut self,
                build: impl FnOnce(StyleRefinement) -> StyleRefinement,
            ) -> Self {
                self.0.editor_style = build(self.0.editor_style);
                self
            }

            pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
                self.0.input.aria_label = Some(label.into());
                self
            }

            pub fn accessibility_id(mut self, id: impl Into<SharedString>) -> Self {
                self.0.input.accessibility_id = Some(id.into());
                self
            }

            pub fn disabled(mut self, disabled: bool) -> Self {
                self.0.input.disabled = disabled;
                self
            }

            pub fn readonly(mut self, readonly: bool) -> Self {
                self.0.input.readonly = readonly;
                self
            }

            /// Replace the built-in native context menu.
            pub fn context_menu(
                mut self,
                build: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
            ) -> Self {
                self.0.input.context_menu_builder = Some(Rc::new(build));
                self
            }
        }

        impl Styled for $control {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.0.style
            }
        }

        impl RenderOnce for $control {
            fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
                self.0.render(GroupPresentation::default(), window, cx)
            }
        }
    };
}

impl_group_control!(InputGroupInput);
impl_group_control!(InputGroupTextarea);

/// Compact text and square-icon button sizes used inside input groups.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputGroupButtonSize {
    #[default]
    XSmall,
    Small,
    IconXSmall,
    IconSmall,
}

/// A native Button with compact input-group presentation and disabled inheritance.
pub struct InputGroupButton {
    button: Button,
    size: InputGroupButtonSize,
    style: StyleRefinement,
    label_style: StyleRefinement,
    icon_style: StyleRefinement,
}

impl InputGroupButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            button: Button::new(id).ghost(),
            size: InputGroupButtonSize::default(),
            style: StyleRefinement::default(),
            label_style: StyleRefinement::default(),
            icon_style: StyleRefinement::default(),
        }
    }

    pub fn with_size(mut self, size: InputGroupButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Configure additional native Button capabilities.
    pub fn with_button(mut self, build: impl FnOnce(Button) -> Button) -> Self {
        self.button = build(self.button);
        self
    }

    /// Refine the visible label, independently of the icon and button frame.
    /// Repeated calls refine the accumulated override; theme defaults stay live.
    pub fn label_style(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.label_style = build(self.label_style);
        self
    }

    /// Refine the icon after its compact size defaults, including its loading icon.
    pub fn icon_style(mut self, build: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.icon_style = build(self.icon_style);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.button = self.button.label(label);
        self
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.button = self.button.icon(icon.into());
        self
    }

    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.button = self.button.accessibility_label(label);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.button = self.button.tooltip(tooltip);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.button = self.button.loading(loading);
        self
    }

    pub fn outline(mut self) -> Self {
        self.button = self.button.outline();
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button = self.button.on_click(handler);
        self
    }

    fn render_in_group(self, disabled: bool, window: &mut Window, cx: &mut App) -> AnyElement {
        let disabled = disabled || self.button.is_disabled();
        let selected = self.button.is_selected();
        let ghost =
            matches!(self.button.variant(), ButtonVariant::Ghost) && !self.button.is_outline();
        let muted = cx.theme().muted;
        let hover = muted.opacity(if cx.theme().is_dark() { 0.5 } else { 1. });
        let icon_size = match self.size {
            InputGroupButtonSize::XSmall => Size::Small,
            _ => Size::Medium,
        };
        let content_style = div()
            .text_sm()
            .line_height(rems(1.25))
            .map(|this| match self.size {
                InputGroupButtonSize::XSmall => this.gap_1(),
                _ => this.gap_2(),
            })
            .style()
            .clone();
        self.button
            .when(disabled, |this| this.disabled(true))
            .with_size(Size::Medium)
            .content_style(content_style, icon_size)
            .part_styles(self.label_style, self.icon_style)
            .when(ghost, |this| {
                this.custom(
                    ButtonCustomVariant::new(cx)
                        .color(cx.theme().transparent)
                        .foreground(cx.theme().foreground)
                        .hover(hover)
                        .active(if selected { muted } else { hover }),
                )
                .text_color(cx.theme().foreground)
                .when(disabled, |this| this.opacity(0.5))
            })
            .when(disabled, |this| this.focus_ring(false))
            .text_sm()
            .font_medium()
            .border_1()
            .shadow_none()
            .map(|this| match self.size {
                InputGroupButtonSize::XSmall => this
                    .h_6()
                    .px_1p5()
                    .gap_1()
                    .rounded(cx.theme().radius_tokens().sm),
                InputGroupButtonSize::Small => {
                    this.h_8().px_2p5().gap_2().rounded(cx.theme().radius)
                }
                InputGroupButtonSize::IconXSmall => {
                    this.size_6().p_0().rounded(cx.theme().radius_tokens().sm)
                }
                InputGroupButtonSize::IconSmall => this.size_8().p_0().rounded(cx.theme().radius),
            })
            .refine_style(&self.style)
            .render(window, cx)
            .into_any_element()
    }
}

impl ButtonVariants for InputGroupButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.button = self.button.with_variant(variant);
        self
    }
}

impl Disableable for InputGroupButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.button = self.button.disabled(disabled);
        self
    }
}

impl Selectable for InputGroupButton {
    fn selected(mut self, selected: bool) -> Self {
        self.button = self.button.selected(selected);
        self
    }

    fn is_selected(&self) -> bool {
        self.button.is_selected()
    }
}

impl InteractiveElement for InputGroupButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.button.interactivity()
    }
}

impl crate::menu::DropdownMenu for InputGroupButton {}

impl IntoElement for InputGroupButton {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        button_element::InputGroupButtonElement::new(self).into_any_element()
    }

    fn into_any_element(self) -> AnyElement {
        self.into_element()
    }
}

impl Styled for InputGroupButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for InputGroupButton {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(elements);
    }
}

impl RenderOnce for InputGroupButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_in_group(false, window, cx)
    }
}

/// Muted helper text, optionally combined with icons, inside an input group.
#[derive(IntoElement, Default)]
pub struct InputGroupText {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl InputGroupText {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for InputGroupText {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for InputGroupText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for InputGroupText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .gap_2()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .children(self.children.into_iter().map(addon_child))
    }
}

fn addon_child(mut child: AnyElement) -> AnyElement {
    if button_element::unwrapped_element(&mut child)
        .downcast_mut::<ViewElement<Icon>>()
        .is_some()
    {
        // Unspecified icon sizes follow Nova's one-rem addon default, while
        // explicit sizes remain owned by the icon itself.
        div()
            .flex_none()
            .text_base()
            .child(child)
            .into_any_element()
    } else {
        child
    }
}

#[cfg(test)]
mod tests;

mod button_element;

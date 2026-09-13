use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, DefiniteLength, Edges, ElementId, Entity, Hsla, InteractiveElement as _,
    IntoElement, ParentElement as _, Rems, RenderOnce, SharedString, StyleRefinement, Styled,
    TextAlign, Window, div, px, relative,
};

use crate::button::{Button, ButtonVariants as _};
use crate::input::clear_button;
use crate::native_menu::NativeMenu;
use crate::spinner::Spinner;
use crate::{ActiveTheme, Colorize, v_flex};
use crate::{IconName, Size};
use crate::{RoleOverride, Selectable, StyledExt, h_flex};
use crate::{Sizable, StyleSized};

use super::control::InputControl;
use super::state::TextInputState;
use super::{InputContentType, InputState};
use crate::ThemeStyled as _;

/// Returns `(background, foreground)` colors for input-like components.
pub(crate) fn input_style(disabled: bool, cx: &App) -> (Hsla, Hsla) {
    if disabled {
        (
            cx.theme().input.mix_oklab(cx.theme().transparent, 0.8),
            cx.theme().muted_foreground,
        )
    } else {
        (cx.theme().input_background(), cx.theme().foreground)
    }
}

/// A text input element bind to an [`InputState`].
#[derive(IntoElement)]
pub struct Input {
    id: Option<ElementId>,
    state: TextInputState,
    style: StyleRefinement,
    size: Size,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    height: Option<DefiniteLength>,
    appearance: bool,
    cleanable: bool,
    mask_toggle: bool,
    disabled: bool,
    readonly: bool,
    bordered: bool,
    focus_bordered: bool,
    tab_index: isize,
    selected: bool,
    content_type: Option<InputContentType>,
    role: RoleOverride,
    accessibility_id: Option<SharedString>,
    aria_label: Option<SharedString>,

    /// An optional context menu builder to allow a custom context menu on the input.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Sizable for Input {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Selectable for Input {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl crate::FocusableExt for Input {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_bordered = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_bordered
    }
}

impl Input {
    /// Sets the GPUI identity of the input frame. By default it uses the state entity ID.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Create a new [`Input`] element bind to the [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self::with_state(state.clone().into())
    }

    /// Builds an input renderer around a state of any kind.
    ///
    /// `Textarea` and `Editor` render through this. Application code uses
    /// [`Input::new`], [`super::Textarea`], or [`super::Editor`].
    pub(crate) fn from_state(state: impl Into<TextInputState>) -> Self {
        Self::with_state(state.into())
    }

    fn with_state(state: TextInputState) -> Self {
        Self {
            id: None,
            state,
            size: Size::default(),
            style: StyleRefinement::default(),
            prefix: None,
            suffix: None,
            height: None,
            appearance: true,
            cleanable: false,
            mask_toggle: false,
            disabled: false,
            readonly: false,
            bordered: true,
            focus_bordered: true,
            tab_index: 0,
            selected: false,
            content_type: None,
            role: RoleOverride::default(),
            accessibility_id: None,
            aria_label: None,
            context_menu_builder: None,
        }
    }

    /// Set the developer-assigned identifier exposed to accessibility clients.
    pub fn accessibility_id(mut self, id: impl Into<SharedString>) -> Self {
        self.accessibility_id = Some(id.into());
        self
    }

    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set full height of the input (Multi-line only).
    pub fn h_full(mut self) -> Self {
        self.height = Some(relative(1.));
        self
    }

    /// Set height of the input (Multi-line only).
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set the appearance of the input field, if false the input field will no border, background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set the bordered for the input, default: true
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set focus border for the input, default is true.
    pub fn focus_bordered(mut self, bordered: bool) -> Self {
        self.focus_bordered = bordered;
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Set to enable toggle button for password mask state.
    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    /// Set the semantic content type for password managers and autofill.
    ///
    /// This is a component-level semantic hint. It does not change the text
    /// value or masked rendering state.
    pub fn content_type(mut self, content_type: InputContentType) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Override the accessible role for the input.
    ///
    /// If unset, the role is inferred from multi-line mode and content type.
    pub fn role(mut self, role: impl Into<RoleOverride>) -> Self {
        self.role = role.into();
        self
    }

    /// Set to disable the input field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the input field to read-only, default is `false`.
    ///
    /// Unlike [`Self::disabled`], a read-only input keeps the normal appearance
    /// and still can be focused, selected and copied, it only rejects the changes
    /// made by the user.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Set the tab index for the input, default is 0.
    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }

    /// Sets a custom context menu builder for the input, shown as a native OS menu.
    ///
    /// If set, this overrides the built-in right-click context menu.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }

    fn render_toggle_mask_button(state: &TextInputState, cx: &App) -> impl IntoElement {
        let masked = state.presentation(cx).is_masked();
        Button::new("toggle-mask")
            .icon(if masked {
                IconName::Eye
            } else {
                IconName::EyeOff
            })
            .xsmall()
            .text()
            .tab_stop(false)
            .on_click({
                let state = state.clone();
                move |_, window, cx| state.toggle_masked(window, cx)
            })
    }

    /// This method must after the refine_style.
    fn render_editor(
        input_state: TextInputState,
        search_panel: Option<AnyElement>,
        _: &Window,
    ) -> impl IntoElement {
        v_flex().size_full().children(search_panel).child(
            div()
                .relative()
                .flex_1()
                .child(input_state.into_any_element()),
        )
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        const LINE_HEIGHT: Rems = Rems(1.25);
        let text_align = self.style.text.text_align.unwrap_or(TextAlign::Left);
        let state = self.state.clone();
        let control = InputControl {
            state: state.clone(),
            disabled: self.disabled,
            readonly: self.readonly,
            content_type: self.content_type,
            role: self.role,
            accessibility_id: self.accessibility_id,
            aria_label: self.aria_label,
            context_menu_builder: self.context_menu_builder,
        };
        let overlays = control.prepare(
            if state.presentation(cx).is_multi_line() {
                Edges {
                    top: self.size.input_py(),
                    right: self.size.input_px(),
                    bottom: self.size.input_py(),
                    left: self.size.input_px(),
                }
            } else {
                Edges::default()
            },
            text_align,
            window,
            cx,
        );

        let presentation = state.presentation(cx);
        let disabled = self.disabled;
        let is_multi_line = presentation.is_multi_line();
        let input_focused =
            presentation.focus_handle().is_focused(window) && !presentation.is_disabled();
        let frame_focus_handle = window
            .use_keyed_state(("input-frame-focus", state.entity_id()), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let focused = input_focused
            || (frame_focus_handle.contains_focused(window, cx) && !presentation.is_disabled());

        let gap_x = match self.size {
            Size::Small => px(4.),
            Size::Large => px(8.),
            _ => px(6.),
        };

        let (bg, _) = input_style(presentation.is_disabled(), cx);
        let bg = if presentation.is_code_editor() {
            cx.theme().editor_background()
        } else {
            bg
        };
        let bg = if presentation.is_disabled() {
            bg.opacity(0.5)
        } else {
            bg
        };
        let prefix = self.prefix;
        let suffix = self.suffix;
        let show_clear_button = self.cleanable
            && presentation.is_editable()
            && !presentation.is_loading()
            && state.text(cx).len() > 0
            && !presentation.is_multi_line();
        let has_suffix =
            suffix.is_some() || presentation.is_loading() || self.mask_toggle || show_clear_button;

        let id = self
            .id
            .unwrap_or_else(|| ("input", state.entity_id()).into());
        control
            .frame(id, window, cx)
            .focused(focused)
            .disabled(disabled)
            .track_focus(&frame_focus_handle)
            .styles(|styles| {
                styles.focused(|style| {
                    style.when(
                        self.appearance && self.bordered && self.focus_bordered,
                        |style| style.border_1().border_color(cx.theme().ring),
                    )
                })
            })
            .flex()
            .size_full()
            .line_height(LINE_HEIGHT)
            .when(!is_multi_line, |this| {
                this.input_px(self.size).input_py(self.size)
            })
            .input_h(self.size)
            .input_text_size(self.size)
            .items_center()
            .when(presentation.is_multi_line(), |this| {
                this.h_auto()
                    .when_some(self.height, |this, height| this.h(height))
            })
            .when(self.appearance, |this| {
                this.bg(bg)
                    .rounded(cx.theme().radius)
                    .when(self.bordered, |this| {
                        this.border_1().border_color(cx.theme().input)
                    })
            })
            .items_center()
            .gap(gap_x)
            .refine_style(&self.style)
            .when(
                focused && self.appearance && self.bordered && self.focus_bordered,
                |this| this.focus_ring_style(window, cx),
            )
            .children(prefix.map(|p| {
                div()
                    .when(presentation.is_disabled(), |this| this.opacity(0.5))
                    .child(p)
            }))
            .when(presentation.is_multi_line(), |this| {
                this.child(Self::render_editor(state.clone(), overlays.search, window))
            })
            .when(!presentation.is_multi_line(), |this| {
                this.child(state.clone().into_any_element())
            })
            .when(has_suffix, |this| {
                this.pr(self.size.input_px()).child(
                    h_flex()
                        .id("suffix")
                        .gap(gap_x)
                        .items_center()
                        .cursor_default()
                        .when(presentation.is_disabled(), |this| this.opacity(0.5))
                        .when(presentation.is_loading(), |this| {
                            this.child(Spinner::new().color(cx.theme().muted_foreground))
                        })
                        .when(self.mask_toggle, |this| {
                            this.child(Self::render_toggle_mask_button(&state, cx))
                        })
                        .when(show_clear_button, |this| {
                            this.child(clear_button(cx).on_click({
                                let state = state.clone();
                                move |_, window, cx| {
                                    state.clean(window, cx);
                                    state.focus(window, cx);
                                }
                            }))
                        })
                        .children(suffix),
                )
            })
            .relative()
            .children(overlays.floating)
            .render(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::super::control::{
        accessibility_role, exposes_accessibility_value, handle_accessibility_set_value,
    };
    use super::*;
    use crate::input::AnyInputState;
    use gpui::{AccessibleAction, Role};

    #[test]
    fn content_types_map_to_accessibility_roles() {
        let cases = [
            (None, Role::TextInput),
            (Some(InputContentType::Name), Role::TextInput),
            (Some(InputContentType::NamePrefix), Role::TextInput),
            (Some(InputContentType::GivenName), Role::TextInput),
            (Some(InputContentType::MiddleName), Role::TextInput),
            (Some(InputContentType::FamilyName), Role::TextInput),
            (Some(InputContentType::NameSuffix), Role::TextInput),
            (Some(InputContentType::Nickname), Role::TextInput),
            (Some(InputContentType::JobTitle), Role::TextInput),
            (Some(InputContentType::OrganizationName), Role::TextInput),
            (Some(InputContentType::Location), Role::TextInput),
            (Some(InputContentType::FullStreetAddress), Role::TextInput),
            (Some(InputContentType::StreetAddressLine1), Role::TextInput),
            (Some(InputContentType::StreetAddressLine2), Role::TextInput),
            (Some(InputContentType::AddressCity), Role::TextInput),
            (Some(InputContentType::AddressState), Role::TextInput),
            (Some(InputContentType::AddressCityAndState), Role::TextInput),
            (Some(InputContentType::Sublocality), Role::TextInput),
            (Some(InputContentType::CountryName), Role::TextInput),
            (Some(InputContentType::PostalCode), Role::TextInput),
            (
                Some(InputContentType::TelephoneNumber),
                Role::PhoneNumberInput,
            ),
            (Some(InputContentType::EmailAddress), Role::EmailInput),
            (Some(InputContentType::Url), Role::UrlInput),
            (Some(InputContentType::CreditCardNumber), Role::TextInput),
            (Some(InputContentType::CreditCardName), Role::TextInput),
            (Some(InputContentType::CreditCardGivenName), Role::TextInput),
            (
                Some(InputContentType::CreditCardMiddleName),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardFamilyName),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardSecurityCode),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpiration),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpirationMonth),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpirationYear),
                Role::TextInput,
            ),
            (Some(InputContentType::CreditCardType), Role::TextInput),
            (Some(InputContentType::Username), Role::TextInput),
            (Some(InputContentType::Password), Role::PasswordInput),
            (Some(InputContentType::NewPassword), Role::PasswordInput),
            (Some(InputContentType::OneTimeCode), Role::TextInput),
            (
                Some(InputContentType::ShipmentTrackingNumber),
                Role::TextInput,
            ),
            (Some(InputContentType::FlightNumber), Role::TextInput),
            (Some(InputContentType::DateTime), Role::DateTimeInput),
            (Some(InputContentType::Birthdate), Role::DateInput),
            (Some(InputContentType::BirthdateDay), Role::TextInput),
            (Some(InputContentType::BirthdateMonth), Role::TextInput),
            (Some(InputContentType::BirthdateYear), Role::TextInput),
            (Some(InputContentType::CellularEid), Role::TextInput),
            (Some(InputContentType::CellularImei), Role::TextInput),
        ];

        for (content_type, role) in cases {
            assert_eq!(
                accessibility_role(false, content_type, RoleOverride::Implicit),
                Some(role)
            );
        }
    }

    #[test]
    fn multiline_inputs_keep_multiline_accessibility_role() {
        assert_eq!(
            accessibility_role(
                true,
                Some(InputContentType::Password),
                RoleOverride::Implicit
            ),
            Some(Role::MultilineTextInput)
        );
    }

    #[test]
    fn explicit_accessibility_role_overrides_defaults() {
        assert_eq!(
            accessibility_role(
                false,
                Some(InputContentType::Password),
                Role::TextInput.into()
            ),
            Some(Role::TextInput)
        );
        assert_eq!(
            accessibility_role(
                true,
                Some(InputContentType::Password),
                Role::TextInput.into()
            ),
            Some(Role::TextInput)
        );
    }

    #[test]
    fn presentational_role_emits_no_accessibility_node() {
        assert_eq!(
            accessibility_role(
                false,
                Some(InputContentType::Password),
                RoleOverride::Presentational
            ),
            None
        );
        assert_eq!(
            accessibility_role(true, None, RoleOverride::Presentational),
            None
        );
    }

    #[test]
    fn role_option_converts_to_the_matching_override() {
        assert_eq!(
            RoleOverride::from(Some(Role::Button)),
            RoleOverride::Role(Role::Button)
        );
        assert_eq!(RoleOverride::from(None), RoleOverride::Presentational);
    }

    #[gpui::test]
    fn editable_input_offers_accessibility_write_action(cx: &mut gpui::TestAppContext) {
        use crate::ElementExt as _;
        use gpui::{AppContext as _, Element as _, IntoElement as _, Render};
        use std::sync::{Arc, Mutex};

        type EmittedState = Option<(Option<String>, bool)>;

        struct InputA11yProbe {
            state: Entity<InputState>,
            emitted: Arc<Mutex<EmittedState>>,
        }

        impl Render for InputA11yProbe {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let state = self.state.clone();
                let emitted = self.emitted.clone();
                div().on_prepaint(move |_, window, cx| {
                    let input = Input::new(&state).render(window, cx).into_element();
                    let mut node = gpui::accesskit::Node::new(Role::TextInput);
                    input.write_a11y_info(&mut node);
                    *emitted.lock().unwrap() = Some((
                        node.value().map(ToOwned::to_owned),
                        node.supports_action(AccessibleAction::SetValue),
                    ));
                })
            }
        }

        cx.update(crate::init);
        let emitted = Arc::new(Mutex::new(None));
        let captured = emitted.clone();
        let (probe, cx) = cx.add_window_view(move |window, cx| InputA11yProbe {
            state: cx.new(|cx| InputState::new(window, cx).default_value("initial")),
            emitted,
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        // Normal builds leave the value lazy without an accessibility client;
        // test-support reads that same production value path eagerly.
        let expected_value = cfg!(feature = "test-support").then(|| "initial".to_owned());
        assert_eq!(*captured.lock().unwrap(), Some((expected_value, true)));

        let state = probe.read_with(cx, |probe, _| probe.state.clone());
        let base: TextInputState = state.clone().into();
        cx.update(|window, cx| {
            handle_accessibility_set_value(&base, None, window, cx);
        });
        assert_eq!(state.read_with(cx, |state, _| state.value()), "initial");

        let action = gpui::accesskit::ActionData::Value("updated".into());
        cx.update(|window, cx| {
            handle_accessibility_set_value(&base, Some(&action), window, cx);
        });
        assert_eq!(state.read_with(cx, |state, _| state.value()), "updated");
    }

    #[gpui::test]
    fn input_emits_accessibility_id(cx: &mut gpui::TestAppContext) {
        use crate::ElementExt as _;
        use gpui::{AppContext as _, Element as _, IntoElement as _, Render};
        use std::sync::{Arc, Mutex};

        type EmittedIds = Vec<Option<String>>;

        struct InputA11yProbe {
            state: Entity<InputState>,
            emitted: Arc<Mutex<EmittedIds>>,
        }

        impl Render for InputA11yProbe {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let state = self.state.clone();
                let emitted = self.emitted.clone();
                div().on_prepaint(move |_, window, cx| {
                    let mut author_id_of = |input: Input| {
                        let mut node = gpui::accesskit::Node::new(Role::TextInput);
                        input
                            .render(window, cx)
                            .into_element()
                            .write_a11y_info(&mut node);
                        node.author_id().map(ToOwned::to_owned)
                    };

                    *emitted.lock().unwrap() = vec![
                        author_id_of(Input::new(&state)),
                        author_id_of(Input::new(&state).accessibility_id("search.query")),
                    ];
                })
            }
        }

        cx.update(crate::init);
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let captured = emitted.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| InputA11yProbe {
            state: cx.new(|cx| InputState::new(window, cx)),
            emitted,
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        assert_eq!(
            *captured.lock().unwrap(),
            vec![None, Some("search.query".into())]
        );
    }

    #[test]
    fn accessibility_value_is_hidden_for_secret_inputs() {
        assert!(exposes_accessibility_value(false, None));
        assert!(!exposes_accessibility_value(true, None));
        assert!(!exposes_accessibility_value(
            false,
            Some(InputContentType::Password)
        ));
        assert!(!exposes_accessibility_value(
            false,
            Some(InputContentType::NewPassword)
        ));
    }

    #[gpui::test]
    fn focused_input_registry_tracks_focus_and_blur(cx: &mut gpui::TestAppContext) {
        use crate::{Root, WindowExt as _};
        use gpui::{AppContext as _, Render};

        struct Probe {
            input: Entity<InputState>,
            textarea: Entity<crate::input::TextareaState>,
            editor: Entity<crate::input::EditorState>,
            otp: Entity<gpui_base::OtpState>,
            other: gpui::FocusHandle,
        }
        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                div()
                    .child(div().track_focus(&self.other))
                    .child(Input::new(&self.input))
                    .child(crate::input::Textarea::new(&self.textarea))
                    .child(crate::input::Editor::new(&self.editor))
                    .child(crate::input::OtpInput::new(&self.otp))
            }
        }

        cx.update(crate::init);
        let mut input = None;
        let mut textarea = None;
        let mut editor = None;
        let mut other_focus = None;
        let mut otp = None;
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let state = cx.new(|cx| InputState::new(window, cx));
                let textarea_state = cx.new(|cx| crate::input::TextareaState::new(window, cx));
                let editor_state =
                    cx.new(|cx| crate::input::EditorState::new(window, cx).language("rust"));
                let otp_state = cx.new(|cx| gpui_base::OtpState::new(6, window, cx));
                input = Some(state.clone());
                textarea = Some(textarea_state.clone());
                editor = Some(editor_state.clone());
                otp = Some(otp_state.clone());
                let other = cx.focus_handle();
                other_focus = Some(other.clone());
                let probe = cx.new(|_| Probe {
                    input: state,
                    textarea: textarea_state,
                    editor: editor_state,
                    otp: otp_state,
                    other,
                });
                cx.new(|cx| Root::new(probe, window, cx))
            })
            .unwrap()
        });
        let input = input.unwrap();
        let textarea = textarea.unwrap();
        let editor = editor.unwrap();
        let otp = otp.unwrap();
        let other_focus = other_focus.unwrap();
        let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

        // Focusing each kind of input registers it, and blurring clears it.
        let cases: Vec<AnyInputState> = vec![
            input.clone().into(),
            textarea.clone().into(),
            editor.clone().into(),
            otp.clone().into(),
        ];
        for expected in cases {
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            cx.update(|window, cx| expected.focus_handle(cx).focus(window, cx));
            cx.run_until_parked();
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            assert_eq!(
                cx.update(|window, cx| window.focused_input(cx)),
                Some(expected)
            );

            cx.update(|window, cx| other_focus.clone().focus(window, cx));
            cx.run_until_parked();
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            assert_eq!(cx.update(|window, cx| window.focused_input(cx)), None);
        }
    }

    #[gpui::test]
    fn focused_input_registry_ignores_input_removed_while_focused(cx: &mut gpui::TestAppContext) {
        use crate::{Root, WindowExt as _};
        use gpui::{AppContext as _, Render};

        struct Probe {
            input: Entity<InputState>,
            show_input: bool,
            other: gpui::FocusHandle,
        }
        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                let base = div().child(div().track_focus(&self.other));
                if self.show_input {
                    base.child(Input::new(&self.input))
                } else {
                    base
                }
            }
        }

        cx.update(crate::init);
        let mut input = None;
        let mut other_focus = None;
        let mut probe_entity = None;
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let state = cx.new(|cx| InputState::new(window, cx));
                input = Some(state.clone());
                let other = cx.focus_handle();
                other_focus = Some(other.clone());
                let probe = cx.new(|_| Probe {
                    input: state,
                    show_input: true,
                    other,
                });
                probe_entity = Some(probe.clone());
                cx.new(|cx| Root::new(probe, window, cx))
            })
            .unwrap()
        });
        let input: AnyInputState = input.unwrap().into();
        let other_focus = other_focus.unwrap();
        let probe = probe_entity.unwrap();
        let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.update(|window, cx| input.focus_handle(cx).focus(window, cx));
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert_eq!(
            cx.update(|window, cx| window.focused_input(cx)),
            Some(input.clone())
        );

        // Remove the input from the tree while it holds focus and move focus
        // elsewhere in the same update (e.g. closing a sidebar containing the
        // input) — the input never re-renders to unregister itself.
        cx.update(|window, cx| {
            probe.update(cx, |probe, cx| {
                probe.show_input = false;
                cx.notify();
            });
            other_focus.focus(window, cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert!(!cx.update(|window, cx| window.has_focused_input(cx)));
        assert_eq!(cx.update(|window, cx| window.focused_input(cx)), None);
    }
}

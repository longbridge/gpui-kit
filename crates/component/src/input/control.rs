//! Shared component integration for the three native text-editing states.
//!
//! This layer supplies theme, platform and accessibility adapters. It owns no
//! input frame layout or adornments, so standalone and grouped inputs can use
//! the same editor without wrapping one another's presentation.

use super::overlay::InputOverlays;
use super::state::{TextInputState, sync_focused_input_registry};
use super::{InputContentType, sync_native_content_type};
use crate::button::{Button, ButtonRounded, ButtonVariants as _};
use crate::native_menu::NativeMenu;
use crate::touch_selection::{EditMenuItem, TouchSelectionOverlay};
use crate::{ActiveTheme as _, IconName, RoleOverride, Selectable as _, Sizable as _};
use gpui::{
    AccessibleAction, AnyElement, App, Edges, ElementId, InteractiveElement as _, IntoElement as _,
    Pixels, Role, SharedString, StatefulInteractiveElement as _, Styled as _, TextAlign,
    TouchPhase, Window, prelude::FluentBuilder as _, px,
};
use gpui_base::InputBase;
use rust_i18n::t;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct InputControl {
    pub(crate) state: TextInputState,
    pub(crate) disabled: bool,
    pub(crate) readonly: bool,
    pub(crate) content_type: Option<InputContentType>,
    pub(crate) role: RoleOverride,
    pub(crate) accessibility_id: Option<SharedString>,
    pub(crate) aria_label: Option<SharedString>,
    pub(crate) context_menu_builder:
        Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
    pub(crate) paste_handler:
        Option<Rc<dyn Fn(&gpui::ClipboardItem, &mut Window, &mut App) -> bool>>,
}

impl InputControl {
    pub(crate) fn new(state: TextInputState) -> Self {
        Self {
            state,
            disabled: false,
            readonly: false,
            content_type: None,
            role: RoleOverride::default(),
            accessibility_id: None,
            aria_label: None,
            context_menu_builder: None,
            paste_handler: None,
        }
    }

    pub(crate) fn prepare(
        &self,
        paddings: Edges<Pixels>,
        text_align: TextAlign,
        window: &mut Window,
        cx: &mut App,
    ) -> InputOverlays {
        let state = &self.state;
        // Which kind of input this registers as follows from the state itself.
        sync_focused_input_registry(state, window, cx);

        state.ensure_highlighter_factory(crate::highlighter::input_highlighter_factory(), cx);
        state.set_editor_style(
            gpui_base::input::InputEditorStyle {
                foreground: cx.theme().foreground,
                muted_foreground: cx.theme().muted_foreground,
                background: cx.theme().editor_background(),
                border: cx.theme().border,
                selection: cx.theme().selection,
                caret: cx.theme().caret,
                diagnostics: gpui_base::input::DiagnosticColors {
                    error: cx.theme().highlight_theme.style.status.error(cx),
                    warning: cx.theme().highlight_theme.style.status.warning(cx),
                    info: cx.theme().highlight_theme.style.status.info(cx),
                    hint: cx.theme().highlight_theme.style.status.hint(cx),
                },
                highlight_styles: cx.theme().highlight_theme.clone(),
                editor_invisible: cx.theme().highlight_theme.style.editor_invisible,
                editor_active_line: cx.theme().highlight_theme.style.editor_active_line,
                editor_gutter_background: cx.theme().highlight_theme.style.editor_gutter_background,
                fold_icon_renderer: Some(Rc::new(|ix, is_folded| {
                    Button::new(("fold-icon", ix))
                        .ghost()
                        .icon(if is_folded {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        })
                        .xsmall()
                        .rounded(ButtonRounded::Small)
                        .size(px(14.))
                        .selected(is_folded)
                        .into_any_element()
                })),
            },
            cx,
        );
        state.set_editor_paddings(paddings, cx);
        state.set_disabled(self.disabled, cx);
        state.set_readonly(self.readonly, cx);
        state.set_text_align(text_align, cx);
        let custom = self.context_menu_builder.clone();
        state.on_context_menu(
            Rc::new(move |_, capabilities, position, window, cx| {
                let menu = if let Some(custom) = custom.as_ref() {
                    custom(NativeMenu::new(), window, cx)
                } else {
                    let enabled = !capabilities.is_disabled();
                    // A read-only input can still navigate the code, it only
                    // rejects the items that would change the text.
                    let editable = enabled && !capabilities.is_readonly();
                    let mut menu = NativeMenu::new();
                    if capabilities.is_code_editor() {
                        menu = menu
                            .menu_with_disabled(
                                t!("Input.Go to Definition"),
                                !(enabled && capabilities.has_definition()),
                                Box::new(gpui_base::input::GoToDefinition),
                            )
                            .menu_with_disabled(
                                t!("Input.Show Code Actions"),
                                !(editable && capabilities.has_code_actions()),
                                Box::new(gpui_base::input::ToggleCodeActions),
                            )
                            .separator();
                    }
                    menu.menu_with_disabled(
                        t!("Input.Cut"),
                        !(editable && capabilities.is_copyable()),
                        Box::new(gpui_base::input::Cut),
                    )
                    .menu_with_disabled(
                        t!("Input.Copy"),
                        !capabilities.is_copyable(),
                        Box::new(gpui_base::input::Copy),
                    )
                    .menu_with_disabled(
                        t!("Input.Paste"),
                        !(editable && cx.read_from_clipboard().is_some()),
                        Box::new(gpui_base::input::Paste),
                    )
                    .separator()
                    .menu(
                        t!("Input.Select All"),
                        Box::new(gpui_base::input::SelectAll),
                    )
                };
                menu.show(position, window, cx);
            }),
            cx,
        );
        let mut overlays = state.render_overlays(window, cx);
        overlays
            .floating
            .extend(Self::render_touch_selection(state, window, cx));
        overlays
    }

    /// The handles and the edit menu of the selection a long press made.
    ///
    /// The menu offers what the native context menu would: Cut, Copy, Paste
    /// and Select All, leaving out what cannot apply right now rather than
    /// disabling it. Cut, Copy and Paste go through the input's actions, so a
    /// custom key binding or an open completion menu sees them the same way.
    fn render_touch_selection(
        state: &TextInputState,
        window: &Window,
        cx: &App,
    ) -> Vec<AnyElement> {
        if state.touch_selection(cx).is_none() {
            return Vec::new();
        }
        let capabilities = state.context_menu_capabilities(cx);
        let editable = capabilities.is_editable();
        let copyable = capabilities.is_copyable();
        // Offered whenever the text can change, without peeking at the
        // clipboard: on iOS every read of it shows the system's paste banner,
        // and an empty clipboard pastes nothing.
        let pasteable = editable;
        let selectable = state.text(cx).len() > 0 && !state.is_all_selected(cx);
        let focus_handle = state.presentation(cx).focus_handle().clone();

        let dispatch = {
            let focus_handle = focus_handle.clone();
            move |action: &dyn gpui::Action, window: &mut Window, cx: &mut App| {
                focus_handle.dispatch_action(action, window, cx);
            }
        };
        let mut items = Vec::with_capacity(4);
        if editable && copyable {
            let dispatch = dispatch.clone();
            items.push(EditMenuItem::new(t!("Input.Cut"), move |window, cx| {
                dispatch(&gpui_base::input::Cut, window, cx);
            }));
        }
        if copyable {
            let dispatch = dispatch.clone();
            let state = state.clone();
            items.push(EditMenuItem::new(t!("Input.Copy"), move |window, cx| {
                dispatch(&gpui_base::input::Copy, window, cx);
                state.close_edit_menu(cx);
            }));
        }
        if pasteable {
            let dispatch = dispatch.clone();
            items.push(EditMenuItem::new(t!("Input.Paste"), move |window, cx| {
                dispatch(&gpui_base::input::Paste, window, cx);
            }));
        }
        if selectable {
            let state = state.clone();
            items.push(EditMenuItem::new(
                t!("Input.Select All"),
                move |window, cx| state.select_all_from_edit_menu(window, cx),
            ));
        }

        let drag_state = state.clone();
        let source_state = state.clone();
        TouchSelectionOverlay::new(
            ("input-touch-selection", state.entity_id()),
            move |_, cx| source_state.touch_selection(cx),
        )
        .handles(move |edge, phase, position, _, cx| match phase {
            TouchPhase::Started => drag_state.begin_edge_drag(edge, position, cx),
            TouchPhase::Moved => drag_state.update_edge_drag(position, cx),
            TouchPhase::Ended | TouchPhase::Cancelled => drag_state.end_edge_drag(cx),
        })
        .items(items)
        .into_elements(window, cx)
    }

    pub(crate) fn frame(
        &self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> InputBase {
        let state = &self.state;
        let presentation = state.presentation(cx);
        let content_type = self.content_type;
        let disabled = self.disabled;
        let is_multi_line = presentation.is_multi_line();
        let accessibility_role = accessibility_role(is_multi_line, content_type, self.role);
        let accessibility_state = state.clone();
        // Tests read the same accessibility value as assistive technology.
        // Avoid materializing the rope in normal builds without a client.
        let accessibility_value = ((window.is_a11y_active() || cfg!(feature = "test-support"))
            && exposes_accessibility_value(presentation.is_masked(), content_type))
        .then(|| state.text(cx).to_string());
        let input_focused =
            presentation.focus_handle().is_focused(window) && !presentation.is_disabled();
        if input_focused {
            sync_native_content_type(window, content_type, presentation.is_editable());
        }
        let placeholder = Some(presentation.placeholder().clone()).filter(|p| !p.is_empty());

        // Don't use a mask-derived placeholder ("(___)___-___") as an aria_label fallback.
        let placeholder_is_mask = presentation.mask_placeholder() == placeholder.as_deref();

        let aria_label = match self.aria_label.clone() {
            Some(label) => Some(label),
            None if placeholder_is_mask => None,
            None => placeholder.clone(),
        };
        // Match the editing engine: disabled and read-only controls never invoke the hook.
        let paste_handler = self
            .paste_handler
            .clone()
            .filter(|_| presentation.is_editable());
        InputBase::new(id)
            .disabled(disabled)
            .role(accessibility_role)
            .when_some(self.accessibility_id.clone(), |this, id| {
                this.accessibility_id(id)
            })
            .when_some(aria_label, |this, label| this.aria_label(label))
            .when_some(placeholder, |this, placeholder| {
                this.aria_placeholder(placeholder)
            })
            .when_some(accessibility_value, |this, value| this.aria_value(value))
            .when(!disabled, |this| {
                this.on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
                    handle_accessibility_set_value(&accessibility_state, data, window, cx);
                })
            })
            .when_some(paste_handler, |this, handler| {
                this.capture_action(move |_: &gpui_base::input::Paste, window, cx| {
                    if let Some(clipboard) = cx.read_from_clipboard() {
                        if handler(&clipboard, window, cx) {
                            cx.stop_propagation();
                        }
                    }
                })
            })
    }
}

pub(super) fn accessibility_role(
    is_multi_line: bool,
    content_type: Option<InputContentType>,
    role: RoleOverride,
) -> Option<Role> {
    role.resolve(|| {
        if is_multi_line {
            return Role::MultilineTextInput;
        }

        match content_type {
            None => Role::TextInput,
            Some(InputContentType::TelephoneNumber) => Role::PhoneNumberInput,
            Some(InputContentType::EmailAddress) => Role::EmailInput,
            Some(InputContentType::Url) => Role::UrlInput,
            Some(InputContentType::Password | InputContentType::NewPassword) => Role::PasswordInput,
            Some(InputContentType::DateTime) => Role::DateTimeInput,
            Some(InputContentType::Birthdate) => Role::DateInput,
            Some(
                InputContentType::Name
                | InputContentType::NamePrefix
                | InputContentType::GivenName
                | InputContentType::MiddleName
                | InputContentType::FamilyName
                | InputContentType::NameSuffix
                | InputContentType::Nickname
                | InputContentType::JobTitle
                | InputContentType::OrganizationName
                | InputContentType::Location
                | InputContentType::FullStreetAddress
                | InputContentType::StreetAddressLine1
                | InputContentType::StreetAddressLine2
                | InputContentType::AddressCity
                | InputContentType::AddressState
                | InputContentType::AddressCityAndState
                | InputContentType::Sublocality
                | InputContentType::CountryName
                | InputContentType::PostalCode
                | InputContentType::CreditCardNumber
                | InputContentType::CreditCardName
                | InputContentType::CreditCardGivenName
                | InputContentType::CreditCardMiddleName
                | InputContentType::CreditCardFamilyName
                | InputContentType::CreditCardSecurityCode
                | InputContentType::CreditCardExpiration
                | InputContentType::CreditCardExpirationMonth
                | InputContentType::CreditCardExpirationYear
                | InputContentType::CreditCardType
                | InputContentType::Username
                | InputContentType::OneTimeCode
                | InputContentType::ShipmentTrackingNumber
                | InputContentType::FlightNumber
                | InputContentType::BirthdateDay
                | InputContentType::BirthdateMonth
                | InputContentType::BirthdateYear
                | InputContentType::CellularEid
                | InputContentType::CellularImei,
            ) => Role::TextInput,
        }
    })
}

pub(super) fn exposes_accessibility_value(
    masked: bool,
    content_type: Option<InputContentType>,
) -> bool {
    !masked
        && !matches!(
            content_type,
            Some(InputContentType::Password | InputContentType::NewPassword)
        )
}

pub(super) fn handle_accessibility_set_value(
    state: &TextInputState,
    data: Option<&gpui::accesskit::ActionData>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(gpui::accesskit::ActionData::Value(value)) = data else {
        return;
    };
    state.replace_all(value.to_string(), window, cx);
}

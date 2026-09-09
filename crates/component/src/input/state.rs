use gpui::{App, Entity, FocusHandle, Focusable as _, SharedString, Window};
use gpui_base::OtpState;
use ropey::Rope;

use super::{EditorState, InputState, TextareaState};
use crate::Root;

/// Any input-like state, regardless of which input element renders it.
///
/// [`InputState`], [`TextareaState`], [`EditorState`] and [`OtpState`] are
/// separate types, so an API that refers to “whatever input is here” takes this
/// enum instead of one of them. [`crate::WindowExt::focused_input`] returns it.
///
/// Use [`Self::as_input`] and friends to get the concrete state back, or
/// [`Self::value`] and [`Self::focus_handle`] when the kind does not matter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyInputState {
    /// A single-line [`crate::input::Input`] state.
    Input(Entity<InputState>),
    /// A multi-line [`crate::input::Textarea`] state.
    Textarea(Entity<TextareaState>),
    /// A source-code [`crate::input::Editor`] state.
    Editor(Entity<EditorState>),
    /// A one-time-code [`crate::input::OtpInput`] state.
    Otp(Entity<OtpState>),
}

/// The three states the [`crate::input::Input`] element can render.
///
/// [`AnyInputState`] also covers `OtpState`, which is a different engine with
/// none of these methods, so the element takes this narrower enum instead.
///
/// Everything here forwards to a method the shared engine offers for every
/// mode, which is why the element does not need to be generic over the mode to
/// call them.
#[derive(Debug, Clone)]
pub(crate) enum TextInputState {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
    Editor(Entity<EditorState>),
}

/// Runs `$body` against whichever concrete state this is.
macro_rules! dispatch {
    ($self:expr, |$state:ident| $body:expr) => {
        match $self {
            TextInputState::Input($state) => $body,
            TextInputState::Textarea($state) => $body,
            TextInputState::Editor($state) => $body,
        }
    };
}

impl TextInputState {
    pub(crate) fn entity_id(&self) -> gpui::EntityId {
        dispatch!(self, |state| state.entity_id())
    }

    pub(crate) fn presentation(&self, cx: &App) -> gpui_base::input::InputPresentation {
        dispatch!(self, |state| state.read(cx).presentation())
    }

    /// The text of the input, borrowed from the state that owns it.
    ///
    /// The state holds the only copy; nothing on the render path snapshots it.
    pub(crate) fn text<'a>(&self, cx: &'a App) -> &'a Rope {
        dispatch!(self, |state| state.read(cx).text())
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, cx| state.focus(window, cx)))
    }

    pub(crate) fn clean(&self, window: &mut Window, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, cx| state.clean(window, cx)))
    }

    pub(crate) fn toggle_masked(&self, window: &mut Window, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, cx| state.toggle_masked(window, cx)))
    }

    pub(crate) fn set_text_align(&self, align: gpui::TextAlign, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, cx| state.set_text_align(align, cx)))
    }

    pub(crate) fn set_disabled(&self, disabled: bool, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, cx| state.set_disabled(disabled, cx)))
    }

    pub(crate) fn set_readonly(&self, readonly: bool, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, cx| state.set_readonly(readonly, cx)))
    }

    pub(crate) fn set_editor_style(&self, style: gpui_base::input::InputEditorStyle, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, _| state.set_editor_style(style)))
    }

    pub(crate) fn set_editor_paddings(&self, paddings: gpui::Edges<gpui::Pixels>, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, _| state.set_editor_paddings(paddings)))
    }

    pub(crate) fn ensure_highlighter_factory(
        &self,
        factory: gpui_base::input::InputHighlighterFactory,
        cx: &mut App,
    ) {
        dispatch!(self, |state| state
            .update(cx, |state, _| state.ensure_highlighter_factory(factory)))
    }

    pub(crate) fn on_context_menu(
        &self,
        handler: std::rc::Rc<
            dyn Fn(
                gpui_base::input::NativeMenu,
                gpui_base::input::InputContextMenuCapabilities,
                gpui::Point<gpui::Pixels>,
                &mut Window,
                &mut App,
            ),
        >,
        cx: &mut App,
    ) {
        dispatch!(self, |state| state
            .update(cx, |state, _| state.on_context_menu(handler)))
    }

    /// Builds and syncs this input's overlays. See [`super::overlay`].
    pub(super) fn render_overlays(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> super::overlay::InputOverlays {
        dispatch!(self, |state| super::overlay::render_overlays(
            state, window, cx
        ))
    }

    pub(crate) fn replace_all(&self, value: String, window: &mut Window, cx: &mut App) {
        dispatch!(self, |state| state
            .update(cx, |state, cx| state.replace_all(value, window, cx)))
    }

    /// The text element itself, as a child to place in the frame.
    pub(crate) fn into_any_element(self) -> gpui::AnyElement {
        use gpui::IntoElement as _;
        dispatch!(self, |state| state.into_any_element())
    }
}

impl From<&TextInputState> for AnyInputState {
    fn from(state: &TextInputState) -> Self {
        match state {
            TextInputState::Input(state) => Self::Input(state.clone()),
            TextInputState::Textarea(state) => Self::Textarea(state.clone()),
            TextInputState::Editor(state) => Self::Editor(state.clone()),
        }
    }
}

impl From<Entity<InputState>> for TextInputState {
    fn from(state: Entity<InputState>) -> Self {
        Self::Input(state)
    }
}

impl From<Entity<TextareaState>> for TextInputState {
    fn from(state: Entity<TextareaState>) -> Self {
        Self::Textarea(state)
    }
}

impl From<Entity<EditorState>> for TextInputState {
    fn from(state: Entity<EditorState>) -> Self {
        Self::Editor(state)
    }
}

impl AnyInputState {
    /// Returns the [`InputState`], if this is an `Input` state.
    pub fn as_input(&self) -> Option<&Entity<InputState>> {
        match self {
            Self::Input(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the [`TextareaState`], if this is a `Textarea` state.
    pub fn as_textarea(&self) -> Option<&Entity<TextareaState>> {
        match self {
            Self::Textarea(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the [`EditorState`], if this is an `Editor` state.
    pub fn as_editor(&self) -> Option<&Entity<EditorState>> {
        match self {
            Self::Editor(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the [`OtpState`], if this is an `OtpInput` state.
    pub fn as_otp(&self) -> Option<&Entity<OtpState>> {
        match self {
            Self::Otp(state) => Some(state),
            _ => None,
        }
    }

    /// Returns the value of the input.
    ///
    /// A masked input returns its masked value, the same as what is rendered.
    pub fn value(&self, cx: &App) -> SharedString {
        match self {
            Self::Input(state) => state.read(cx).value(),
            Self::Textarea(state) => state.read(cx).value(),
            Self::Editor(state) => state.read(cx).value(),
            Self::Otp(state) => state.read(cx).value().clone(),
        }
    }

    /// Returns the focus handle of the input.
    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self {
            Self::Input(state) => state.focus_handle(cx),
            Self::Textarea(state) => state.focus_handle(cx),
            Self::Editor(state) => state.focus_handle(cx),
            Self::Otp(state) => state.focus_handle(cx),
        }
    }
}

impl From<Entity<InputState>> for AnyInputState {
    fn from(state: Entity<InputState>) -> Self {
        Self::Input(state)
    }
}

impl From<Entity<TextareaState>> for AnyInputState {
    fn from(state: Entity<TextareaState>) -> Self {
        Self::Textarea(state)
    }
}

impl From<Entity<EditorState>> for AnyInputState {
    fn from(state: Entity<EditorState>) -> Self {
        Self::Editor(state)
    }
}

impl From<Entity<OtpState>> for AnyInputState {
    fn from(state: Entity<OtpState>) -> Self {
        Self::Otp(state)
    }
}

/// Registers `state` as the window's focused input while it holds focus, and
/// unregisters it once focus moves elsewhere.
pub(super) fn sync_focused_input_registry(
    state: impl Into<AnyInputState>,
    window: &mut Window,
    cx: &mut App,
) {
    let state = state.into();
    let focused = state.focus_handle(cx).is_focused(window);
    Root::try_update(window, cx, |root, _, cx| {
        if focused {
            root.focused_input = Some(state.clone());
        } else if root.focused_input.as_ref() == Some(&state) {
            root.focused_input = None;
        }
        cx.notify();
    });
}

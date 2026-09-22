//! Presentation callbacks and measured geometry. None of this enters document history.
use super::{InlineToken, InlineTokenSpan, InputBaseState, InputModeKind};
use gpui::{AnyElement, App, Bounds, ClickEvent, Font, IntoElement, Pixels, Window};
use std::{collections::HashMap, ops::Range, rc::Rc};

/// Read-only context for a single inline renderer. Width is the full available row.
#[derive(Clone)]
pub struct InlineTokenContext {
    span: InlineTokenSpan,
    selected: bool,
    disabled: bool,
    readonly: bool,
    line_height: Pixels,
    available_width: Pixels,
}
impl InlineTokenContext {
    pub fn token(&self) -> &InlineToken {
        self.span.token()
    }
    pub fn range(&self) -> Range<usize> {
        self.span.range()
    }
    pub fn is_selected(&self) -> bool {
        self.selected
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }
    pub fn line_height(&self) -> Pixels {
        self.line_height
    }
    pub fn available_width(&self) -> Pixels {
        self.available_width
    }
}

/// Current token snapshot delivered after releasing the editor's update borrow.
#[derive(Clone)]
pub struct InlineTokenClickEvent {
    span: InlineTokenSpan,
    bounds: Bounds<Pixels>,
    event: ClickEvent,
}
impl InlineTokenClickEvent {
    pub fn token(&self) -> &InlineToken {
        self.span.token()
    }
    pub fn range(&self) -> Range<usize> {
        self.span.range()
    }
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
    /// The click that opened the token; keyboard activation reports a
    /// keyboard click.
    pub fn click(&self) -> &ClickEvent {
        &self.event
    }
}

/// A renderer installed by a styled control. Not part of the supported API.
#[doc(hidden)]
pub type InlineTokenRenderer = Rc<dyn Fn(&InlineTokenContext, &mut Window, &mut App) -> AnyElement>;
/// A click listener installed by a styled control. Not part of the supported API.
#[doc(hidden)]
pub type InlineTokenClickListener = Rc<dyn Fn(&InlineTokenClickEvent, &mut Window, &mut App)>;

/// Presentation shared by Base and styled controls. It owns no content.
#[derive(Clone, Default)]
pub(crate) struct InlineTokenPresentation {
    renderer: Option<InlineTokenRenderer>,
    listener: Option<InlineTokenClickListener>,
    secret: bool,
}
impl InlineTokenPresentation {
    pub(crate) fn token<R: IntoElement>(
        mut self,
        render: impl Fn(&InlineTokenContext, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.renderer = Some(Rc::new(move |token, window, cx| {
            render(token, window, cx).into_any_element()
        }));
        self
    }
    pub(crate) fn on_token_click(
        mut self,
        listener: impl Fn(&InlineTokenClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.listener = Some(Rc::new(listener));
        self
    }
    pub(super) fn has_listener(&self) -> bool {
        self.listener.is_some()
    }
    pub(super) fn render(
        &self,
        token: &InlineTokenContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        if let Some(render) = &self.renderer {
            render(token, window, cx)
        } else {
            gpui::div()
                .child(token.token().label().clone())
                .into_any_element()
        }
    }
}
use gpui::ParentElement as _;

#[derive(Default)]
pub(super) struct TokenLayoutCache {
    pub(super) key: Option<(Font, Pixels, Pixels, Pixels, bool)>,
    pub(super) revision: u64,
    pub(super) unwrapped_width: Pixels,
    pub(super) metrics: Rc<[(Range<usize>, Pixels)]>,
    pub(super) widths: HashMap<InlineToken, Pixels>,
}

impl<M: InputModeKind> InputBaseState<M> {
    /// Inject presentation from a view without editing or notifying the document.
    pub(crate) fn set_token_presentation(&mut self, presentation: InlineTokenPresentation) {
        self.token_presentation = presentation;
    }
    /// Install a styled control's renderer, click listener and secrecy without
    /// editing or notifying the document. Not part of the supported API.
    #[doc(hidden)]
    pub fn install_token_presentation(
        &mut self,
        renderer: Option<InlineTokenRenderer>,
        listener: Option<InlineTokenClickListener>,
        secret: bool,
    ) {
        self.token_presentation = InlineTokenPresentation {
            renderer,
            listener,
            secret,
        };
    }
    pub(super) fn tokens_visible(&self) -> bool {
        !self.masked
            && !self.token_presentation.secret
            && self.mask_pattern.is_none()
            && !self.token_spans().is_empty()
    }
    pub(super) fn token_context(
        &self,
        span: &InlineTokenSpan,
        line_height: Pixels,
        width: Pixels,
    ) -> InlineTokenContext {
        let range = span.range();
        let selection = self.selected_range();
        InlineTokenContext {
            span: span.clone(),
            selected: selection.start < range.end && range.start < selection.end,
            disabled: self.disabled,
            readonly: self.readonly,
            line_height,
            available_width: width,
        }
    }
    /// The token starting at `start`, paired with the listener that opens it.
    pub(super) fn token_activation(
        &self,
        start: usize,
        bounds: Bounds<Pixels>,
        event: ClickEvent,
    ) -> Option<(InlineTokenClickListener, InlineTokenClickEvent)> {
        if self.disabled || !self.tokens_visible() {
            return None;
        }
        let span = self
            .token_spans()
            .iter()
            .find(|span| span.range().start == start)?
            .clone();
        Some((
            self.token_presentation.listener.clone()?,
            InlineTokenClickEvent {
                span,
                bounds,
                event,
            },
        ))
    }
    pub(super) fn token_is_secret(&self) -> bool {
        self.token_presentation.secret
    }
}

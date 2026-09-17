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
    pub fn event(&self) -> &ClickEvent {
        &self.event
    }
}

type Renderer = Rc<dyn Fn(&InlineTokenContext, &mut Window, &mut App) -> AnyElement>;
type Listener = Rc<dyn Fn(&InlineTokenClickEvent, &mut Window, &mut App)>;
type Fallback = fn(&InlineTokenContext, &mut Window, &mut App) -> AnyElement;

/// Presentation adapter shared by Base and styled controls. It owns no content.
#[derive(Clone, Default)]
pub struct InlineTokenPresentation {
    renderer: Option<Renderer>,
    listener: Option<Listener>,
    fallback: Option<Fallback>,
    secret: bool,
}
impl InlineTokenPresentation {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn render_token<R: IntoElement>(
        mut self,
        render: impl Fn(&InlineTokenContext, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        self.renderer = Some(Rc::new(move |token, window, cx| {
            render(token, window, cx).into_any_element()
        }));
        self
    }
    pub fn on_token_click(
        mut self,
        listener: impl Fn(&InlineTokenClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.listener = Some(Rc::new(listener));
        self
    }
    /// Set a default skin without allocating a callback on every plain-text render.
    pub fn with_fallback(mut self, fallback: Fallback) -> Self {
        self.fallback = Some(fallback);
        self
    }
    /// Suppress token labels and actions when the host is a secret field.
    pub fn secret(mut self, secret: bool) -> Self {
        self.secret = secret;
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
        } else if let Some(render) = self.fallback {
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
    pub(super) widths: HashMap<gpui::SharedString, (InlineToken, Pixels)>,
}

impl<M: InputModeKind> InputBaseState<M> {
    /// Inject presentation from a view without editing or notifying the document.
    #[doc(hidden)]
    pub fn set_token_presentation(&mut self, presentation: InlineTokenPresentation) {
        self.token_presentation = presentation;
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
    pub(super) fn token_activation(
        &self,
        id: &str,
        bounds: Bounds<Pixels>,
        event: ClickEvent,
    ) -> Option<(Listener, InlineTokenClickEvent)> {
        if self.disabled || !self.tokens_visible() {
            return None;
        }
        let span = self
            .token_spans()
            .iter()
            .find(|span| span.token().id().as_ref() == id)?
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

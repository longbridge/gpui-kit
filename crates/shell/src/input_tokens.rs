//! Shared token data conversion for built-in and registered input state bindings.
use crate::{ComponentDataValue as Data, StateMethodDescriptor};
use anyhow::{Result, anyhow, bail};
use gpui::{App, Entity, Window};
use gpui_base::input::{
    InlineToken, InlineTokenClickEvent, InlineTokenContext, InlineTokenError, InputContent,
    InputState, Rope, RopeExt as _, TextareaState,
};
use std::rc::Rc;

fn object(fields: impl IntoIterator<Item = (&'static str, Data)>) -> Data {
    Data::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
fn field<'a>(value: &'a Data, name: &str) -> Result<&'a Data> {
    let Data::Object(fields) = value else {
        bail!("expected a plain object");
    };
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, v)| v)
        .ok_or_else(|| anyhow!("missing `{name}`"))
}
fn string(value: &Data) -> Result<&str> {
    if let Data::String(value) = value {
        Ok(value)
    } else {
        bail!("expected a string")
    }
}
fn token(value: &Data) -> Result<InlineToken> {
    let result = InlineToken::new(
        string(field(value, "id")?)?.to_owned(),
        string(field(value, "text")?)?.to_owned(),
    );
    Ok(if let Ok(label) = field(value, "label") {
        result.with_label(string(label)?.to_owned())
    } else {
        result
    })
}
fn byte_offset(text: &str, value: &Data) -> Result<usize> {
    let Data::Number(offset) = value else {
        return Err(InlineTokenError::InvalidRange.into());
    };
    if !offset.is_finite() || *offset < 0. || offset.fract() != 0. || *offset > text.len() as f64 {
        return Err(InlineTokenError::InvalidRange.into());
    }
    let target = *offset as usize;
    let mut utf16 = 0;
    for (ix, ch) in text.char_indices() {
        if utf16 == target {
            return Ok(ix);
        }
        utf16 += ch.len_utf16();
        if utf16 > target {
            return Err(InlineTokenError::InvalidBoundary.into());
        }
    }
    if utf16 == target {
        Ok(text.len())
    } else {
        Err(InlineTokenError::InvalidRange.into())
    }
}
fn range(text: &str, value: &Data) -> Result<std::ops::Range<usize>> {
    let range =
        byte_offset(text, field(value, "start")?)?..byte_offset(text, field(value, "end")?)?;
    if range.start > range.end {
        return Err(InlineTokenError::InvalidRange.into());
    }
    Ok(range)
}
fn decode_content(value: &Data) -> Result<InputContent> {
    let text = string(field(value, "text")?)?;
    let Data::Array(tokens) = field(value, "tokens")? else {
        bail!("content.tokens must be an array");
    };
    let mut content = InputContent::new(text.to_owned());
    for span in tokens {
        content = content.with_token(
            range(text, field(span, "range")?)?,
            token(field(span, "token")?)?,
        )?;
    }
    Ok(content)
}
fn token_data(token: &InlineToken) -> Data {
    object([
        ("id", Data::String(token.id().to_string())),
        ("text", Data::String(token.text().to_string())),
        ("label", Data::String(token.label().to_string())),
    ])
}
fn range_data(text: &str, range: std::ops::Range<usize>) -> Data {
    object([
        (
            "start",
            Data::Number(text[..range.start].encode_utf16().count() as f64),
        ),
        (
            "end",
            Data::Number(text[..range.end].encode_utf16().count() as f64),
        ),
    ])
}
fn content_data(content: &InputContent) -> Data {
    object([
        ("text", Data::String(content.text().to_string())),
        (
            "tokens",
            Data::Array(
                content
                    .tokens()
                    .iter()
                    .map(|span| {
                        object([
                            ("range", range_data(content.text(), span.range())),
                            ("token", token_data(span.token())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn rope_range_data(text: &Rope, range: std::ops::Range<usize>) -> Data {
    object([
        (
            "start",
            Data::Number(text.offset_to_offset_utf16(range.start) as f64),
        ),
        (
            "end",
            Data::Number(text.offset_to_offset_utf16(range.end) as f64),
        ),
    ])
}

/// Plain JS renderer context. Range coordinates follow JavaScript strings.
pub fn inline_token_context_data(token: &InlineTokenContext, text: &Rope) -> Data {
    object([
        ("token", token_data(token.token())),
        ("range", rope_range_data(text, token.range())),
        ("selected", Data::Boolean(token.is_selected())),
        ("disabled", Data::Boolean(token.is_disabled())),
        ("readonly", Data::Boolean(token.is_readonly())),
        (
            "line_height",
            Data::Number(f32::from(token.line_height()) as f64),
        ),
        (
            "available_width",
            Data::Number(f32::from(token.available_width()) as f64),
        ),
    ])
}
/// Plain JS activation event, with current token identity and pointer modifiers.
pub fn inline_token_click_data(event: &InlineTokenClickEvent, text: &Rope) -> Data {
    let modifiers = event.click().modifiers();
    let bounds = event.bounds();
    object([
        ("token", token_data(event.token())),
        ("range", rope_range_data(text, event.range())),
        (
            "bounds",
            object([
                ("x", Data::Number(f32::from(bounds.origin.x) as f64)),
                ("y", Data::Number(f32::from(bounds.origin.y) as f64)),
                ("width", Data::Number(f32::from(bounds.size.width) as f64)),
                ("height", Data::Number(f32::from(bounds.size.height) as f64)),
            ]),
        ),
        (
            "modifiers",
            object([
                ("shift", Data::Boolean(modifiers.shift)),
                ("alt", Data::Boolean(modifiers.alt)),
                ("control", Data::Boolean(modifiers.control)),
                ("platform", Data::Boolean(modifiers.platform)),
            ]),
        ),
    ])
}

pub(crate) const METHODS: &[(&str, &str, bool)] = &[
    ("value", "(): string", true),
    ("set_value", "(value: string | InputContent): void", false),
    ("content", "(): InputContent", true),
    ("tokens", "(): InlineTokenSpan[]", true),
    ("replace_with_token", "(token: InlineToken): void", false),
    (
        "replace_range_with_token",
        "(range: InputRange, token: InlineToken): void",
        false,
    ),
    ("set_selected_range", "(range: InputRange): void", false),
    ("replace", "(text: string): void", false),
];

macro_rules! state_binding {
    ($state:ty, $invoke:ident, $methods:ident) => {
        pub(crate) fn $invoke(
            entity: &Entity<$state>,
            name: &str,
            args: &[Data],
            window: &mut Window,
            cx: &mut App,
        ) -> Result<Data> {
            match (name, args) {
                ("value", []) => Ok(Data::String(entity.read(cx).value().to_string())),
                ("content", []) => Ok(content_data(&entity.read(cx).content())),
                ("tokens", []) => {
                    Ok(field(&content_data(&entity.read(cx).content()), "tokens")?.clone())
                }
                ("set_value", [value]) => {
                    let content = match value {
                        Data::String(text) => InputContent::new(text.to_owned()),
                        value => decode_content(value)?,
                    };
                    entity.update(cx, |state, cx| state.set_value(content, window, cx));
                    Ok(Data::Null)
                }
                ("replace_with_token", [value]) => {
                    let token = token(value)?;
                    entity.update(cx, |state, cx| state.replace_with_token(token, window, cx))?;
                    Ok(Data::Null)
                }
                ("replace_range_with_token", [value, new_token]) => {
                    let range = range(&entity.read(cx).value(), value)?;
                    let token = token(new_token)?;
                    entity.update(cx, |state, cx| {
                        state.replace_range_with_token(range, token, window, cx)
                    })?;
                    Ok(Data::Null)
                }
                ("set_selected_range", [value]) => {
                    let range = range(&entity.read(cx).value(), value)?;
                    entity.update(cx, |state, cx| state.set_selected_range(range, cx));
                    Ok(Data::Null)
                }
                ("replace", [text]) => {
                    let text = string(text)?;
                    entity.update(cx, |state, cx| state.replace(text, window, cx));
                    Ok(Data::Null)
                }
                _ => bail!("invalid arguments for input state operation `{name}`"),
            }
        }
        /// Retained input operations for component adapters. All ranges use UTF-16.
        pub fn $methods() -> Vec<StateMethodDescriptor> {
            METHODS
                .iter()
                .map(|&(name, signature, readonly)| {
                    StateMethodDescriptor::new::<Entity<$state>>(
                        name,
                        signature,
                        move |entity, args, window, cx| $invoke(entity, name, args, window, cx),
                    )
                    .with_readonly(readonly)
                })
                .collect()
        }
    };
}
state_binding!(InputState, invoke_input, input_token_state_methods);
state_binding!(TextareaState, invoke_textarea, textarea_token_state_methods);

/// Script callbacks for one input's tokens, adapted to the native
/// `token` / `on_token_click` builders of any Input or Textarea element.
pub struct InlineTokenCallbacks {
    renderer: Option<gpui_base::input::InlineTokenRenderer>,
    listener: Option<gpui_base::input::InlineTokenClickListener>,
}
impl InlineTokenCallbacks {
    /// Bind generation-scoped script callbacks to the text of `state`, whose
    /// UTF-16 offsets the payloads carry.
    pub fn new<M: gpui_base::input::InputModeKind>(
        state: &Entity<gpui_base::input::InputBaseState<M>>,
        renderer: Option<crate::ComponentElementCallback>,
        listener: Option<crate::ComponentCallback>,
    ) -> Self {
        use gpui::{IntoElement as _, ParentElement as _};
        let renderer = renderer.map(|renderer| {
            let state = state.clone();
            let render: gpui_base::input::InlineTokenRenderer =
                Rc::new(move |token, window, cx| {
                    let data = inline_token_context_data(token, state.read(cx).text());
                    match renderer.build_interactive_data_with(&[data], window, cx) {
                        Ok(Some(element)) => element,
                        result => {
                            if let Err(error) = result {
                                tracing::error!("inline token renderer failed: {error:#}");
                            }
                            gpui::div()
                                .child(token.token().label().clone())
                                .into_any_element()
                        }
                    }
                });
            render
        });
        let listener = listener.map(|listener| {
            let state = state.clone();
            let listen: gpui_base::input::InlineTokenClickListener =
                Rc::new(move |event, window, cx| {
                    let data = inline_token_click_data(event, state.read(cx).text());
                    if let Err(error) = listener.invoke_data_with(&[data], window, cx) {
                        tracing::error!("inline token activation failed: {error:#}");
                    }
                });
            listen
        });
        Self { renderer, listener }
    }
    /// Install the callbacks on an element through its own builders.
    pub fn apply<E>(
        &self,
        element: E,
        token: impl FnOnce(E, gpui_base::input::InlineTokenRenderer) -> E,
        on_token_click: impl FnOnce(E, gpui_base::input::InlineTokenClickListener) -> E,
    ) -> E {
        let element = match &self.renderer {
            Some(renderer) => token(element, renderer.clone()),
            None => element,
        };
        match &self.listener {
            Some(listener) => on_token_click(element, listener.clone()),
            None => element,
        }
    }
}

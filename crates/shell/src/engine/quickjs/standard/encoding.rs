//! `TextEncoder` and `TextDecoder`, the WHATWG Encoding globals.
//!
//! UTF-8 only. That is the one encoding `TextEncoder` has ever had, and the one
//! a `TextDecoder` is asked for in practice; a label for anything else is a
//! `RangeError` naming the label rather than a decoder that quietly reads the
//! bytes as something they are not.

use crate::engine::quickjs::js_error;
use std::cell::{Cell, RefCell};

use rquickjs::{
    ArrayBuffer, Class, Ctx, FromJs, Function, Object, Result, TypedArray, Value,
    class::Trace,
    convert::Coerced,
    function::{Opt, This},
};

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    Class::<TextEncoder>::define(&globals)?;
    Class::<TextDecoder>::define(&globals)?;
    Ok(())
}

#[derive(Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub(super) struct TextEncoder {}

#[rquickjs::methods(rename_all = "camelCase")]
impl TextEncoder {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {}
    }

    #[qjs(get)]
    fn encoding(&self) -> &'static str {
        "utf-8"
    }

    fn encode<'js>(&self, ctx: Ctx<'js>, input: Opt<Value<'js>>) -> Result<TypedArray<'js, u8>> {
        let text = match input.0 {
            Some(value) if !value.is_undefined() => usv_string(&ctx, value)?,
            _ => String::new(),
        };
        TypedArray::new(ctx, text.into_bytes())
    }

    /// Writes as many whole characters as fit, and reports the UTF-16 code
    /// units it consumed and the bytes it wrote — never half a character.
    fn encode_into<'js>(
        &self,
        ctx: Ctx<'js>,
        input: Value<'js>,
        destination: Value<'js>,
    ) -> Result<Object<'js>> {
        let text = usv_string(&ctx, input)?;
        let destination = TypedArray::<u8>::from_value(destination).map_err(|_| {
            js_error::type_error(
                &ctx,
                "TextEncoder.encodeInto(source, destination) expects a Uint8Array destination",
            )
        })?;
        let capacity = destination.len();
        let mut written = 0;
        let mut read = 0;
        for character in text.chars() {
            if written + character.len_utf8() > capacity {
                break;
            }
            written += character.len_utf8();
            read += character.len_utf16();
        }
        if written > 0 {
            // `set` on the destination itself, rather than a write through its
            // backing store: the copy stays inside the engine's own bounds.
            let chunk = TypedArray::<u8>::new(ctx.clone(), &text.as_bytes()[..written])?;
            let set: Function = destination.get("set")?;
            set.call::<_, ()>((This(destination.clone()), chunk))?;
        }
        let result = Object::new(ctx)?;
        result.set("read", read)?;
        result.set("written", written)?;
        Ok(result)
    }
}

#[derive(Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub(super) struct TextDecoder {
    fatal: bool,
    ignore_bom: bool,
    /// The bytes of a character a streamed chunk ended inside of.
    #[qjs(skip_trace)]
    pending: RefCell<Vec<u8>>,
    /// Whether this stream is past the point where a BOM could appear.
    #[qjs(skip_trace)]
    started: Cell<bool>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl TextDecoder {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, label: Opt<Value<'js>>, options: Opt<Value<'js>>) -> Result<Self> {
        if let Some(label) = label.0.filter(|label| !label.is_undefined()) {
            let label = Coerced::<String>::from_js(&ctx, label)?.0;
            if !matches!(
                label.trim().to_ascii_lowercase().as_str(),
                "utf-8" | "utf8" | "unicode-1-1-utf-8"
            ) {
                return Err(js_error::range_error(
                    &ctx,
                    &format!(
                        "TextDecoder encoding `{label}` is not supported; only utf-8 is available"
                    ),
                ));
            }
        }
        let (fatal, ignore_bom) = match options.0.and_then(Value::into_object) {
            Some(options) => (
                options.get::<_, Option<bool>>("fatal")?.unwrap_or(false),
                options
                    .get::<_, Option<bool>>("ignoreBOM")?
                    .unwrap_or(false),
            ),
            None => (false, false),
        };
        Ok(Self {
            fatal,
            ignore_bom,
            pending: RefCell::new(Vec::new()),
            started: Cell::new(false),
        })
    }

    #[qjs(get)]
    fn encoding(&self) -> &'static str {
        "utf-8"
    }

    #[qjs(get)]
    fn fatal(&self) -> bool {
        self.fatal
    }

    #[qjs(get, rename = "ignoreBOM")]
    fn ignore_bom(&self) -> bool {
        self.ignore_bom
    }

    /// `{ stream: true }` holds back a character split across chunks until the
    /// next call; a call without it flushes, and the next call starts afresh.
    fn decode<'js>(
        &self,
        ctx: Ctx<'js>,
        input: Opt<Value<'js>>,
        options: Opt<Value<'js>>,
    ) -> Result<String> {
        let input = match input.0 {
            Some(value) if !value.is_undefined() && !value.is_null() => buffer_source(&ctx, value)?,
            _ => Vec::new(),
        };
        let stream = match options.0.and_then(Value::into_object) {
            Some(options) => options.get::<_, Option<bool>>("stream")?.unwrap_or(false),
            None => false,
        };

        let mut bytes = std::mem::take(&mut *self.pending.borrow_mut());
        bytes.extend_from_slice(&input);

        let mut start = 0;
        if !self.started.get() {
            if !self.ignore_bom && stream && bytes.len() < BOM.len() && BOM.starts_with(&bytes) {
                // Too short to tell whether it is a BOM yet.
                *self.pending.borrow_mut() = bytes;
                return Ok(String::new());
            }
            if !self.ignore_bom && bytes.starts_with(&BOM) {
                start = BOM.len();
            }
            self.started.set(!bytes.is_empty());
        }

        let mut end = bytes.len();
        if stream {
            end -= incomplete_tail(&bytes[start..]);
            self.pending.borrow_mut().extend_from_slice(&bytes[end..]);
        } else {
            self.started.set(false);
        }

        let bytes = &bytes[start..end];
        if self.fatal {
            match std::str::from_utf8(bytes) {
                Ok(text) => Ok(text.to_owned()),
                Err(_) => {
                    self.pending.borrow_mut().clear();
                    self.started.set(false);
                    Err(js_error::type_error(
                        &ctx,
                        "TextDecoder.decode received invalid UTF-8 data",
                    ))
                }
            }
        } else {
            // Rust replaces each maximal invalid subpart with one U+FFFD, which
            // is the substitution the Encoding standard specifies.
            Ok(String::from_utf8_lossy(bytes).into_owned())
        }
    }
}

/// The length of a UTF-8 sequence cut short at the end of `bytes`, or 0 when
/// the end is complete or invalid — invalid bytes are the decoder's to replace,
/// not this function's to hold back.
fn incomplete_tail(bytes: &[u8]) -> usize {
    let len = bytes.len();
    for back in 1..=len.min(3) {
        let byte = bytes[len - back];
        if byte & 0xC0 == 0x80 {
            continue;
        }
        let expected = match byte {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => return 0,
        };
        if back >= expected {
            return 0;
        }
        // The first continuation byte narrows for some lead bytes; a tail that
        // could never complete is invalid now, not later.
        let valid = bytes.get(len - back + 1).is_none_or(|&next| match byte {
            0xE0 => (0xA0..=0xBF).contains(&next),
            0xED => (0x80..=0x9F).contains(&next),
            0xF0 => (0x90..=0xBF).contains(&next),
            0xF4 => (0x80..=0x8F).contains(&next),
            _ => true,
        });
        return if valid { back } else { 0 };
    }
    0
}

/// The bytes of an `ArrayBuffer` or any view onto one (`Uint8Array`, another
/// typed array, `DataView`), copied out.
fn buffer_source<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Vec<u8>> {
    let not_buffer = || {
        js_error::type_error(
            ctx,
            "TextDecoder.decode(input) expects an ArrayBuffer, a typed array or a DataView",
        )
    };
    if let Some(buffer) = ArrayBuffer::from_value(value.clone()) {
        // SAFETY: no JavaScript runs between borrowing and copying the bytes.
        return Ok(unsafe { buffer.as_bytes() }.unwrap_or_default().to_vec());
    }
    let array_buffer: Object = ctx.globals().get("ArrayBuffer")?;
    let is_view: Function = array_buffer.get("isView")?;
    if !is_view.call::<_, bool>((value.clone(),))? {
        return Err(not_buffer());
    }
    let view = value.into_object().ok_or_else(not_buffer)?;
    let buffer = ArrayBuffer::from_value(view.get("buffer")?).ok_or_else(not_buffer)?;
    let offset: usize = view.get("byteOffset")?;
    let length: usize = view.get("byteLength")?;
    // A detached buffer reads as no bytes, as the standard's copy step does.
    // SAFETY: all JS property reads finished above; only slice and copy the bytes.
    Ok(unsafe { buffer.as_bytes() }
        .and_then(|bytes| bytes.get(offset..offset + length))
        .unwrap_or_default()
        .to_vec())
}

/// A JavaScript string as Rust text, with each lone surrogate replaced by
/// U+FFFD — the USVString conversion `TextEncoder` is specified to apply.
fn usv_string<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<String> {
    let string = Coerced::<rquickjs::String>::from_js(ctx, value)?.0;
    match string.to_string() {
        Ok(text) => Ok(text),
        Err(rquickjs::Error::Utf8(_)) => {
            let prototype: Object = ctx.globals().get::<_, Object>("String")?.get("prototype")?;
            let well_formed: Function = prototype.get("toWellFormed")?;
            well_formed.call::<_, String>((This(string),))
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::incomplete_tail;

    #[test]
    fn only_a_completable_utf8_tail_is_held_back() {
        assert_eq!(incomplete_tail(b"abc"), 0);
        assert_eq!(incomplete_tail(&[b'a', 0xE4]), 1);
        assert_eq!(incomplete_tail(&[b'a', 0xE4, 0xB8]), 2);
        assert_eq!(incomplete_tail(&[0xE4, 0xB8, 0xAD]), 0);
        assert_eq!(incomplete_tail(&[0xF0, 0x9F, 0x98]), 3);
        // A lead byte whose next byte can never complete it is invalid now.
        assert_eq!(incomplete_tail(&[0xE0, 0x80]), 0);
        assert_eq!(incomplete_tail(&[0xFF]), 0);
    }
}

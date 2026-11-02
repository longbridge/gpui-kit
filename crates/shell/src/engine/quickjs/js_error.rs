//! Throwing a script error with its whole message.
//!
//! rquickjs's `Exception::throw_type` and its siblings copy the message into a
//! 256-byte buffer for QuickJS's printf-style constructors, so anything longer
//! is cut off mid-word. The runtime's messages say what went wrong and then
//! what to do instead, and the second half is the part that was lost. These
//! build the error through the realm's own constructor instead, which takes a
//! message of any length and captures the script's stack the same way.

use rquickjs::{Ctx, Error, Exception, Value, function::Constructor};

/// Throws a `TypeError`.
pub(crate) fn type_error(ctx: &Ctx<'_>, message: &str) -> Error {
    throw(ctx, "TypeError", message).unwrap_or_else(|| Exception::throw_type(ctx, message))
}

/// Throws a `RangeError`.
pub(crate) fn range_error(ctx: &Ctx<'_>, message: &str) -> Error {
    throw(ctx, "RangeError", message).unwrap_or_else(|| Exception::throw_range(ctx, message))
}

/// Throws a plain `Error`.
pub(crate) fn error(ctx: &Ctx<'_>, message: &str) -> Error {
    throw(ctx, "Error", message).unwrap_or_else(|| Exception::throw_message(ctx, message))
}

/// `None` when the constructor could not be reached or failed, in which case
/// the caller falls back to rquickjs's own, truncating, form: a short message
/// is better than none.
fn throw(ctx: &Ctx<'_>, constructor: &str, message: &str) -> Option<Error> {
    let constructor: Constructor = ctx.globals().get(constructor).ok()?;
    match constructor.construct::<_, Value>((message,)) {
        Ok(value) => Some(ctx.throw(value)),
        Err(_) => {
            // Constructing threw; that exception is not the one to report.
            let _ = ctx.catch();
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context, Function, Runtime};

    use super::*;

    #[test]
    fn a_long_message_survives_whole() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let message = format!("{} and then what to do instead", "x".repeat(400));
        context.with(|ctx| {
            let thrower = Function::new(ctx.clone(), {
                let message = message.clone();
                move |ctx: Ctx<'_>| -> rquickjs::Result<()> { Err(type_error(&ctx, &message)) }
            })
            .unwrap();
            ctx.globals().set("thrower", thrower).unwrap();
            let caught: String = ctx
                .eval("try { thrower(); 'none' } catch (e) { `${e instanceof TypeError} ${e.message}` }")
                .unwrap();
            assert_eq!(caught, format!("true {message}"));
        });
    }
}

//! The QuickJS side of [`crate::host_modules`]: a resolver, a loader, and two
//! conversions.
//!
//! Everything interesting about HostModule registrations is engine independent and lives
//! above this file. What is left here is the module-loader wiring and exactly
//! the two conversions the seam forbids the registry from knowing about
//! (§6.5 rule 1): a script value becomes a [`HostValue`], and a [`HostValue`]
//! becomes a script value.
//!
//! ```js
//! import { steps } from "release";
//!
//! steps();
//! ```
//!
//! # How a registered module becomes an importable one
//!
//! [`HostModuleLoader`] sits in the resolver chain after the built-ins and
//! before the application's own files, so a host can neither shadow `gpui-kit` nor
//! be shadowed by a file the script happens to have. Resolving produces a
//! generated source module — one `export const` per registered function:
//!
//! ```js
//! export const steps = __host_function("release", "steps");
//! ```
//!
//! An asynchronous function gets an arrow instead, because `Promise<'js>`
//! borrows the context lifetime and so has to be produced by a free function
//! rather than a bound closure:
//!
//! ```js
//! export const fetch_notes = (...args) => __host_async_call("release", "fetch_notes", ...args);
//! ```
//!
//! Two consequences follow from that shape, and both are the point:
//!
//! - **A wrong export name fails at link time.** `import { setps }` is a
//!   `SyntaxError` naming the module, raised before any of the application
//!   runs. The generated declarations catch it earlier still.
//! - **The binding is a stub, not the closure.** Each export forwards through
//!   [`host_modules::dispatch`] on every call, so the registry stays the
//!   authority: revoking a module refuses the next call through an already
//!   imported name, and a plugin's frame decides whose module answers.
//!
//! The resolved name carries the registry's generation, because QuickJS caches
//! a linked module by name for the lifetime of the runtime. Without the tag,
//! two plugins importing `workspace` would share whichever was linked first —
//! one plugin's grant serving another's script.
//!
//! # Why conversion lives in `FromJs`/`IntoJs`
//!
//! A closure passed to `Func::from` cannot unify the `Ctx<'js>` of its
//! parameter with a `Value<'js>` in its return type — the two elided lifetimes
//! are distinct to the compiler. Both directions are therefore expressed as
//! conversions on `'static` wrapper types, where `'js` appears once.

use std::fmt::Write as _;

use rquickjs::{
    Array, Ctx, Error as JsError, Exception, FromJs, IntoJs, Module, Object, Promise,
    Result as JsResult, Value,
    function::{Func, Rest},
    loader::{ImportAttributes, Loader, Resolver},
    module::Declared,
};

use crate::{
    host_modules::{self, HostArguments, HostValue},
    scope,
};

use super::{ShellRuntime, scheduler};

/// How deep an argument may nest.
///
/// A script can hand over a structure of any depth, and conversion is
/// recursive; a limit turns "the host was passed a 100k-deep list" from a
/// blown Rust stack into a message at the call site. Sixteen is far past any
/// record a host function has business receiving.
const MAX_DEPTH: usize = 16;

/// Maximum number of array slots converted across a JavaScript/Rust boundary.
///
/// Sparse arrays cost just as much as dense ones here because bridge semantics
/// preserve every hole as `null`. Keeping one limit for host values and host
/// JSON prevents either conversion path becoming an allocation bypass.
pub(super) const MAX_BRIDGE_ARRAY_ITEMS: usize = 10_000;

/// Separates a module name from the registry generation it was resolved
/// against. Distinct from the `?v=` an application's own files carry, so the
/// two loaders never answer for each other's names.
const GENERATION_TAG: &str = "?m=";

pub(super) fn bridge_array_len(ctx: &Ctx<'_>, array: &Array<'_>) -> JsResult<usize> {
    // Do not use `Array::len`: rquickjs 0.12 asserts that QuickJS returned a
    // signed integer, while valid JS arrays may have lengths above i32::MAX and
    // QuickJS represents those as floating-point values.
    let length: Value = array.as_object().get("length")?;
    let Some(length) = length.as_number() else {
        return Err(Exception::throw_type(ctx, "array length is not a number"));
    };
    if !length.is_finite() || length < 0.0 || length.fract() != 0.0 {
        return Err(Exception::throw_type(
            ctx,
            "array length must be a finite non-negative integer",
        ));
    }
    if length > MAX_BRIDGE_ARRAY_ITEMS as f64 {
        return Err(Exception::throw_range(
            ctx,
            &format!(
                "array has {length:.0} items, over the {MAX_BRIDGE_ARRAY_ITEMS}-item bridge limit"
            ),
        ));
    }
    Ok(length as usize)
}

/// Installs the global the generated modules call to build their bindings.
///
/// It is a global rather than an export of some module because generated source
/// cannot import anything: the module it would import from is the one being
/// declared. The name is in the `__` space the sandbox withholds from scripts.
pub fn install(ctx: &Ctx<'_>) -> JsResult<()> {
    let globals = ctx.globals();
    globals.set(
        "__host_function",
        Func::from(|module: String, function: String| Binding { module, function }),
    )?;
    // A free function rather than a bound closure, and called through an arrow
    // in the generated module rather than returned from one. `Promise<'js>`
    // borrows the context lifetime, and a closure cannot be inferred as
    // polymorphic over a lifetime appearing in both its parameter and its
    // return type — the same constraint `scheduler::js_sleep` works around.
    globals.set("__host_async_call", Func::from(host_async_call))
}

/// One asynchronous call, from the arrow the generated module exports.
fn host_async_call<'js>(
    ctx: Ctx<'js>,
    module: String,
    function: String,
    arguments: Rest<Argument>,
) -> JsResult<Promise<'js>> {
    let arguments = HostArguments::new(arguments.0.into_iter().map(|it| it.0));
    // Argument checking happens here, on the main thread and inside the calling
    // script's scope, so a refusal is a thrown `TypeError` at the call site
    // rather than a rejected promise the script has to await to hear about.
    let work = host_modules::dispatch_async(&module, &function, &arguments).map_err(|error| {
        Exception::throw_message(&ctx, &format!("`{module}.{function}`: {error}"))
    })?;

    scheduler::awaiting(&ctx, ASYNC_API, async move {
        work.await
            .map(Bridged)
            .map_err(|error| format!("`{module}.{function}`: {error}"))
    })
}

/// What an asynchronous host call is called in task diagnostics.
///
/// One name for all of them, because the scheduler's task label is `&'static
/// str` and a module's function names are not. The failure a script sees is
/// unaffected: a rejection carries `module.function` in its message.
const ASYNC_API: &str = "a HostModule's async function";

/// Resolves and loads the modules a host registered.
///
/// Stateless: the registry is read through the calling frame on every resolve,
/// which is what lets two plugins in one runtime see two different module sets.
///
/// A miss is a plain [`JsError::new_resolving`], never a thrown exception. This
/// resolver is not the last in the chain — the application's own files are —
/// and a resolver that throws leaves the exception pending on the context, so
/// the file resolver behind it never gets to answer. `import "./ui.js"` failing
/// with "not a HostModule" is what that mistake looks like.
#[derive(Clone, Copy, Default)]
pub(super) struct HostModuleLoader;

impl HostModuleLoader {
    /// Splits a resolved name back into the module and the generation it was
    /// resolved against.
    fn untag(name: &str) -> Option<(&str, u64)> {
        let (module, generation) = name.rsplit_once(GENERATION_TAG)?;
        Some((module, generation.parse().ok()?))
    }
}

impl Resolver for HostModuleLoader {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<String> {
        // Only a bare specifier: a relative or rooted path is the application's
        // own file, and answering for one here would let a registered module
        // stand in for a file the author is looking straight at.
        if name.starts_with('.') || name.starts_with('/') {
            return Err(JsError::new_resolving(base, name));
        }

        let registry = host_modules::modules();
        if registry.get(name).is_err() {
            return Err(JsError::new_resolving(base, name));
        }

        Ok(format!("{name}{GENERATION_TAG}{}", registry.generation()))
    }
}

impl Loader for HostModuleLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<Module<'js, Declared>> {
        let Some((module, generation)) = Self::untag(name) else {
            // Not a name this resolver produced, so it belongs to a loader
            // further along the chain.
            return Err(JsError::new_loading(name));
        };

        let registry = host_modules::modules();
        // Resolve and load are one import apart, so a mismatch means the host
        // swapped its registry in between. Saying so beats declaring a module
        // whose exports describe a set that is already gone.
        if registry.generation() != generation {
            return Err(Exception::throw_message(
                ctx,
                &format!(
                    "the HostModule registry changed while `{module}` was being imported; \
                     export them before loading an application"
                ),
            ));
        }

        let found = registry
            .get(module)
            .map_err(|error| Exception::throw_message(ctx, error.message()))?;

        let mut source = String::new();
        for function in found.function_names() {
            // `function_names` are the keys a host registered from Rust, and a
            // JavaScript identifier is the one shape that can be exported by
            // name. Refusing here rather than emitting broken source turns a
            // host's typo into a sentence naming it.
            if !is_identifier(function) {
                return Err(Exception::throw_message(
                    ctx,
                    &format!(
                        "HostModule `{module}` registered `{function}`, which is not a \
                         JavaScript identifier and so cannot be exported"
                    ),
                ));
            }
            let _ = if found.is_async(function) {
                writeln!(
                    source,
                    "export const {function} = (...args) => \
                     __host_async_call({module:?}, {function:?}, ...args);"
                )
            } else {
                writeln!(
                    source,
                    "export const {function} = __host_function({module:?}, {function:?});"
                )
            };
        }
        for component in found.component_names() {
            if !is_identifier(component) {
                return Err(Exception::throw_message(
                    ctx,
                    &format!(
                        "HostModule `{module}` component `{component}` is not a JavaScript identifier"
                    ),
                ));
            }
            let _ = writeln!(
                source,
                "export const {component} = globalThis.__gpui.component({module:?}, {component:?});"
            );
        }

        Module::declare(ctx.clone(), name, source)
    }
}

fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$')
        && characters.all(|it| it.is_ascii_alphanumeric() || it == '_' || it == '$')
}

/// One export, on its way into the generated module.
///
/// It holds names rather than a closure for the same reason the registry does:
/// the function behind a name is looked up per call, so revoking a module
/// reaches a script that already imported it.
struct Binding {
    module: String,
    function: String,
}

impl<'js> IntoJs<'js> for Binding {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        let Self { module, function } = self;
        Func::from(
            move |ctx: Ctx<'_>, arguments: Rest<Argument>| -> JsResult<Bridged> {
                let arguments = HostArguments::new(arguments.0.into_iter().map(|it| it.0));
                // Timed so a script render can be told apart from the host work
                // inside it: `quotes()` reading a board out of an entity is not
                // the script describing itself, and charging it to JavaScript
                // would be a lie in the flattering direction.
                // Looked up and released before dispatching: a host function
                // reaches for the ambient `App` itself, and holding it across
                // the call would be two borrows of one thing.
                let runtime = scope::current_runtime()
                    .or_else(|| scope::with_current_app(|cx| ShellRuntime::global(cx)).flatten());
                let dispatched = match &runtime {
                    Some(runtime) => runtime
                        .metrics()
                        .time_native(|| host_modules::dispatch(&module, &function, &arguments)),
                    None => host_modules::dispatch(&module, &function, &arguments),
                };
                match dispatched {
                    Ok(value) => Ok(Bridged(value)),
                    // The registry's messages never name their own function, so
                    // the call site is named exactly once.
                    Err(error) => Err(Exception::throw_message(
                        &ctx,
                        &format!("`{module}.{function}`: {error}"),
                    )),
                }
            },
        )
        .into_js(ctx)
    }
}

/// One argument, converted on the way in.
pub(super) struct Argument(pub(super) HostValue);

impl<'js> FromJs<'js> for Argument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        Ok(Self(from_js(ctx, value, 0)?))
    }
}

pub(super) fn from_js<'js>(ctx: &Ctx<'js>, value: Value<'js>, depth: usize) -> JsResult<HostValue> {
    if depth > MAX_DEPTH {
        return Err(Exception::throw_type(
            ctx,
            &format!("a host argument may not nest more than {MAX_DEPTH} levels deep"),
        ));
    }

    if value.is_null() || value.is_undefined() {
        return Ok(HostValue::Null);
    }
    if let Some(flag) = value.as_bool() {
        return Ok(HostValue::Bool(flag));
    }
    if let Some(number) = value.as_number() {
        return Ok(HostValue::Number(number));
    }
    if let Some(text) = value.as_string() {
        return Ok(HostValue::Str(text.to_string()?));
    }
    // Before the object case: an array is an object too.
    if let Some(array) = value.as_array() {
        let length = bridge_array_len(ctx, &array)?;
        let mut values = Vec::new();
        values.try_reserve_exact(length).map_err(|_| {
            Exception::throw_range(ctx, "host array could not be reserved within memory limits")
        })?;
        for index in 0..length {
            values.push(from_js(ctx, array.get(index)?, depth + 1)?);
        }
        return Ok(HostValue::Array(values));
    }
    // A function would be a handle, and a handle is the one thing that must not
    // cross: the host could hold it past the call, and past the scope frame
    // that made the surrounding context valid.
    if value.as_function().is_some() {
        return Err(Exception::throw_type(
            ctx,
            "a host function cannot be passed a callback; host calls take and return \
             plain data only",
        ));
    }
    if let Some(object) = value.as_object() {
        let mut fields = Vec::new();
        for entry in object.props::<String, Value>() {
            let (key, value) = entry?;
            fields.push((key, from_js(ctx, value, depth + 1)?));
        }
        return Ok(HostValue::Object(fields));
    }

    Err(Exception::throw_type(
        ctx,
        "unsupported host argument; expected null, a boolean, a number, a string, \
         an array or a plain object",
    ))
}

/// One result, converted on the way out.
struct Bridged(HostValue);

impl<'js> IntoJs<'js> for Bridged {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        into_js(ctx, self.0)
    }
}

pub(super) fn into_js<'js>(ctx: &Ctx<'js>, value: HostValue) -> JsResult<Value<'js>> {
    Ok(match value {
        HostValue::Null => Value::new_null(ctx.clone()),
        HostValue::Bool(flag) => Value::new_bool(ctx.clone(), flag),
        HostValue::Number(number) => Value::new_number(ctx.clone(), number),
        HostValue::Str(text) => rquickjs::String::from_str(ctx.clone(), &text)?.into_value(),
        HostValue::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (index, value) in values.into_iter().enumerate() {
                array.set(index, into_js(ctx, value)?)?;
            }
            array.into_value()
        }
        HostValue::Object(fields) => {
            let object = Object::new(ctx.clone())?;
            for (key, value) in fields {
                object.set(key, into_js(ctx, value)?)?;
            }
            object.into_value()
        }
    })
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context as JsContext, Error as JsError, Runtime as JsRuntime};

    use super::*;

    #[test]
    fn a_sparse_huge_host_array_is_a_catchable_error() {
        let runtime = JsRuntime::new().expect("runtime");
        let context = JsContext::full(&runtime).expect("context");
        context.with(|ctx| {
            let value: Value = ctx
                .eval("const values = []; values.length = 0xffffffff; values")
                .expect("sparse array");
            let error = match Argument::from_js(&ctx, value) {
                Ok(_) => panic!("the bridge must refuse a huge sparse array"),
                Err(error) => error,
            };
            assert!(matches!(error, JsError::Exception), "{error}");
            let thrown = ctx.catch();
            let message = thrown
                .as_exception()
                .and_then(|exception| exception.message())
                .unwrap_or_else(|| format!("{thrown:?}"));
            assert!(
                message.contains("array") && message.contains("limit"),
                "{message}"
            );
        });
    }

    #[test]
    fn only_a_javascript_identifier_can_be_exported() {
        assert!(is_identifier("quotes"));
        assert!(is_identifier("_watch"));
        assert!(is_identifier("$0"));
        assert!(!is_identifier(""));
        assert!(!is_identifier("2fast"));
        assert!(!is_identifier("watch all"));
        assert!(!is_identifier("watch-all"));
    }

    /// The list a host is refused at registration must be exactly the set of
    /// names that resolve before this loader does. A module added to either
    /// resolver and not to the list would be a name a host could register and
    /// then never be able to import — which is the failure
    /// [`host_modules::RESERVED_SPECIFIERS`] exists to prevent.
    #[test]
    fn the_reserved_names_are_every_specifier_that_resolves_first() {
        let mut resolving_first: Vec<&str> = super::super::builtin_specifier_list();
        resolving_first.extend_from_slice(super::super::standard::NAMES);
        resolving_first.sort_unstable();

        let mut reserved = host_modules::RESERVED_SPECIFIERS.to_vec();
        reserved.sort_unstable();

        assert_eq!(
            reserved, resolving_first,
            "a specifier that resolves before this loader must be in \
             RESERVED_SPECIFIERS, or a host can register that name and then \
             never be able to import it. Add it to the list rather than \
             loosening this assertion."
        );
    }

    #[test]
    fn a_resolved_name_round_trips_through_its_generation_tag() {
        assert_eq!(
            HostModuleLoader::untag(&format!("market{GENERATION_TAG}7")),
            Some(("market", 7))
        );
        assert_eq!(HostModuleLoader::untag("market"), None);
    }
}

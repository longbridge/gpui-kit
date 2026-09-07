//! HostModule registrations — Rust functions the host lends to a script, imported by name.
//!
//! A script cannot load a native extension. `dlopen`-ed Rust has no stable ABI
//! and, once inside the process, holds every permission the process holds — a
//! sandbox that permits it does not mean anything. So the direction is
//! reversed: **the host registers, at compile time, the Rust it is willing to
//! expose**, and a script may reach exactly that and nothing else (design doc
//! §17.6).
//!
//! ```no_run
//! use gpui_shell::{HostModule, HostValue};
//!
//! gpui_shell::export_module(
//!     HostModule::new("workspace")
//!         .function("project_name", |_| Ok(HostValue::from("gpui-component"))),
//! )
//! .expect("`workspace` is not one of the runtime's own module names");
//! ```
//!
//! ```js
//! import { project_name } from "workspace";
//!
//! project_name();
//! ```
//!
//! # Why an import rather than a lookup
//!
//! A registered module is resolved by the engine's module loader, so a host
//! module is imported exactly like `gpui-kit` or `path` is. The obvious alternative
//! is a lookup — `native("workspace")` answering with a bag of functions — and
//! it loses twice, both times on *when* a mistake surfaces.
//!
//! A lookup puts every misspelled export on the run-time path: the call
//! type-checks, loads, renders, and throws on the frame that first reaches it.
//! An import fails while the module graph is linked, before a line of the
//! application runs.
//!
//! A lookup also leaves the type declarations with nothing to say. Only the
//! host knows what it registered, so the best a generated `.d.ts` can offer for
//! A runtime registry lookup can only return a bag of untyped functions. A module specifier is
//! a name declarations can be written against, so the generated ones name the
//! exports in this registry directly.
//!
//! What the import does *not* freeze is the function behind the name. Every
//! export is a forwarding stub that resolves through the registry on each call,
//! so revoking a module still takes effect immediately — a script holding an
//! imported function gets a refusal rather than the withdrawn closure. Only the
//! *set of names* is fixed, at the moment the importing module is linked, which
//! is why a host registers before it loads an application.
//!
//! # Why an asynchronous function is two halves
//!
//! [`HostModule::async_function`] takes a closure that runs on the main thread
//! and returns a `Send` future that does not. The script gets a promise, and
//! slow work — a request, a query, a large file — stops holding the thread that
//! renders.
//!
//! The split is not a concession to `Send`. It is the rule below made physical:
//! the synchronous half may read host state because it runs inside the caller's
//! scope, and the future cannot re-enter the engine because on another thread
//! there is no `Ctx` to re-enter it with.
//!
//! # Why the boundary is plain data
//!
//! A host function receives [`HostArguments`] and returns a [`HostValue`]:
//! null, boolean, number, string, array, object. It never receives a script
//! handle. That is not a convenience — a handle would let the host keep a
//! reference to a script value past the call that produced it, and past the
//! internal call-scope frame that made the surrounding context valid. It is
//! also what lets one registry serve both engines, since neither engine's value
//! type appears in this file.
//!
//! # Why a host function must not re-enter the engine
//!
//! A host call happens *inside* a script call, which is itself inside a host
//! call. Calling back into the VM from there would run script code with an
//! engine frame already on the stack — re-entering QuickJS, and re-entering the
//! render pass that is currently building an element tree. Holding no script
//! handle makes that impossible to express, and the internal dispatcher refuses a nested
//! call outright so a host that finds another route (pumping GPUI until a view
//! re-renders, say) gets a diagnosable error instead of undefined behavior.
//!
//! Reading and writing host state is fine, and is the point: a function may
//! reach for the ambient `App` through [`crate::scope::with_current_app`] and
//! request a re-render with `cx.notify()`, which is delivered after the current
//! call unwinds.
//!
//! # Reaching HostModule registrations is itself a capability
//!
//! The default registry is empty, and every entry point into it fails while it
//! stays that way — the same shape as [`crate::Capabilities::default`], which
//! permits nothing. A host that installs no modules has granted no extension
//! surface, and a script that imports one is told so by name. There is
//! deliberately no per-module grant: the host chose the module list, so the
//! list *is* the grant.
//!
//! # Why the runtime's own modules cannot be taken
//!
//! A HostModule shares one specifier namespace with the built-in modules and
//! the Standard Runtime, and the resolver chain reaches the built-ins first. A
//! host registering `path` would therefore not shadow the real `path` — it
//! would register a module nothing can ever import, and never find out. So the
//! names in [`RESERVED_SPECIFIERS`] are refused at registration, where the
//! author can still see the sentence.

use std::{
    cell::Cell, collections::BTreeMap, fmt, fmt::Write as _, future::Future, pin::Pin, rc::Rc,
};

use crate::component::ComponentFactory;

/// A value crossing the host boundary, in either direction.
///
/// The six cases are the intersection of what a script engine and JSON can both
/// carry, which is what keeps one registry usable from any engine behind the
/// seam rather than from QuickJS alone.
#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<HostValue>),
    /// Insertion-ordered, because an object is frequently a record the script
    /// renders in order, and a map would decide that order for it.
    Object(Vec<(String, HostValue)>),
}

impl HostValue {
    pub fn is_null(&self) -> bool {
        matches!(self, HostValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            HostValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            HostValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            HostValue::Str(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[HostValue]> {
        match self {
            HostValue::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, HostValue)]> {
        match self {
            HostValue::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// The value of one object field, or `None` for a non-object.
    pub fn get(&self, key: &str) -> Option<&HostValue> {
        self.as_object()?
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The type name used in error messages.
    pub fn describe(&self) -> &'static str {
        match self {
            HostValue::Null => "null",
            HostValue::Bool(_) => "a boolean",
            HostValue::Number(_) => "a number",
            HostValue::Str(_) => "a string",
            HostValue::Array(_) => "an array",
            HostValue::Object(_) => "an object",
        }
    }
}

impl From<bool> for HostValue {
    fn from(value: bool) -> Self {
        HostValue::Bool(value)
    }
}

macro_rules! from_number {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for HostValue {
                fn from(value: $type) -> Self {
                    HostValue::Number(value as f64)
                }
            }
        )*
    };
}

from_number!(f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl From<String> for HostValue {
    fn from(value: String) -> Self {
        HostValue::Str(value)
    }
}

impl From<&str> for HostValue {
    fn from(value: &str) -> Self {
        HostValue::Str(value.to_owned())
    }
}

impl<T: Into<HostValue>> From<Vec<T>> for HostValue {
    fn from(values: Vec<T>) -> Self {
        HostValue::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<HostValue>> From<Option<T>> for HostValue {
    fn from(value: Option<T>) -> Self {
        value.map(Into::into).unwrap_or(HostValue::Null)
    }
}

/// Builds a [`HostValue::Object`] one field at a time.
///
/// A record is the common return shape — a row, a settings snapshot, a
/// progress report — and building it with a builder keeps the field order the
/// host wrote it in.
#[derive(Clone, Debug, Default)]
pub struct HostObject {
    fields: Vec<(String, HostValue)>,
}

impl HostObject {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field. A repeated name replaces the earlier value in place, so a
    /// caller cannot accidentally emit the same key twice.
    pub fn field(mut self, name: impl Into<String>, value: impl Into<HostValue>) -> Self {
        let name = name.into();
        let value = value.into();
        match self.fields.iter_mut().find(|(key, _)| *key == name) {
            Some(existing) => existing.1 = value,
            None => self.fields.push((name, value)),
        }
        self
    }
}

impl From<HostObject> for HostValue {
    fn from(object: HostObject) -> Self {
        HostValue::Object(object.fields)
    }
}

/// The positional arguments of one host call.
///
/// The typed readers exist so that a wrong argument reports which position was
/// wrong and what arrived there, rather than the host writing that sentence
/// once per function.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HostArguments {
    values: Vec<HostValue>,
}

impl HostArguments {
    pub fn new(values: impl IntoIterator<Item = HostValue>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&HostValue> {
        self.values.get(index)
    }

    /// The argument at `index`, or an error naming the position.
    pub fn value(&self, index: usize) -> Result<&HostValue, HostError> {
        self.values.get(index).ok_or_else(|| {
            HostError::new(format!(
                "argument {} is missing; {} were passed",
                index + 1,
                self.values.len()
            ))
        })
    }

    pub fn string(&self, index: usize) -> Result<&str, HostError> {
        let value = self.value(index)?;
        value
            .as_str()
            .ok_or_else(|| mistyped(index, "a string", value))
    }

    pub fn number(&self, index: usize) -> Result<f64, HostError> {
        let value = self.value(index)?;
        value
            .as_number()
            .ok_or_else(|| mistyped(index, "a number", value))
    }

    /// A number that has to be whole — an identifier, a count, an index.
    pub fn integer(&self, index: usize) -> Result<i64, HostError> {
        let number = self.number(index)?;
        if number.fract() != 0. {
            return Err(HostError::new(format!(
                "argument {} must be a whole number, got {number}",
                index + 1
            )));
        }
        Ok(number as i64)
    }

    pub fn boolean(&self, index: usize) -> Result<bool, HostError> {
        let value = self.value(index)?;
        value
            .as_bool()
            .ok_or_else(|| mistyped(index, "a boolean", value))
    }
}

fn mistyped(index: usize, expected: &str, got: &HostValue) -> HostError {
    HostError::new(format!(
        "argument {} must be {expected}, got {}",
        index + 1,
        got.describe()
    ))
}

/// A host function said no.
///
/// It carries a sentence and nothing else: the engine adds the module and
/// function names when it turns this into a script exception, so a host writing
/// a function never repeats its own name in the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HostError {}

impl From<String> for HostError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for HostError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// What a host function returns.
pub type HostResult = Result<HostValue, HostError>;

/// `Rc` rather than `Box` because the registry is handed out by clone on every
/// call — see [`modules`] — and a boxed closure cannot be shared that way.
type HostFunction = Rc<dyn Fn(&HostArguments) -> HostResult>;

/// The work an asynchronous host function hands back.
///
/// `Send` because it is driven on the background executor, and `'static`
/// because it outlives the call that produced it. Both fall out of the split
/// described on [`HostModule::async_function`]: whatever the future needs from
/// the `App` was copied out before it was built.
pub type HostFuture = Pin<Box<dyn Future<Output = HostResult> + Send>>;

/// Builds one asynchronous call. Fallible so that argument checking can refuse
/// before any work is scheduled.
type HostAsyncFunction = Rc<dyn Fn(&HostArguments) -> Result<HostFuture, HostError>>;

/// One registered module: a name and the functions under it.
#[derive(Clone)]
pub struct HostModule {
    name: String,
    /// Sorted, so the "it provides: …" line in a diagnostic reads the same on
    /// every run regardless of registration order.
    functions: BTreeMap<String, HostFunction>,
    /// Sorted alongside `functions`, and disjoint from it: one name is either
    /// synchronous or asynchronous, never both.
    async_functions: BTreeMap<String, HostAsyncFunction>,
    components: BTreeMap<String, ComponentFactory>,
    /// The module's TypeScript face, if the host wrote one. See
    /// [`HostModule::declarations`].
    declarations: Option<String>,
}

impl HostModule {
    /// Names a module a script may import.
    ///
    /// The name is the specifier: `HostModule::new("workspace")` is what makes
    /// `import { … } from "workspace"` resolve. A name the runtime already owns
    /// is refused by [`Self::validate`], which [`crate::export_module`] calls —
    /// see [`RESERVED_SPECIFIERS`].
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: BTreeMap::new(),
            async_functions: BTreeMap::new(),
            components: BTreeMap::new(),
            declarations: None,
        }
    }

    /// Exports one Rust-built element constructor from this module.
    pub fn component(
        mut self,
        name: impl Into<String>,
        build: impl for<'a> Fn(
            crate::ComponentArgs<'a>,
            &mut gpui::Window,
            &mut gpui::App,
        ) -> gpui::AnyElement
        + 'static,
    ) -> Self {
        let name = name.into();
        self.functions.remove(&name);
        self.async_functions.remove(&name);
        self.components.insert(name, ComponentFactory::new(build));
        self
    }

    pub(crate) fn component_names(&self) -> impl Iterator<Item = &str> {
        self.components.keys().map(String::as_str)
    }

    pub(crate) fn resolve_component(&self, name: &str) -> Result<ComponentFactory, HostError> {
        self.components.get(name).cloned().ok_or_else(|| {
            HostError::new(format!(
                "HostModule `{}` has no component `{name}`",
                self.name
            ))
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Registers one function.
    ///
    /// The body must not call back into the script engine; see the module
    /// header. It may read and write host state, and may ask a view to
    /// re-render — the notification is delivered after the call unwinds.
    pub fn function(
        mut self,
        name: impl Into<String>,
        body: impl Fn(&HostArguments) -> HostResult + 'static,
    ) -> Self {
        let name = name.into();
        self.components.remove(&name);
        self.async_functions.remove(&name);
        self.functions.insert(name, Rc::new(body));
        self
    }

    /// Registers one function whose work runs off the main thread.
    ///
    /// The script gets a promise. What it awaits is driven on GPUI's background
    /// executor, so a slow call — a request, a query, a large file — does not
    /// hold the thread that renders.
    ///
    /// ```no_run
    /// # use gpui_shell::{HostModule, HostValue};
    /// HostModule::new("db")
    ///     .declarations("export function query(sql: string): Promise<unknown[]>;")
    ///     .async_function("query", |arguments| {
    ///         // Synchronous half: on the main thread, so it may read host
    ///         // state, and refusing here costs nothing.
    ///         let sql = arguments.string(0)?.to_owned();
    ///
    ///         // Asynchronous half: on the background executor.
    ///         Ok(Box::pin(async move {
    ///             let _ = sql;
    ///             Ok(HostValue::Null)
    ///         }))
    ///     });
    /// ```
    ///
    /// # The two halves, and why the split is the design
    ///
    /// The closure runs on the main thread, inside the calling script's scope,
    /// and returns the future. So the *arguments* can be checked against host
    /// state — [`crate::with_current_app`] works here, exactly as it does in a
    /// synchronous function — and anything the work needs is copied out before
    /// the future is built.
    ///
    /// The future is then `Send + 'static` and driven on another thread, where
    /// there is no `App` and no `Ctx` to reach for. That is not a restriction
    /// bolted on: it is the same rule the module header states for synchronous
    /// functions — a host function may not re-enter the engine — made physical.
    /// A synchronous body is held to it by a runtime guard; an asynchronous one
    /// simply cannot express the violation.
    ///
    /// # Cancellation
    ///
    /// If the view that made the call goes away, or its application is
    /// reloaded, the promise stays pending for ever and the script's
    /// continuation never runs. No error is invented for code that was asked to
    /// stop — the same answer `cx.sleep` gives.
    ///
    /// # Declaring it
    ///
    /// Write the return type as a `Promise` in [`Self::declarations`]. The
    /// registry checks that the *names* on both sides agree; it does not read
    /// signatures, so nothing stops a declaration from omitting the `Promise`
    /// and nothing catches it later either.
    pub fn async_function<F>(
        mut self,
        name: impl Into<String>,
        body: impl Fn(&HostArguments) -> Result<F, HostError> + 'static,
    ) -> Self
    where
        F: Future<Output = HostResult> + Send + 'static,
    {
        let name = name.into();
        self.components.remove(&name);
        // One name, one kind. Registering over the other table rather than
        // beside it keeps `function_names` a single list and stops a call
        // having to decide which of two entries it meant.
        self.functions.remove(&name);
        self.async_functions.insert(
            name,
            Rc::new(move |arguments| {
                let future = body(arguments)?;
                Ok(Box::pin(future) as HostFuture)
            }),
        );
        self
    }

    /// Describes this module in TypeScript, for the generated declarations.
    ///
    /// The body of a `declare module`, so it may carry helper types alongside
    /// the exports:
    ///
    /// ```no_run
    /// # use gpui_shell::{HostModule, HostValue};
    /// HostModule::new("market")
    ///     .function("quotes", |_| Ok(HostValue::Null))
    ///     .declarations(
    ///         r#"
    ///         export interface Quote { symbol: string; last: string }
    ///         export function quotes(): Quote[];
    ///         "#,
    ///     );
    /// ```
    ///
    /// Writing it here rather than in a `.d.ts` beside the script is what keeps
    /// the two halves one thing. A `.d.ts` is a second file, in a second
    /// language, with nothing holding it to the registry; here, [`Self::validate`]
    /// checks that every registered function is declared and every declared
    /// function is registered, so renaming one half fails at start-up instead of
    /// at the call site.
    ///
    /// Declaring nothing is allowed and costs only precision: an undeclared
    /// module is emitted with `(...args: any[]) => any` signatures, which still
    /// checks the module name and every export name.
    pub fn declarations(mut self, typescript: impl Into<String>) -> Self {
        self.declarations = Some(typescript.into());
        self
    }

    /// The TypeScript body [`Self::declarations`] was given.
    pub fn declared(&self) -> Option<&str> {
        self.declarations.as_deref()
    }

    /// Every name this module exports, synchronous and asynchronous together,
    /// sorted.
    ///
    /// One list because the script sees one list: both kinds are `export const`
    /// in the generated module, and which is which is the return type's
    /// business, not the import's.
    pub fn function_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .functions
            .keys()
            .chain(self.async_functions.keys())
            .map(String::as_str)
            .collect();
        names.sort_unstable();
        names
    }

    pub fn has(&self, function: &str) -> bool {
        self.functions.contains_key(function) || self.async_functions.contains_key(function)
    }

    fn has_export(&self, name: &str) -> bool {
        self.has(name) || self.components.contains_key(name)
    }

    /// Whether this name answers with a promise.
    ///
    /// Read by the engine when it builds the binding, because the two return
    /// different things to the script and the difference cannot be deferred to
    /// call time: a promise borrows the context lifetime.
    pub fn is_async(&self, function: &str) -> bool {
        self.async_functions.contains_key(function)
    }

    /// Refuses a name the runtime owns, and a TypeScript face that disagrees
    /// with what was registered.
    ///
    /// Called by [`crate::export_module`] and [`crate::policy::Policy::with_host_module`],
    /// which is where a host still has somewhere to put the sentence. Building
    /// a module is infallible on purpose: a builder that returned a `Result`
    /// from every step could not be a chain.
    pub fn validate(&self) -> Result<(), HostError> {
        if RESERVED_SPECIFIERS.contains(&self.name.as_str()) {
            return Err(HostError::new(format!(
                "`{}` is one of the runtime's own module names and cannot be \
                 registered: a script importing it reaches the runtime, never this \
                 module. The reserved names are: {}",
                self.name,
                list(RESERVED_SPECIFIERS)
            )));
        }
        self.check_declarations()
    }

    /// Compares the declared exports with the registered ones.
    ///
    /// Only `export function` and `export const` at the start of a line are
    /// read as exports — enough to name every function a module can have, and
    /// deliberately not a TypeScript parser: anything it cannot recognize it
    /// leaves alone, so a helper type or a comment is never mistaken for a
    /// missing function.
    fn check_declarations(&self) -> Result<(), HostError> {
        let Some(declarations) = self.declared() else {
            return Ok(());
        };

        let declared: Vec<&str> = declarations
            .lines()
            .filter_map(|line| {
                let line = line.trim_start();
                let rest = line
                    .strip_prefix("export function ")
                    .or_else(|| line.strip_prefix("export declare function "))
                    .or_else(|| line.strip_prefix("export const "))?;
                let end =
                    rest.find(|it: char| !it.is_ascii_alphanumeric() && it != '_' && it != '$');
                Some(&rest[..end.unwrap_or(rest.len())])
            })
            .collect();

        let missing: Vec<&str> = self
            .function_names()
            .into_iter()
            .chain(self.component_names())
            .filter(|name| !declared.contains(name))
            .collect();
        let extra: Vec<&str> = declared
            .iter()
            .copied()
            .filter(|name| !self.has_export(name))
            .collect();

        if missing.is_empty() && extra.is_empty() {
            return Ok(());
        }

        let mut message = format!(
            "HostModule `{}` declares a different set of exports than it registers",
            self.name
        );
        if !missing.is_empty() {
            let _ = write!(message, "; registered but not declared: {}", list(&missing));
        }
        if !extra.is_empty() {
            let _ = write!(message, "; declared but not registered: {}", list(&extra));
        }
        Err(HostError::new(message))
    }

    /// Calls one synchronous function, reporting an unknown name against what
    /// this module actually provides.
    pub fn call(&self, function: &str, arguments: &HostArguments) -> HostResult {
        let Some(body) = self.functions.get(function) else {
            return Err(self.no_such_function(function));
        };
        body(arguments)
    }

    /// Starts one asynchronous call, returning the work to be driven.
    ///
    /// The argument checking inside `body` happens here, on the main thread and
    /// inside the caller's scope, which is why this is fallible separately from
    /// the future it returns.
    pub fn begin(
        &self,
        function: &str,
        arguments: &HostArguments,
    ) -> Result<HostFuture, HostError> {
        let Some(body) = self.async_functions.get(function) else {
            return Err(self.no_such_function(function));
        };
        body(arguments)
    }

    fn no_such_function(&self, function: &str) -> HostError {
        HostError::new(format!(
            "HostModule `{}` has no function `{function}`; it provides: {}",
            self.name,
            list(&self.function_names())
        ))
    }
}

/// The specifiers a host may not register, because something else answers them.
///
/// Two groups, and the reason is the same for both: the resolver chain reaches
/// them before it reaches this registry, so a HostModule wearing one of these
/// names is unreachable rather than overriding. The engine asserts this list
/// against its own resolvers, so a module added there and not here is a failing
/// test rather than a name a host can quietly lose.
pub const RESERVED_SPECIFIERS: &[&str] = &[
    // The runtime's own modules.
    "gpui-kit",
    "gpui",
    "gpui-base",
    "gpui-component",
    "gpui-shell",
    "gpui-fps",
    // The Standard Runtime.
    "buffer",
    "console",
    "crypto",
    "fs/promises",
    "net",
    "os",
    "path",
    "process",
    "url",
    "websocket",
    "zlib",
];

/// Allocates the identity of one registry. See [`HostModules::generation`].
fn next_generation() -> u64 {
    thread_local! {
        static NEXT: Cell<u64> = const { Cell::new(1) };
    }
    NEXT.with(|next| {
        let generation = next.get();
        next.set(generation.wrapping_add(1));
        generation
    })
}

/// Every HostModule a host has granted.
///
/// Crate-internal: a host builds [`HostModule`]s and hands them over one at a
/// time, through [`crate::export_module`] or [`crate::Policy::with_host_module`].
/// Registering several at once is rare enough that a second public type for it
/// would cost more than it saved.
///
/// Empty by default, which denies everything.
#[derive(Clone)]
pub(crate) struct HostModules {
    modules: BTreeMap<String, HostModule>,
    generation: u64,
}

impl Default for HostModules {
    fn default() -> Self {
        Self {
            modules: BTreeMap::new(),
            generation: next_generation(),
        }
    }
}

impl HostModules {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// What distinguishes this registry from every other one built on this
    /// thread.
    ///
    /// The engine caches a linked module by its resolved name, for the lifetime
    /// of the runtime — so two plugins importing `workspace`, or one host adding
    /// a module after another was linked, would otherwise share whichever module
    /// was linked first, exports and all. Tagging the resolved name with this
    /// makes each version of the registry a distinct module as far as that cache
    /// is concerned, the same trick a reload uses to re-read an application's
    /// own files.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Adds one module, and takes a new identity.
    ///
    /// The identity changes because the set of importable names just did: a
    /// module linked against the old set must not be served for the new one.
    ///
    /// Registering the same name twice replaces the earlier module rather than
    /// merging into it: two registrations of one name are a mistake, and
    /// merging would hide it behind a module that half works.
    pub(crate) fn insert(&mut self, module: HostModule) {
        self.modules.insert(module.name.clone(), module);
        self.generation = next_generation();
    }

    /// Whether this host has granted anything at all. Read by the tests that
    /// check a refusal left nothing installed; `get` asks the map directly, so
    /// that its two failure sentences stay side by side.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub(crate) fn module_names(&self) -> Vec<&str> {
        self.modules.keys().map(String::as_str).collect()
    }

    /// Looks a module up, reporting a miss against the granted set.
    ///
    /// The two failures are different facts and get different sentences: a host
    /// that granted nothing is a host that has not wired its extension surface
    /// up, and telling that author "unknown module" would send them hunting for
    /// a typo that is not there.
    pub(crate) fn get(&self, name: &str) -> Result<&HostModule, HostError> {
        if let Some(module) = self.modules.get(name) {
            return Ok(module);
        }

        Err(HostError::new(if self.modules.is_empty() {
            format!(
                "HostModule `{name}` is not available: this Host registered none. \
                 HostModule access is granted by the embedding application, with \
                 gpui_shell::export_module(...)."
            )
        } else {
            format!(
                "unknown HostModule `{name}`; this Host registered: {}",
                list(&self.module_names())
            )
        }))
    }

    /// Resolves and calls in one step. Reached through [`dispatch`], which adds
    /// the re-entry guard.
    pub(crate) fn call(
        &self,
        module: &str,
        function: &str,
        arguments: &HostArguments,
    ) -> HostResult {
        self.get(module)?.call(function, arguments)
    }
}

fn list(names: &[&str]) -> String {
    if names.is_empty() {
        "nothing".to_owned()
    } else {
        names.join(", ")
    }
}

thread_local! {
    /// The installed registry.
    ///
    /// Thread-local for the same reason the capability grant is: the VM and
    /// Depth guard for [`dispatch`].
    static IN_CALL: Cell<bool> = const { Cell::new(false) };
}

/// Adds one module to the set a script may import, replacing a same-named one.
///
/// Must be called before the application is loaded, because a script's imports
/// are resolved against this registry while its module graph is linked. Only
/// the set of names is fixed at that point: an export forwards through
/// [`dispatch`] on every call, so revoking a module takes effect on the next
/// call rather than on the next restart.
pub(crate) fn add_module(module: HostModule) -> Result<(), HostError> {
    module.validate()?;
    crate::policy::update_default(move |policy| policy.with_module_unchecked(module));
    Ok(())
}

/// Removes every installed module.
///
/// A host closure typically captures a GPUI entity handle — that is how a host
/// function reaches host state at all — so the registry keeps those handles
/// alive for as long as it holds the closure. A host that goes away without
/// clearing leaves them registered, which GPUI reports as a leaked handle at
/// shutdown and which would be a real leak for a plugin host that unloads and
/// reloads.
///
/// So clearing is the installer's job, in the same place it would drop anything
/// else it owns.
pub(crate) fn clear_modules() {
    crate::policy::update_default(|policy| policy.without_host_modules());
}

/// The registry the code now running may reach.
///
/// Read through the calling frame, so a plugin sees the modules its own host
/// registered for it rather than whichever set was installed most recently.
pub(crate) fn modules() -> Rc<HostModules> {
    crate::scope::policy().modules()
}

/// The one path from an engine into host code.
///
/// Refuses a nested call: a host function that has found a way to run script
/// code — and so to reach a second host function — has re-entered the engine,
/// which the module header explains is not allowed. Reporting it is the whole
/// value here; the alternative is a re-entrant render pass that fails somewhere
/// else entirely.
pub(crate) fn dispatch(module: &str, function: &str, arguments: &HostArguments) -> HostResult {
    if IN_CALL.with(Cell::get) {
        return Err(HostError::new(format!(
            "`{module}.{function}` was reached from inside another host call: \
             a host function may not call back into the script engine"
        )));
    }

    let registry = modules();
    let _guard = CallGuard::enter();
    registry.call(module, function, arguments)
}

/// Starts an asynchronous call, under the same guard.
///
/// The guard covers the synchronous half only — checking the arguments and
/// building the future. The future itself is driven on another thread, where
/// there is no engine to re-enter, so nothing needs to be held across it. That
/// is also why a nested *asynchronous* call is not a special case: the body
/// that would make one is not running under this guard by the time it could.
pub(crate) fn dispatch_async(
    module: &str,
    function: &str,
    arguments: &HostArguments,
) -> Result<HostFuture, HostError> {
    if IN_CALL.with(Cell::get) {
        return Err(HostError::new(format!(
            "`{module}.{function}` was reached from inside another host call: \
             a host function may not call back into the script engine"
        )));
    }

    let registry = modules();
    let _guard = CallGuard::enter();
    registry.get(module)?.begin(function, arguments)
}

/// Clears the depth guard however the call leaves — returned, failed, or
/// unwound. A flag left set would deny every later call.
struct CallGuard;

impl CallGuard {
    fn enter() -> Self {
        IN_CALL.with(|flag| flag.set(true));
        Self
    }
}

impl Drop for CallGuard {
    fn drop(&mut self) {
        IN_CALL.with(|flag| flag.set(false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> HostModules {
        let mut modules = HostModules::new();
        modules.insert(
            HostModule::new("workspace")
                .function("project_name", |_| Ok(HostValue::from("gpui-component")))
                .function("open_count", |arguments| {
                    Ok(HostValue::from(arguments.len()))
                })
                .function("echo", |arguments| Ok(arguments.value(0)?.clone()))
                .function("close", |arguments| {
                    let id = arguments.integer(0)?;
                    Err(HostError::new(format!("tab {id} is already closed")))
                }),
        );
        modules
            .insert(HostModule::new("editor").function("line_count", |_| Ok(HostValue::from(12))));
        modules
    }

    #[test]
    fn a_registered_function_is_callable_and_returns_its_value() {
        let modules = registry();
        assert_eq!(
            modules
                .call("workspace", "project_name", &HostArguments::default())
                .unwrap(),
            HostValue::Str("gpui-component".into())
        );
        assert_eq!(
            modules
                .call(
                    "workspace",
                    "open_count",
                    &HostArguments::new([HostValue::from(1), HostValue::from(2)])
                )
                .unwrap(),
            HostValue::Number(2.)
        );
    }

    #[test]
    fn an_unregistered_module_reports_the_registered_ones() {
        let error = registry()
            .call("workspce", "project_name", &HostArguments::default())
            .unwrap_err();
        assert_eq!(
            error.message(),
            "unknown HostModule `workspce`; this Host registered: editor, workspace"
        );
    }

    #[test]
    fn an_empty_registry_says_the_host_granted_nothing() {
        let error = HostModules::new()
            .call("workspace", "project_name", &HostArguments::default())
            .unwrap_err();
        // Both halves of the sentence are the invariant, not incidental
        // wording: a host that granted nothing has not wired its extension
        // surface up, and telling that author "unknown module" would send them
        // hunting for a typo that is not there. So the message has to say which
        // of the two failures this is, and where to go to fix it.
        assert!(
            error.message().contains("this Host registered none"),
            "an empty registry must not read like an unknown name: {}",
            error.message()
        );
        assert!(
            error.message().contains("export_module"),
            "the refusal must name the entry point that fixes it: {}",
            error.message()
        );
    }

    #[test]
    fn an_unknown_function_reports_what_the_module_provides() {
        let error = registry()
            .call("editor", "line_cont", &HostArguments::default())
            .unwrap_err();
        assert_eq!(
            error.message(),
            "HostModule `editor` has no function `line_cont`; it provides: line_count"
        );
    }

    #[test]
    fn a_failing_function_surfaces_its_message() {
        let error = registry()
            .call(
                "workspace",
                "close",
                &HostArguments::new([HostValue::from(7)]),
            )
            .unwrap_err();
        assert_eq!(error.message(), "tab 7 is already closed");
    }

    #[test]
    fn a_mistyped_argument_names_its_position_and_what_arrived() {
        let error = registry()
            .call(
                "workspace",
                "close",
                &HostArguments::new([HostValue::from("seven")]),
            )
            .unwrap_err();
        assert_eq!(error.message(), "argument 1 must be a number, got a string");
    }

    #[test]
    fn a_nested_value_round_trips_through_the_boundary_type() {
        let value: HostValue = HostObject::new()
            .field("name", "release")
            .field("done", true)
            .field("progress", 0.5)
            .field("owner", None::<String>)
            .field(
                "steps",
                vec![
                    HostValue::from(HostObject::new().field("id", 1).field("title", "Tag")),
                    HostValue::from(HostObject::new().field("id", 2).field("title", "Ship")),
                ],
            )
            .into();

        let returned = registry()
            .call("workspace", "echo", &HostArguments::new([value.clone()]))
            .unwrap();

        assert_eq!(returned, value);
        assert_eq!(
            returned.get("name").and_then(HostValue::as_str),
            Some("release")
        );
        assert!(returned.get("owner").is_some_and(HostValue::is_null));
        assert_eq!(
            returned
                .get("steps")
                .and_then(HostValue::as_array)
                .and_then(|steps| steps[1].get("title"))
                .and_then(HostValue::as_str),
            Some("Ship")
        );
    }

    #[test]
    fn a_repeated_field_replaces_the_earlier_value_in_place() {
        let value: HostValue = HostObject::new()
            .field("id", 1)
            .field("title", "Tag")
            .field("id", 2)
            .into();

        assert_eq!(
            value,
            HostValue::Object(vec![
                ("id".into(), HostValue::Number(2.)),
                ("title".into(), HostValue::Str("Tag".into())),
            ])
        );
    }

    #[test]
    fn a_reserved_name_is_refused_and_says_who_owns_it() {
        assert!(
            HostModule::new("market")
                .function("quotes", |_| Ok(HostValue::Null))
                .validate()
                .is_ok()
        );

        let error = HostModule::new("path")
            .function("join", |_| Ok(HostValue::Null))
            .validate()
            .unwrap_err();
        assert!(
            error.message().contains("the runtime's own module names"),
            "{}",
            error.message()
        );
        assert!(error.message().contains("websocket"), "{}", error.message());
    }

    /// Two registries are two modules to the engine's module cache, which is
    /// what keeps one plugin's `workspace` from being served to another.
    #[test]
    fn every_registry_has_its_own_generation() {
        let first = HostModules::new();
        let second = HostModules::new();
        assert_ne!(first.generation(), second.generation());
    }

    #[test]
    fn exporting_a_reserved_name_fails_rather_than_silently_dropping_it() {
        assert!(
            add_module(HostModule::new("console").function("log", |_| Ok(HostValue::Null)))
                .is_err()
        );
        assert!(
            modules().is_empty(),
            "the refused module must not be installed"
        );
    }

    #[test]
    fn a_declared_face_that_matches_the_registry_passes() {
        let module = HostModule::new("market")
            .function("quotes", |_| Ok(HostValue::Null))
            .function("watch", |_| Ok(HostValue::Null))
            .declarations(
                "export interface Quote { symbol: string }\n\
                 export function quotes(): Quote[];\n\
                 export function watch(symbol: string): boolean;\n",
            );
        module
            .validate()
            .expect("a face that names exactly the registered functions is valid");
    }

    /// The drift this check exists for: the host renamed a function and the
    /// declarations still describe the old one.
    #[test]
    fn a_declared_face_that_drifts_names_both_sides() {
        let error = HostModule::new("market")
            .function("quotes", |_| Ok(HostValue::Null))
            .declarations("export function prices(): unknown[];\n")
            .validate()
            .unwrap_err();
        assert!(
            error
                .message()
                .contains("registered but not declared: quotes"),
            "{}",
            error.message()
        );
        assert!(
            error
                .message()
                .contains("declared but not registered: prices"),
            "{}",
            error.message()
        );
    }

    /// A helper type is not an export, and must not be read as a missing one.
    #[test]
    fn only_exported_functions_are_read_as_exports() {
        let module = HostModule::new("market")
            .function("quotes", |_| Ok(HostValue::Null))
            .declarations(
                "// export function commented(): void;\n\
                 export interface Quote { symbol: string }\n\
                 type Row = Quote;\n\
                 export function quotes(): Quote[];\n",
            );
        assert!(module.validate().is_ok(), "{:?}", module.validate());
    }

    #[test]
    fn registering_a_name_twice_replaces_the_module() {
        let mut modules = HostModules::new();
        modules.insert(HostModule::new("workspace").function("first", |_| Ok(HostValue::Null)));
        modules.insert(HostModule::new("workspace").function("second", |_| Ok(HostValue::Null)));

        assert_eq!(
            modules.get("workspace").unwrap().function_names(),
            vec!["second"]
        );
    }

    /// Adding a module changes which names are importable, so the registry has
    /// to become a different one as far as the engine's module cache is
    /// concerned.
    #[test]
    fn adding_a_module_takes_a_new_generation() {
        let mut modules = HostModules::new();
        let before = modules.generation();
        modules.insert(HostModule::new("workspace"));
        assert_ne!(modules.generation(), before);
    }

    #[test]
    fn the_installed_registry_is_what_dispatch_calls() {
        add_module(HostModule::new("editor").function("line_count", |_| Ok(HostValue::from(12))))
            .unwrap();

        assert_eq!(
            dispatch("editor", "line_count", &HostArguments::default()).unwrap(),
            HostValue::Number(12.)
        );

        clear_modules();
        assert!(
            modules().is_empty(),
            "clearing must actually revoke: a module left registered keeps its \
             closure's GPUI handles alive past the host that installed them"
        );
    }

    #[test]
    fn a_host_function_cannot_reach_a_second_one() {
        add_module(
            HostModule::new("loop")
                .function("outer", |_| {
                    dispatch("loop", "inner", &HostArguments::default())
                })
                .function("inner", |_| Ok(HostValue::Null)),
        )
        .unwrap();

        let error = dispatch("loop", "outer", &HostArguments::default()).unwrap_err();
        assert!(
            error
                .message()
                .contains("may not call back into the script engine"),
            "{}",
            error.message()
        );

        // The guard is released even after a refusal, so the next call works.
        assert_eq!(
            dispatch("loop", "inner", &HostArguments::default()).unwrap(),
            HostValue::Null
        );

        clear_modules();
    }
}

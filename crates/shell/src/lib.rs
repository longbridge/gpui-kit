//! GPUI Shell — a scriptable application runtime built on `gpui-base`.
//!
//! > **Experimental:** scripting interfaces, Standard Runtime compatibility,
//! > capability semantics and module behavior may change between minor
//! > releases.
//!
//! The host owns rendering, layout, text editing, virtualization and system
//! capabilities; the script owns composition, presentation and business logic. See
//! `docs/gpui-shell.md` for the design this implements.
//!
//! # Cargo feature impact
//!
//! The JavaScript fluent-style surface is generated from GPUI's inspector
//! reflection table, including in release builds. This crate therefore enables
//! `gpui-base/inspector`, which Cargo unifies across the embedding application's
//! dependency graph.

// # The surface a host may rely on
//
// Everything `pub` here is a promise. That is the point of the list being
// short: a module published because one item across a crate boundary needed it
// is not thereby an interface, and the crate spent a while with `engine`,
// `spec`, `materialize` and `scope` open for exactly that reason — an external
// test needed them. Tests moved inside instead (`src/tests`), because an
// integration test is a consumer like any other and a reason to publish an
// internal representation is still a reason to publish it.
//
// **What a host may name.** The root: `init`, the `set_*` entry points,
// `on_exit_request`, `resolve_app_root`, `failure_surface`, and the types
// re-exported below. The modules: `host_modules` and `policy` to configure what a
// script may reach, `root` and `theme` for the window it lives in, `view` and
// `snapshot` for the view itself, `metrics` to measure. Hot reload is exposed
// through `ShellRuntime::watch`; its watcher implementation remains internal.
// `write_type_declarations_with_components` is the explicit tooling hook for `gpui-kit.d.ts`;
// ordinary application loading updates it automatically.
//
// **Crate-private, and why.** `engine` is the seam and its shape follows
// whatever is behind it. `spec`, `materialize`, `store`, `style` and `a11y` are
// an internal representation. `capability` publishes `Capabilities` and
// `ExecuteGrant` through the root and keeps the resolver — `Access`, `Grant` —
// to itself. `scope` publishes `with_current_app`, which is how a HostModule
// reaches the ambient `App`, and hides the frame stack. `scroll` is the one
// scroll area `materialize` needs, kept here because the shell builds on
// `gpui-base` alone and cannot borrow `gpui-component`'s copy. `runtime`,
// `error` and `assets` publish their types through the root.
//
// **Public because a script drives it.** `dock`. A script contributes panels
// and draws a dock's chrome through the engine, and a Rust host can build a
// `ScriptPanel` or install a `ScriptDockSkin` over the same seam. The plugin
// manifest is public because the shipped binary now applies a local
// application's declared capabilities before loading it.
//
// **Not reachable at all.** `value` and `entities`: a `Bridged` and an entity
// handle are the runtime talking to itself.
pub(crate) mod a11y;
pub mod action;
pub(crate) mod assets;
pub(crate) mod capability;
mod component;
mod component_registry;
pub(crate) mod dependencies;
pub mod dock;
pub(crate) mod engine;
pub(crate) mod entities;
pub(crate) mod error;
pub mod host;
pub mod host_modules;
pub(crate) mod materialize;
pub mod metrics;
pub(crate) mod path;
pub mod plugin;
pub mod policy;
pub(crate) mod process;
pub mod root;
pub(crate) mod runtime;
pub(crate) mod scope;
mod script_callback;
pub(crate) mod scroll;
pub mod snapshot;
pub(crate) mod spec;
pub(crate) mod storage;
pub(crate) mod style;
#[cfg(test)]
mod tests;
pub(crate) mod theme_tokens;
mod typings;
pub(crate) mod value;
pub mod view;
pub(crate) mod watch;

pub use anyhow;
pub use assets::AppAssets;
pub use capability::{Capabilities, ExecuteGrant, HttpRequestGrant};
pub use component::ComponentArgs;
#[cfg(test)]
pub(crate) use component_registry::ComponentState;
pub use component_registry::{
    ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentArgument,
    ComponentCallback, ComponentCallbackArgument, ComponentDataCallback, ComponentDataValue,
    ComponentDelegateSnapshot, ComponentDescriptor, ComponentElementCallback,
    ComponentElementFactory, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, DEFAULT_COMPONENT_MODULE, FrozenComponentRegistry, MaterializeRequest,
    MethodDescriptor, RegistryError, StateDescriptor,
};
pub(crate) use component_registry::{ComponentCallbackValue, ComponentId, RecordedComponentMethod};
pub use engine::{LoadedApplication, ShellRuntime};
pub use error::ShellError;
pub use gpui;
pub use host_modules::{
    HostArguments, HostError, HostModule, HostObject, HostResult, HostValue, RESERVED_SPECIFIERS,
};
pub use metrics::RuntimeMetrics;
pub use root::{DialogOptions, ShellRoot, ToastLevel, ToastRequest};
pub use runtime::{
    ExitHandler, ExitRequest, clear_exit_handler, failure_surface, on_exit_request,
    resolve_app_root,
};
pub use scope::{ScopePhase, with_current_app};
pub use snapshot::RenderSnapshot;
pub use view::ScriptView;
pub use watch::Watcher;

use std::path::PathBuf;

use gpui::App;

/// Returns declarations for one frozen component registry without installing
/// process-global component state.
pub fn type_declarations(components: &FrozenComponentRegistry) -> String {
    typings::declarations_with_components(components)
}

/// Writes declarations for one frozen component registry into an application
/// tree without changing another runtime's catalog.
pub fn write_type_declarations_with_components(
    root: &std::path::Path,
    components: &FrozenComponentRegistry,
) -> std::io::Result<Vec<PathBuf>> {
    typings::write_application_with_components(root, components)
}

/// Links an application's declared Git dependencies where an editor finds them.
///
/// `gpui-kit.d.ts` describes the runtime; this describes the packages the manifest
/// adds to it. Without it `import { style } from "omarchy-ui"` is a module an
/// editor cannot resolve, so the names behind it have no types, no parameter
/// hints and no documentation even though the runtime resolves them fine.
///
/// The dependencies are fetched if the cache does not already hold them, which
/// is why this is separate from [`write_type_declarations`]: one writes a file,
/// the other may reach the network. [`ShellRuntime::load`] does both, so an
/// ordinary host needs neither. This explicit operation exists for tooling such
/// as `gpui-shell types` that must report a failure to its caller.
///
/// An application without a manifest, or one that declares no dependencies, has
/// nothing to link and is not an error. Returns the links that were written.
pub fn write_dependency_links(root: &std::path::Path) -> anyhow::Result<Vec<PathBuf>> {
    if !root.join(plugin::MANIFEST_FILE).is_file() {
        return Ok(Vec::new());
    }
    let manifest = plugin::PluginManifest::read(root)?;
    if manifest.dependencies().is_empty() {
        return Ok(Vec::new());
    }
    let store = dependencies::GitDependencyStore::for_user()?;
    let dependencies = store.materialize_all(&manifest)?;
    store.link_for_editor(root, &dependencies)
}

/// Grants an application its capabilities.
///
/// Nothing is permitted until this is called: a script gets no file, storage,
/// clipboard or process access by default (design doc §5.7). The host decides,
/// because only the host knows how much the code it is about to run is trusted.
///
/// The grant lives above the engine seam, so no engine can be built that
/// quietly ignores it. It sets the *default* policy — what a call inherits when
/// nothing narrower is in force. A host running several applications at once
/// gives each its own [`policy::Policy`] instead, so that two of them can hold
/// two grants at the same time.
pub fn set_capabilities(capabilities: Capabilities) {
    capability::install(capabilities);
}

/// Names the application, and puts its data where that name says.
///
/// **This is how a host should place storage.** The bundle id is the
/// application's identity, so its data survives the directory being renamed,
/// moved, or replaced by an upgrade — which is what a user means by "my
/// settings". Keying on the path instead means an upgrade silently starts the
/// user over.
///
/// The id is the host's to decide and the runtime does not go looking for it in
/// a file: only the layer that installed the application knows what it is
/// called, and a runtime that read it out of a manifest of its own choosing
/// would be claiming authority over something it does not own.
///
/// Returns the directory it chose, because a host that grants filesystem access
/// needs to name it. The store is one file inside, which leaves room for other
/// per-application state later.
///
/// The id also becomes the namespace this application's dock panels persist
/// under — `shell:<id>/<panel>` — for the same reason it places storage: a
/// layout file has to find the panel again after a restart, and only a name
/// survives a move.
///
/// ```rust,ignore
/// let data = gpui_shell::set_bundle_id("com.example.notes")?;
/// gpui_shell::set_capabilities(Capabilities::new().write_roots([data]));
/// ```
///
/// A host running a directory it was pointed at — a command line, a dev
/// server — has no such name, and passing the path is right there: the path is
/// the identity while you are editing something. [`bundle_id_for_path`] builds
/// one.
///
/// Fails when the id could reach outside the data directory: it is joined onto
/// it, so `a-z`, `0-9`, `.`, `-`, `_` and no `..`.
pub fn set_bundle_id(id: &str) -> anyhow::Result<PathBuf> {
    let directory = runtime::app_data_dir(id)?;
    set_storage_path(directory.join("store.json"));
    // The same name namespaces the application's dock panels. Storage and a
    // persisted layout are the two things that have to survive a restart under
    // a name rather than a path, so they take the name from one place.
    policy::update_default(|policy| policy.with_application(id));
    Ok(directory)
}

/// A bundle id for a directory that has no name of its own.
///
/// The directory name with a digest of its full path: the same directory always
/// reaches the same data, and two never collide — including two checkouts of one
/// source, which really are two installations of something being edited.
pub fn bundle_id_for_path(root: &std::path::Path) -> String {
    runtime::path_identity(root)
}

/// Points `localStorage` at an exact file.
///
/// The mechanism under [`set_bundle_id`], which is what a host should normally
/// call. This is for a host that places its own data — a test, or an embedder
/// with its own layout.
///
/// Storage is per application, and the host chooses where that is — an
/// application cannot name its own storage location, or two applications could
/// collide on purpose. Like [`set_capabilities`], this configures the default
/// policy.
pub fn set_storage_path(path: PathBuf) {
    engine::set_storage_path(path);
}

/// Relaxes the sandbox for a development session.
///
/// Restores `eval` and unfreezes the built-in prototypes, which a REPL needs
/// and a shipped application must not have.
pub fn set_development_mode(enabled: bool) {
    engine::set_development_mode(enabled);
}

/// Initializes the base layer and style reflection table.
///
/// Must be called once at application startup, before any script runs. This is
/// enough for the bare runtime. A host carrying a component catalog calls
/// [`init_with_components`] instead, so the catalog's own globals are
/// installed too.
pub fn init(cx: &mut App) {
    gpui_base::init(cx);
    style::init();
}

/// Initializes the runtime and the catalog it will render.
///
/// The runtime still knows nothing about any component library: it calls the
/// startup function the catalog registered with
/// [`ComponentRegistry::with_initializer`] and nothing else. A catalog that
/// registered none behaves exactly like [`init`].
pub fn init_with_components(cx: &mut App, components: &FrozenComponentRegistry) {
    if let Some(initializer) = components.initializer() {
        initializer(cx);
    }
    init(cx);
}

#[cfg(test)]
mod init_tests {
    use gpui::TestAppContext;

    #[gpui::test]
    fn shell_init_installs_the_base_globals_without_a_component_catalog(cx: &mut TestAppContext) {
        cx.update(super::init);

        cx.read(|cx| assert!(cx.has_global::<gpui_base::Theme>()));
    }

    /// A host that only holds a frozen catalog — the shipped command — has no
    /// other way to install what that catalog's components need at run time.
    #[gpui::test]
    fn init_with_components_runs_the_catalog_initializer(cx: &mut TestAppContext) {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static CALLS: AtomicUsize = AtomicUsize::new(0);
        fn record(_: &mut gpui::App) {
            CALLS.fetch_add(1, Ordering::SeqCst);
        }

        let components = crate::ComponentRegistry::new(
            crate::COMPONENT_REGISTRY_API_VERSION,
            crate::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap()
        .with_initializer(record)
        .freeze()
        .unwrap();

        cx.update(|cx| crate::init_with_components(cx, &components));

        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        cx.read(|cx| assert!(cx.has_global::<gpui_base::Theme>()));
    }

    #[gpui::test]
    fn init_with_components_matches_init_for_a_catalog_without_one(cx: &mut TestAppContext) {
        let components = crate::FrozenComponentRegistry::default();
        assert!(components.initializer().is_none());

        cx.update(|cx| crate::init_with_components(cx, &components));

        cx.read(|cx| assert!(cx.has_global::<gpui_base::Theme>()));
    }
}

/// Exports one Rust module for scripts to import by name.
///
/// This is the host's whole extension surface. A script cannot load a native
/// extension — `dlopen`ed Rust holds every permission the process holds, so a
/// sandbox that permits it does not mean anything — and instead reaches exactly
/// the modules registered here, and nothing else (design doc §17.6).
///
/// ```no_run
/// use gpui_shell::{HostModule, HostValue};
///
/// gpui_shell::export_module(
///     HostModule::new("workspace")
///         .declarations(
///             r#"
///             export function project_name(): string;
///             export function version(): string;
///             "#,
///         )
///         .function("project_name", |_| Ok(HostValue::from("gpui-component")))
///         .function("version", |_| Ok(HostValue::from("0.1.0"))),
/// )?;
/// # Ok::<(), gpui_shell::HostError>(())
/// ```
///
/// A script imports that by name, the way it imports `gpui-kit` or `path`:
///
/// ```js
/// import { project_name } from "workspace";
/// ```
///
/// # Slow work
///
/// [`HostModule::function`] is synchronous, so a slow one holds the thread that
/// renders. [`HostModule::async_function`] takes a closure that returns a
/// future instead: the script gets a promise, and the work runs on GPUI's
/// background executor.
///
/// # Call it before loading the application
///
/// An import is resolved while the module graph is linked, so a module exported
/// after `load_app` is not in the graph — a script importing it fails to link,
/// naming the modules that do exist. Registration is start-up work.
///
/// Replacing or withdrawing a module *afterwards* is a different matter and
/// does work: every export is a forwarding stub that resolves through the
/// registry on each call, so a script holding a function it imported earlier
/// gets the new behavior, or a refusal, on its next call. What an import fixes
/// is the set of names, not the functions behind them.
///
/// # One module per call
///
/// Exporting a name twice replaces the earlier module rather than merging into
/// it: two registrations of one name are a mistake, and merging would hide it
/// behind a module that half works. Call this once per module.
///
/// # Give it a TypeScript face
///
/// [`HostModule::declarations`] is optional but worth writing. With it, the
/// generated `gpui-kit.d.ts` describes the module exactly, and this function checks
/// that description against the functions actually registered — so renaming one
/// half is a sentence at start-up rather than an editor that keeps completing a
/// function you deleted. Without it, the module is still declared, with
/// `(...args: any[]) => any` signatures that check the module name and every
/// export name but nothing further.
///
/// # A host that runs more than one application
///
/// This installs into the policy new views are born holding, which is one
/// thread-local set — right for a host running a single application. A host
/// running several gives each its own [`policy::Policy`] and builds the
/// registry with [`policy::Policy::with_host_module`] instead, so one plugin's
/// modules are not reachable from another's script.
///
/// # Withdraw before going away
///
/// A module's closures typically capture GPUI entity handles — that is how a
/// host function reaches host state at all — so the registry keeps those
/// handles alive for as long as it holds them. A host that goes away without
/// calling [`clear_exported_modules`] leaves them registered, which GPUI
/// reports as a leaked handle at shutdown.
///
/// # Errors
///
/// Two, both reported before anything is installed:
///
/// * The name belongs to the runtime — see [`RESERVED_SPECIFIERS`]. The
///   built-ins and the Standard Runtime resolve first, so such a module could
///   never be imported; refusing it here is what stops that being silent.
/// * The declared exports and the registered functions disagree, naming both
///   sides of the difference.
pub fn export_module(module: HostModule) -> Result<(), HostError> {
    host_modules::add_module(module)
}

/// Withdraws every module [`export_module`] installed.
///
/// Takes effect immediately, including for a script that has already imported
/// one: the next call through an imported name is refused rather than answered
/// by a withdrawn closure.
///
/// A host that registered modules capturing GPUI entities should call this when
/// it goes away — see "Withdraw before going away" on [`export_module`].
pub fn clear_exported_modules() {
    host_modules::clear_modules();
}

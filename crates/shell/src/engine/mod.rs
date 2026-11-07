//! Where the scripting engine is confined.
//!
//! Everything above this module — the snapshot, the spec arena, the
//! materializer, the call scope, the style table, the theme, the capability
//! model — is engine independent. Only this module knows what a script value is.
//!
//! # The surface the rest of the crate calls
//!
//! One type, `ShellRuntime`, with exactly this shape. Nothing outside this
//! directory calls anything else:
//!
//! ```text
//! ShellRuntime::new(&mut App) -> anyhow::Result<Rc<Self>>
//! ShellRuntime::new_isolated() -> anyhow::Result<Rc<Self>>
//! ShellRuntime::arena_mut(&self) -> RefMut<'_, SpecArena>
//!
//! ShellRuntime::load_app(&Rc<Self>, &Path, entry: &str) -> anyhow::Result<ViewType>
//! ShellRuntime::load_source(&Rc<Self>, &str, &str) -> anyhow::Result<ViewType>
//! ShellRuntime::instantiate(&Rc<Self>, &ViewType) -> anyhow::Result<ViewObject>
//! ShellRuntime::instantiate_view(&Rc<Self>, &ViewType, &mut Window, &mut App)
//!     -> anyhow::Result<Entity<ScriptView>>
//! ShellRuntime::instantiate_view_with_policy(&Rc<Self>, &ViewType, Rc<Policy>,
//!     &mut Window, &mut App) -> anyhow::Result<Entity<ScriptView>>
//! ShellRuntime::instantiate_for_view(&Rc<Self>, &ViewType, Entity<ScriptView>,
//!     &mut Window, &mut App) -> anyhow::Result<ViewObject>
//!
//! ShellRuntime::build_snapshot(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
//!     &mut Window, &mut App) -> anyhow::Result<RenderSnapshot>
//! ShellRuntime::render_to_spec(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
//!     &mut Window, &mut App) -> anyhow::Result<String>
//! ShellRuntime::retire_callbacks(&self, generation: u64)
//! ShellRuntime::script_renders(&self) -> u64
//!
//! ShellRuntime::dispatch_click(&Rc<Self>, CallbackId, &ClickEvent, &mut Window, &mut App)
//! ShellRuntime::dispatch_change(&Rc<Self>, CallbackId, bool, &mut Window, &mut App)
//! ShellRuntime::dispatch_index(&Rc<Self>, CallbackId, usize, &mut Window, &mut App)
//! ShellRuntime::dispatch_signal(&Rc<Self>, CallbackId, &mut Window, &mut App)
//! ShellRuntime::render_virtual_items(&Rc<Self>, CallbackId, Range<usize>,
//!     &mut Window, &mut App) -> Option<ItemSpecs>
//! ShellRuntime::item_keys(&Rc<Self>, CallbackId, &mut Window, &mut App,
//!     FnOnce(&mut dyn FnMut(usize) -> Option<String>) -> R) -> Option<R>
//! ShellRuntime::dispatch_visible_range(&Rc<Self>, CallbackId, Range<usize>,
//!     usize, &mut Window, &mut App)
//!
//! set_storage_path(PathBuf)
//! set_development_mode(bool)
//! ```
//!
//! # Host configuration goes through here, or above it
//!
//! The two module-level functions are part of the contract for a reason. They
//! used to be called from the crate root straight into the QuickJS module, with
//! a silent no-op for any other build — so a second engine could compile, run,
//! and ignore host configuration without a word. There is no fallback now: an
//! engine either provides them or does not build.
//!
//! Capabilities went further and left this file entirely. A grant is a decision
//! about the *application*, not about the interpreter, so it lives in
//! [`crate::capability`] where an engine can read it and cannot answer it.
//!
//! plus the handle types `ViewType` and `ViewObject`, which every caller treats
//! as opaque — though nothing in the type system makes them so, which is part of
//! the point below.
//!
//! `arena_mut` is the *scratch* arena the script builder records into during a
//! `build_snapshot` call. It is reset at the start of every build and taken at
//! the end; nothing outside a build should read it. Published descriptions live
//! in [`crate::snapshot::RenderSnapshot`].
//!
//! # The one rule this boundary enforces
//!
//! `build_snapshot` is the only entry into the script's `render`, and nothing
//! calls it per frame. Rendering opportunistically — on a repaint, on a hover,
//! on a timer — would put script cost back on GPUI's frame budget, which is the
//! coupling the whole design exists to prevent. `metrics().script_renders()` is
//! the counter, and benchmark C is the test that fails rather than merely
//! getting slower.
//!
//! `render_virtual_items` is the one deliberate exception, and it is narrow
//! enough to state in a sentence: a virtualized list cannot know which rows
//! exist until GPUI has laid it out, so its item renderer runs from inside
//! layout and costs a frame. It describes a *window* rather than a collection,
//! which is the trade virtualization is; it never enters the script's
//! `render`, so the counter above still means what it says; and it runs under
//! `ScopePhase::Layout`, which forbids `notify`, forbids creating retained
//! state, and refuses to register a handler. `item_keys` is part of the same
//! exception: it is how a `list` asks its `get_key`, under the same scope,
//! which item is at the top before and after its item count changes. See
//! `crate::materialize`.
//!
//! # What this seam is, and what it is not
//!
//! It is a **dependency isolation layer**, and calling it a replaceable-engine
//! contract would be flattering it. `ShellRuntime`, `ViewObject` and `ViewType`
//! are re-exports of QuickJS types, not associated types behind a trait; adding
//! a second engine would mean editing this file, matching a structural surface
//! nothing checks, and discovering by compile error which of the two dozen
//! entry points above it had missed. That is a port, not an implementation of a
//! contract.
//!
//! What the isolation does buy is real, and is the reason to keep it:
//!
//! * **No module above this one names a script value.** Not the snapshot, the
//!   spec arena, the materializer, the call scope, the style table, the theme,
//!   the capability model, the dock, or hot reload. A change to the engine has a
//!   blast radius you can see from the directory listing.
//! * **Host configuration cannot be quietly dropped.** Capabilities live above
//!   the seam entirely, and `set_storage_path` and `set_development_mode` are
//!   entries here with no fallback compiled in — an engine either provides them
//!   or does not build. An earlier arrangement had silent no-ops, which meant a
//!   second engine could ignore the security configuration without a word.
//! * **The render-frequency rule has one enforcement point.** `build_snapshot`
//!   is the only entry into script `render`, and benchmark C fails if a repaint
//!   ever reaches it.
//!
//! Making it an actual contract — an internal trait with associated opaque
//! handles, and a minimal fake engine to compile the contract against — is worth
//! doing when there is a second engine to write, and is make-work before that.
//! The honest description today is the one above.
//!
//! # Why the engine was worth isolating at all
//!
//! The engine choice is the one decision in this runtime that cannot be
//! validated on paper: per-call cost across the language boundary decides
//! whether the whole approach is viable (see `docs/gpui-shell.md` §20). QuickJS
//! is what ships, because application code reads better in JavaScript.

#[cfg(not(feature = "quickjs"))]
compile_error!("enable a scripting engine: `quickjs` is the default and the only one today");

#[cfg(feature = "quickjs")]
pub(crate) mod quickjs;
/// Only `dock`'s tests name it; every other caller passes it straight from
/// `load_app` into `instantiate` without spelling the type.
#[cfg(test)]
pub(crate) use quickjs::ViewType;
#[cfg(feature = "quickjs")]
pub use quickjs::{LoadedApplication, ShellRuntime, ViewObject};

/// Points the script-visible store at its backing file.
///
/// Part of the contract rather than something the crate root reaches into an
/// engine for. There is deliberately no fallback: an engine that cannot honour
/// this does not compile, because the alternative — the one this replaced — was
/// a build that quietly accepted the call and did nothing with it.
#[cfg(feature = "quickjs")]
pub fn set_storage_path(path: std::path::PathBuf) {
    quickjs::host::set_storage_path(path);
}

/// Relaxes the sandbox for a development session.
#[cfg(feature = "quickjs")]
pub fn set_development_mode(enabled: bool) {
    quickjs::sandbox::set_development_mode(enabled);
}

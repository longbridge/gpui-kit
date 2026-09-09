# GPUI Component Shell Integration Design

## Goal

Expose the complete public `gpui-component` component catalog to JavaScript
applications hosted by `gpui-shell`, without implementing those components in
the `gpui-shell` crate. Provide a JavaScript gallery comparable to the Rust
Story application so every binding is visible and exercisable.

The integration will live in a new workspace crate named
`gpui-component-shell`. This avoids a dependency cycle: the adapter can depend
on both `gpui-shell` and `gpui-component`, while neither foundational crate
depends on the adapter library.

## Crate boundaries

### `gpui-shell`

The existing `gpui-shell` package, library, binary, and Rust crate name remain
unchanged. It remains the script runtime and generic host bridge. It owns:

- the JavaScript engine, modules, callbacks, entities, capabilities, and hot
  reload;
- the script-side element description arena and generated type metadata;
- a public component-registration API;
- generic primitives such as `div`, `h_flex`, `v_flex`, text, SVG, input
  events, styles, and window operations;
- dispatch from a described component name to its registered materializer.

It must not import or construct a `gpui-component` control. Existing
`gpui-base` materializers remain part of the base-only host: despite their
historical `materialize/components` directory name, they implement the generic
base surface and do not depend on the themed component crate. Only concrete
`gpui-component` registration and materialization belongs in the adapter. The
shell binary may depend on the adapter to assemble the default executable;
that composition dependency does not put component implementation in the
shell library.

### `gpui-component-shell`

The new adapter crate depends directly on both `gpui-shell` and
`gpui-component`; `gpui-shell` depends only on `gpui-base`. Its
`src/shell/` directory contains one focused module per component family and a
single public registration entry point:

```rust
pub fn register(runtime: &mut gpui_shell::ComponentRegistry);
```

The adapter owns:

- component constructors and builder-method schemas exposed to JavaScript;
- conversion from shell values/specifications to `gpui-component` values;
- retained state creation and lookup for stateful components;
- materialization into real `gpui-component` elements;
- component-specific callbacks, slots, validation, diagnostics, and generated
  TypeScript declarations;
- initialization required by the component library.

Registration is explicit and deterministic. Duplicate component names or
method names are startup errors. The registry is frozen before a script is
loaded so runtime rendering does not mutate global schemas.

### Executable composition

Using arrows from a Cargo consumer to its dependency, the dependency graph is
`app -> gpui-component-shell -> { gpui-shell, gpui-component }`, with
`gpui-shell -> gpui-base` as the base-only path. `gpui-shell` depends on neither
the adapter nor the themed component crate, so Cargo sees no cycle and a
base-only host does not link unused themed controls. Concrete component
registration and materialization remain exclusively in the adapter.

`gpui-component-shell` exposes the full-catalog registration/startup entry
point an application uses before loading scripts. The JavaScript Story host is
the reference composition. A host that wants only base bindings may continue
using `gpui-shell` directly.

## Registration model

Each registered component supplies a descriptor and a materializer. The
descriptor is engine-neutral data containing its JavaScript constructor,
builder methods, accepted values, slots, events, retained-state requirements,
documentation, and TypeScript signatures. QuickJS consumes this metadata to
install the JavaScript API; the type generator consumes the same metadata so
runtime and editor APIs cannot drift.

The render snapshot stores a registered component identifier plus component
data owned through a shell-defined erased payload boundary. The adapter's
materializer receives the resolved style, behavior, children, named slots,
window, application context, and access to shell callbacks/entities. This
keeps recursive arena traversal and snapshot lifetime in `gpui-shell`, while
all knowledge of concrete `gpui-component` types stays in the adapter.

Stateful controls use shell entity handles whose payload behavior is supplied
by the registration. Handle creation, release, generation checking, and script
ownership remain generic shell services; `gpui-component-shell` supplies the
concrete state factory and materializer. Callbacks continue to use snapshot
callback IDs so an older painted frame remains safe during replacement.

## Component coverage

Coverage is defined from two checked-in inventories:

1. public component modules exported by `crates/ui/src/lib.rs`; and
2. user-facing stories exported by `crates/story/src/stories/mod.rs`.

Every user-facing component must have a registration or an explicit inventory
entry classifying it as infrastructure rather than a renderable component.
Complex components such as dialogs, menus, notifications, dock, tables,
trees, editor/input controls, lists, charts, and overlays remain in scope.
Infrastructure modules such as themes, history, and highlighters are covered
through the APIs of the controls that consume them rather than fake visual
constructors.

The migration starts by moving every existing themed-component binding out of
`gpui-shell`; base bindings remain the generic host surface. It is not complete
until searches and dependency checks show that the shell library no longer
references concrete `gpui-component` controls.
New bindings then cover the remainder of the inventory. Unsupported behavior
is not silently ignored: registration or materialization reports a precise
diagnostic naming the component, property, and supported alternative.

## JavaScript Story application

Add `examples/js_story/` as a normal `gpui-shell` application. It contains:

- a navigation sidebar grouped like the Rust Story catalog;
- one JavaScript module per component family;
- interactive examples for normal, focused, selected, disabled, loading,
  validation, overlay, and destructive states where relevant;
- retained demo data for tables, lists, trees, charts, editors, and docks;
- a generated `gpui-kit.d.ts` and `jsconfig.json` for editor validation;
- an index/manifest that makes the complete gallery auditable.

The gallery uses only the public JavaScript API. It must not add Rust host
modules merely to construct a component. Platform-only controls may display a
clear availability panel on unsupported platforms, while still having a real
binding on supported platforms.

## Compatibility and migration

Existing JavaScript constructor and builder names remain compatible wherever
the same component already exists. The move changes ownership, not script
syntax. If an existing name conflicts with the canonical `gpui-component`
name, keep a deprecated alias in registry metadata and emit a migration
warning.

The adapter and shell share a registry API version. A mismatched adapter fails
at startup with a version error rather than producing missing methods during a
render.

## Testing and completion gates

The implementation is complete only when all of the following pass:

- registry unit tests reject duplicates and freeze before script execution;
- adapter tests construct and materialize every registered component;
- callback and retained-state tests cover representative stateless, stateful,
  collection, and overlay components;
- an inventory test proves every public user-facing component/story is mapped
  to a registration or an explicit infrastructure classification;
- generated TypeScript declarations match the registry snapshot;
- existing shell tests pass after concrete component code is removed;
- `cargo check` and targeted tests pass for `gpui-shell`,
  `gpui-component-shell`, and the workspace;
- the JS Story loads through the standard `gpui-shell` command and every
  catalog route builds without a script or materialization error;
- source and dependency audits prove concrete component implementations live
  in `gpui-component-shell`, not in the `gpui-shell` library.

Visual review follows the GPUI Component design guide: semantic theme tokens,
stable element identity, keyboard navigation, visible interaction states, and
correct overlay dismissal/focus restoration are required throughout the JS
gallery.

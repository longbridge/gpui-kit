# GPUI Component Shell Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Register the complete `gpui-component` catalog with `gpui-shell` from an independent adapter crate and ship a JavaScript Story gallery that exercises every binding.

**Architecture:** The existing `gpui-shell` package remains intact, depends on `gpui-base` and `gpui-component`, and owns the engine-neutral registry without implementing concrete bindings. `gpui-component-shell` depends on `gpui-shell`, owns all concrete schemas/state/materializers, and exposes the full-catalog application startup/registration entry point. Runtime JavaScript exports and generated TypeScript declarations are derived from the same descriptors.

**Tech Stack:** Rust 2024, GPUI, gpui-base, gpui-component, rquickjs, schemars/serde, Cargo workspace tests, JavaScript ES modules.

**Spec:** `docs/superpowers/specs/2026-08-29-gpui-component-shell-design.md`

## Global Constraints

- The existing `gpui-shell` package and Rust crate name must not be renamed or split.
- `gpui-shell` must not import or construct concrete `gpui-component` controls in its library implementation.
- `gpui-shell` depends on `gpui-base` and not on `gpui-component`; the adapter owns the themed dependency and every concrete themed binding.
- `gpui-component-shell` depends on `gpui-shell`; `gpui-shell` never depends back on the adapter.
- Base-only applications may continue using `gpui-shell`; full component applications use the adapter startup entry point.
- Existing JavaScript constructor and builder names remain compatible; renamed forms are deprecated aliases with diagnostics.
- Registration is deterministic, rejects duplicate names, and freezes before scripts load.
- Runtime exports and TypeScript declarations come from the same descriptor inventory.
- Every public user-facing UI module and Rust Story has a registered binding or an explicit infrastructure classification.
- The JS gallery uses only public JavaScript APIs and semantic theme/layout values.
- Work in the current checkout; do not create a worktree.

---

### Task 1: Add the frozen component registry to `gpui-shell`

**Files:**
- Create: `crates/shell/src/component_registry.rs`
- Modify: `crates/shell/src/lib.rs`
- Test: `crates/shell/src/component_registry.rs`

**Interfaces:**
- Produces: `ComponentRegistry::new(api_version)`, `register`, `freeze`, `descriptor`, `descriptors`, and `RegistryError`.
- Produces: `ComponentDescriptor`, `ConstructorDescriptor`, `MethodDescriptor`, `TypeScriptDescriptor`, `ComponentId`, `ComponentPayload`, `MaterializeRequest`, and `ComponentMaterializer`.
- Consumes: shell-owned `SpecId`, `Bridged`, callback IDs, child/slot access, `Window`, `App`, and `AnyElement`.

- [ ] **Step 1: Write registry behavior tests**

Add tests that register two descriptors, assert stable insertion-order IDs, reject a duplicate constructor/export/method, freeze successfully, reject registration after freeze, and reject an API version other than `COMPONENT_REGISTRY_API_VERSION`.

```rust
#[test]
fn registry_is_deterministic_and_immutable_after_freeze() {
    let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
    let first = registry.register(test_descriptor("Button", &["button"])).unwrap();
    let second = registry.register(test_descriptor("Badge", &["badge"])).unwrap();
    assert_eq!(first.as_u32(), 0);
    assert_eq!(second.as_u32(), 1);
    let frozen = registry.freeze().unwrap();
    assert_eq!(frozen.descriptors().map(|d| d.name).collect::<Vec<_>>(), ["Button", "Badge"]);
    assert_eq!(frozen.descriptor(first).unwrap().name, "Button");
}

#[test]
fn duplicate_and_late_registrations_are_errors() {
    let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION).unwrap();
    registry.register(test_descriptor("Button", &["button"])).unwrap();
    assert!(matches!(registry.register(test_descriptor("Button", &["button2"])), Err(RegistryError::DuplicateComponent(_))));
    let _frozen = registry.freeze().unwrap();
    assert!(matches!(registry.register(test_descriptor("Badge", &["badge"])), Err(RegistryError::Frozen)));
}
```

- [ ] **Step 2: Run the focused tests and confirm RED**

Run: `cargo test -p gpui-shell component_registry --lib`

Expected: compilation fails because `component_registry` and its public types do not exist.

- [ ] **Step 3: Implement the registry and erased payload boundary**

Use `Arc<dyn Any + Send + Sync>` for immutable recorded payloads and an object-safe materializer receiving a shell-owned request:

```rust
pub const COMPONENT_REGISTRY_API_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentId(u32);

#[derive(Clone)]
pub struct ComponentPayload(Arc<dyn Any + Send + Sync>);

pub trait ComponentMaterializer: Send + Sync + 'static {
    fn materialize(&self, request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement>;
}

pub struct ComponentDescriptor {
    pub name: &'static str,
    pub constructors: &'static [ConstructorDescriptor],
    pub methods: &'static [MethodDescriptor],
    pub typescript: TypeScriptDescriptor,
    pub materializer: Arc<dyn ComponentMaterializer>,
}
```

Keep `ComponentRegistry` mutable until `freeze(&mut self)` marks it frozen and returns a `FrozenComponentRegistry` backed by `Arc<[RegisteredComponent]>`. Return descriptive errors containing the conflicting component/export/method.

- [ ] **Step 4: Run registry and shell library tests**

Run: `cargo test -p gpui-shell component_registry --lib && cargo test -p gpui-shell --lib`

Expected: all selected tests pass.

- [ ] **Step 5: Commit the registry seam**

```bash
git add crates/shell/src/component_registry.rs crates/shell/src/lib.rs
git commit -m "shell: add frozen component registry"
```

### Task 2: Record and materialize registered component nodes

**Files:**
- Modify: `crates/shell/src/spec.rs`
- Modify: `crates/shell/src/materialize.rs`
- Modify: `crates/shell/src/engine/mod.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Test: `crates/shell/src/tests/render.rs`

**Interfaces:**
- Consumes: `FrozenComponentRegistry`, `ComponentId`, `ComponentPayload`, and `ComponentMaterializer` from Task 1.
- Produces: `Component::Registered(RegisteredComponentSpec)` and registry-backed dispatch during materialization.
- Produces: `ShellRuntime::component_registry() -> &FrozenComponentRegistry`.

- [ ] **Step 1: Write a failing registered-node render test**

Create a test-only descriptor named `TestBox` whose constructor records a string payload and whose materializer returns a `div().id(("test-box", payload))`. Install it in an isolated runtime, evaluate `new TestBox("alpha")`, build a snapshot, and assert the snapshot component name is `TestBox` and materialization completes.

```rust
#[gpui::test]
fn registered_component_records_and_materializes(cx: &mut gpui::TestAppContext) {
    let runtime = runtime_with_components([test_box_descriptor()]);
    let snapshot = runtime.render_script("export default () => new TestBox('alpha')", cx).unwrap();
    assert_eq!(snapshot.root_component_name(), Some("TestBox"));
    cx.update(|window, cx| assert!(materialize(&runtime, &snapshot, window, cx).is_ok()));
}
```

- [ ] **Step 2: Run the test and confirm RED**

Run: `cargo test -p gpui-shell registered_component_records_and_materializes --lib`

Expected: compilation fails because runtime registration and `Component::Registered` are absent.

- [ ] **Step 3: Add registered nodes and dispatch**

Add:

```rust
#[derive(Clone)]
pub struct RegisteredComponentSpec {
    pub id: ComponentId,
    pub payload: ComponentPayload,
}

pub enum Component {
    Div,
    HFlex,
    VFlex,
    ChildView(ChildViewSpec),
    Text(String),
    Svg(String),
    Registered(RegisteredComponentSpec),
}
```

Preserve shell primitives as built-ins. In `materialize_component`, resolve the descriptor by ID and pass a `MaterializeRequest` containing resolved refinement/behavior/states, children, named slots, arena access, runtime callback access, window, and app. Convert materializer errors into the existing error element/diagnostic path.

- [ ] **Step 4: Run registered-node, snapshot, and render tests**

Run: `cargo test -p gpui-shell registered_component_records_and_materializes --lib && cargo test -p gpui-shell tests::snapshot --lib && cargo test -p gpui-shell tests::render --lib`

Expected: all selected tests pass while legacy concrete variants still work.

- [ ] **Step 5: Commit registered-node dispatch**

```bash
git add crates/shell/src/spec.rs crates/shell/src/materialize.rs crates/shell/src/engine/mod.rs crates/shell/src/engine/quickjs/mod.rs crates/shell/src/tests/render.rs
git commit -m "shell: dispatch registered components"
```

### Task 3: Derive QuickJS exports and typings from descriptors

**Files:**
- Modify: `crates/shell/src/component_registry.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Modify: `crates/shell/src/typings.rs`
- Test: `crates/shell/src/engine/quickjs/mod.rs`
- Test: `crates/shell/src/typings.rs`

**Interfaces:**
- Consumes: frozen descriptors from Tasks 1–2.
- Produces: generic constructor/method installation for registered components.
- Produces: `typings::declarations(registry: &FrozenComponentRegistry)` with descriptor-generated exports.

- [ ] **Step 1: Write failing parity tests**

Register a descriptor with constructor `Demo.new(id: string)` and methods `disabled(boolean)` and `on_change(callback)`. Assert QuickJS exports all three and generated declarations contain the same signatures. Add an alias `OldDemo` and assert it constructs the same component ID while emitting one deprecation warning.

```rust
#[test]
fn descriptor_drives_runtime_and_typescript() {
    let registry = frozen([demo_descriptor()]);
    let declarations = declarations(&registry);
    assert!(declarations.contains("export const Demo"));
    assert!(declarations.contains("disabled(value: boolean): Element"));
    assert!(declarations.contains("on_change(handler: (value: boolean, cx: Context) => void): Element"));
    assert_eq!(quickjs_export_names(&registry), ["Demo", "OldDemo"]);
}
```

- [ ] **Step 2: Run parity tests and confirm RED**

Run: `cargo test -p gpui-shell descriptor_drives_runtime_and_typescript --lib`

Expected: fails because hard-coded runtime exports and declarations ignore the registry.

- [ ] **Step 3: Implement descriptor-driven installation**

Represent arguments with a closed shell-owned schema (`String`, `Number`, `Boolean`, `Element`, `Entity(kind)`, `Callback(signature)`, enums, arrays, and optional values). Each descriptor constructor/method carries a recorder function that validates bridged values and returns a payload or `SpecOp`. QuickJS loops over descriptors to install constructors and methods. Typings loops over the same entries to render deterministic declarations and JSDoc.

- [ ] **Step 4: Run QuickJS and typings tests**

Run: `cargo test -p gpui-shell engine::quickjs --lib && cargo test -p gpui-shell typings --lib`

Expected: all tests pass and declaration ordering is deterministic.

- [ ] **Step 5: Commit descriptor-driven APIs**

```bash
git add crates/shell/src/component_registry.rs crates/shell/src/engine/quickjs/mod.rs crates/shell/src/typings.rs
git commit -m "shell: generate component APIs from registry"
```

### Task 4: Create `gpui-component-shell` and migrate existing bindings

**Files:**
- Create: `crates/component-shell/Cargo.toml`
- Create: `crates/component-shell/src/lib.rs`
- Create: `crates/component-shell/src/shell/mod.rs`
- Move: `crates/shell/src/materialize/components/*.rs` to `crates/component-shell/src/shell/`
- Modify: `Cargo.toml`
- Modify: `crates/shell/Cargo.toml`
- Modify: `crates/shell/src/bin/gpui-shell.rs`
- Modify: `crates/shell/src/materialize.rs`
- Modify: `crates/shell/src/spec.rs`
- Test: `crates/component-shell/src/lib.rs`

**Interfaces:**
- Produces: `gpui_component_shell::register(&mut ComponentRegistry) -> Result<(), RegistryError>`.
- Consumes: registry descriptors/materialization request and public shell callback/entity services.
- Preserves: all existing JavaScript names and behavior.

- [ ] **Step 1: Write failing adapter registration tests**

Create a catalog expectation for every existing concrete `Component` variant (`Button`, `Link`, `Checkbox`, through `VirtualList`) and assert `register` supplies its constructor and materializer. Add a source-boundary test that scans shell Rust sources and fails on `use gpui_component` or imports of concrete `gpui_base` controls in `materialize`.

```rust
#[test]
fn migrated_catalog_is_registered() {
    let registry = registered_catalog();
    for name in EXISTING_COMPONENT_NAMES {
        assert!(registry.component_named(name).is_some(), "missing {name}");
    }
}
```

- [ ] **Step 2: Run adapter test and confirm RED**

Run: `cargo test -p gpui-component-shell migrated_catalog_is_registered`

Expected: Cargo reports that package `gpui-component-shell` does not exist.

- [ ] **Step 3: Add the crate and registration entry point**

Declare package `gpui-component-shell`, library name `gpui_component_shell`, and dependencies on workspace `gpui`, `gpui-base`, `gpui-component`, and `gpui-shell`. Implement:

```rust
pub fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    shell::register_foundations(registry)?;
    shell::register_inputs(registry)?;
    shell::register_collections(registry)?;
    shell::register_overlays(registry)?;
    Ok(())
}
```

- [ ] **Step 4: Migrate each existing materializer and schema**

Move component construction, component-specific warnings, typed child/slot handling, and entity lookup into adapter modules. Expose the smallest required shell services publicly rather than duplicating runtime logic. Replace legacy `Component` variants with registered payloads and delete the old `materialize/components` directory after parity tests cover every variant.

- [ ] **Step 5: Compose the default executable**

Expose an adapter startup helper that builds the registry before runtime startup; applications and the JS Story host call it instead of constructing a base-only runtime directly:

```rust
let mut components = gpui_shell::ComponentRegistry::new(
    gpui_shell::COMPONENT_REGISTRY_API_VERSION,
)?;
gpui_component_shell::register(&mut components)?;
let components = components.freeze()?;
gpui_shell::run_with_components(options, components)
```

- [ ] **Step 6: Run migration parity and dependency audits**

Run: `cargo test -p gpui-component-shell && cargo test -p gpui-shell --lib && cargo check -p gpui-shell -p gpui-component-shell && cargo tree -p gpui-component-shell | rg 'gpui-(shell|component|base)'`

Expected: tests/check pass; the final audit shows `gpui-component-shell` consuming `gpui-shell`, `gpui-component`, and `gpui-base`, with no reverse adapter edge.

- [ ] **Step 7: Commit the adapter migration**

```bash
git add Cargo.toml Cargo.lock crates/shell crates/component-shell
git commit -m "component-shell: migrate shell component bindings"
```

### Task 5: Register the remaining stateless and layout components

**Files:**
- Create: `crates/component-shell/src/shell/display.rs`
- Create: `crates/component-shell/src/shell/navigation.rs`
- Create: `crates/component-shell/src/shell/forms.rs`
- Modify: `crates/component-shell/src/shell/mod.rs`
- Test: `crates/component-shell/tests/catalog.rs`

**Interfaces:**
- Consumes: adapter registration helpers from Task 4.
- Produces registrations for Alert, Badge, Breadcrumb, Clipboard, DescriptionList, Form/Field, GroupBox, Icon, Image, Kbd, Label, Rating, Separator, Skeleton, Spinner, StatusBar, Stepper, Tag, TitleBar, Switch, Checkbox, Radio, Toggle, Tabs, Pagination, Accordion, Collapsible, Progress, Avatar, and buttons/dropdown buttons.

- [ ] **Step 1: Add failing constructor/method catalog tests**

For each component, define required constructor, value/state builders, callbacks, and slots in a checked-in `EXPECTED_STATELESS_BINDINGS` table. Assert every expected entry exists and no registered method is undocumented.

```rust
#[test]
fn stateless_catalog_has_required_api() {
    let registry = registered_catalog();
    for expected in EXPECTED_STATELESS_BINDINGS {
        let descriptor = registry.component_named(expected.name).unwrap();
        assert_eq!(descriptor.public_methods(), expected.methods);
        assert_eq!(descriptor.public_slots(), expected.slots);
    }
}
```

- [ ] **Step 2: Run catalog test and confirm RED**

Run: `cargo test -p gpui-component-shell --test catalog stateless_catalog_has_required_api`

Expected: fails listing the first missing component or method.

- [ ] **Step 3: Implement stateless registrations**

Use real public APIs from `crates/ui/src`; map semantic variants/sizes to closed enums, preserve stable IDs, forward callbacks through shell callback IDs, and use named slots only where the Rust component has semantic parts. Do not emulate controls with generic `div`s.

- [ ] **Step 4: Run stateless catalog and materialization tests**

Run: `cargo test -p gpui-component-shell --test catalog stateless_catalog_has_required_api && cargo test -p gpui-component-shell stateless_materialization`

Expected: all selected tests pass.

- [ ] **Step 5: Commit stateless bindings**

```bash
git add crates/component-shell/src/shell crates/component-shell/tests/catalog.rs
git commit -m "component-shell: register stateless component catalog"
```

### Task 6: Register stateful inputs and overlays

**Files:**
- Create: `crates/component-shell/src/shell/state.rs`
- Create: `crates/component-shell/src/shell/inputs.rs`
- Create: `crates/component-shell/src/shell/overlays.rs`
- Modify: `crates/component-shell/src/shell/mod.rs`
- Modify: `crates/shell/src/entities.rs`
- Modify: `crates/shell/src/engine/quickjs/entity_api.rs`
- Modify: `crates/shell/src/engine/quickjs/overlay.rs`
- Test: `crates/component-shell/tests/stateful.rs`

**Interfaces:**
- Produces adapter-owned factories and registrations for Input, Textarea, NumberInput, OtpInput, Slider, Select, Combobox, ColorPicker, Calendar, DatePicker, SearchableList, Dialog, AlertDialog, Sheet, Popover, HoverCard, Menu, Command, Notification, and Tooltip.
- Consumes generic shell `EntityHandle`, typed adapter state key, callback dispatch, and window overlay operations.

- [ ] **Step 1: Write failing state lifecycle tests**

Test that an input value survives snapshot replacement, a select emits one change for one user action, a dialog restores focus to its trigger on dismissal, and releasing an adapter entity invalidates its generation without invalidating an older painted snapshot.

```rust
#[gpui::test]
fn input_state_survives_script_snapshot_replacement(cx: &mut TestAppContext) {
    let app = js_app("const state = new InputState('one'); export default () => new Input(state)", cx);
    app.set_input("two", cx);
    app.refresh(cx);
    assert_eq!(app.input_value(cx), "two");
}
```

- [ ] **Step 2: Run stateful tests and confirm RED**

Run: `cargo test -p gpui-component-shell --test stateful`

Expected: fails on missing adapter state factories/overlay registrations.

- [ ] **Step 3: Generalize shell entity storage**

Replace concrete base-state enum branches with registered entity kinds and adapter-provided create/read/update/subscribe hooks. Keep numeric handle allocation, ownership, generation checks, and callback lifetime in shell. Move concrete input/calendar/slider/date conversion into `component-shell`.

- [ ] **Step 4: Implement input and overlay descriptors/materializers**

Use `Entity<T>` for retained state, subscribe through adapter hooks, call `ScriptView::refresh` only when script-visible state changes, and preserve Root/window overlay focus rules. Report unsupported platform behavior during construction with the component name.

- [ ] **Step 5: Run lifecycle, QuickJS, and overlay suites**

Run: `cargo test -p gpui-component-shell --test stateful && cargo test -p gpui-shell tests::snapshot --lib && cargo test -p gpui-shell engine::quickjs --lib`

Expected: all selected tests pass.

- [ ] **Step 6: Commit stateful bindings**

```bash
git add crates/component-shell crates/shell/src/entities.rs crates/shell/src/engine/quickjs/entity_api.rs crates/shell/src/engine/quickjs/overlay.rs
git commit -m "component-shell: register stateful inputs and overlays"
```

### Task 7: Register collections, rich content, charts, and dock

**Files:**
- Create: `crates/component-shell/src/shell/collections.rs`
- Create: `crates/component-shell/src/shell/content.rs`
- Create: `crates/component-shell/src/shell/charts.rs`
- Create: `crates/component-shell/src/shell/dock.rs`
- Modify: `crates/component-shell/src/shell/mod.rs`
- Modify: `crates/shell/src/dock.rs`
- Modify: `crates/shell/src/engine/quickjs/dock_api.rs`
- Test: `crates/component-shell/tests/complex.rs`

**Interfaces:**
- Produces List, VirtualList, Tree, Table/DataTable, Settings, Text/Markdown/Editor, Area/Bar/Line/Pie/Radar charts, Plot, Sidebar, Resizable, Scrollbar, and Dock registrations.
- Consumes registered entity kinds, script range render callbacks, typed slot materialization, and generic dock persistence/capability services.

- [ ] **Step 1: Write failing representative complex-component tests**

Test virtual-list range rendering and stable keys, tree expansion/selection, table sorting, chart series conversion, editor text persistence, and dock save/load with script panels. Assert callbacks are not invoked during unchanged snapshot repaint.

```rust
#[gpui::test]
fn virtual_list_only_calls_script_for_requested_ranges(cx: &mut TestAppContext) {
    let app = js_virtual_list(10_000, cx);
    app.paint(cx);
    assert!(app.rendered_ranges(cx).iter().all(|range| range.end - range.start < 200));
    let calls = app.script_render_count(cx);
    app.repaint_without_invalidation(cx);
    assert_eq!(app.script_render_count(cx), calls);
}
```

- [ ] **Step 2: Run complex tests and confirm RED**

Run: `cargo test -p gpui-component-shell --test complex`

Expected: fails listing missing registered complex components.

- [ ] **Step 3: Implement collection and content adapters**

Use stable domain keys supplied by scripts; require explicit row/node IDs where data can reorder. Keep range callback leases tied to snapshots. Convert script arrays/records into typed delegate data once per invalidation, not once per paint.

- [ ] **Step 4: Implement charts and dock adapters**

Map chart data to gpui-component chart/plot APIs using semantic colors. Move concrete dock renderer/panel integration from shell into adapter while retaining generic storage, capability, and path policy in shell.

- [ ] **Step 5: Run complex and existing dock tests**

Run: `cargo test -p gpui-component-shell --test complex && cargo test -p gpui-shell tests::dock --lib`

Expected: all selected tests pass.

- [ ] **Step 6: Commit complex bindings**

```bash
git add crates/component-shell crates/shell/src/dock.rs crates/shell/src/engine/quickjs/dock_api.rs
git commit -m "component-shell: register complex component catalog"
```

### Task 8: Enforce complete inventory and generated typings

**Files:**
- Create: `crates/component-shell/component-inventory.json`
- Create: `crates/component-shell/tests/inventory.rs`
- Modify: `crates/shell/src/typings.rs`
- Modify: `website/shell/api.md`
- Modify: `website/zh-CN/shell/api.md`

**Interfaces:**
- Consumes: public module list in `crates/ui/src/lib.rs`, Story exports in `crates/story/src/stories/mod.rs`, and frozen descriptors.
- Produces: auditable inventory classifications `component`, `infrastructure`, and `platform`.
- Produces: complete deterministic `gpui-kit.d.ts`.

- [ ] **Step 1: Write failing inventory audit**

Parse the two authoritative Rust source files, normalize module/story names, and assert every entry appears in `component-inventory.json`; then assert each `component` entry resolves to a descriptor and each `infrastructure` entry carries a non-empty explanation.

```rust
#[test]
fn every_public_component_and_story_is_accounted_for() {
    let inventory = Inventory::load();
    for public_name in public_ui_modules().chain(public_story_names()) {
        assert!(inventory.contains(public_name), "unclassified public item: {public_name}");
    }
    inventory.assert_registered(&registered_catalog());
}
```

- [ ] **Step 2: Run inventory test and confirm RED**

Run: `cargo test -p gpui-component-shell --test inventory`

Expected: fails with missing inventory file or unclassified items.

- [ ] **Step 3: Add and satisfy the full inventory**

Classify only non-renderable modules (`global_state`, `history`, `highlighter`, theme helpers, and behavior infrastructure) as infrastructure, with explanations. Classify native-menu/platform entries as platform while requiring descriptors under supported targets. Add missing bindings until all user-facing entries are `component` and registered.

- [ ] **Step 4: Snapshot generated declarations and update API docs**

Generate declarations from the full registry, assert byte-for-byte deterministic output on a second run, and document adapter registration plus the standard binary's included catalog.

- [ ] **Step 5: Run inventory and typings tests**

Run: `cargo test -p gpui-component-shell --test inventory && cargo test -p gpui-shell typings --lib`

Expected: all selected tests pass.

- [ ] **Step 6: Commit inventory enforcement**

```bash
git add crates/component-shell/component-inventory.json crates/component-shell/tests/inventory.rs crates/shell/src/typings.rs website/shell/api.md website/zh-CN/shell/api.md
git commit -m "component-shell: enforce complete binding inventory"
```

### Task 9: Build the JavaScript Story gallery

**Files:**
- Create: `examples/js_story/main.js`
- Create: `examples/js_story/app.js`
- Create: `examples/js_story/catalog.js`
- Create: `examples/js_story/stories/*.js`
- Create: `examples/js_story/fixtures/*.json`
- Create: `examples/js_story/jsconfig.json`
- Create: `examples/js_story/gpui-kit.d.ts`
- Create: `examples/js_story/README.md`
- Test: `crates/component-shell/tests/js_story.rs`
- Modify: `website/shell/examples.md`
- Modify: `website/zh-CN/shell/examples.md`

**Interfaces:**
- Consumes: full JavaScript catalog and generated declarations from Task 8.
- Produces: standard runnable application `cargo run -p gpui-shell -- examples/js_story`.
- Produces: catalog manifest exporting one route for every component inventory entry.

- [ ] **Step 1: Write failing gallery coverage/load tests**

Load `catalog.js` in the shell runtime, assert route names equal the component inventory, instantiate every route, build its first snapshot, and fail with the route plus script/materialization error. Also assert no gallery module imports a custom Rust host module.

```rust
#[test]
fn js_story_covers_and_builds_every_component_route() {
    let app = JsStoryHarness::load("../../examples/js_story").unwrap();
    assert_eq!(app.routes(), registered_component_story_routes());
    for route in app.routes() {
        app.build(route).unwrap_or_else(|error| panic!("{route}: {error}"));
    }
}
```

- [ ] **Step 2: Run gallery test and confirm RED**

Run: `cargo test -p gpui-component-shell --test js_story`

Expected: fails because `examples/js_story` does not exist.

- [ ] **Step 3: Implement gallery shell and navigation**

Build a desktop sidebar/content layout with stable route IDs, search/filter, keyboard selection, theme-aware surfaces, and a content header. Keep state in JavaScript and use the public component API.

- [ ] **Step 4: Add one story module per catalog family**

Each module exports `{ id, title, group, render }`. Cover normal and relevant interactive states, retained data for complex controls, and platform availability panels. Derive `catalog.js` from explicit imports so missing files are reviewable and bundler-free.

- [ ] **Step 5: Generate declarations and document running the gallery**

Run: `cargo run -p gpui-shell -- types examples/js_story`

Expected: writes `examples/js_story/gpui-kit.d.ts` without errors.

- [ ] **Step 6: Run gallery coverage and load tests**

Run: `cargo test -p gpui-component-shell --test js_story`

Expected: every catalog route builds successfully.

- [ ] **Step 7: Commit the JS Story**

```bash
git add examples/js_story crates/component-shell/tests/js_story.rs website/shell/examples.md website/zh-CN/shell/examples.md
git commit -m "examples: add JavaScript component story"
```

### Task 10: Complete migration audits and full verification

**Files:**
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: files exposed by verification failures only when backed by a failing regression test.

**Interfaces:**
- Consumes: all prior tasks.
- Produces: evidence that the requested architecture, coverage, and example are complete.

- [ ] **Step 1: Audit crate ownership and dependency direction**

Run:

```bash
rg -n "gpui_component::|use gpui_component|materialize/components" crates/shell/src
cargo tree -p gpui-component-shell | rg "gpui-shell v"
find crates/component-shell/src/shell -type f -name '*.rs' -print | sort
```

Expected: first two commands produce no concrete-component hits/dependency; adapter modules are listed by the third.

- [ ] **Step 2: Audit inventory and JS Story coverage**

Run: `cargo test -p gpui-component-shell --test inventory --test js_story`

Expected: all inventory entries are classified/registered and all gallery routes build.

- [ ] **Step 3: Run formatting and focused verification**

Run: `cargo fmt --all -- --check && cargo test -p gpui-shell --lib && cargo test -p gpui-component-shell && cargo check -p gpui-shell -p gpui-component-shell`

Expected: all commands exit zero with no test failures.

- [ ] **Step 4: Run workspace verification**

Run: `cargo test --workspace --all-targets`

Expected: all workspace targets pass. If an existing unrelated failure occurs, record its exact command/output and keep the goal incomplete until it is resolved or explicitly waived.

- [ ] **Step 5: Add top-level discovery documentation**

Document the adapter crate, how embedders call `register`, and the JS Story run command in both READMEs. Do not duplicate generated API listings.

- [ ] **Step 6: Re-run final diff and documentation checks**

Run: `git diff --check && git status --short && cargo test -p gpui-component-shell --test inventory --test js_story`

Expected: no whitespace errors; only intended changes remain; completion gates pass.

- [ ] **Step 7: Commit final documentation and verification fixes**

```bash
git add README.md README.zh-CN.md
git commit -m "docs: document component shell adapter"
```

# gpui-base Architecture

## Status and Scope

This document describes the architecture implemented by `crates/base`. It is a
source-derived reference, not a migration plan. Public exports in
`crates/base/src/lib.rs` and the Rust API documentation remain authoritative for
individual methods.

`gpui-base` is the reusable foundation below the styled `gpui-component` crate.
It is designed for both of these callers:

- `crates/component`, which adapts base behavior into GPUI Component's complete visual
  system;
- applications that build and own a different visual system directly on top of
  base behavior.

## Architectural Thesis

The durable rule is:

> Base owns reusable behavior and the geometry required to implement it. The
> presentation layer owns the product's visual language.

“Headless” does not mean that every base module is a single empty `Div`.
Keyboard navigation, text editing, popup collision handling, virtualization,
calendar grids, resizable panels, and toast stacking require internal structure,
measurement, and retained state. Base owns that complexity when moving it to
callers would duplicate difficult behavior.

Base does not own product-level choices such as brand colors, typography,
control density, borders, radii, icons, variants, or final composition.

## Dependency Direction

```text
                       application
                     /             \
                    ▼               ▼
       application-owned UI     gpui-component
                    \               /
                     └──────┬──────┘
                            ▼
                        gpui-base
                            ▼
                           GPUI
```

Dependencies point downward. `gpui-base` must not import `gpui-component`
themes, assets, or façade types. `gpui-component::init` may initialize and theme
the base layer, but the base layer must also work when initialized directly.

There is no Registry or CLI crate in the current workspace. Source distribution
can be added above this seam without changing the ownership model, but it is not
part of the implemented architecture documented here.

## Module Families

The public surface contains four distinct module families. Treating all of them
as identical “primitives” hides important interface differences.

### 1. Semantic elements

Examples include Button, Checkbox, Radio, Switch, Toggle, Link, Tabs, Progress,
Avatar, and the semantic Table parts.

These modules typically implement GPUI interfaces directly:

```text
IntoElement
+ Styled
+ ParentElement where composition is meaningful
+ InteractiveElement for interactive roots
```

They provide a stable element identity, event normalization, focus behavior,
keyboard activation, accessibility semantics, controlled values, and optional
semantic-state styles. The caller supplies visible children and presentation.

`Button::new("save")`, for example, receives an `ElementId`, not a label. It has
no default height, padding, background, border, or radius.

### 2. Compound behavior roots

Examples include Accordion, Dialog, AlertDialog, Sheet, Popover, HoverCard,
Select, Combobox, DatePicker, and Popup.

These modules coordinate multiple parts or application-owned children. Their
interfaces encode behavior that would be fragile if every caller rebuilt it:

- open-state requests and change reasons;
- trigger and content focus transfer;
- Escape, Confirm, and directional key actions;
- dismissal ordering;
- backdrop hit testing;
- focus trapping;
- trigger measurement and popup placement.

Parts such as `DialogTitle`, `DialogDescription`, and `DialogClose` are explicit
semantic seams. Base does not walk arbitrary descendant trees to discover them.

### 3. Stateful systems

Examples include InputState, TextareaState, EditorState, CalendarState, TreeState, SliderState,
ResizableState, OtpState, ColorPickerState, ToastManager, ToastStackState, NavStackState, DockArea,
TabGroup, and TilesState.

These modules retain data because their behavior spans frames or requires
measurement, subscriptions, history, focus, or incremental updates. State is
usually stored in a GPUI `Entity`, a keyed element state, or an application-owned
model passed back to the element.

Stateful systems expose application rendering seams rather than leaking their
implementation. Calendar provides a pre-wired `CalendarItem` and semantic
`CalendarItemState` to an item renderer. Tree owns flattening, expansion,
selection, keyboard movement, and virtualization while the caller renders each
visible `TreeEntry`.

### 4. Infrastructure and utility modules

Examples include Positioner, Scrollbar, VirtualList, FocusTrapElement,
AutoScroll, motion, History, geometry helpers, measurement, theme tokens, and
global initialization.

These are deep modules: a small interface hides layout, lifecycle, or data
structure complexity used by many controls. Their interface is also their test
seam; callers should not need to reproduce the hidden algorithm to verify it.

## State Ownership

State ownership follows behavioral needs rather than one universal pattern.

### Controlled values

Checkbox, Radio, Switch, Toggle, Select, and similar roots accept the current
value and report requested changes. They do not silently mutate application
state.

```text
application value
      │
      ▼
base element ── activation ──▶ on_change(next_value)
      ▲                            │
      └──── next render ───────────┘
```

Callbacks describe intent. Pointer-originated value changes include the
`ClickEvent` when modifier keys are useful. Model-driven changes, such as
pagination requests, do not invent a pointer event.

### Handles and explicitly shared state

Dialog supports `DialogHandle` for imperative open and close requests while
still reporting `DialogChangeReason`. Scrollbar adapters share an underlying
scroll handle. Toast stack geometry is shared through `ToastStackState`.

Handles coordinate one logical behavior. They must not be reused across
unrelated viewports or component instances.

### Entity-backed state

Complex modules use `Entity<State>` when they need GPUI observation,
subscriptions, focus handles, or incremental notification. The state entity owns
behavioral data; the presentation layer still owns how that data looks.

### Keyed element state

Small pieces of ephemeral state tied to an element identity use
`window.use_keyed_state`. Stable `ElementId` values are therefore part of the
interface, not an implementation detail. Changing an ID can reset focus,
measurement, animation, or open-state bookkeeping.

## Styling Model

Base separates three styling mechanisms:

1. ordinary GPUI `Styled` calls for instance presentation;
2. typed semantic-state style builders such as checked, selected, or disabled;
3. GPUI native runtime modifiers such as hover, active, and focus-visible.

The shared `state_style::resolve_style` function makes semantic precedence
consistent across controls. See [Styling and Motion](STYLING-AND-MOTION.md) for
the complete contract.

Compound and stateful modules expose explicit presentation seams:

- GPUI `Styled` on the root or part;
- application-provided children;
- typed part elements;
- item renderer callbacks;
- style refinements for internal virtualized containers;
- presentation snapshots such as `InputPresentation`.

## Public Data Types Across the Seam

A public struct that crosses the base/application seam must not expose `pub`
fields. Every field a caller can name is a field that cannot be added later:
adding one breaks any struct literal, and removing or renaming one breaks every
reader. The seam types are the ones that grow the most, because each new
capability of a control shows up as another flag in the state it hands out.

The shape to use instead:

- private fields;
- a builder for construction — `new()` plus one chained setter per field;
- a reader per field.

Setters and readers must not collide, which decides the naming:

- a type whose fields are all boolean names its setters after the field and its
  readers `is_<adjective>`/`has_<noun>`, never `can_`, matching how elements
  read — `CalendarItemState`, `InputContextMenuCapabilities`;
- a type carrying non-boolean fields prefixes every setter with `with_`, so the
  readers keep the plain field name — `RenderOptions::with_item_ix` against
  `RenderOptions::item_ix`. This follows `Sizable::with_size`.

A `with_`-style setter that takes `self` by value also replaces functional
update syntax, which stops compiling once the fields are private:

```rust
// was: RenderOptions { item_ix, ..*options }
item.render_item(&options.with_item_ix(item_ix), window, cx)
```

A type that is only ever built inside its own module — `InputPresentation` is
built by `InputBaseState::presentation` and nowhere else — needs the private
fields and the readers, but not a public builder. Private fields already close
the breaking-change hole, and a public builder there would hand out a
construction path the seam does not want. Such a type reads its non-boolean
fields under the plain field name, which is why it cannot also carry
field-named setters.

```rust
let capabilities = InputContextMenuCapabilities::new()
    .code_editor(true)
    .selection(true);

if capabilities.is_editable() && capabilities.has_selection() { /* ... */ }
```

Derived answers belong on the type rather than at each call site. When several
readers are always combined the same way — `!disabled && !readonly` — publish
that combination as its own reader (`is_editable`) so the rule has one
definition and new inputs to it stay invisible to callers.

Name such a type in full: `ComboboxTriggerContext`, never `…Ctx`. In a GPUI
codebase `cx` is reserved for `App`, `Context<T>`, and `AsyncApp`, so an
abbreviated `ctx` for anything else reads as a second, competing context. A
callback that receives both takes the GPUI one as `cx` and gives the other a
name describing what it holds.

This applies to state snapshots (`InputPresentation`, `CalendarItemState`),
capability sets (`InputContextMenuCapabilities`), render contexts
(`ComboboxTriggerContext`), and option sets (`RenderOptions`). It does not
apply to value types whose fields _are_ the definition and cannot grow, such as
`Point`, `Selection`, `Edges`, `IndexPath`, or `FoldRange`, nor to types that
mirror an external schema, such as the LSP `Diagnostic`.

Design-token records (`ColorTokens`, `RadiusTokens`, and the rest of
`theme_tokens`) still carry `pub` fields. They have the same growth problem, and
converting them is tracked separately because every theme in `gpui-component`
constructs them.

## Input, Textarea, and Editor Architecture

Text editing is intentionally deeper than the semantic elements, but callers do
not need to learn the complete editor interface for every text field.

### Public forms

Both `gpui-base` and `gpui-component` expose three purpose-specific forms:

| Form       | State           | Intended interface                                                                                        |
| ---------- | --------------- | --------------------------------------------------------------------------------------------------------- |
| `Input`    | `InputState`    | Single-line values, placeholders, masks, validation, and submission                                       |
| `Textarea` | `TextareaState` | Ordinary multi-line text, fixed rows, soft wrapping, and optional auto-grow limits                        |
| `Editor`   | `EditorState`   | Source text, language-aware highlighting, line numbers, folding, search, diagnostics, and LSP integration |

`gpui-base` provides unstyled forms. `gpui-component` adapts the same behavior
into the product theme and sizing system. `InputBase` is the foundational frame
used for input semantics, state styling, accessibility, and application-owned
content; it is not one of the three editing forms.

Existing `gpui-component::Input::new(&Entity<InputState>)` call sites remain the
single-line compatibility path. `InputState` is a real facade, not a type alias
for the editing engine: multiline, auto-grow, gutter, folding, diagnostics, and
LSP configuration are absent from its API. Multi-line code must construct
`TextareaState` or `EditorState` instead.

### Shared engine

`InputBaseState` owns mechanics shared by all three states:

- Rope-backed text and edit history;
- cursor, selection, IME, clipboard, and focus;
- shaping, layout, hit testing, selection and caret painting;
- auto-scroll, viewport scrolling, and cursor visibility;
- native text-content integration where supported.

The implementation under `crates/base/src/input` is organized by responsibility:

- `base/` contains the shared editing engine and foundational mechanics;
- `input/` contains the single-line control and state facade;
- `textarea/` contains the multi-line control and state facade;
- `editor/` contains the editor control and state facade, plus display mapping,
  highlighting, search, diagnostics, decorations, indentation, and LSP.

These are implementation folders rather than public Rust module segments. The
external seam remains `gpui_base::input`, with stable re-exports in `mod.rs`.

Purpose-specific state facades configure the shared engine and forward
`InputEvent` without duplicating those mechanics. `InputState`, `TextareaState`,
and `EditorState` are distinct GPUI entity types. Their private bridge to
`InputBaseState` exists for component composition; it is not the application
API.

Editor-only implementation includes indentation, folding, decorations,
diagnostics, search, LSP providers, overlays, line-number/gutter painting, and
syntax highlighting. Textarea owns rows, soft wrapping, Enter submission, and
auto-grow policy. Masking, validation, and number stepping remain input-only
concepts.

### Presentation and geometry

Presentation is injected through `InputEditorStyle`, highlighter interfaces,
fold-icon renderers, context-menu adapters, and the higher-level UI forms.
`gpui-component` supplies editor insets from its size system. Base consumes
those insets only as geometry so text, the fixed gutter, and scrollbars share a
coordinate system:

- text remains inset from the frame;
- vertical and horizontal scrollbars terminate at the frame edge;
- the gutter background covers the complete fixed column, including top,
  bottom, and leading insets;
- editor focus does not add the single-line input focus-border treatment.

This keeps product values in the presentation layer while keeping coupled text,
gutter, and scrollbar geometry local to the editing engine.

Platform-specific behavior is isolated behind adapters. For example, folding is
disabled on WebAssembly, time uses `web_time` where needed, and native text
content support is conditionally compiled.

## Overlay and Positioning Architecture

Overlay modules separate lifecycle from placement and presentation.

### Positioner

`Positioner` is the shared placement implementation. It supports:

- side placement with preferred-side selection, flipping, alignment, offset,
  and viewport clamping;
- corner placement compatible with anchored trigger geometry, with viewport
  clamping but no side flip.

Popup measures its trigger during prepaint, stores the captured bounds in keyed
state, and renders positioned content through a deferred element on the next
frame. Tooltip reuses Positioner rather than maintaining another collision
algorithm.

### Modal hosts

Dialog and Sheet create viewport-sized hosts. They own focus trapping, keyboard
actions, backdrop or overlay dismissal, and callback ordering. They do not own
the final popup or panel placement: the application styles the supplied popup or
surface as centered, right-aligned, bottom-aligned, or another product-specific
layout.

Overlay visuals and overlay hit targets are separate layers. The surface is
painted above the dismissal target so a click inside content does not dismiss
the modal.

## Scrolling and Virtualization

`ScrollbarHandle` is the seam between scrolling implementations and the shared
Scrollbar element. An adapter reports:

- viewport bounds;
- current offset;
- content size;
- offset updates;
- optional drag lifecycle notifications.

Adapters exist for GPUI `ScrollHandle`, `UniformListScrollHandle`, and
`ListState`, plus base `VirtualListScrollHandle`.

Scrollbar normally overlays the viewport reported by its handle. Two explicit
alternatives support deeper widgets:

- `viewport_bounds` supplies custom-painted bounds, as the editor does;
- `viewport_from_layout` uses same-frame layout bounds, as DataTable does when
  excluding fixed headers and columns.

VirtualList owns variable-size item layout, visible-range calculation, content
masking, deferred scroll-to-item requests, and clamped offsets. The caller owns
item sizes and renders only the requested range. Tree builds on GPUI's uniform
list because its visible entries share a row height.

`ScrollableMask` owns wheel dispatch, not scrolling. GPUI's own overflow
listener runs in the bubble phase and never stops propagation, so a scroll area
nested inside another one cannot win: `gpui::list` registers its listener after
its items paint, and bubble runs in reverse registration order. The mask is a
sibling of the scrolled element that consumes axis-dominant wheel events in the
capture phase, locks each gesture to the axis it started on, and stays inert
while occluded. Edge semantics differ per axis, matching platform scrollers: a
vertical mask chains to the ancestor at the edge, a horizontal one keeps
consuming. `horizontal_scroll_area` is the paired viewport and mask, used by the
Markdown scrolling table.

One handle represents one logical viewport. Sharing a handle between nested or
unrelated scroll areas causes offsets, hitboxes, and scrollbar geometry to
interfere.

## Dock Layout Architecture

`crates/base/src/dock` owns the layout tree, persistence, drag hit-testing,
resize arithmetic, the active-panel state machine, zoom, focus, and the panel
registry. `crates/component/src/dock` is a skin: `DockSkin` implements the renderer
traits below to supply the tab bar, toolbar, drop-indicator, and dock-toggle
appearance. A `DockArea` built without a renderer still docks, drags, and
persists — it draws no chrome at all.

### The layout tree

`PaneTree` is the single source of truth for one region — the center, or
one of the left/bottom/right docks. It stores no GPUI entity handles:
containers are addressed by `NodeId`, panels by `PanelId` (the panel entity's
`EntityId`), and a container's `NodeKind` is one of `Split`, `Tabs`, or
`Tiles`. There is no leaf variant, so a panel can only ever live inside a
`Tabs` or `Tiles` node — the invariant a runtime assertion checked in the old
implementation is expressed in the type instead.

`NodeKind` stays private. Callers read a node through the borrowed `PaneRef`
projection and never construct one directly; every mutation goes through
`PaneTree`'s edit methods (`insert_panel`, `remove_panel`, `move_panel`,
`split`, `set_active`, `set_sizes`, `set_tile_bounds`, `bring_to_front`), each
of which runs `normalize` before returning. When base needs a fact about a
panel — its name, its visibility, its dump — it asks a `PanelSource` rather
than the entity directly, which is what makes the layout algebra testable as
pure functions with no `TestAppContext`.

Building a layout is entity-free too: `DockLayout` (`h_split`, `v_split`,
`tabs`, `tiles`, chained with `.child(...)`, `.panel_view(...)`,
`.tile_view(...)`, and `.active_index(...)`) produces a tree rather than
constructing containers. `DockArea::set_center` and `set_dock` reconcile a
`DockLayout` into live entities when it is installed.

Panels go in wrapped: `gpui_component::dock::panel_handle(panel)` is what
carries a panel's presentation across the renderer seam, and every entry point
takes one — `DockLayout::panel_view` / `tile_view` when describing a layout,
`DockArea::add_panel_view` / `add_tile_view` when adding to a live one, and
the closure a `register_panel` builder returns.

```rust,ignore
let center = DockLayout::h_split()
    .child(DockLayout::tabs().panel_view(panel_handle(files), cx), Some(px(240.)))
    .child(DockLayout::tabs().panel_view(panel_handle(editor), cx), None);
dock_area.update(cx, |area, cx| area.set_center(center, window, cx));
```

Base's own `DockLayout::panel` / `tile` and `DockArea::add_panel` / `add_tile`
take a bare `Entity<P>` instead. They are the base-only forms: the panel docks,
drags and persists exactly the same, but base stores the bare entity and the
skin cannot recover presentation from it, so **every tab draws the panel's
`panel_name` where its title belongs**. The only signal is a one-off
`tracing::warn!`. Use them only when there is no skin over the dock at all.

### Normalization

One post-order pass, repeated to a fixpoint, replaces the mutually recursive
parent-pointer collapse the old `StackPanel`/`TabPanel` pair used:

1. An empty `Tabs`, `Tiles`, or `Split` is removed from its parent; the root
   is exempt.
2. A `Split` with one child is replaced by that child, which keeps its own
   `NodeId` and inherits the split's slot size.
3. A `Split` containing a `Split` of the same axis splices the inner
   children into the outer node.
4. `active_ix` is clamped to the panel count.
5. Root shape is enforced per `RootKind` — the center's root is always a
   `Split`, so an empty center still serializes as a stack; a dock's root is
   unconstrained.

`normalize` is idempotent and needs no parent pointers or deferred work: the
tree is self-consistent the instant an edit operation returns.

### Reconciliation

`DockArea` holds the tree plus a cache of container and panel entities keyed
by `NodeId` and `PanelId`. After any edit that reports a change, it walks the
tree, creates an entity for each container id the cache does not yet have,
drops cache entries for ids no longer present — calling `on_removed` on the
panels that departed — pushes sizes and `active_ix` into the surviving
entities, and emits `DockEvent::LayoutChanged`.

Because `NodeId` survives every edit operation and every `normalize` rule, a
steady-state reconciliation pass creates and drops nothing: only genuinely
new or dead containers churn. That is what keeps a drag from resetting the
state of panels it did not touch.

### Rendering seam

`TabGroup` owns the panel list mirrored from the tree, the active index, the
focus handle, drag-and-drop hit state, and the zoom flag; it renders a
skeleton and delegates all appearance to a `TabGroupRenderer`.
`DockAreaRenderer` and `TilesRenderer` do the same for the area frame and a
tiles canvas. Base attaches the drag source, drop-target hit testing,
keyboard actions, and focus handling — a renderer implementation never sees a
drag event, only resolved state through `TabGroupContext`, `DockContext`, and
`TileContext`.

`Panel` splits at the seam the same way: `gpui_base::dock::Panel` covers
behavior (`panel_name`, `visible`, `closable`, `zoomable`, `set_active`,
`set_zoomed`, `on_added_to`, `on_removed`, `dump`), and
`gpui_component::dock::Panel` extends it with presentation (`title`,
`tab_name`, `toolbar_buttons`, `dropdown_menu`, `zoom_control`). A panel type
implements both.

## Theme Projection

The base `Theme` contains:

- `SemanticThemeTokens` for colors, radius, spacing, typography, and shadows;
- global Scrollbar defaults;
- the minimal Resizable handle colors required by its infrastructure.

Semantic tokens describe roles and scales, never component names. They do not
automatically style base controls. A presentation layer reads and applies them.

`gpui-component` projects its active theme into the base theme during
initialization and theme changes. Direct base users can mutate
`gpui_base::Theme::global_mut(cx)` themselves.

## Motion and Lifecycle

Ordinary semantic controls do not install product motion. The generic
`motion::transition` function manages keyed interpolation, timing, reversal,
animation frames, and reduced-motion behavior while the caller chooses the
animated property.

Some deep behavior modules own configurable motion that is inseparable from
their layout lifecycle. `ToastStack`, for example, owns stack expansion,
collapse, measurement, and overlap motion through `ToastMotion`. Its child
`Toast` remains an unstyled semantic root, and the application owns toast
content and visual styling.

This distinction is intentional:

- product decoration and choreography belong to presentation;
- motion required to keep a deep behavioral layout coherent may live in that
  module and must be configurable.

## Initialization

Call `gpui_base::init(cx)` before constructing base controls. Initialization:

- installs the base global theme if absent;
- initializes shared global state;
- registers key bindings and infrastructure for dialog, focus traps, popover,
  sheet, combobox, color picker, select, number input, input, and tree.

Applications that call `gpui_component::init(cx)` must not initialize base a
second time; the styled crate includes base initialization.

## Relationship to `gpui-component`

`gpui-component` is a presentation adapter and compatibility layer above base.
It may:

- map its Theme into base tokens and infrastructure defaults;
- wrap base elements with labels, icons, sizes, variants, and product layout;
- provide application-specific popup menus, LSP views, and native integrations;
- preserve historical public interfaces while delegating behavior to base.

The migration of a UI control to a base module is complete only when behavior,
focus, keyboard interaction, accessibility, overlay geometry, and visual output
remain correct. Sharing a type name or compiling an adapter is not sufficient.

## Design Invariants

Changes to `gpui-base` should preserve these invariants:

1. Base remains independent of `gpui-component` presentation and assets.
2. Reusable behavior is implemented once behind the lowest useful interface.
3. Applications can replace the visual language without reimplementing the
   behavior module.
4. Behavioral geometry stays with the module that must keep it correct.
5. Controlled elements report intent and do not own application values.
6. Stable element identity is documented wherever keyed state is used.
7. Pointer and keyboard paths converge on the same semantic action.
8. Accessibility is behavior, not optional decoration.
9. Overlay coordinates, paint order, and hitboxes use one viewport model.
10. One scroll handle represents one logical viewport.
11. Platform differences are isolated behind explicit adapters or conditional
    implementations.
12. Public types that cross the seam keep their fields private, are built with a
    builder, and are read through methods, so a new field is not a breaking
    change.
13. Long-lived architectural facts belong here; progress logs and temporary
    reviews belong in issues and pull requests.

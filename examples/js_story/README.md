# JavaScript Story gallery

This is the JavaScript Story gallery scaffold: an auditable, reviewable route
catalog for the component-shell work.

The gallery imports the registered public `gpui-component` surface through the
completed public component-shell host. Infrastructure routes remain explicit
status panels rather than fabricated constructors.

## Coverage audit

The independent coverage check derives all tracked component-shell surfaces
from `crates/component-shell/component-inventory.json` and checks them against
the explicit imports, routes, and `coveredBy` metadata in `stories/coverage.js`:

```bash
node examples/js_story/fixtures/verify-coverage.mjs
```

The gallery imports only public `gpui-kit`, `gpui-base`, and `gpui-component`
script modules. `catalog.js` explicitly imports each family module and every
route records its Rust Story source. The inventory currently supplies 69 mirrored Story
entries and 70 tracked catalog surfaces. The check fails if either side changes
without matching catalog coverage and status.

## Registration status

Registered inventory surfaces render a real public constructor and invoke a
descriptor-backed method when that descriptor exposes one. `StatusBar` exposes
component-specific `left_content(element)` and `right_content(element)` regions instead of
adding those names to every script element. The verifier checks the
status projection against the inventory, so a missing binding cannot be hidden
by an unreviewed third status.

Registered routes use the same presentation hierarchy as the Rust Story app:
each example is a titled section with an optional description and one focused
demonstration surface. Interactive controls remain controlled by the gallery,
and compound examples compose the public parts used in real applications.
`Dock` and `VirtualList` are infrastructure routes with live examples. Dock
exercises the public `gpui-base` `DockArea` subsystem with a side dock, panels,
and a center tab group. VirtualList renders 10,000 stable rows through
`v_virtual_list` and a paired scrollbar, while only materializing its visible
range.

Editable `Input` examples use the public `gpui-base` `InputState` created in
the owning view's `init()` phase. This preserves the same state lifecycle and
placeholder behavior as the Rust Story instead of recreating input state from
render.

`NativeMenuTrigger` provides the registered native-menu surface used by the
JavaScript story under the inventory's `platform-integration` category.

Two Rust Stories are deliberately not mirrored, and `verify-coverage.mjs` holds
the list with a reason for each: `ShellStory`, which embeds a script view inside
a Rust story and would demonstrate this gallery to itself, and
`ThemeColorsStory`, since every route already renders through the active theme.
The verifier refuses an exclusion that is not `infrastructure` in the inventory,
so the list cannot quietly hide a component that has something to show.

## Editor checking

`gpui-kit.d.ts` is generated from the public component-shell host's declaration
API and is not hand-authored by this example:

```bash
cargo run -p gpui-component-shell --bin gpui-component-shell -- types examples/js_story
```

`jsconfig.json` enables strict JSDoc checking for the gallery and all family
modules against that generated surface.

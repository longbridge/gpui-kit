# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Canonical Design and Coding Guides

Before changing UI, interaction, interface language, layout, styling,
components, or application architecture, read and follow the repository's
canonical guides:

- [Design Guides](website/docs/design-guides.md)
- [Coding Guides](website/docs/coding-guides.md)

These guides are requirements, not optional inspiration. Do not copy generic
web conventions, infer a design system from one existing screen, or add a
control merely because the underlying feature exists. Preserve the documented
task hierarchy, interaction promise, desktop conventions, spacing, alignment,
theme tokens, component boundaries, naming, and crate architecture. Review the
finished work against both guides before considering it complete.

For Chinese documentation and UI, apply the terminology rules in Design Guides.
Keep established framework, component, and API names in their canonical English
form when translation would reduce precision; write the surrounding Chinese as
natural Chinese rather than word-for-word translation.

## Project Overview

GPUI Kit is a Rust desktop application framework built on GPUI, published at <https://gpui-kit.com>. It ships as three crates: `gpui-base` (unstyled behavior and infrastructure), `gpui-shell` (JavaScript extensions for a Rust host), and `gpui-component` (GPUI Component, the styled component library with 60+ cross-platform desktop UI components, inspired by macOS/Windows controls and combined with shadcn/ui design). Applications depend on the umbrella crate `gpui-kit` (`crates/kit`), which pins the matching `gpui-pre-*` snapshot of GPUI and puts GPUI at its root (`use gpui_kit::*;`) with `gpui_kit::platform`, `gpui_kit::base`, `gpui_kit::component` and `gpui_kit::assets` reachable by name, so they never list GPUI itself. `gpui-shell` is not part of `gpui-kit` and is not published yet (its `llrt_*` dependencies are git-only); use it as a git dependency.

This is a Rust workspace project with the following main crates:

- `crates/kit` - Umbrella crate applications depend on (published as `gpui-kit`)
- `crates/component` - Core UI component library (published as `gpui-component`)
- `crates/story` - Gallery application for showcasing and testing components
- `crates/story-web` - Web version of the story gallery (using WebAssembly)
- `crates/component-macros` - Procedural macros (`IntoPlot` derive)
- `crates/assets` - Static assets
- `crates/webview` - WebView component support
- `examples/` - Various example applications

## Common Commands

### Development and Testing

```bash
# Run Story Gallery (component showcase application)
cargo run

# Run individual examples
cargo run --example hello_world
cargo run --example table

# Build the project
cargo build

# Lint check
cargo clippy -- --deny warnings

# Format check
cargo fmt --check

# Spell check
typos

# Check for unused dependencies
cargo machete
```

### Testing

**Note**: Per user configuration, tests do not need to be run.

For pure UI visual or sizing adjustments, do not add automated tests solely to
assert presentation dimensions. Add tests when the change affects behavior,
interaction, data flow, or prevents a meaningful regression.

```bash
# Run all tests
cargo test --all

# Run tests for a specific crate
cargo test -p gpui-component

# Run doc tests
cargo test -p gpui-component --doc
```

### Performance Profiling

```bash
# View FPS on macOS (using Metal HUD)
MTL_HUD_ENABLED=1 cargo run

# Profile performance using samply
samply record cargo run
```

## Core Architecture

### Architecture Refactoring Constraints

The implemented foundation architecture is documented in
`docs/ARCHITECTURE.md`, with styling and motion rules in
`docs/STYLING-AND-MOTION.md`. Preserve these constraints when designing or
implementing this architecture:

- Do not modify `gpui-base` unless the user explicitly requests a Base-layer
  change. By default, implement component behavior and visual styling in
  `crates/component` or the application layer.

- Keep GPUI Kit as the ecosystem and product brand; `gpui-component` is its
  styled component layer, alongside `gpui-base` and `gpui-shell`.
- Name the foundation crate `gpui-base`.
- Follow the ownership boundary: the framework owns behavior and infrastructure;
  the application owns component source and visual style.
- Keep the base layer visually unopinionated. It may provide interaction behavior,
  accessibility, focus, overlay and popup infrastructure, positioning, animation,
  virtual lists, dock infrastructure, and semantic design tokens.
- Theme APIs must expose semantic tokens (colors, spacing, radius, typography, and
  shadows), not an ever-growing set of component-specific styling fields.
- Keep source distribution or registry tooling above the `gpui-base` seam; no
  registry or CLI crate is currently part of the workspace.
- Preserve 100% backward compatibility for existing consumers, including current
  imports such as `use gpui_kit::component::button::Button;`.

### Component Initialization

**Critical requirement**: You must call `gpui_component::init(cx)` at your application's entry point before using any GPUI Component features.

```rust
fn main() {
    let app = Application::new();
    app.run(move |cx| {
        // This must be called first
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| MyView);
                // The first level view in a window must be a Root
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        }).detach();
    });
}
```

### Root View System

`Root` is the top-level view for a window and manages:

- Sheet (side panels)
- Dialog (dialogs)
- Notification (notifications)
- Keyboard navigation (Tab/Shift-Tab)

The first view of every window must be a `Root`.

### Theme System

- Uses `Theme` global singleton for theme configuration
- Supports light/dark mode switching
- Access theme via `ActiveTheme` trait: `cx.theme()`
- Theme configuration includes:
  - Colors (`ThemeColor`)
  - Syntax highlighting theme (`HighlightTheme`)
  - Font configuration (system font and monospace font)
  - UI parameters like border radius, shadows
  - Scrollbar display mode

### Dock System

Layout behavior lives in `crates/base/src/dock`; `crates/component/src/dock` is a
presentation skin (`DockSkin`) over it. See `docs/ARCHITECTURE.md`.

- **`LayoutTree`**: Pure-data layout tree, the single source of truth.
  - `NodeKind::Split` / `Tabs` / `Tiles`: containers, addressed by `NodeId`
  - Panels are addressed by `PanelId`; the tree holds no entity handles
- **`DockArea`**: Owns the center and dock trees, reconciles them into a
  cache of container entities keyed by `NodeId`
- **`TabGroup`** / **`TilesState`**: The `Tabs`/`Tiles` container entities
- **`Panel`**: Split at the seam — `gpui_base::dock::Panel` for behavior,
  `gpui_component::dock::Panel` for presentation; a panel implements both
- **`PanelRegistry`**: Resolves a persisted `panel_name` back to a panel type

The Dock system supports:

- Panel drag-and-drop reordering
- Panel zoom
- Layout locking
- Layout serialization/restoration

### Input System

Text input system based on Rope data structure:

- **InputState**: Input state management
- **Rope**: Efficient text storage (from ropey crate)
- LSP integration support (diagnostics, completion, hover)
- Syntax highlighting support (Tree-sitter)
- Multiple input modes:
  - Regular input (`Input`)
  - Number input (`NumberInput`)
  - OTP input (`OtpInput`)

### Component Design Principles

1. **Stateless design**: Use `RenderOnce` trait, components should be stateless when possible
2. **Size system**: Supports `xs`, `sm`, `md` (default), `lg` sizes via `Sizable` trait.
3. **Mouse cursor**: Buttons use `default` cursor not `pointer` (desktop app convention), unless it's a link button
4. **Style system**: Provides CSS-like styling API via `Styled` trait and `ElementExt` extensions
5. **Base controls are no-style**: Base controls and parts do not install layout,
   positioning, colors, sizing, gaps, radius, borders, shadows, variants, or animation.
   Complete presentation belongs to `crates/component` or the application. The deliberate
   exception is the foundational Base Input frame, which provides only a semantic
   one-pixel input border and semantic radius baseline; UI/application layers own
   its background, sizing, padding, typography, adornments, and richer focus style.
6. **GPUI builder style**: Keep element construction as one fluent builder chain. Express
   conditions with `when`, `when_some`, `when_none`, and `map`; do not split a chain into a
   mutable temporary element followed by imperative reassignment when the builder API can
   express the same operation.
7. **No `pub` fields on public data types**: A public struct handed across the
   `gpui-base`/application seam — a state snapshot, capability set, render context, or
   option set — keeps its fields private, is constructed with a builder, and is read
   through methods. Adding a `pub` field is a breaking change; adding one behind a builder
   is not. Setters and readers must not collide: an all-boolean type names setters after
   the field and readers `is_<adjective>`/`has_<noun>`, never `can_`; a type with
   non-boolean fields prefixes every setter with `with_` and keeps the field name for
   readers. Value types whose fields are the definition and cannot grow (`Point`,
   `Selection`, `Edges`) are exempt. See the "Public Data Types Across the Seam" section
   of `docs/ARCHITECTURE.md`.
8. **Spell `Context` out**: Name a context type `ComboboxTriggerContext`, never `…Ctx`.
   `cx` is reserved for GPUI's `App`, `Context<T>`, and `AsyncApp`, so `ctx` for anything
   else reads as a competing context. A callback receiving both takes the GPUI one as `cx`
   and names the other after what it holds (`trigger`, `state`).

## Code Style

- Follow naming and organization patterns from existing code
- Reference macOS/Windows control API design for naming
- AI-generated code must be refactored to match project style
- Mark AI-generated portions when submitting PRs
- When creating a PR, inspect previous PR titles in the repository and match
  that style. Do not blindly use conventional prefixes like `fix:` or `feat:`
  unless the existing PR title style uses them.
- When a PR changes the public API of `crates/component`, add a `## Breaking Changes`
  section with `diff` blocks showing the old and new usage. See PR #2691 and
  `.claude/skills/gpui-component-dev/references/pr-description.md`.
- Avoid `Kind` as a type-name suffix. It says an enum classifies something
  without saying what it classifies, and carries no meaning a reader could not
  already infer from `enum`. Name the type after what its variants _are_
  instead. Keep `Kind` only when no honest name covers the variant set —
  `NodeKind`'s variants straddle two levels (`Split` is an interior node,
  `Tabs` and `Tiles` are leaves), and every domain word for the leaf level
  (`Pane`, most of all) would misdescribe `Split`; a vaguer name is better
  than a precise wrong one. Prefer confining such a type to `pub(crate)`.
  This governs new code; existing `Kind` names are not a rewrite target on
  their own, and names owned by external crates (`CodeActionKind`,
  `CompletionItemKind`, `WindowKind`) keep their upstream spelling.

## Icon System

The `Icon` element does not include SVG files by default. You need to:

- Use [Lucide](https://lucide.dev) or other icon libraries
- Name SVG files according to the `IconName` enum definition (located in `crates/component/src/icon.rs`)

## Dependencies

- GPUI: Git version from Zed repository
- Tree-sitter: For syntax highlighting
- Ropey: Rope data structure for text, and `RopeExt` trait with more features.
- Markdown rendering: `markdown` crate
- HTML rendering: `html5ever` (basic support)
- Charts: Built-in chart components
- LSP: `lsp-types` crate

## Internationalization

Uses `rust-i18n` crate.

- Localization files are located in `crates/component/locales/`.
- Only add `en`, `zh-CN`, `zh-HK` by default.

## Documentation

- The documentation site source is in `website/`.
- Site docs have two locales: English (`website/docs/`) and Chinese (`website/zh-CN/docs/`).
- When modifying any documentation file, always sync changes to both `en` and `zh-CN` versions.
- `docs/` holds internal architecture specifications (RFC, migration status, reviews).
  These are single-language and are not published to the site; see `docs/README.md`.
- `skills/gpui-kit/references/coding-guides.md` and
  `skills/gpui-kit-design-guides/references/design-guides.md` are verbatim copies
  of the English `website/docs/` originals, vendored so the skills work after
  `npx skills add` in a project that does not have this repo. After editing either
  guide, copy it across:

  ```bash
  cp website/docs/design-guides.md skills/gpui-kit-design-guides/references/design-guides.md
  cp website/docs/coding-guides.md skills/gpui-kit/references/coding-guides.md
  ```

  CI fails if the copies drift. Never edit the copy directly — edit `website/docs/`.

## Platform Support

- macOS (aarch64, x86_64)
- Linux (x86_64)
- Windows (x86_64)

CI runs full test suite on each platform.

## Skills Reference

This project has custom Claude Code skills to assist with common development tasks:

- **gpui-kit** (`skills/`) - Building applications on the `gpui-kit` crate: setup, component catalog, stateless/stateful patterns, theming, GPUI mechanics (actions, async, contexts, custom elements, entities, events, focus, global state, layout, `ElementId`, testing), and the normative Coding Guides
- **gpui-kit-design-guides** (`skills/`) - The normative Design Guides; load before any UI, layout, interaction, or interface-copy work
- **gpui-component-dev** (`.claude/skills/`) - Contributing to gpui-component: creating new components, writing stories, writing documentation, writing PR descriptions

When working on tasks related to these areas, Claude Code will automatically use the appropriate skill to provide specialized guidance and patterns.

## Testing Guidelines

See `.claude/COMPONENT_TEST_RULES.md` for detailed testing principles:

- **Simplicity First**: Focus on complex logic and core functionality, avoid excessive simple tests
- **Builder Pattern Testing**: Every component should have a `test_*_builder` test covering the builder pattern
- **Complex Logic Testing**: Test conditional branching, state transitions, and edge cases

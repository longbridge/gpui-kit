# Executable application recipes

This consumer crate imports only `gpui-kit`, the way an application does. It holds the small set of **Tested consumer recipes**: complete sources for public ownership patterns that need compilation and interaction evidence. The settings recipe retains state and subscriptions, uses the shared `on_change` convention for Checkbox, Switch, and RadioGroup, separates typed Form fields from its footer, installs Root, and renders dialog, sheet, and notification layers.

From the repository root:

```sh
cargo run -p gpui-kit-recipes
cargo test -p gpui-kit-recipes
script/check-ai docs
```

The interaction test types into the input, checks the owner receives changes, forces an unrelated redraw, and types again. It catches both dropped subscriptions and state lifetime regressions. This is a GPUI test-window check; visual layout and OS accessibility require native review.

## Maintaining recipes

Documentation has two material classes:

- **Tested consumer recipes** are complete sources in this workspace. Use one for a public initialization sequence, ownership boundary, extension-trait import, or lifecycle that can otherwise drift. Add its stable ID, source, destinations, and `Tested consumer recipe` trust label to `recipes.json`, then add marker-delimited Rust fences at every destination.
- **Contextual fragments** explain one local API detail. Keep them concise and label their context when a reader could mistake them for a complete application; do not create a recipe merely to duplicate every component page.

Choose a canonical recipe when consumers need to combine components, retain state, or import a trait to make a public call work. Keep the authoritative source in `src/`, import only what `gpui-kit` exposes, and add an interaction test for behavior-bearing recipes.

`recipes.json` maps canonical source files to published documentation fragments. After editing and formatting a source, run `script/check-ai-recipes --sync`. Sync replaces only Rust fenced content between matching `<!-- recipe:<id>:start -->` and `<!-- recipe:<id>:end -->` markers; it never rewrites prose outside those markers. CI rejects an empty inventory, missing or duplicated IDs/destinations, invalid trust labels, and stale fragments.

## Acceptance standards

A change is ready when its observable behavior has a regression check, the relevant profile passes on the submitted revision, and the PR states what was verified and what remains untested:

| Change | Required command | Evidence |
| --- | --- | --- |
| Published recipe prose or fragments | `script/check-ai docs` | Source and documentation agree; drift detection tests pass |
| Rust application recipes | `cargo test -p gpui-kit-recipes` | Consumer crate compiles against `gpui-kit` alone and passes interaction tests |
| Component conventions | `script/check-ai rust` | Control callbacks, legacy aliases, and Form geometry pass contract tests without default features |
| Shell runtime or generated types | `script/check-ai shell` | Render tests, component binding tests, actual CLI failures, and pinned TypeScript positive/negative contracts pass |
| Changes across these areas | `script/check-ai all` and `cargo test -p gpui-kit-recipes` | All of the above |

Run additional existing tests for any changed subsystem. UI changes also need native evidence for the affected interactions, focus, layout, and accessibility. Shell `check` validates eager materialization; it does not prove deferred rendering, layout, paint, or later interactions.

These gates make compiler, runtime, and example failures reproducible for human and AI developers. They do not establish an AI model success rate. Any model evaluation must separately record the model/version, prompt, repository revision, task, first-attempt result, repair attempts, and independent behavior checks. Do not report a first-attempt success after a repair, or count a skipped check as a pass.

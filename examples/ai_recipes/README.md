# Executable application recipes

This standalone consumer imports only `gpui-kit`. Its own workspace prevents unrelated workspace members from supplying missing features or dependencies. The settings example retains state and subscriptions, uses the shared `on_change` convention for Checkbox, Switch, and RadioGroup, separates typed Form fields from its footer, installs Root, and renders dialog, sheet, and notification layers.

From the repository root:

```sh
cargo run --locked --manifest-path examples/ai_recipes/Cargo.toml
script/check-ai rust
```

The interaction test types into the input, checks the owner receives changes, forces an unrelated redraw, and types again. It catches both dropped subscriptions and state lifetime regressions. This is a GPUI test-window check; visual layout and OS accessibility require native review.

`recipes.json` maps source files to published documentation fragments. After editing and formatting the source, run `script/check-ai-recipes --sync`. CI rejects stale, missing, or duplicated fragments.

## Acceptance standards

A change is ready when its observable behavior has a regression check, the relevant profile passes on the submitted revision, and the PR states what was verified and what remains untested:

| Change | Required command | Evidence |
| --- | --- | --- |
| Published recipe prose or fragments | `script/check-ai docs` | Source and documentation agree; drift detection tests pass |
| Rust application recipes and component conventions | `script/check-ai rust` | Isolated consumer compiles and passes interaction tests; control callbacks, legacy aliases, and Form geometry pass contract tests |
| Shell runtime or generated types | `script/check-ai shell` | Render tests, component binding tests, actual CLI failures, and pinned TypeScript positive/negative contracts pass |
| Changes across these areas | `script/check-ai all` | All of the above |

Run additional existing tests for any changed subsystem. UI changes also need native evidence for the affected interactions, focus, layout, and accessibility. Shell `check` validates eager materialization; it does not prove deferred rendering, layout, paint, or later interactions.

These gates make compiler, runtime, and example failures reproducible for human and AI developers. They do not establish an AI model success rate. Any model evaluation must separately record the model/version, prompt, repository revision, task, first-attempt result, repair attempts, and independent behavior checks. Do not report a first-attempt success after a repair, or count a skipped check as a pass.

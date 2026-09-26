# Input UI regression tests

These tests mount the production `Input`, `Textarea` and `Editor` components
under the normal window `Root`. They exercise pointer hit testing, keyboard
bindings, focus routing and rendered state together. The test target is
[`input.rs`](../input.rs); the modules here group related user workflows.

Run the complete suite from the repository root:

```sh
cargo test -p gpui-kit --features test-support --test input --locked
```

To investigate one area, add its module name after `--`, for example:

```sh
cargo test -p gpui-kit --features test-support --test input --locked -- history::
```

The existing CI test matrix runs this target on Linux, macOS and Windows.
Keyboard cases use each platform's actual command bindings. A local Linux
pass does not replace the macOS and Windows jobs.

## Recorded workflows

| Module | Interaction contracts |
| --- | --- |
| [`editing.rs`](editing.rs) | Selection direction and replacement; Home/End; deletion boundaries; clipboard normalization; double-click and drag; Enter events; Tab through decorated inputs |
| [`history.rs`](history.rs) | Typing groups; cursor and blur boundaries; atomic paste; restored selections; no-op edits preserving redo; redo branch invalidation; emoji and combining-mark boundaries |
| [`constraints.rs`](constraints.rs) | Read-only/disabled transitions; rejected edits and event counts; owner updates; masking and password privacy; validation; mask formatting; clear affordance |
| [`textarea.rs`](textarea.rs) | Enter versus submit; multiline clipboard and CRLF; wrapped-row navigation; caret reveal after scrolling; bounded auto-grow; resize reflow |
| [`editor.rs`](editor.rs) | Generated pairs and closers; Unicode inside quotes; language differences; paired deletion and undo; indentation; multi-cursor replacement; search and replace overlays |
| [`completions.rs`](completions.rs) | Provider requests through typed input; popup acceptance/cancellation; keyboard selection; filtered suggestions; completion undo boundaries |
| [`lifecycle.rs`](lifecycle.rs) | Focus routing across all three controls; selection across parent renders; read-only transitions; unmount/remount with retained state |

The original cases in [`input.rs`](../input.rs) also cover scoped duplicate IDs,
cross-scope keyboard rejection and masked values. The table is an index to
concrete tests, not a claim of exhaustive coverage. Inline tokens, touch selection,
InputGroup composition and language-service providers retain their separate
Base/component tests; they are not all exercised by this target. The completion
fixture supplies deterministic responses through the public provider interface;
it does not connect to a language-server process.

## Writing a regression case

Start with the smallest user sequence that demonstrates the bug. Mount the
real component with retained state and a stable ID, click it, send keyboard or
pointer events, and assert the result after each meaningful step. See
[`lifecycle.rs`](lifecycle.rs) for a workflow applied to all three controls.

Use `window.input` for typing and `window.press` for commands such as Enter,
Backspace and Undo. Prepare clipboard data through the test application's
clipboard, then send the Paste shortcut. Calling `set_value`, `replace_all`,
`undo` or a private event handler to perform the interaction would bypass
the routing this suite is intended to protect. Public setters are appropriate
for initial fixtures and explicit external-owner updates.

Observe two sides of important interactions: a fresh `window.find(...)`
snapshot for exposed value/focus, and public state or an owner event for
selection, cursor, text or callback behavior. Snapshots are immutable; never
reuse one to assert a later frame. Password snapshots intentionally omit text;
read the retained state only to prove editing still happened without exposing
the secret through accessibility.

Leave `update_window` before checking deferred owner callbacks. Use
`cx.run_until_parked()` for queued work or `wait_for` for a bounded asynchronous
condition. Do not add wall-clock sleeps. For geometry, check relationships
such as caret containment, scroll direction or relative height, rather than
font-dependent pixel constants.

For a bug fix, confirm the new test fails with the bug present and passes with
the fix. Keep its name tied to the user-visible contract, and include the
boundary that caused the failure: Unicode, selection direction, a read-only
transition, a no-op edit, or a focus change.

## What a green run establishes

This suite establishes the recorded interaction contracts for the tested
configurations. Existing Base tests continue to cover editing algorithms,
IME composition state transitions and language-specific parsing cases.
Neither set exhausts every document, language, configuration or event order.

Changes to native input-method integration, accessibility adapters, or drawing
still need the corresponding platform checks:

- Full OS IME composition, candidate windows and keyboard layouts need a real
  platform input method. Typing Unicode into the test window does not exercise
  those facilities.
- An exposed value/focus snapshot does not dispatch an OS accessibility action.
  Test packaged-app Focus/SetValue and screen-reader behavior using the
  [accessibility testing guide](../../../../docs/ACCESSIBILITY-UI-TESTING.md).
- Layout/state assertions do not compare pixels. The separate macOS Metal
  `rendering` target checks selected drawing contracts.
- Real clipboard integration and system context menus also need platform
  coverage when those adapters change.

For ordinary editing changes, use the suite plus the existing Base regressions
and review the changed behavior's case. For platform changes, add the relevant
platform evidence to the PR so reviewers know what has actually been checked.

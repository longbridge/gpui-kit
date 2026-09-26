# Input UI regression tests

These tests mount the production `Input`, `Textarea` and `Editor` components
under the normal window `Root`. They exercise pointer hit testing, keyboard
bindings, focus routing and rendered state together. The test target is
[`input.rs`](../input.rs); the modules here group related user workflows.

For an Input change, run the complete regression gate from the repository root
(Bash, including Git Bash on Windows):

```sh
script/test-input
```

This runs Base editing/IME/token tests, Component InputGroup/overlay tests,
and Kit input, focus, touch-selection and keyboard-helper workflows. These
targets also run in the existing Linux, macOS and Windows CI matrix.

For a quicker editing/focus iteration:

```sh
cargo test -p gpui-kit --features test-support --test input --test input_focus --locked
```

To investigate one area, add its module name after `--`, for example:

```sh
cargo test -p gpui-kit --features test-support --test input --locked -- history::
```

To list the selected cases without executing them, append `-- --list` to the
combined command above. To rerun one workflow with an exact name:

```sh
cargo test -p gpui-kit --features test-support --test input --locked -- history::paste_is_atomic_and_separate_from_surrounding_typing --exact
cargo test -p gpui-kit --features test-support --test input_focus --locked -- reverse_tab_cycles_three_inputs_with_passive_addons --exact
```

These commands are reproduction instructions, not a record of passing results.
Record the revision, platform, command and actual outcome when reporting a run.

The existing CI test matrix runs these targets on Linux, macOS and Windows.
Keyboard cases use each platform's actual command bindings. A local Linux
pass does not replace the macOS and Windows jobs.

## Regressions found by this suite

- `constraints::disabled_single_and_double_click_do_not_focus_the_editor`:
  single and double clicks on a disabled input must not focus its editor.
  Enabling is covered separately by
  `constraints::enabling_a_disabled_input_allows_mouse_focus_and_replacement`.
- `textarea::selection_across_soft_wraps_copies_and_replaces_buffer_text`:
  Shift-Down should retain the selection anchor and reach the same visual row as
  Down, including when a logical line spans multiple wrapped rows.

Reproduce each contract independently from the repository root:

```sh
cargo test -p gpui-kit --features test-support --test input --locked -- constraints::disabled_single_and_double_click_do_not_focus_the_editor --exact
cargo test -p gpui-kit --features test-support --test input --locked -- textarea::selection_across_soft_wraps_copies_and_replaces_buffer_text --exact
```

These and the additional regressions below run in the full `input` target,
without ignored cases:

- Selection at document edges, the first vertical move after Alt-click,
  horizontal-to-vertical selection, and soft-wrap End affinity.
- Drag autoscroll extends into newly revealed text instead of jumping to EOF.
- Pending completion/inline requests cannot reappear after cancellation,
  editing, blur, protection changes or popup acceptance.
- IME selection direction and canceled-preedit undo boundaries on all controls.
- Replacement-mode toggles preserve replacement focus through the next frame.
- Keyboard definition requests work without prior hover, survive pointer leave,
  and reject superseded requests or changed text, caret and focus.

## Recorded workflows

| Module | Interaction contracts |
| --- | --- |
| [`editing.rs`](editing.rs) | Selection direction and replacement; Home/End; deletion boundaries; clipboard normalization; double-click and drag; Enter events; Tab through decorated inputs |
| [`history.rs`](history.rs) | Typing groups; cursor and blur boundaries; atomic paste; restored selections; no-op edits preserving redo; redo branch invalidation; emoji and combining-mark boundaries |
| [`constraints.rs`](constraints.rs) | Read-only/disabled transitions; rejected edits and event counts; owner updates; masking and password privacy; validation; mask formatting; clear affordance |
| [`textarea.rs`](textarea.rs) | Enter versus submit; multiline clipboard and CRLF; wrapped-row navigation; caret reveal after scrolling; bounded auto-grow; resize reflow |
| [`editor.rs`](editor.rs) | Generated pairs and closers; Unicode inside quotes; language differences; paired deletion and undo; indentation; multi-cursor replacement; search and replace overlays |
| [`completions.rs`](completions.rs) | Provider requests through typed input; popup acceptance/cancellation; keyboard selection; filtered suggestions; completion undo boundaries |
| [`composition.rs`](composition.rs) | IME bridge preedit, commit/cancel, UTF-16 selection direction and history boundaries |
| [`lifecycle.rs`](lifecycle.rs) | Focus routing across all three controls; selection across parent renders; read-only transitions; unmount/remount with retained state |

The original cases in [`input.rs`](../input.rs) also cover scoped duplicate IDs,
cross-scope keyboard rejection and masked values. The table is an index to
concrete tests, not a claim of exhaustive coverage. Inline tokens, touch selection,
InputGroup composition are covered by separate Base/component tests included in
`script/test-input`.
Touch selection also has a separate Kit `touch_selection` target. Completion, code-action, inline-suggestion and definition
fixtures supply deterministic responses through the public provider interfaces;
they do not connect to a language-server process.

The composition fixture calls the public `ElementInputHandler` bridge for the
mounted state. It exercises the IME protocol methods and subsequent rendered
state, rather than the platform-installed handler or a real input method.
The in-flight completion disposal fixture is deliberately inactive because
GPUI TestPlatform retains the active window's input handler after removal;
active-window disposal needs separate platform evidence.

The multi-cursor vertical-selection case starts directly after Alt-click and
checks both replacement ranges and their restoration by Undo, without an
intervening horizontal move to repair a missing preferred column.

## Operation coverage and review gate

| Operation family | Automated evidence |
| --- | --- |
| Typing, replacement, direction, empty boundaries | `editing`, `history`, `lifecycle`; text and selection endpoints |
| Character/word/line/document movement and deletion | `editing`, `textarea`, `editor`; platform bindings, Unicode scalar and CRLF boundaries |
| Visual-row and page movement | `textarea`; soft wrap, document edges, preferred columns, short rows and reversal |
| Pointer selection | `editing`, `textarea`, `editor`; clicks, Shift-click, word/paragraph selection, drag direction, autoscroll and multi-cursor placement |
| Clipboard and history | `history`, `constraints`, `editor`, `lifecycle`; sentinels, normalization, undo/redo branches, restored ranges |
| Protection, validation and masking | `constraints`, `lifecycle`; rejected edits, selection/history, callbacks, re-enabling and password privacy |
| Focus and lifecycle | `input_focus`, `lifecycle`; traversal, addons, Focus/Blur events, parent renders, unmount and release |
| Multiline layout | `textarea`; caret reveal, scrolling, resize, fixed rows and bounded auto-grow |
| Code editing | `editor`; pairs, language-specific indentation, block selection, folding, multiple cursors, literal search and replacement |
| Language providers | `completions`; providers, acceptance/cancellation, stale results, failure, focus/protection transitions inline suggestions and definition navigation |
| IME protocol | `composition` and Base tests; UTF-16 ranges, preedit replacement, commit/cancel, selection direction and undo boundaries on all three controls |
| Inline tokens and grouped inputs | Base `input::state::tests::test_inline_token_*` and Component `input::group::tests`; atomic edits, clipboard, IME geometry, addons and paste hooks |
| Touch selection | Kit `touch_selection` and Base touch tests; long press, handles, copying and scrolling |

For a normal editing PR, require the platform CI matrix to pass and require a
regression case for its changed interaction. Check that the case fails before
the fix; a green suite alone cannot prove a new behavior. Review assertions for
text **and** caret/selection/focus or owner callbacks, as appropriate. This is
the normal gate instead of repeating a manual tour of every input operation.

Do not hide a defect by inserting a click that cancels an asynchronous request,
reusing clipboard contents from an earlier control, or resetting the caret
before checking its direction. Focus-cancellation tests activate the window,
observe Blur, and refocus through Tab. Provider responses are released explicitly.

Text movement currently follows Unicode scalar boundaries, with CRLF treated as
one newline; it does not promise grapheme-cluster movement for combining marks
or ZWJ emoji. Tests preserve that existing contract. Duplicate/move-line commands,
regex search and Shift-Page selection are not implemented operations.

The separate [`input_focus.rs`](../input_focus.rs) target covers repeated Tab and
Shift-Tab cycles with passive prefixes/suffixes, focus and activation of addon
buttons, and clicking the body of Textarea/Editor before editing. Run it alongside
`input` when changing focus routing; a module filter on `input` does not select it.

## Example workflows

- History: type a prefix, paste from the test clipboard, then type a suffix;
  Undo/Redo should preserve the asserted edit boundaries. See
  `history::paste_is_atomic_and_separate_from_surrounding_typing`.
- Textarea: enable submit-on-Enter, press Enter, then Shift-Enter; check the
  emitted submit event and the resulting text separately. See
  `textarea::submit_on_enter_preserves_text_but_shift_enter_inserts`.
- Editor: type a completion trigger, inspect the popup, accept with Enter,
  then verify text and Undo boundaries. See `completions.rs`.
- Focus: repeatedly traverse decorated inputs in both directions, allow queued
  focus callbacks to settle, then type into the destination. See `input_focus.rs`.

## Writing a regression case

Start with the smallest user sequence that demonstrates the bug. Mount the
real component through `gpui_kit::open_window` after `gpui_kit::init`, retain its
state and give it a stable ID. Click it, send keyboard or
pointer events, and assert the result after each meaningful step. See
[`lifecycle.rs`](lifecycle.rs) for a workflow applied to all three controls.

Use `window.input` for typing and `window.press` for commands such as Enter,
Backspace and Undo. Command presses use native key-down/key-up events; do not
simulate Enter by injecting a newline through an IME text callback. Unicode
typing through `window.input` does not establish OS IME coverage. Prepare clipboard
data through the test application's clipboard, then send the Paste shortcut. Calling `set_value`, `replace_all`,
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
condition. History tests that depend on focus/blur callbacks must first activate
the window with `window.activate_window()` inside `cx.update_window`, then allow
queued work to settle. See `history::blur_splits_typing_without_moving_the_caret`;
assigning a focus handle alone does not establish an active-window callback flow.
Do not add wall-clock sleeps. For geometry, check relationships
such as caret containment, scroll direction or relative height, rather than
font-dependent pixel constants.

For a bug fix, confirm the new test fails with the bug present and passes with
the fix. Keep its name tied to the user-visible contract, and include the
boundary that caused the failure: Unicode, selection direction, a read-only
transition, a no-op edit, or a focus change.

## What a green run establishes

A passing run establishes the recorded interaction contracts for the tested
configurations. Existing Base tests
continue to cover editing algorithms,
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

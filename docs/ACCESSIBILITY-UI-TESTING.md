# Accessibility-driven UI testing

When a UI change needs manual interactive verification, use the macOS
accessibility tree by default for behavior that depends on focus, keyboard
input, selection, menus, or other real window-system state. Review the relevant
automated UI coverage first; do not repeat manual checks for behavior those
tests already cover. Use this guide for uncovered behavior that needs a real
window-system check.

## Start the application

Run the Story gallery as a signed macOS application bundle:

```sh
./script/run-story-macos
```

Do not use a bare `cargo run` process for accessibility testing. macOS cannot
reliably address an unbundled executable as an application. The script builds
`/tmp/GPUIComponentStory.app`, assigns the stable bundle identifier
`com.longbridge.gpui-component-story`, signs it locally, and starts its binary.

## Drive the accessibility tree

1. Read the complete accessibility tree for
   `com.longbridge.gpui-component-story`.
2. Locate controls by role, accessible label, placeholder, and current value.
3. Prefer actions on accessibility element indexes over screen coordinates.
4. After every state-changing action, fetch the tree again. Element indexes
   are snapshots and must not be reused without verifying the current tree.
5. Assert behavior from semantic properties such as role, enabled/settable
   state, value, label, focus, selection, and exposed secondary actions.
6. Use screenshots only when the accessibility tree cannot express a visual
   requirement. Coordinate input is a fallback, not the default.

The gallery search field can navigate directly to a story. For example, set
the `Search…` text field to `Input` to expose the Input story controls in the
tree.

## Keyboard interaction tests

Use real key events after focusing the target through the accessibility tree.
For an editable component, cover the interaction boundaries relevant to the
change:

- typing and editing values;
- focus and keyboard navigation;
- selection movement and replacement;
- undo and redo;
- Backspace versus Forward Delete;
- paste, cut, Enter, Escape, and other command boundaries;
- disabled, read-only, and secret-value accessibility behavior.

Read the tree after each checkpoint and assert the control's exposed value.
For undo history, a representative sequence is:

```text
type "ab" -> Left -> type "x" -> value "axb"
Undo                                -> value "ab"
Undo                                -> empty
Redo                                -> value "ab"
```

Also verify that a no-op edit, such as Backspace at offset zero, does not
destroy an existing redo branch.

## Manual verification evidence

When manual accessibility verification is needed, report:

- the app and story tested;
- the accessibility roles/labels used to find the controls;
- the input sequence and values observed at each checkpoint;
- any behavior that required screenshot or coordinate fallback;
- automated test, formatting, and lint results separately.

Accessibility-driven testing complements Rust tests. It does not replace unit
or integration coverage for the same state transitions. In a review, identify
the specific behavior the automated tests do not cover before requesting a
manual check; report automated and manual evidence separately.

# Block editor (spike)

Block-based WYSIWYG Markdown editor for gpui-kit, behind the off-by-default
`block-editor` feature (`gpui-kit/block-editor` forwards to
`gpui-base/block-editor`; nothing is re-exported from `gpui-component`).
Design: `docs/design/block-editor.md` (tracking: longbridge/gpui-kit#3049).

## Public API

| Item | Role |
|---|---|
| `BlockEditorState` | Entity: model + focus + IME + key bindings. `new`, `with_source`, `set_source`, `source`, `run(EditorCommand)`, `undo`/`redo`, `set_selection`, `set_readonly`, `set_editor_style`, `set_scroll_handle`. Emits `BlockEditorEvent::{Changed, SelectionChanged}`; no event for no-op commands. |
| `BlockEditor` | View: `BlockEditor::new(&state)`. Renders through gpui-kit's text renderer. |
| `BlockEditorModel` | Headless model (document, selection, history, commands). |
| `EditorCommand` | `ToggleMark`, `ApplyMark`, `SetBlockKind`, `ToggleList`, `ToggleTask`, `Indent`, `Outdent`, `MoveBlocks`, `InsertBlock`, `SelectAll`, `DeleteSelection`. |
| `BlockEditorStyle` | Caret / selection / composition colours (defaults come from the theme). |

## Scrolling

Put the editor in your own scroll container and pass that container's handle
with `set_scroll_handle`; the caret is then scrolled into view after it moves.
Without a handle, caret-into-view is a no-op. Either way, blocks outside the
visible area are culled (light virtualization; full virtualization is
Phase 5).

## Paste

- Cmd/Ctrl+V: single-line text is inserted inline. Multi-block Markdown is
  converted to blocks in one undo step; a leading or trailing paragraph merges
  into the text around the caret.
- Cmd/Ctrl+Shift+V: plain text; each line becomes a paragraph.

## Story

`cargo run -p gpui-component-story --features block-editor -- BlockEditor`

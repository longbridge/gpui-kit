//! Block editor spike (`block-editor` feature): paragraphs, headings, lists,
//! bold/italic/code marks, cross-block selection, IME, undo/redo.

use gpui_kit::base::text::block_editor::{
    BlockEditor, BlockEditorEvent, BlockEditorState, BlockKind, EditorCommand, MarkKind, TableEdit,
};
use gpui_kit::component::{
    button::Button,
    h_flex,
    input::{Input, InputState},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _,
    Styled, Subscription, Window, div, rems,
};

use crate::section;

const SAMPLE: &str = "# Block editor\n\nType here. **Bold** with Ctrl/Cmd-B, *italic* with Ctrl/Cmd-I, `code` with Ctrl/Cmd-E (or type `code`, **bold**, *italic*, ~~strike~~).\n\n## Lists\n\n- Bullet one\n- Bullet two\n  - Nested (Tab / Shift-Tab)\n\n1. First\n2. Second\n\nUndo with Ctrl/Cmd-Z, redo with Ctrl/Cmd-Shift-Z. Drag or Shift-arrows to select across blocks.\n\n```rust\nfn main() {\n    println!(\"code blocks: type ``` then Enter\");\n}\n```\n\n| Feature | Status |\n| --- | --- |\n| Tables | click a cell to edit, Tab to move |\n| Lists | Enter on an empty item exits |";

/// A generated document of 1000+ blocks for the perf note.
pub fn long_document(blocks: usize) -> String {
    let mut doc = String::new();
    for i in 0..blocks {
        match i % 4 {
            0 => doc.push_str(&format!("## Section {i}\n\n")),
            1 => doc.push_str(&format!("- Item {i} with **bold** and `code`\n\n")),
            2 => doc.push_str(&format!("1. Ordered {i}\n\n")),
            _ => doc.push_str(&format!(
                "Paragraph {i} with *italic* text that is long enough to wrap onto a second line in a narrow panel.\n\n"
            )),
        }
    }
    doc
}

pub struct BlockEditorStory {
    editor: Entity<BlockEditorState>,
    /// The editor's scroll container; the editor keeps the caret in view.
    scroll: ScrollHandle,
    status: SharedString,
    revision: u64,
    /// Markdown preview, refreshed on `Changed` only (never per frame).
    markdown: SharedString,
    /// The link popover (Cmd/Ctrl+K, D14): open while `Some`.
    link_input: Entity<InputState>,
    link_open: bool,
    /// A Cmd/Ctrl+K request not yet shown (needs a window to fill the input).
    pending_link: Option<SharedString>,
    /// The url under the mouse, shown as the link hover.
    hovered_url: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for BlockEditorStory {
    fn title() -> &'static str {
        "BlockEditor"
    }

    fn description() -> &'static str {
        "Block-based WYSIWYG Markdown editor (experimental spike)."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl BlockEditorStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let scroll = ScrollHandle::new();
        let editor = cx.new(|cx| {
            let mut state = BlockEditorState::new(window, cx).with_source(SAMPLE);
            state.set_scroll_handle(scroll.clone());
            state
        });
        let subscription = cx.subscribe(&editor, |this, editor, event: &BlockEditorEvent, cx| {
            // The status follows every caret move, not only edits, so it
            // never shows the previous action's caret.
            match event {
                BlockEditorEvent::Changed { revision, .. } => {
                    this.revision = *revision;
                    this.refresh_markdown(cx);
                }
                BlockEditorEvent::SelectionChanged => {}
                BlockEditorEvent::EditLinkRequested { url } => {
                    this.pending_link = Some(url.clone().unwrap_or_default());
                    cx.notify();
                    return;
                }
                BlockEditorEvent::LinkHovered { url } => {
                    this.hovered_url = url.clone();
                    cx.notify();
                    return;
                }
                BlockEditorEvent::LinkClicked { url } => {
                    // The host decides what to do; the story only reports it.
                    this.status = format!("link clicked: {url}").into();
                    cx.notify();
                    return;
                }
                _ => return,
            }
            let kind = editor.read(cx).block_kind_at_caret();
            this.status = format!("revision {}, caret in {kind:?}", this.revision).into();
            cx.notify();
        });
        let link_input = cx.new(|cx| InputState::new(window, cx).placeholder("https://…"));
        let mut this = Self {
            link_input,
            link_open: false,
            pending_link: None,
            hovered_url: None,
            editor,
            scroll,
            // Never empty, so the toolbar row keeps its height and the
            // editor does not jump when the first status arrives.
            status: "revision 0".into(),
            revision: 0,
            markdown: "".into(),
            _subscriptions: vec![subscription],
        };
        this.refresh_markdown(cx);
        this
    }

    fn refresh_markdown(&mut self, cx: &App) {
        let source = self.editor.read(cx).source();
        self.markdown = source.chars().take(2000).collect::<String>().into();
    }

    fn load(&mut self, source: &str, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| {
            let _ = e.set_source(source, cx);
        });
        self.refresh_markdown(cx);
        cx.notify();
    }

    fn close_link(&mut self, url: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = url {
            self.editor.update(cx, |e, cx| {
                e.set_link(Some(&url).filter(|u| !u.is_empty()).map(|u| u.as_str()), cx)
            });
        }
        self.link_open = false;
        self.editor.update(cx, |e, cx| e.focus(window, cx));
        cx.notify();
    }

    fn command_button(
        id: &'static str,
        label: &'static str,
        command: EditorCommand,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                let command = command.clone();
                this.editor.update(cx, |e, cx| {
                    let _ = e.run(command, cx);
                    e.focus(window, cx);
                });
            }))
    }
}

impl Render for BlockEditorStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(url) = self.pending_link.take() {
            self.link_input
                .update(cx, |input, cx| input.set_value(url, window, cx));
            self.link_open = true;
            let focus = self.link_input.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
        let link_bar =
            self.link_open.then(|| {
                h_flex()
                    .gap_2()
                    .child(div().text_sm().child("Link"))
                    .child(div().w(rems(20.)).child(Input::new(&self.link_input)))
                    .child(
                        Button::new("link-apply")
                            .label("Apply")
                            .on_click(cx.listener(|this, _, window, cx| {
                                let url = this.link_input.read(cx).value().to_string();
                                this.close_link(Some(url), window, cx);
                            })),
                    )
                    .child(
                        Button::new("link-remove")
                            .label("Remove")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_link(Some(String::new()), window, cx)
                            })),
                    )
                    .child(Button::new("link-cancel").label("Cancel").on_click(
                        cx.listener(|this, _, window, cx| this.close_link(None, window, cx)),
                    ))
            });
        let hover = self
            .hovered_url
            .clone()
            .map(|url| format!("{url}  (Ctrl/Cmd-click to follow)"))
            .unwrap_or_default();
        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(Self::command_button(
                        "bold",
                        "B",
                        EditorCommand::ToggleMark(MarkKind::Bold),
                        cx,
                    ))
                    .child(Self::command_button(
                        "italic",
                        "I",
                        EditorCommand::ToggleMark(MarkKind::Italic),
                        cx,
                    ))
                    .child(Self::command_button(
                        "strike",
                        "S",
                        EditorCommand::ToggleMark(MarkKind::Strikethrough),
                        cx,
                    ))
                    .child(Self::command_button(
                        "inline-code",
                        "`code`",
                        EditorCommand::ToggleMark(MarkKind::Code),
                        cx,
                    ))
                    .child(Self::command_button(
                        "p",
                        "Text",
                        EditorCommand::SetBlockKind(BlockKind::Paragraph),
                        cx,
                    ))
                    .child(Self::command_button(
                        "h1",
                        "H1",
                        EditorCommand::SetBlockKind(BlockKind::Heading { level: 1 }),
                        cx,
                    ))
                    .child(Self::command_button(
                        "h2",
                        "H2",
                        EditorCommand::SetBlockKind(BlockKind::Heading { level: 2 }),
                        cx,
                    ))
                    .child(Self::command_button(
                        "h3",
                        "H3",
                        EditorCommand::SetBlockKind(BlockKind::Heading { level: 3 }),
                        cx,
                    ))
                    .child(Self::command_button(
                        "ul",
                        "Bullets",
                        EditorCommand::ToggleList { ordered: false },
                        cx,
                    ))
                    .child(Self::command_button(
                        "ol",
                        "Numbered",
                        EditorCommand::ToggleList { ordered: true },
                        cx,
                    ))
                    .child(Self::command_button(
                        "task",
                        "Task",
                        EditorCommand::ToggleTask,
                        cx,
                    ))
                    .child(Button::new("link").label("Link").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.editor.update(cx, |e, cx| e.request_link_edit(cx));
                        },
                    )))
                    .child(Self::command_button(
                        "code",
                        "Code",
                        EditorCommand::SetCodeBlock { lang: None },
                        cx,
                    ))
                    .child(Self::command_button(
                        "table",
                        "Table",
                        EditorCommand::InsertTable { rows: 3, cols: 3 },
                        cx,
                    ))
                    .child(Self::command_button(
                        "row+",
                        "+Row",
                        EditorCommand::EditTable(TableEdit::AddRowBelow),
                        cx,
                    ))
                    .child(Self::command_button(
                        "row-",
                        "-Row",
                        EditorCommand::EditTable(TableEdit::RemoveRow),
                        cx,
                    ))
                    .child(Self::command_button(
                        "col+",
                        "+Col",
                        EditorCommand::EditTable(TableEdit::AddColumnRight),
                        cx,
                    ))
                    .child(Self::command_button(
                        "col-",
                        "-Col",
                        EditorCommand::EditTable(TableEdit::RemoveColumn),
                        cx,
                    ))
                    .child(
                        Button::new("sample")
                            .label("Sample")
                            .on_click(cx.listener(|this, _, _, cx| this.load(SAMPLE, cx))),
                    )
                    .child(Button::new("long").label("Load 1200 blocks").on_click(
                        cx.listener(|this, _, _, cx| this.load(&long_document(1200), cx)),
                    ))
                    .child(div().text_sm().child(self.status.clone())),
            )
            .children(link_bar)
            .child(div().h(rems(1.25)).text_xs().child(hover))
            .child(
                section("Editor").child(
                    div()
                        .id("block-editor-scroll")
                        .flex_1()
                        .w_full()
                        .max_h(rems(28.))
                        .overflow_y_scroll()
                        // Scroll containment: wheel events over this pane
                        // must not also scroll the story/gallery.
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        .track_scroll(&self.scroll)
                        .child(BlockEditor::new(&self.editor)),
                ),
            )
            .child(
                section("Markdown").child(
                    div()
                        .id("block-editor-markdown")
                        .flex_1()
                        .w_full()
                        .max_h(rems(28.))
                        .overflow_y_scroll()
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        .text_xs()
                        .whitespace_normal()
                        .child(self.markdown.clone()),
                ),
            )
    }
}

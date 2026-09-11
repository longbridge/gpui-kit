mod input;
mod model;
#[cfg(test)]
mod tests;

use crate::text::{
    MarkdownExtensions, TextViewDefaults, TextViewStyle,
    document::NodeRenderOptions,
    inline::{Inline, InlineInteraction},
    node::{NodeContext, TextMark},
};
use crate::{input::blink_cursor::BlinkCursor, undo_history::UndoHistory};
use gpui::*;
use model::{Document, Position};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};

actions!(
    markdown_editor,
    [
        Indent,
        Outdent,
        Backspace,
        Delete,
        Enter,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        Home,
        End,
        SelectAll,
        Undo,
        Redo,
        Copy,
        Cut,
        Paste,
        Bold,
        Italic
    ]
);

// Register one shared action context for every document editor.
pub(super) fn init(cx: &mut App) {
    let modifier = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    cx.bind_keys([
        KeyBinding::new("tab", Indent, Some("MarkdownEditor")),
        KeyBinding::new("shift-tab", Outdent, Some("MarkdownEditor")),
        KeyBinding::new("backspace", Backspace, Some("MarkdownEditor")),
        KeyBinding::new("delete", Delete, Some("MarkdownEditor")),
        KeyBinding::new("enter", Enter, Some("MarkdownEditor")),
        KeyBinding::new("left", Left, Some("MarkdownEditor")),
        KeyBinding::new("right", Right, Some("MarkdownEditor")),
        KeyBinding::new("up", Up, Some("MarkdownEditor")),
        KeyBinding::new("down", Down, Some("MarkdownEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("MarkdownEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("MarkdownEditor")),
        KeyBinding::new("shift-up", SelectUp, Some("MarkdownEditor")),
        KeyBinding::new("shift-down", SelectDown, Some("MarkdownEditor")),
        KeyBinding::new("home", Home, Some("MarkdownEditor")),
        KeyBinding::new("end", End, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-a"), SelectAll, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-z"), Undo, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-shift-z"), Redo, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-y"), Redo, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-c"), Copy, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-x"), Cut, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-v"), Paste, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-b"), Bold, Some("MarkdownEditor")),
        KeyBinding::new(&format!("{modifier}-i"), Italic, Some("MarkdownEditor")),
    ]);
}

// Notifications describe committed document changes.
#[derive(Clone, Debug)]
pub enum MarkdownEditorEvent {
    Change,
}

#[derive(Clone)]
struct Snapshot {
    document: Document,
    anchor: Position,
    cursor: Position,
}

#[derive(Clone)]
struct Edit {
    before: Snapshot,
    after: Snapshot,
}

struct Layout {
    source: Range<usize>,
    text: TextLayout,
    bounds: Bounds<Pixels>,
}

// One entity owns all editing state across the document.
pub struct MarkdownEditorState {
    document: Document,
    anchor: Position,
    cursor: Position,
    focus: FocusHandle,
    layouts: HashMap<(u64, usize), Layout>,
    history: UndoHistory<Edit>,
    composition: Option<(Position, Position)>,
    composition_before: Option<Snapshot>,
    stored: Option<TextMark>,
    // Retain the initial selection while dragging after a multi-click.
    dragging: Option<(Position, Position)>,
    readonly: bool,
    active: bool,
    blink: Entity<BlinkCursor>,
    _subscriptions: Vec<Subscription>,
    scroll: ScrollHandle,
}

impl EventEmitter<MarkdownEditorEvent> for MarkdownEditorState {}

impl Focusable for MarkdownEditorState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl MarkdownEditorState {
    // Construct a document editor from Markdown.
    pub fn new(source: &str, cx: &mut Context<Self>) -> Result<Self, SharedString> {
        Self::new_with_extensions(source, MarkdownExtensions::default(), cx)
    }

    // Use the same parser and renderer plugins as a Markdown reading view.
    pub fn new_with_extensions(
        source: &str,
        extensions: MarkdownExtensions,
        cx: &mut Context<Self>,
    ) -> Result<Self, SharedString> {
        let document = Document::parse_with_extensions(source, extensions)?;
        let cursor = document.first();
        let blink = cx.new(|_| BlinkCursor::new());
        let subscriptions = vec![cx.observe(&blink, |state: &mut Self, _, cx| {
            if state.active {
                cx.notify();
            }
        })];
        Ok(Self {
            document,
            anchor: cursor,
            cursor,
            focus: cx.focus_handle(),
            layouts: HashMap::new(),
            history: UndoHistory::new().max_undos(200),
            composition: None,
            composition_before: None,
            stored: None,
            dragging: None,
            readonly: false,
            active: false,
            blink,
            _subscriptions: subscriptions,
            scroll: ScrollHandle::new(),
        })
    }

    // Export the structured document as Markdown.
    pub fn source(&self) -> String {
        self.document.source()
    }

    // Replace the document without emitting a user edit event.
    pub fn set_source(&mut self, source: &str, cx: &mut Context<Self>) -> Result<(), SharedString> {
        let document = Document::parse_with_extensions(
            source,
            self.document.context.markdown_extensions.as_ref().clone(),
        )?;
        self.cursor = document.first();
        self.anchor = self.cursor;
        self.document = document;
        self.history.clear();
        self.layouts.clear();
        self.composition = None;
        self.composition_before = None;
        cx.notify();
        Ok(())
    }

    pub fn set_readonly(&mut self, readonly: bool, cx: &mut Context<Self>) {
        if self.readonly == readonly {
            return;
        }
        // Settle the current composition before disabling user edits.
        self.finish_composition(cx);
        self.readonly = readonly;
        self.blink.update(cx, |blink, cx| {
            if readonly {
                blink.stop(cx);
            } else {
                blink.pause(cx);
            }
        });
        if readonly {
            self.active = false;
        }
        cx.notify();
    }
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
        if !self.readonly {
            self.pause_cursor(cx);
        }
    }

    fn pause_cursor(&self, cx: &mut App) {
        if !self.readonly {
            self.blink.update(cx, |blink, cx| blink.pause(cx));
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            document: self.document.clone(),
            anchor: self.anchor,
            cursor: self.cursor,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.document = snapshot.document;
        self.anchor = snapshot.anchor;
        self.cursor = snapshot.cursor;
        self.composition = None;
        self.composition_before = None;
        self.layouts.clear();
        self.stored = None;
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        self.layouts.clear();
        self.pause_cursor(cx);
        cx.emit(MarkdownEditorEvent::Change);
        cx.notify();
    }

    fn edit(&mut self, cx: &mut Context<Self>, operation: impl FnOnce(&mut Self)) {
        if self.readonly {
            return;
        }
        self.finish_composition(cx);
        let before = self.snapshot();
        operation(self);
        if before.document.blocks != self.document.blocks {
            self.record(before);
            self.changed(cx);
        } else {
            cx.notify();
        }
    }

    fn place(&mut self, position: Position, extend: bool, cx: &mut Context<Self>) {
        self.cursor = position;
        if !extend {
            self.anchor = position;
        }
        self.stored = None;
        self.pause_cursor(cx);
        cx.notify();
    }

    fn insert(&mut self, text: &str, cx: &mut Context<Self>) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        self.edit(cx, |this| {
            let mut lines = text.split('\n');
            this.cursor = this.document.replace(
                this.anchor,
                this.cursor,
                lines.next().unwrap_or_default(),
                this.stored.as_ref(),
            );
            for line in lines {
                this.cursor = this.document.split(this.cursor);
                this.cursor =
                    this.document
                        .replace(this.cursor, this.cursor, line, this.stored.as_ref());
            }
            this.anchor = this.cursor;
            // Markdown prefixes become structural formatting after a committed space.
            if text == " " {
                let prefix = this.document.text(this.cursor.block);
                let task = match prefix.as_str() {
                    "-[] " | "-[ ] " | "- [] " | "- [ ] " => Some((false, false)),
                    "-[x] " | "-[X] " | "- [x] " | "- [X] " => Some((true, false)),
                    "[] " | "[ ] " => Some((false, true)),
                    "[x] " | "[X] " => Some((true, true)),
                    _ => None,
                };
                if let Some((checked, require_list)) = task {
                    if this
                        .document
                        .make_task(this.cursor.block, checked, require_list)
                    {
                        let start = Position {
                            offset: 0,
                            ..this.cursor
                        };
                        this.cursor = this.document.replace(start, this.cursor, "", None);
                        this.anchor = this.cursor;
                    }
                    return;
                }
                let heading = match prefix.as_str() {
                    "# " => Some(1),
                    "## " => Some(2),
                    "### " => Some(3),
                    _ => None,
                };
                let list = match prefix.as_str() {
                    "- " | "* " => Some(false),
                    "1. " => Some(true),
                    _ => None,
                };
                if heading.is_some() || list.is_some() {
                    let start = Position {
                        offset: 0,
                        ..this.cursor
                    };
                    this.cursor = this.document.replace(start, this.cursor, "", None);
                    this.anchor = this.cursor;
                    if let Some(ordered) = list {
                        this.document.make_list(this.cursor.block, ordered);
                    }
                    if let Some(level) = heading {
                        this.document.heading(this.cursor.block, level);
                    }
                }
            }
        });
    }

    fn delete(&mut self, forward: bool, cx: &mut Context<Self>) {
        self.edit(cx, |this| {
            if this.anchor == this.cursor {
                if !forward
                    && this.cursor.offset == 0
                    && this.document.text(this.cursor.block).is_empty()
                    && this.document.outdent_list(this.cursor.block)
                {
                    return;
                }
                this.anchor = this.document.adjacent(this.cursor, forward);
            }
            this.cursor = this.document.replace(this.anchor, this.cursor, "", None);
            this.anchor = this.cursor;
        });
    }

    fn enter(&mut self, cx: &mut Context<Self>) {
        self.edit(cx, |this| {
            this.cursor = this.document.replace(this.anchor, this.cursor, "", None);
            if this.document.text(this.cursor.block).is_empty()
                && this.document.outdent_list(this.cursor.block)
            {
                this.anchor = this.cursor;
                return;
            }
            this.cursor = this.document.split(this.cursor);
            this.anchor = this.cursor;
        });
    }

    pub fn indent_list(&mut self, cx: &mut Context<Self>) {
        self.edit(cx, |this| {
            this.document.indent_list(this.cursor.block);
        });
    }

    pub fn outdent_list(&mut self, cx: &mut Context<Self>) {
        self.edit(cx, |this| {
            this.document.outdent_list(this.cursor.block);
        });
    }

    pub fn toggle_bold(&mut self, cx: &mut Context<Self>) {
        self.toggle_format(true, cx);
    }
    pub fn toggle_italic(&mut self, cx: &mut Context<Self>) {
        self.toggle_format(false, cx);
    }
    fn toggle_format(&mut self, bold: bool, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        if self.anchor == self.cursor {
            let mark = self
                .stored
                .get_or_insert_with(|| self.document.mark_at(self.cursor));
            if bold {
                mark.bold = !mark.bold;
            } else {
                mark.italic = !mark.italic;
            }
            cx.notify();
        } else {
            self.edit(cx, |this| {
                this.document.format(this.anchor, this.cursor, bold)
            });
        }
    }
    pub fn set_heading(&mut self, level: u8, cx: &mut Context<Self>) {
        self.edit(cx, |this| this.document.heading(this.cursor.block, level));
    }
    pub fn toggle_list(&mut self, cx: &mut Context<Self>) {
        self.edit(cx, |this| this.document.list(this.cursor.block));
    }
    pub fn move_block_up(&mut self, cx: &mut Context<Self>) {
        self.edit(cx, |this| {
            this.document.move_block(this.cursor.block, false)
        });
    }
    pub fn move_block_down(&mut self, cx: &mut Context<Self>) {
        self.edit(cx, |this| this.document.move_block(this.cursor.block, true));
    }
    fn record(&mut self, before: Snapshot) {
        self.history.push(Edit {
            before,
            after: self.snapshot(),
        });
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        self.finish_composition(cx);
        if let Some(edits) = self.history.undo() {
            for edit in edits {
                self.restore(edit.before);
            }
            self.changed(cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if self.readonly {
            return;
        }
        self.finish_composition(cx);
        if let Some(edits) = self.history.redo() {
            for edit in edits {
                self.restore(edit.after);
            }
            self.changed(cx);
        }
    }

    fn copy(&self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.document.selected(self.anchor, self.cursor),
        ));
    }
    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.insert(&text, cx);
        }
    }

    fn horizontal(&mut self, forward: bool, extend: bool, cx: &mut Context<Self>) {
        let (a, b) = self.document.ordered(self.anchor, self.cursor);
        let next = if !extend && a != b {
            if forward { b } else { a }
        } else {
            self.document.adjacent(self.cursor, forward)
        };
        self.place(next, extend, cx);
    }

    fn layout_for(&self, at: Position) -> Option<&Layout> {
        self.layouts
            .iter()
            .filter(|((id, _), layout)| {
                *id == at.block
                    && layout.source.start <= at.offset
                    && at.offset <= layout.source.end
            })
            .max_by_key(|(_, layout)| layout.source.start)
            .map(|(_, layout)| layout)
    }

    fn hit(&self, point: Point<Pixels>) -> Option<Position> {
        let ((id, _), layout) = self.layouts.iter().min_by(|(_, a), (_, b)| {
            let distance = |r: &Bounds<Pixels>| {
                let dy = (r.top() - point.y).max(point.y - r.bottom()).max(px(0.));
                let dx = (r.left() - point.x).max(point.x - r.right()).max(px(0.));
                (dy, dx)
            };
            distance(&a.bounds)
                .partial_cmp(&distance(&b.bounds))
                .unwrap()
        })?;
        let local = layout
            .text
            .index_for_position(point)
            .unwrap_or_else(|ix| ix)
            .min(layout.source.len());
        Some(Position {
            block: *id,
            offset: layout.source.start + local,
        })
    }

    fn vertical(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) {
        if let Some(layout) = self.layout_for(self.cursor) {
            if let Some(pos) = layout
                .text
                .position_for_index(self.cursor.offset - layout.source.start)
            {
                let point = point(
                    pos.x,
                    pos.y + layout.text.line_height() * if down { 1.5 } else { -0.5 },
                );
                if let Some(next) = self.hit(point) {
                    self.place(next, extend, cx);
                }
            }
        }
    }
}

// The existing Inline element draws document selection and reports its layout.
pub(in crate::text) struct InlineEditing {
    owner: WeakEntity<MarkdownEditorState>,
    id: u64,
}

impl InlineInteraction for InlineEditing {
    fn paint(
        &self,
        layout: &TextLayout,
        bounds: Bounds<Pixels>,
        source: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let _ = self.owner.update(cx, |state, cx| {
            let active = !state.readonly && state.focus.is_focused(window);
            if active != state.active {
                state.active = active;
                state.blink.update(cx, |blink, cx| {
                    if active {
                        blink.pause(cx);
                    } else {
                        blink.stop(cx);
                    }
                });
            }
            let color = crate::Theme::global(cx).tokens.colors.selection;
            if let Some(range) = state
                .document
                .selection_in(self.id, state.anchor, state.cursor)
            {
                let start = range.start.max(source.start);
                let end = range.end.min(source.end);
                if start < end {
                    Inline::paint_selection(
                        &((start - source.start)..(end - source.start)).into(),
                        layout,
                        &bounds,
                        window,
                        color,
                    );
                }
            }
            let at = state.cursor.offset;
            let owns_caret = source.contains(&at)
                || (at == source.end && at == state.document.text(self.id).len());
            if active
                && state.cursor.block == self.id
                && owns_caret
                && state.blink.read(cx).visible()
            {
                if let Some(origin) = layout.position_for_index(at - source.start) {
                    window.paint_quad(fill(
                        Bounds::new(origin, size(px(1.), layout.line_height())),
                        crate::Theme::global(cx).tokens.colors.foreground,
                    ));
                }
            }
            if let Some((start, end)) = state
                .composition
                .filter(|(start, _)| start.block == self.id)
            {
                let start = start.offset.max(source.start);
                let end = end.offset.min(source.end);
                if start < end {
                    if let (Some(a), Some(b)) = (
                        layout.position_for_index(start - source.start),
                        layout.position_for_index(end - source.start),
                    ) {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(a.x, a.y + layout.line_height() - px(1.)),
                                size((b.x - a.x).max(px(1.)), px(1.)),
                            ),
                            crate::Theme::global(cx).tokens.colors.foreground,
                        ));
                    }
                }
            }
            state.layouts.insert(
                (self.id, source.start),
                Layout {
                    source,
                    text: layout.clone(),
                    bounds,
                },
            );
        });
    }
}

impl Render for MarkdownEditorState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.layouts.clear();
        let owner = cx.entity().downgrade();
        let editable: HashSet<_> = self
            .document
            .paragraphs()
            .iter()
            .map(|p| p.editor_id())
            .collect();
        let defaults = TextViewDefaults::global(cx);
        let node_context = NodeContext {
            style: defaults
                .style
                .unwrap_or_else(|| TextViewStyle::from_theme(&crate::Theme::global(cx)))
                .with_heading_base_font_size(window.rem_size()),
            code_block_highlighter: defaults.code_block_highlighter,
            markdown_extensions: self.document.context.markdown_extensions.clone(),
            link_refs: self.document.context.link_refs.clone(),
            paragraph_interaction: Some(Arc::new(move |p| {
                if !editable.contains(&p.editor_id()) {
                    return None;
                }
                Some(Arc::new(InlineEditing {
                    owner: owner.clone(),
                    id: p.editor_id(),
                }) as Arc<dyn InlineInteraction>)
            })),
            ..NodeContext::default()
        };
        let blocks = self
            .document
            .blocks
            .iter()
            .enumerate()
            .map(|(ix, node)| {
                node.render_block(
                    NodeRenderOptions {
                        ix,
                        ..Default::default()
                    },
                    &node_context,
                    window,
                    cx,
                )
            })
            .collect::<Vec<_>>();
        let input = cx.entity();
        div()
            .id("markdown-editor")
            .key_context("MarkdownEditor")
            .track_focus(&self.focus)
            .role(Role::MultilineTextInput)
            .aria_label("Markdown editor")
            .aria_value(
                self.document
                    .selected(self.document.first(), self.document.last()),
            )
            .size_full()
            .min_h_0()
            .relative()
            .on_action(cx.listener(|this, _: &Indent, _, cx| this.indent_list(cx)))
            .on_action(cx.listener(|this, _: &Outdent, _, cx| this.outdent_list(cx)))
            .on_action(cx.listener(|this, _: &Backspace, _, cx| this.delete(false, cx)))
            .on_action(cx.listener(|this, _: &Delete, _, cx| this.delete(true, cx)))
            .on_action(cx.listener(|this, _: &Enter, _, cx| this.enter(cx)))
            .on_action(cx.listener(|this, _: &Left, _, cx| this.horizontal(false, false, cx)))
            .on_action(cx.listener(|this, _: &Right, _, cx| this.horizontal(true, false, cx)))
            .on_action(cx.listener(|this, _: &SelectLeft, _, cx| this.horizontal(false, true, cx)))
            .on_action(cx.listener(|this, _: &SelectRight, _, cx| this.horizontal(true, true, cx)))
            .on_action(cx.listener(|this, _: &Up, _, cx| this.vertical(false, false, cx)))
            .on_action(cx.listener(|this, _: &Down, _, cx| this.vertical(true, false, cx)))
            .on_action(cx.listener(|this, _: &SelectUp, _, cx| this.vertical(false, true, cx)))
            .on_action(cx.listener(|this, _: &SelectDown, _, cx| this.vertical(true, true, cx)))
            .on_action(cx.listener(|this, _: &Home, _, cx| {
                this.place(
                    Position {
                        offset: 0,
                        ..this.cursor
                    },
                    false,
                    cx,
                )
            }))
            .on_action(cx.listener(|this, _: &End, _, cx| {
                this.place(
                    Position {
                        offset: this.document.text(this.cursor.block).len(),
                        ..this.cursor
                    },
                    false,
                    cx,
                )
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                this.anchor = this.document.first();
                this.cursor = this.document.last();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.undo(cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.redo(cx)))
            .on_action(cx.listener(|this, _: &Copy, _, cx| this.copy(cx)))
            .on_action(cx.listener(|this, _: &Cut, _, cx| {
                this.copy(cx);
                this.insert("", cx);
            }))
            .on_action(cx.listener(|this, _: &Paste, _, cx| this.paste(cx)))
            .on_action(cx.listener(|this, _: &Bold, _, cx| this.toggle_bold(cx)))
            .on_action(cx.listener(|this, _: &Italic, _, cx| this.toggle_italic(cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    crate::global_state::GlobalState::suppress_text_selection(cx);
                    this.finish_composition(cx);
                    this.focus.focus(window, cx);
                    if let Some(position) = this.hit(event.position) {
                        this.place(position, event.modifiers.shift, cx);
                        this.dragging = Some((this.anchor, this.anchor));
                        if event.click_count == 2 {
                            let text = this.document.text(position.block);
                            if let Some(range) =
                                crate::text::selection::word_range_at(&text, position.offset)
                            {
                                this.anchor = Position {
                                    offset: range.start,
                                    ..position
                                };
                                this.cursor = Position {
                                    offset: range.end,
                                    ..position
                                };
                                this.dragging = Some((this.anchor, this.cursor));
                            }
                        }
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if let Some((start, end)) = this.dragging
                    && event.pressed_button == Some(MouseButton::Left)
                {
                    if let Some(position) = this.hit(event.position) {
                        if this.document.ordered(position, start).0 == position {
                            this.anchor = end;
                            this.place(position, true, cx);
                        } else if this.document.ordered(end, position).0 == end {
                            this.anchor = start;
                            this.place(position, true, cx);
                        } else {
                            this.anchor = start;
                            this.place(end, true, cx);
                        }
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = None),
            )
            .child(
                div()
                    .id("document-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(blocks),
            )
            .child(
                canvas(
                    move |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let focus = input.read(cx).focus.clone();
                        window.handle_input(&focus, ElementInputHandler::new(bounds, input), cx);
                    },
                )
                .absolute()
                .size_full(),
            )
    }
}

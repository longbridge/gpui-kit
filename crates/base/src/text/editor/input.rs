use super::{MarkdownEditorState, model::Position};
use gpui::*;
use std::ops::Range;

// Native text APIs count UTF-16 units; the document counts UTF-8 bytes.
pub(super) fn byte_offset(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (ix, ch) in text.char_indices() {
        if units + ch.len_utf16() > utf16 {
            return ix;
        }
        units += ch.len_utf16();
    }
    text.len()
}

pub(super) fn utf16_offset(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].encode_utf16().count()
}

impl MarkdownEditorState {
    pub(super) fn finish_composition(&mut self, cx: &mut Context<Self>) {
        self.composition = None;
        if let Some(before) = self.composition_before.take() {
            if before.document.blocks != self.document.blocks {
                self.record(before);
                self.changed(cx);
            }
        }
    }

    fn native_range(&self, range: Range<usize>) -> (Position, Position) {
        let text = self.document.text(self.cursor.block);
        (
            Position {
                offset: byte_offset(&text, range.start),
                ..self.cursor
            },
            Position {
                offset: byte_offset(&text, range.end),
                ..self.cursor
            },
        )
    }
}

impl EntityInputHandler for MarkdownEditorState {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.document.text(self.cursor.block);
        let (a, b) = self.native_range(range);
        *adjusted = Some(utf16_offset(&text, a.offset)..utf16_offset(&text, b.offset));
        Some(text[a.offset..b.offset].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let text = self.document.text(self.cursor.block);
        let (a, b) = if self.anchor.block == self.cursor.block {
            self.document.ordered(self.anchor, self.cursor)
        } else {
            (self.cursor, self.cursor)
        };
        Some(UTF16Selection {
            range: utf16_offset(&text, a.offset)..utf16_offset(&text, b.offset),
            reversed: self.cursor == a && a != b,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let (a, b) = self.composition?;
        let text = self.document.text(a.block);
        Some(utf16_offset(&text, a.offset)..utf16_offset(&text, b.offset))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.finish_composition(cx);
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        if let Some((a, b)) = range.map(|r| self.native_range(r)).or(self.composition) {
            self.anchor = a;
            self.cursor = b;
        }
        if self.composition_before.is_some() {
            self.cursor =
                self.document
                    .replace(self.anchor, self.cursor, text, self.stored.as_ref());
            self.anchor = self.cursor;
            self.finish_composition(cx);
        } else {
            self.insert(text, cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        if self.composition_before.is_none() {
            self.composition_before = Some(self.snapshot());
        }
        let (a, b) = range
            .map(|r| self.native_range(r))
            .or(self.composition)
            .unwrap_or((self.anchor, self.cursor));
        let (start, _) = self.document.ordered(a, b);
        let end = self.document.replace(a, b, text, self.stored.as_ref());
        self.composition = Some((start, end));
        self.cursor = Position {
            offset: start.offset
                + selected
                    .as_ref()
                    .map(|r| byte_offset(text, r.end))
                    .unwrap_or(text.len()),
            ..start
        };
        self.anchor = Position {
            offset: start.offset
                + selected
                    .map(|r| byte_offset(text, r.start))
                    .unwrap_or(text.len()),
            ..start
        };
        self.layouts.clear();
        self.pause_cursor(cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (a, b) = self.native_range(range);
        let layout = self.layout_for(a)?;
        let start = layout
            .text
            .position_for_index(a.offset - layout.source.start)?;
        let end = layout
            .text
            .position_for_index(
                b.offset
                    .saturating_sub(layout.source.start)
                    .min(layout.source.len()),
            )
            .unwrap_or(start);
        Some(Bounds::new(
            start,
            size(
                if start.y == end.y {
                    (end.x - start.x).max(px(1.))
                } else {
                    px(1.)
                },
                layout.text.line_height(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let hit = self.hit(point)?;
        if hit.block != self.cursor.block {
            return None;
        }
        Some(utf16_offset(&self.document.text(hit.block), hit.offset))
    }
}

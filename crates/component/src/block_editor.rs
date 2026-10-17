//! Themed, content-sized Markdown editor. Base retains all editing behavior.
use crate::ActiveTheme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, rems,
};
pub use gpui_base::text::block_editor::{
    BlockEditorEvent, BlockEditorState, CapturedRange, CapturedRangeError, ClipboardPolicy,
    ImagePolicy, PasteRequest, SourceSnapshot, SubmitPolicy,
};

// Medium input-family proportions (10px/8px at the default 16px rem),
// expressed on the same relative axis as the editor's content rows.
const INPUT_PADDING_X: f32 = 0.625;
const INPUT_PADDING_Y: f32 = 0.5;
const ROW_HEIGHT: f32 = 1.5;

#[derive(IntoElement)]
pub struct BlockEditor {
    state: Entity<BlockEditorState>,
    min_rows: usize,
    max_rows: usize,
}
impl BlockEditor {
    pub fn new(state: &Entity<BlockEditorState>) -> Self {
        Self {
            state: state.clone(),
            min_rows: 2,
            max_rows: 16,
        }
    }
    /// Content-sized viewport clamped to content rows plus internal insets. The
    /// editor's retained handle owns scrolling and caret reveal; no outer
    /// scrolling node or application height mirror is needed.
    pub fn auto_grow(mut self, min: usize, max: usize) -> Self {
        self.min_rows = min.max(1);
        self.max_rows = max.max(self.min_rows);
        self
    }
}
impl RenderOnce for BlockEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll = self.state.read(cx).scroll_handle();
        let min_height = rems(self.min_rows as f32 * ROW_HEIGHT + INPUT_PADDING_Y * 2.);
        let max_height = rems(self.max_rows as f32 * ROW_HEIGHT + INPUT_PADDING_Y * 2.);
        div()
            .id(("block-editor-viewport", self.state.entity_id()))
            .w_full()
            .min_w_0()
            .min_h(min_height)
            .max_h(max_height)
            .overflow_y_scroll()
            .when_some(scroll, |this, scroll| this.track_scroll(&scroll))
            .text_color(cx.theme().foreground)
            // Padding belongs to the native root, inside the sole scroll owner.
            .child(
                gpui_base::text::block_editor::BlockEditor::new(&self.state)
                    .min_h(min_height)
                    .px(rems(INPUT_PADDING_X))
                    .py(rems(INPUT_PADDING_Y)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Bounds, Context, EntityInputHandler as _, Focusable as _, MouseButton,
        Pixels, Render, TestAppContext, VisualTestContext, point, px,
    };
    use gpui_base::text::block_editor::{BlockPosition, BlockSelection};

    struct Harness(Entity<BlockEditorState>, f32);

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(rems(self.1))
                .text_size(rems(1.))
                .child(BlockEditor::new(&self.0).auto_grow(2, 4))
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn caret(editor: &Entity<BlockEditorState>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
        cx.update(|window, cx| {
            editor.update(cx, |state, cx| {
                let offset = state.model().caret().unwrap().offset;
                state
                    .bounds_for_range(offset..offset, Bounds::default(), window, cx)
                    .expect("the selected leaf is laid out")
            })
        })
    }

    #[gpui::test]
    fn inset_pointer_edit_and_reveal_survive_zoom_and_rewrap(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_http_client(gpui::http_client::FakeHttpClient::create(|_| async {
                Err(anyhow::anyhow!(
                    "outbound requests forbidden in editor test"
                ))
            }));
            crate::init(cx);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            Harness(
                cx.new(|cx| BlockEditorState::new(window, cx).with_source("alpha beta")),
                20.,
            )
        });
        let editor = root.read_with(cx, |root, _| root.0.clone());
        draw(cx);
        let scroll = editor.read_with(cx, |state, _| state.scroll_handle().unwrap());
        let inset = cx.update(|window, _| {
            let view = scroll.bounds();
            point(
                view.left() + window.rem_size() / 4.,
                view.top() + window.rem_size() / 4.,
            )
        });
        cx.simulate_click(inset, Default::default());
        cx.simulate_input("X");
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |state, _| state.source()),
            "Xalpha beta"
        );
        assert!(cx.update(|window, cx| editor.read(cx).focus_handle(cx).is_focused(window)));

        // Selection is driven by the actual native text geometry, not a
        // separately translated source/cursor coordinate system.
        let start = caret(&editor, cx).center();
        let end = cx.update(|window, cx| {
            editor.update(cx, |state, cx| {
                state
                    .bounds_for_range(6..6, Bounds::default(), window, cx)
                    .unwrap()
                    .center()
            })
        });
        cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
        cx.simulate_mouse_move(end, Some(MouseButton::Left), Default::default());
        cx.simulate_mouse_up(end, MouseButton::Left, Default::default());
        cx.simulate_input("word");
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |state, _| state.source()),
            "Xword beta"
        );

        let trailing_inset = cx.update(|window, _| {
            let view = scroll.bounds();
            point(
                view.right() - window.rem_size() / 4.,
                view.bottom() - window.rem_size() / 4.,
            )
        });
        cx.simulate_click(trailing_inset, Default::default());
        cx.simulate_input("Y");
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |state, _| state.source()),
            "Xword betaY"
        );

        let source = (0..80)
            .map(|_| "one two three four five six seven eight nine ten")
            .collect::<Vec<_>>()
            .join("\n\n");
        editor.update(cx, |state, cx| state.set_source(&source, cx).unwrap());
        draw(cx);
        editor.update(cx, |state, cx| {
            let last = state.model().document().leaves().last().unwrap();
            state
                .set_selection(
                    BlockSelection::caret(BlockPosition::new(
                        last.id(),
                        last.text().unwrap().len(),
                    )),
                    cx,
                )
                .unwrap();
        });
        draw(cx);
        assert!(
            scroll.offset().y < Pixels::ZERO,
            "the retained owner scrolls to the last leaf"
        );
        assert!(scroll.bounds().contains(&caret(&editor, cx).center()));

        // Change both rem and available width without changing the source;
        // cached off-screen heights must not leave the caret outside its owner.
        cx.update(|window, _| window.set_rem_size(px(24.)));
        root.update(cx, |root, cx| {
            root.1 = 12.;
            cx.notify();
        });
        // Settle the layout invalidation and native reveal follow-up frame.
        draw(cx);
        draw(cx);
        assert!(scroll.bounds().contains(&caret(&editor, cx).center()));
        editor.update(cx, |state, cx| {
            let first = state.model().document().leaves().next().unwrap().id();
            state
                .set_selection(BlockSelection::caret(BlockPosition::new(first, 0)), cx)
                .unwrap();
        });
        draw(cx);
        let inset = cx.update(|window, _| {
            let view = scroll.bounds();
            point(
                view.left() + window.rem_size() / 4.,
                view.top() + window.rem_size() / 4.,
            )
        });
        cx.simulate_click(inset, Default::default());
        cx.simulate_input("Z");
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |state, _| state.source()),
            format!("Z{source}")
        );
    }
}

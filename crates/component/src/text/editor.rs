use crate::{ActiveTheme, StyledExt};
use gpui::*;
pub use gpui_base::text::editor::{MarkdownEditorEvent, MarkdownEditorState};

// The styled editing surface retains its state in the owning application.
#[derive(IntoElement)]
pub struct MarkdownEditor {
    state: Entity<MarkdownEditorState>,
    style: StyleRefinement,
}

impl MarkdownEditor {
    pub fn new(state: &Entity<MarkdownEditorState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for MarkdownEditor {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MarkdownEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .min_h_0()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .refine_style(&self.style)
            .child(self.state)
    }
}

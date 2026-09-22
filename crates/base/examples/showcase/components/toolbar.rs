use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let search = self.toolbar_search.clone();
        let command = |id: &'static str, label: &'static str| {
            let view = cx.weak_entity();
            Button::new(id)
                .h_7()
                .px_2()
                .border_1()
                .border_color(example_rgb(0xd4d4d4))
                .hover(|style| style.bg(example_rgb(0xf5f5f5)))
                .focus_visible(|style| style.border_color(example_rgb(0x171717)))
                .accessibility_label(label)
                .child(label)
                .on_click(move |_, _, cx| {
                    _ = view.update(cx, |this, cx| {
                        this.toolbar_action = format!("{label} command").into();
                        cx.notify();
                    });
                })
        };

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(
                Toolbar::new("example-toolbar")
                    .p_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .border_1()
                    .border_color(example_rgb(0xd4d4d4))
                    .child(
                        ToolbarGroup::new("document-commands")
                            .label("Document")
                            .flex()
                            .gap_1()
                            .child(command("toolbar-new", "New"))
                            .child(command("toolbar-save", "Save")),
                    )
                    .child(div().w_px().h_5().bg(example_rgb(0xd4d4d4)))
                    .child(
                        ToolbarGroup::new("history-commands")
                            .label("History")
                            .flex()
                            .gap_1()
                            .child(command("toolbar-undo", "Undo"))
                            .child(command("toolbar-redo", "Redo")),
                    )
                    .child(div().w_px().h_5().bg(example_rgb(0xd4d4d4)))
                    .child(
                        InputBase::new("toolbar-search")
                            .w_40()
                            .h_7()
                            .px_2()
                            .flex()
                            .items_center()
                            .border_1()
                            .border_color(example_rgb(0xd4d4d4))
                            .styles(|styles| {
                                styles.focused(|style| style.border_color(example_rgb(0x171717)))
                            })
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                search.update(cx, |state, cx| state.focus(window, cx));
                            })
                            .child(Input::new(&self.toolbar_search)),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(example_rgb(0x737373))
                    .child(self.toolbar_action.clone()),
            )
    }
}

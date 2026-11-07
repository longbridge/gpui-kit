use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn combobox(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.combobox_open;
        let query = self.combobox_query.read(cx).value().to_lowercase();
        let selected = self.combobox_selection.clone();
        let entity = cx.entity().downgrade();
        let query_state = self.combobox_query.clone();
        let open_query_state = self.combobox_query.clone();
        let trigger_entity = cx.entity().downgrade();
        let trigger_query_state = self.combobox_query.clone();

        let combobox = Combobox::new("example-combobox")
            .open(open)
            .on_open_change(move |open, window, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.combobox_open = open;
                    cx.notify();
                });
                if open {
                    open_query_state.update(cx, |state, cx| state.focus(window, cx));
                }
            })
            .w_56()
            .child(
                div()
                    .id("combobox-trigger")
                    .w_full()
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .text_xs()
                    .bg(super::example_rgb(0xffffff))
                    .on_click(move |_, window, cx| {
                        _ = trigger_entity.update(cx, |this, cx| {
                            this.combobox_open = !open;
                            cx.notify();
                        });
                        if !open {
                            trigger_query_state.update(cx, |state, cx| state.focus(window, cx));
                        }
                    })
                    .child(selected)
                    .child(
                        div()
                            .text_color(super::example_rgb(0x737373))
                            .child(super::chevron(false)),
                    ),
            );
        let popup = div()
            .w_56()
            .text_xs()
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .bg(super::example_rgb(0xffffff))
            .child(
                InputBase::new("combobox-search")
                    .w_full()
                    .h_7()
                    .px_2()
                    .border_b_1()
                    .border_color(super::example_rgb(0xe5e5e5))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        query_state.update(cx, |state, cx| state.focus(window, cx));
                    })
                    .child(self.combobox_query.clone()),
            )
            .child(
                div().p_1().children(
                    ["GPUI", "React", "SwiftUI", "Vue"]
                        .into_iter()
                        .filter(|label| query.is_empty() || label.to_lowercase().contains(&query))
                        .map(|label| {
                            let entity = cx.entity().downgrade();
                            div()
                                .id(format!("combobox-{label}"))
                                .px_2()
                                .h_7()
                                .flex()
                                .items_center()
                                .text_xs()
                                .hover(|s| s.bg(super::example_rgb(0xf5f5f5)))
                                .on_click(move |_, _, cx| {
                                    _ = entity.update(cx, |this, cx| {
                                        this.combobox_selection = label.into();
                                        this.combobox_open = false;
                                        cx.notify();
                                    });
                                })
                                .child(label)
                        }),
                ),
            );

        Popup::new("example-combobox-popup", combobox).when(open, |this| this.content(popup))
    }
}

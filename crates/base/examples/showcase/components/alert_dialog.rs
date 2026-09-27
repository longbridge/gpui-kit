use gpui::relative;

use super::*;

impl BaseShowcase {
    pub(in super::super) fn alert_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.alert_dialog_open;
        let entity = cx.entity().downgrade();
        let open_entity = entity.clone();
        let ok_entity = entity.clone();
        let cancel_entity = entity.clone();
        let action_entity = entity.clone();

        div()
            .child(
                Button::new("open-alert-dialog")
                    .h_7()
                    .line_height(relative(1.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(gpui::black())
                    .text_color(gpui::white())
                    .on_click(move |_, _, cx| {
                        _ = open_entity.update(cx, |this, cx| {
                            this.alert_dialog_open = true;
                            cx.notify();
                        });
                    })
                    .child("Delete project"),
            )
            .child(
                AlertDialog::new(cx)
                    .open(open)
                    .on_open_change(move |open, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.alert_dialog_open = open;
                            cx.notify();
                        });
                    })
                    .on_ok(move |_, _, cx| {
                        _ = ok_entity.update(cx, |this, cx| {
                            this.alert_dialog_open = false;
                            cx.notify();
                        });
                        true
                    })
                    .backdrop(
                        AlertDialogBackdrop::new()
                            .absolute()
                            .inset_0()
                            .bg(super::example_rgb(0x000000))
                            .opacity(0.18),
                    )
                    .popup(
                        AlertDialogPopup::new()
                            .w_72()
                            .p_3()
                            .bg(super::example_rgb(0xffffff))
                            .border_1()
                            .border_color(super::example_rgb(0x171717))
                            .child(AlertDialogTitle::new().child("Delete project?"))
                            .child(
                                AlertDialogDescription::new()
                                    .mt_2()
                                    .text_xs()
                                    .text_color(super::example_rgb(0x525252))
                                    .child(
                                        "This permanently deletes Acme Studio and all of its data.",
                                    ),
                            )
                            .child(
                                div()
                                    .mt_3()
                                    .flex()
                                    .justify_end()
                                    .gap_2()
                                    .child(
                                        AlertDialogCancel::new().child(
                                            Button::new("cancel-delete")
                                                .px_3()
                                                .h_7()
                                                .flex()
                                                .items_center()
                                                .text_xs()
                                                .border_1()
                                                .border_color(super::example_rgb(0xd4d4d4))
                                                .on_click(move |_, _, cx| {
                                                    _ = cancel_entity.update(cx, |this, cx| {
                                                        this.alert_dialog_open = false;
                                                        cx.notify();
                                                    });
                                                })
                                                .child("Cancel"),
                                        ),
                                    )
                                    .child(
                                        AlertDialogAction::new().child(
                                            Button::new("confirm-delete")
                                                .px_3()
                                                .h_7()
                                                .flex()
                                                .items_center()
                                                .text_xs()
                                                .border_1()
                                                .border_color(super::example_rgb(0x171717))
                                                .bg(super::example_rgb(0x171717))
                                                .text_color(super::example_rgb(0xffffff))
                                                .on_click(move |_, _, cx| {
                                                    _ = action_entity.update(cx, |this, cx| {
                                                        this.alert_dialog_open = false;
                                                        cx.notify();
                                                    });
                                                })
                                                .child("Delete"),
                                        ),
                                    ),
                            ),
                    ),
            )
    }
}

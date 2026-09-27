use super::*;

impl BaseShowcase {
    pub(in super::super) fn date_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.date_open;
        let entity = cx.entity().downgrade();
        let trigger_entity = entity.clone();
        let trigger = Button::new("date-trigger")
            .w_full()
            .h_7()
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .border_1()
            .border_color(super::example_rgb(0xa3a3a3))
            .bg(super::example_rgb(0xffffff))
            .on_click(move |_, _, cx| {
                _ = trigger_entity.update(cx, |this, cx| {
                    this.date_open = !open;
                    cx.notify();
                });
            })
            .child("Aug 12, 2026")
            .child(super::chevron(false));
        let popup = Popup::new("date-picker-popup", trigger).when(open, |this| {
            this.content(
                div()
                    .w(px(250.))
                    .bg(super::example_rgb(0xffffff))
                    .child(self.calendar()),
            )
        });

        DatePicker::new("example-date-picker", &self.date_focus)
            .open(open)
            .on_open_change(move |open, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.date_open = open;
                    cx.notify();
                });
            })
            .w(px(250.))
            .text_xs()
            .child(popup)
    }
}

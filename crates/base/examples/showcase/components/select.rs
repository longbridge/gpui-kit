use super::*;

impl BaseShowcase {
    pub(in super::super) fn select(
        &self,
        combobox: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.select_open;
        let selected = self.select_index.min(3);
        let labels = ["GPUI", "React", "SwiftUI", "Vue"];
        let entity = cx.entity().downgrade();
        let trigger_entity = entity.clone();
        let trigger = div()
            .id("select-trigger")
            .h_7()
            .px_2()
            .text_xs()
            .flex()
            .items_center()
            .justify_between()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .on_click(move |_, _, cx| {
                _ = trigger_entity.update(cx, |this, cx| {
                    this.select_open = !open;
                    cx.notify();
                });
            })
            .child(labels[selected])
            .child(super::chevron(open));
        let options = div()
            .mt_1()
            .p_1()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .bg(super::example_rgb(0xffffff))
            .children(labels.into_iter().enumerate().map(|(ix, label)| {
                let entity = entity.clone();
                div()
                    .id(("select-option", ix))
                    .px_2()
                    .py_1()
                    .flex()
                    .justify_between()
                    .hover(|this| this.bg(super::example_rgb(0xf5f5f5)))
                    .child(label)
                    .when(ix == selected, |this| this.child("✓"))
                    .on_click(move |_, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.select_index = ix;
                            this.select_open = false;
                            cx.notify();
                        });
                    })
            }));
        if combobox {
            let root = Combobox::new("example-combobox")
                .open(open)
                .w_56()
                .child(trigger);
            Popup::new("example-combobox-options", root)
                .when(open, |this| this.content(options))
                .into_any_element()
        } else {
            let root = Select::new("example-select")
                .open(open)
                .on_open_change({
                    let entity = entity.clone();
                    move |next, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.select_open = next;
                            cx.notify();
                        });
                    }
                })
                .accessibility_label("Framework")
                .w_56()
                .child(trigger);
            Popup::new("example-select-options", root)
                .when(open, |this| this.content(options))
                .into_any_element()
        }
    }
}

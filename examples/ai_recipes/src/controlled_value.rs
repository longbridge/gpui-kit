use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::{Context, IntoElement, Render, Window};

pub struct ControlledCheckbox {
    checked: bool,
}

impl ControlledCheckbox {
    pub fn new() -> Self {
        Self { checked: false }
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }
}

impl Render for ControlledCheckbox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Checkbox::new("marketing-emails")
            .label("Receive product updates")
            .checked(self.checked)
            .on_change(cx.listener(|this, checked, _, cx| {
                this.checked = *checked;
                cx.notify();
            }))
    }
}

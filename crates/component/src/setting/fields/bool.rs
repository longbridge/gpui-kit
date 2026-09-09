use std::rc::Rc;

use crate::{
    Disableable, Sizable, StyledExt,
    checkbox::Checkbox,
    setting::{
        AnySettingField, RenderOptions,
        fields::{SettingFieldRender, get_value, set_value},
    },
    switch::Switch,
};
use gpui::{AnyElement, App, IntoElement, ParentElement as _, StyleRefinement, Window, div};

pub(crate) struct BoolField {
    use_switch: bool,
}

impl BoolField {
    pub(crate) fn new(use_switch: bool) -> Self {
        Self { use_switch }
    }
}

impl SettingFieldRender for BoolField {
    fn render(
        &self,
        field: Rc<dyn AnySettingField>,
        options: &RenderOptions,
        style: &StyleRefinement,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let checked = get_value::<bool>(&field, cx);
        let set_value = set_value::<bool>(&field, cx);

        div()
            .refine_style(style)
            .child(if self.use_switch {
                Switch::new("check")
                    .checked(checked)
                    .disabled(options.is_disabled())
                    .with_size(options.size())
                    .on_click(move |checked: &bool, _, cx: &mut App| {
                        set_value(*checked, cx);
                    })
                    .into_any_element()
            } else {
                Checkbox::new("check")
                    .checked(checked)
                    .disabled(options.is_disabled())
                    .with_size(options.size())
                    .on_click(move |checked: &bool, _, cx: &mut App| {
                        set_value(*checked, cx);
                    })
                    .into_any_element()
            })
            .into_any_element()
    }
}

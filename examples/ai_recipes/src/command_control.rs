use gpui_kit::IntoElement;
use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
};

pub fn primary_command() -> impl IntoElement {
    Button::new("save").primary().small().label("Save changes")
}

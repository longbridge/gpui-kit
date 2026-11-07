use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

pub struct Example;
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    gpui_kit::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        // Opens a window with a `Root` wrapping the view, so dialogs, sheets,
        // notifications and menus work in it.
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Example))
            .expect("Failed to open window");
    });
}

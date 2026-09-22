use gpui_kit::{
    AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div,
};

pub fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            // The window's root view is a `Root` wrapping the view, which
            // renders dialogs, sheets and notifications above it.
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| BootstrapView)
            })
            .expect("failed to open window");
        });
}

struct BootstrapView;

impl Render for BootstrapView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("My application")
    }
}

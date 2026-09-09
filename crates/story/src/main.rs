use gpui_component_story::{Gallery, create_new_window, init};
use gpui_kit::assets::Assets;

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    // Parse `cargo run -- <story_name>`
    let name = std::env::args().nth(1);

    app.run(move |cx| {
        init(cx);
        // Required for system notifications on Windows; macOS only shows them
        // when running from a bundled .app.
        cx.set_app_identity("com.longbridge.gpui-component.story", "GPUI Component");
        cx.activate(true);

        create_new_window(
            "GPUI Component",
            move |window, cx| Gallery::view(name.as_deref(), window, cx),
            cx,
        );
    });
}

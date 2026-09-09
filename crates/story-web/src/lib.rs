use std::borrow::Cow;
use std::cell::RefCell;

use gpui_component_story::{Gallery, StoryRoot};
use gpui_kit::assets::Assets;
use gpui_kit::component::{
    Root,
    theme::{Theme, ThemeMode},
};
use gpui_kit::{prelude::*, *};
use wasm_bindgen::prelude::*;

thread_local! {
    static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
}

/// Applies a theme mode and restores the bundled web fonts.
///
/// `Theme::change` reapplies the theme config, which can carry its own font
/// family; the host system fonts are unavailable in wasm, so the bundled ones
/// are put back afterwards.
fn apply_theme(mode: ThemeMode, cx: &mut App) {
    Theme::change(mode, None, cx);
    let theme = cx.global_mut::<Theme>();
    theme.font_family = "Inter Variable".into();
    theme.mono_font_family = "JetBrains Mono".into();
}

/// Switches the gallery between light and dark after it is running.
///
/// The embedding documentation page calls this when its own appearance
/// changes, so the gallery never sits in a dark page wearing a light theme.
#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub fn set_theme(dark: bool) {
    let mode = if dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    APPLICATION.with(|application| {
        if let Some(handle) = application.borrow().as_ref() {
            handle.update(|cx| {
                apply_theme(mode, cx);
                cx.refresh_windows();
            });
        }
    });
}

#[wasm_bindgen]
pub fn run(story: Option<String>, dark: Option<bool>) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    // Initialize logging to browser console
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");

    // Also initialize tracing for WASM
    tracing_wasm::set_as_global_default();

    #[cfg(target_family = "wasm")]
    gpui_kit::platform::web_init();
    #[cfg(not(target_family = "wasm"))]
    let app = gpui_kit::application();
    #[cfg(target_family = "wasm")]
    let app = gpui_kit::platform::single_threaded_web();

    let app = app.with_assets(Assets::new("https://gpui-kit.com/gallery/"));
    let launch = move |cx: &mut App| {
        gpui_component_story::init(cx);

        // Load a compact, offline font stack for WASM, where host system fonts
        // are unavailable. Inter gives the UI a neutral system-font feel, while
        // the other fonts contain only glyphs used by the story application.
        let ui_font = Cow::Borrowed(include_bytes!("../fonts/Inter-Regular.ttf").as_slice());
        let cjk_font =
            Cow::Borrowed(include_bytes!("../fonts/NotoSansSC-Regular-subset.ttf").as_slice());
        let emoji_font = Cow::Borrowed(include_bytes!("../fonts/NotoEmoji-Regular.ttf").as_slice());
        let jetbrains_mono =
            Cow::Borrowed(include_bytes!("../fonts/JetBrainsMono-Regular.ttf").as_slice());
        // The web platform resolves GPUI's `.SystemUIFont` alias to IBM Plex
        // Sans and ships no fonts of its own. Text measured before the first
        // frame, such as the search input's initial value, still carries the
        // window's default text style, so that family has to exist or the
        // text system panics.
        let system_font =
            Cow::Borrowed(include_bytes!("../fonts/IBMPlexSans-Regular.ttf").as_slice());
        cx.text_system()
            .add_fonts(vec![
                ui_font,
                cjk_font,
                emoji_font,
                jetbrains_mono,
                system_font,
            ])
            .expect("Failed to load fonts");

        // Apply the embedding page's appearance before the first frame, so an
        // embedded gallery never flashes a light theme inside a dark page.
        apply_theme(
            match dark {
                Some(true) => ThemeMode::Dark,
                _ => ThemeMode::Light,
            },
            cx,
        );

        cx.open_window(WindowOptions::default(), move |window, cx| {
            let embedded = story.is_some();
            let view = match story.as_deref() {
                Some(story) => Gallery::embedded_view(story, window, cx),
                None => Gallery::view(None, window, cx),
            };
            let story_root = cx.new(|cx| {
                if embedded {
                    StoryRoot::embedded(view, window, cx)
                } else {
                    StoryRoot::new("GPUI Component", view, window, cx)
                }
            });
            cx.new(|cx| Root::new(story_root, window, cx))
        })
        .expect("Failed to open window");
        cx.activate(true);
    };

    #[cfg(target_family = "wasm")]
    APPLICATION.with(|application| {
        *application.borrow_mut() = Some(app.run_embedded(launch));
    });
    #[cfg(not(target_family = "wasm"))]
    app.run(launch);

    Ok(())
}

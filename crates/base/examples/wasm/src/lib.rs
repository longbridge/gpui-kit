#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;
#[cfg(feature = "gpui-fast")]
extern crate gpui_platform_fast as gpui_platform;

#[cfg(target_family = "wasm")]
use gpui::{Application, ApplicationHandle};
#[cfg(target_family = "wasm")]
use std::cell::RefCell;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[path = "../../showcase/mod.rs"]
#[allow(dead_code)]
mod showcase;

#[path = "../../motion/mod.rs"]
#[allow(dead_code)]
mod motion;

#[cfg(target_family = "wasm")]
thread_local! {
    static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub fn run(component: Option<String>) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    tracing_wasm::set_as_global_default();
    gpui_platform::web_init();
    let handle = if component.as_deref() == Some("motion") {
        motion::run_embedded(web_application())
    } else {
        showcase::run_embedded(
            web_application(),
            component.unwrap_or_else(|| "overview".to_owned()),
        )
    };
    APPLICATION.with(|application| *application.borrow_mut() = Some(handle));
    Ok(())
}

#[cfg(target_family = "wasm")]
fn web_application() -> Application {
    gpui_platform::single_threaded_web()
}

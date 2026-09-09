#[path = "../../../showcase/mod.rs"]
mod showcase;

use std::sync::Arc;

fn main() {
    let component = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "overview".to_string());

    let http_client = reqwest_client::ReqwestClient::user_agent("gpui-base/examples").unwrap();
    let app = gpui_platform::application().with_http_client(Arc::new(http_client));
    showcase::run(app, component);
}

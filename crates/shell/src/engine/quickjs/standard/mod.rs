//! LLRT-backed, authority-free Standard Runtime modules.
//!
//! Privileged modules live behind Shell adapters; this module only registers
//! implementations that cannot bypass the active [`crate::Policy`].
//!
//! So the upstream crates behind `fs`, `net`, `os`, `console` and `fetch` are
//! not dependencies at all. Registering one directly would grant ambient
//! filesystem, process or network authority — the thing [`crate::Capabilities`]
//! exists to prevent — so Shell writes those five itself and depends only on
//! the ones it actually serves: `buffer`, `crypto`, `path`, `url` and `zlib`.

use rquickjs::{Ctx, Result, loader::BuiltinResolver, loader::ModuleLoader};

mod connect;
mod console;
mod fetch;
mod fs;
mod net;
mod os;
mod process;
mod websocket;

#[cfg(test)]
pub(super) fn direct_test_http_client()
-> std::result::Result<reqwest::blocking::Client, reqwest::Error> {
    fetch::direct_test_client()
}

pub(super) const NAMES: &[&str] = &[
    "buffer",
    "console",
    "crypto",
    "fs/promises",
    "net",
    "os",
    "path",
    "process",
    "url",
    "websocket",
    "zlib",
];

pub(super) fn resolver() -> BuiltinResolver {
    NAMES
        .iter()
        .fold(BuiltinResolver::default(), |resolver, name| {
            resolver.with_module(*name)
        })
}

pub(super) fn loader() -> ModuleLoader {
    ModuleLoader::default()
        .with_module("buffer", quickjs_jit_stdlib::buffer::BufferModule)
        .with_module("console", console::ConsoleModule)
        .with_module("crypto", quickjs_jit_stdlib::crypto::CryptoModule)
        .with_module("fs/promises", fs::FsModule)
        .with_module("net", net::NetModule)
        .with_module("os", os::OsModule)
        .with_module("path", quickjs_jit_stdlib::path::PathModule)
        .with_module("process", process::ProcessModule)
        .with_module("url", quickjs_jit_stdlib::url::UrlModule)
        .with_module("websocket", websocket::WebSocketModule)
        .with_module("zlib", quickjs_jit_stdlib::zlib::ZlibModule)
}

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    // Order is significant: URL and Crypto consume Buffer-compatible byte
    // classes installed by the first initializer.
    quickjs_jit_stdlib::init(ctx)?;
    console::install(ctx)?;
    super::sandbox::install_process(ctx)?;
    fetch::install(ctx)?;
    Ok(())
}

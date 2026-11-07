//! Embed `Info.plist` into the macOS executable.
//!
//! macOS asks for microphone and speech recognition access only when the
//! application describes why it needs them. An unbundled `cargo run` binary has
//! no bundle to carry that description, but the system also reads a property
//! list linked into the executable's `__TEXT,__info_plist` section.
//!
//! The list holds only the two usage descriptions. A bundle identifier would
//! make frameworks treat the binary as an application bundle, and GPUI's system
//! notifications then fail to start without one.

fn main() {
    println!("cargo:rerun-if-changed=Info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let plist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Info.plist");
        println!(
            "cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            plist.display()
        );
    }
}

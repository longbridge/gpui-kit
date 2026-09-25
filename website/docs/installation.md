---
title: Installation
description: Install GPUI Kit and the platform dependencies required to build Rust desktop applications on macOS, Windows, and Linux.
order: -1
---

# Installation

Install the native toolchain for your operating system, then add the `gpui-kit` crate to a Rust application. The steps below are for desktop development; [WebAssembly](./webassembly) and [Mobile](./mobile) have separate target setup.

## Platform requirements

<div class="doc-tabs" role="group" aria-label="Operating system">
  <input class="doc-tabs__input" type="radio" name="install-platform-en" id="install-macos-en" checked>
  <input class="doc-tabs__input" type="radio" name="install-platform-en" id="install-windows-en">
  <input class="doc-tabs__input" type="radio" name="install-platform-en" id="install-linux-en">
  <div class="doc-tabs__list">
    <label for="install-macos-en">macOS</label>
    <label for="install-windows-en">Windows</label>
    <label for="install-linux-en">Linux</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <ul><li>macOS 15 or later</li><li>Xcode Command Line Tools, installed with <code>xcode-select --install</code>. Run <code>xcode-select -p</code> afterward; it should print the selected developer directory.</li></ul>
    </section>
    <section class="doc-tabs__panel">
      <ul><li>Windows 10 or later</li><li>Visual Studio 2022 Build Tools or Community with the <strong>Desktop development with C++</strong> workload, including MSVC and a Windows SDK. Use the MSVC Rust toolchain, not the GNU toolchain.</li><li>CMake available on <code>PATH</code>; check with <code>cmake --version</code> in the terminal where you will run Cargo.</li></ul>
    </section>
    <section class="doc-tabs__panel">
      <p>The following packages are verified on Ubuntu 24.04:</p>
      <pre><code class="language-bash">sudo apt update
sudo apt install -y gcc g++ clang libfontconfig-dev libwayland-dev \
  libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libssl-dev libzstd-dev vulkan-validationlayers libvulkan1</code></pre>
      <p>This matches the repository's <code>script/install-linux.sh</code> for Ubuntu 24.04. Other distributions need equivalent development packages. To display a window, run in a graphical Wayland or X11 session with a working Vulkan driver; installing <code>libvulkan1</code> alone does not install a GPU driver.</p>
    </section>
  </div>
</div>

## Rust and Cargo

Install Rust and Cargo with the [official Rust installer](https://rust-lang.org/tools/install/). Use Rust 1.92 or later for this repository's current locked dependency graph: its Linux GPUI platform dependency includes `oo7 0.6.0`, which declares Rust 1.92 as its minimum. Then check the tools in the same terminal that will build the app:

```sh
rustc --version
cargo --version
```

Both commands should print a version. On Windows, <code>rustup show active-toolchain</code> should identify an <code>msvc</code> target. If a terminal cannot find Cargo immediately after installation, open a new terminal and run the checks again.

Add GPUI Kit to the application's `Cargo.toml` under `[dependencies]`:

```toml
gpui-kit = "0.6"
```

The `0.6` requirement selects a compatible 0.6.x Kit release; this repository currently declares version `0.6.5`. Kit's default features include the styled components and default icon assets. It brings in matching GPUI crates, so an application using this setup does not need to list GPUI separately. `use gpui_kit::*;` imports GPUI's re-exported API; the layers are reachable as `gpui_kit::component`, `gpui_kit::base`, `gpui_kit::assets`, and `gpui_kit::platform`.

### Why the dependency is named `gpui-pre`

Throughout these docs, **GPUI** means [Zed's GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui). `gpui-pre` is the crates.io package name used to publish a snapshot of GPUI from a recorded Zed commit, alongside its related GPUI crates. It provides a reproducible publication and version alignment path for GPUI Kit; it is not another rendering implementation. The publication process adjusts package names and dependency manifests for crates.io, so API references in this manual target the GPUI version pinned by this Kit release. In this repository that is `gpui-pre = {{gpui_pre_version}}`; application code normally depends only on `gpui-kit` and imports GPUI through `gpui_kit::*`. A newer `gpui-pre` snapshot does not by itself mean that the current GPUI Kit release supports it.

## Verify the installation

If you checked out this repository, run its existing example from the repository root:

```sh
cargo run -p hello_world
```

The first build downloads and compiles dependencies and may take several minutes. Once it starts, a window shows “Hello, World!” and a “Let's Go!” button. Clicking the button prints `Clicked!` in the terminal. Close the window to end the process. This checks the build toolchain, window creation, rendering, and input on your machine; it is not a performance benchmark.

For a new project instead, follow [Getting Started](./getting-started). It creates an application with the same single dependency and explains each part of the first [View](./render) and [Window](./window).

## Troubleshooting

| Symptom | Check |
| --- | --- |
| `cargo` or `rustc` is not found. | Install Rust with rustup, open a new terminal, and rerun the version checks above. |
| A build reports an unsupported Rust version. | Check `rustc --version`; update the active Rust toolchain. A dependency may require a newer compiler than this guide's baseline. |
| Windows reports `link.exe` missing or cannot find a Windows SDK. | Confirm the Visual Studio C++ workload and SDK are installed, then build with an MSVC Rust toolchain from a Visual Studio Developer PowerShell if needed. |
| Linux reports a missing `pkg-config` executable, X11, Wayland, fontconfig, or WebKit header. | Install the Ubuntu packages above, or their equivalents for your distribution. If `pkg-config` itself is missing, install the `pkg-config` package too. The error identifies the missing tool or system library. |
| The program compiles but no window appears on Linux. | Check that the process is running in a graphical Wayland or X11 session and that a Vulkan driver works for that session. A headless shell or Vulkan loader without a driver is insufficient. |
| Cargo cannot resolve `gpui-pre` or APIs differ from these examples. | Keep the `gpui-kit` requirement and update dependencies together. Kit pins a matching `gpui-pre-*` snapshot; do not override one GPUI package to a different version. In a repository checkout, use the checked-in `Cargo.lock`. |

For errors after a window opens, continue with [Getting Started](./getting-started) and inspect the relevant guide for the feature you are using.

## Improve development runtime performance

Rust Debug builds leave GPUI, the component library, layout, and text rendering
largely unoptimized. As a result, an application started with `cargo run` can
render and respond much more slowly than its release build. The profile below
optimizes those framework dependencies while your application code remains in
Debug mode and keeps its normal debugging workflow.

This setting does **not** make compilation faster. Compiling the optimized
dependencies can take longer, especially on the first build; the benefit is
better runtime performance while developing and running the application.
Cargo's [package profile overrides](https://doc.rust-lang.org/cargo/reference/profiles.html#overrides) only take effect in the root `Cargo.toml` of your application
or workspace:

```toml
[profile.dev.package]
gpui-pre = { opt-level = 3 }
gpui-component = { opt-level = 3 }
gpui-kit = { opt-level = 3 }
gpui-kit-assets = { opt-level = 3 }
gpui-pre-macros = { opt-level = 3 }
gpui-pre-platform = { opt-level = 3 }
rustybuzz = { opt-level = 3 }
taffy = { opt-level = 3 }
ttf-parser = { opt-level = 3 }
```

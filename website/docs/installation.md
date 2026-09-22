---
title: Installation
description: Install GPUI Kit and the platform dependencies required to build Rust desktop applications on macOS, Windows, and Linux.
order: -1
---

# Installation

Before you start to build your application with `gpui-component`, you need to install the library.

## Platform requirements

<div class="doc-tabs">
  <input class="doc-tabs__input" type="radio" name="install-platform-en" id="install-macos-en" checked>
  <input class="doc-tabs__input" type="radio" name="install-platform-en" id="install-windows-en">
  <input class="doc-tabs__input" type="radio" name="install-platform-en" id="install-linux-en">
  <div class="doc-tabs__list" role="tablist" aria-label="Operating system">
    <label for="install-macos-en" role="tab">macOS</label>
    <label for="install-windows-en" role="tab">Windows</label>
    <label for="install-linux-en" role="tab">Linux</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <ul><li>macOS 15 or later</li><li>Xcode Command Line Tools, installed with <code>xcode-select --install</code></li></ul>
    </section>
    <section class="doc-tabs__panel">
      <ul><li>Windows 10 or later</li><li>Visual Studio 2022 Build Tools or Community with the <strong>Desktop development with C++</strong> workload, including MSVC and a Windows SDK</li><li>CMake available on <code>PATH</code></li></ul>
    </section>
    <section class="doc-tabs__panel">
      <p>The following packages are verified on Ubuntu 24.04:</p>
      <pre><code class="language-bash">sudo apt update
sudo apt install -y gcc g++ clang libfontconfig-dev libwayland-dev \
  libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libssl-dev libzstd-dev vulkan-validationlayers libvulkan1</code></pre>
    </section>
  </div>
</div>

## Rust and Cargo

We use Rust programming language to build the `gpui-component` library. Make sure you have Rust and Cargo installed on your system.

- Rust 1.90 or later
- Cargo (comes with Rust)

To install the `gpui-component` library, you can use Cargo, the Rust package manager. Add the following line to your `Cargo.toml` file under the `[dependencies]` section:

```toml
gpui-kit = "0.6"
```

`gpui-kit` depends on the matching GPUI crates for you, so your application never lists GPUI itself. `use gpui_kit::*;` is GPUI, and the layers are reachable by name: `gpui_kit::component` (the styled components), `gpui_kit::base`, `gpui_kit::assets` and `gpui_kit::platform`.

For experimental iOS support and Swift UIView embedding, see [Mobile](/docs/mobile). Mobile uses `gpui-pre-mobile` and a different application bootstrap from the desktop setup above.

## Improve development runtime performance

Rust Debug builds leave GPUI, the component library, layout, and text rendering
largely unoptimized. As a result, an application started with `cargo run` can
render and respond much more slowly than its release build. The profile below
optimizes those framework dependencies while your application code remains in
Debug mode and keeps its normal debugging workflow.

This setting does **not** make compilation faster. Compiling the optimized
dependencies can take longer, especially on the first build; the benefit is
better runtime performance while developing and running the application.
Package profiles only take effect in the root `Cargo.toml` of your application
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

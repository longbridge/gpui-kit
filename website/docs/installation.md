---
title: Installation
description: Install GPUI Kit and prepare the system dependencies required to build Rust desktop applications on macOS, Windows, and Linux.
order: -1
---

# Installation

Before you start to build your application with `gpui-component`, you need to install the library.

## System Requirements

We can development application on macOS, Windows or Linux.

### macOS

- macOS 15 or later
- Xcode command line tools

## Windows

- Windows 10 or later

There have a bootstrap script to help install the required toolchain and dependencies.

You can run the script in PowerShell:

```ps
.\script\install-window.ps1
```

## Linux

Run `./script/bootstrap` to install system dependencies.

## Rust and Cargo

We use Rust programming language to build the `gpui-component` library. Make sure you have Rust and Cargo installed on your system.

- Rust 1.90 or later
- Cargo (comes with Rust)

To install the `gpui-component` library, you can use Cargo, the Rust package manager. Add the following line to your `Cargo.toml` file under the `[dependencies]` section:

```toml
gpui-kit = "0.6"
```

`gpui-kit` depends on the matching GPUI crates for you, so your application never lists GPUI itself. `use gpui_kit::*;` is GPUI, and the layers are reachable by name: `gpui_kit::component` (the styled components), `gpui_kit::base`, `gpui_kit::assets` and `gpui_kit::platform`.

## Faster development builds

Debug builds compile GPUI, the component library and the text stack without
optimizations, which makes a `cargo run` build of your application render
noticeably slower than a release build. Optimize just those crates while your
own code stays a fast, debuggable debug build. Profiles only take effect in
the root `Cargo.toml` of your application (or workspace):

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

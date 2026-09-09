---
title: 安装
description: 安装 GPUI Kit，并准备在 macOS、Windows 和 Linux 上构建 Rust 桌面应用所需的系统依赖。
order: -1
---

# 安装

在开始使用 `gpui-component` 构建应用之前，需要先准备对应的开发环境并安装依赖。

## 系统要求

目前可以在 macOS、Windows 和 Linux 上进行开发。

### macOS

- macOS 15 或更高版本
- Xcode Command Line Tools

## Windows

- Windows 10 或更高版本

仓库提供了一个脚本用于安装所需工具链和依赖。可以在 PowerShell 中运行：

```ps
.\script\install-window.ps1
```

## Linux

在 Linux 上，可以运行下面的脚本安装系统依赖：

```bash
./script/bootstrap
```

## Rust 和 Cargo

`gpui-component` 使用 Rust 构建，因此请确保系统已经安装 Rust 和 Cargo。

- Rust 1.90 或更高版本
- Cargo（通常随 Rust 一起安装）

安装库时，只需要在 `Cargo.toml` 的 `[dependencies]` 中加入：

```toml
gpui-kit = "0.6"
```

`gpui-kit` 会替你引入配套的 GPUI crate，应用无需再单独声明 GPUI。`use gpui_kit::*;` 就是 GPUI 本身，各层按名访问：`gpui_kit::component`（带样式的组件）、`gpui_kit::base`、`gpui_kit::assets`、`gpui_kit::platform`。

## 加快开发构建

Debug 构建下 GPUI、组件库和文字排版相关的 crate 都不做优化，`cargo run` 出来的应用渲染会明显比 release 慢。可以只对这些依赖开启优化，你自己的代码仍然保持快速、可调试的 debug 构建。Profile 只在应用（或 workspace）根目录的 `Cargo.toml` 中生效：

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

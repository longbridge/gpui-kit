---
title: 安装
description: 安装 GPUI Kit，并准备在 macOS、Windows 和 Linux 上构建 Rust 桌面应用所需的平台依赖。
order: -1
---

# 安装

在开始使用 `gpui-component` 构建应用之前，需要先准备对应的开发环境并安装依赖。

实验性的 iOS 支持与 Swift UIView 嵌入方式请参阅[移动端](/zh-CN/docs/mobile)。移动端使用 `gpui-pre-mobile`，应用启动方式与桌面端不同。

## 平台要求

<div class="doc-tabs">
  <input class="doc-tabs__input" type="radio" name="install-platform-zh" id="install-macos-zh" checked>
  <input class="doc-tabs__input" type="radio" name="install-platform-zh" id="install-windows-zh">
  <input class="doc-tabs__input" type="radio" name="install-platform-zh" id="install-linux-zh">
  <div class="doc-tabs__list" role="tablist" aria-label="操作系统">
    <label for="install-macos-zh" role="tab">macOS</label>
    <label for="install-windows-zh" role="tab">Windows</label>
    <label for="install-linux-zh" role="tab">Linux</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <ul><li>macOS 15 或更高版本</li><li>Xcode Command Line Tools，可运行 <code>xcode-select --install</code> 安装</li></ul>
    </section>
    <section class="doc-tabs__panel">
      <ul><li>Windows 10 或更高版本</li><li>Visual Studio 2022 Build Tools 或 Community，并安装 <strong>Desktop development with C++</strong> workload，其中包含 MSVC 与 Windows SDK</li><li>确保 <code>PATH</code> 中可以使用 CMake</li></ul>
    </section>
    <section class="doc-tabs__panel">
      <p>以下依赖已在 Ubuntu 24.04 验证：</p>
      <pre><code class="language-bash">sudo apt update
sudo apt install -y gcc g++ clang libfontconfig-dev libwayland-dev \
  libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libssl-dev libzstd-dev vulkan-validationlayers libvulkan1</code></pre>
    </section>
  </div>
</div>

## Rust 和 Cargo

`gpui-component` 使用 Rust 构建，因此请确保系统已经安装 Rust 和 Cargo。

- Rust 1.90 或更高版本
- Cargo（通常随 Rust 一起安装）

安装库时，只需要在 `Cargo.toml` 的 `[dependencies]` 中加入：

```toml
gpui-kit = "0.6"
```

`gpui-kit` 会替你引入配套的 GPUI crate，应用无需再单独声明 GPUI。`use gpui_kit::*;` 就是 GPUI 本身，各层按名访问：`gpui_kit::component`（带样式的组件）、`gpui_kit::base`、`gpui_kit::assets`、`gpui_kit::platform`。

## 提升开发模式运行性能

Rust Debug 构建下，GPUI、组件库、布局和文字渲染相关 crate 基本没有优化，因此通过 `cargo run` 启动的应用，其渲染与交互性能会明显低于 release build。下面的配置只优化这些框架依赖，应用自身代码仍保持 Debug mode，可以继续使用正常的调试流程。

这个配置**不会加快编译**。启用优化后，这些依赖的编译时间可能更长，尤其是首次构建；它改善的是开发过程中运行 GPUI 应用时的性能。Package profile 只在应用或 workspace 根目录的 `Cargo.toml` 中生效：

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

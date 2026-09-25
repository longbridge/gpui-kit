---
title: 安装
description: 安装 GPUI Kit，并准备在 macOS、Windows 和 Linux 上构建 Rust 桌面应用所需的平台依赖。
order: -1
---

# 安装

先为操作系统安装原生构建工具链，再在 Rust 应用中添加 `gpui-kit`。以下步骤面向桌面开发；[WebAssembly](./webassembly) 和[移动端](./mobile)有各自的目标平台配置。

## 平台要求

<div class="doc-tabs" role="group" aria-label="操作系统">
  <input class="doc-tabs__input" type="radio" name="install-platform-zh" id="install-macos-zh" checked>
  <input class="doc-tabs__input" type="radio" name="install-platform-zh" id="install-windows-zh">
  <input class="doc-tabs__input" type="radio" name="install-platform-zh" id="install-linux-zh">
  <div class="doc-tabs__list">
    <label for="install-macos-zh">macOS</label>
    <label for="install-windows-zh">Windows</label>
    <label for="install-linux-zh">Linux</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <ul><li>macOS 15 或更高版本</li><li>Xcode Command Line Tools，可运行 <code>xcode-select --install</code> 安装。之后运行 <code>xcode-select -p</code>；应输出所选的开发者工具目录。</li></ul>
    </section>
    <section class="doc-tabs__panel">
      <ul><li>Windows 10 或更高版本</li><li>Visual Studio 2022 Build Tools 或 Community，并安装 <strong>Desktop development with C++</strong> workload，其中包含 MSVC 与 Windows SDK。Rust 工具链应选择 MSVC 目标，而非 GNU 目标。</li><li>确保构建所用终端可以执行 <code>cmake --version</code>。</li></ul>
    </section>
    <section class="doc-tabs__panel">
      <p>以下依赖已在 Ubuntu 24.04 验证：</p>
      <pre><code class="language-bash">sudo apt update
sudo apt install -y gcc g++ clang libfontconfig-dev libwayland-dev \
  libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libssl-dev libzstd-dev vulkan-validationlayers libvulkan1</code></pre>
      <p>此清单与仓库的 <code>script/install-linux.sh</code> 一致，适用于 Ubuntu 24.04；其他发行版需要安装对应的开发包。显示窗口还需要可用的 Wayland 或 X11 图形会话及 Vulkan 驱动；单独安装 <code>libvulkan1</code> 并不会安装 GPU 驱动。</p>
    </section>
  </div>
</div>

## Rust 和 Cargo

通过 [Rust 官方安装页面](https://rust-lang.org/tools/install/)安装 Rust 和 Cargo。按本仓库当前锁定的依赖图，请使用 Rust 1.92 或更新版本：其中的 Linux GPUI 平台依赖 `oo7 0.6.0` 明确要求至少 Rust 1.92。然后在准备用来构建应用的同一个终端中检查：

```sh
rustc --version
cargo --version
```

两条命令都应输出版本号。在 Windows 上，<code>rustup show active-toolchain</code> 应显示 <code>msvc</code> 目标。如果刚安装完仍找不到 Cargo，请打开新终端重试。

在应用的 `Cargo.toml` 的 `[dependencies]` 中加入：

```toml
gpui-kit = "0.6"
```

`0.6` 要求会选择兼容的 0.6.x 版 Kit；本仓库当前声明的是 `0.6.5`。Kit 的默认特性包含带样式的组件和默认图标资源，并会引入配套的 GPUI crate；按此方式构建的应用无需单独声明 GPUI。`use gpui_kit::*;` 导入 Kit 重导出的 GPUI API；各层分别可通过 `gpui_kit::component`、`gpui_kit::base`、`gpui_kit::assets` 和 `gpui_kit::platform` 访问。

### 为什么依赖名是 `gpui-pre`

本文中的 **GPUI** 指 [Zed 的 GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)。`gpui-pre` 是将 Zed 指定提交的 GPUI crate 以一组配套版本发布到 crates.io 时使用的 Cargo 包名，供 GPUI Kit 固定可复现的依赖；它不是另一套渲染实现。发布过程会调整包名及依赖清单以适应 crates.io，所以本文的 API 说明以当前 GPUI Kit 固定的 GPUI 版本为准。本仓库固定 `gpui-pre = 0.3.6`；应用通常只依赖 `gpui-kit`，通过 `gpui_kit::*` 使用 GPUI。`gpui-pre` 发布了更新的快照，也不代表当前 GPUI Kit 版本已支持它。

## 验证安装

如果已检出本仓库，请在仓库根目录运行现有示例：

```sh
cargo run -p hello_world
```

第一次构建需要下载和编译依赖，可能耗时数分钟。启动后，窗口会显示 “Hello, World!” 和 “Let's Go!” 按钮；点击按钮，终端会打印 `Clicked!`。关闭窗口即可结束进程。这个步骤验证本机的构建工具链、窗口创建、绘制和输入，不能作为性能 benchmark。

如果要创建自己的项目，请接着阅读[Getting Started](./getting-started)。它用同一个依赖构建应用，并逐步解释第一个 [View](./render) 和 [Window](./window)。

## 常见问题

| 现象 | 排查方法 |
| --- | --- |
| 找不到 `cargo` 或 `rustc`。 | 用 rustup 安装 Rust，打开新终端，再运行上面的版本检查命令。 |
| 构建提示 Rust 版本不受支持。 | 检查 `rustc --version` 并更新当前 Rust 工具链。依赖所需的编译器版本可能高于本文的基线。 |
| Windows 提示缺少 `link.exe` 或 Windows SDK。 | 检查 Visual Studio C++ workload 和 SDK，并确认使用 MSVC Rust 工具链；必要时从 Visual Studio Developer PowerShell 构建。 |
| Linux 提示缺少 `pkg-config` 命令、X11、Wayland、fontconfig 或 WebKit 头文件。 | 安装上面的 Ubuntu 开发包，或对应发行版的等价包。如果缺少 `pkg-config` 命令本身，还需安装 `pkg-config` 包；根据报错定位具体工具或系统库。 |
| Linux 上编译成功，但没有出现窗口。 | 确认程序运行于 Wayland 或 X11 图形会话，并且会话中有可用的 Vulkan 驱动。无图形环境的终端或只有 Vulkan loader 都不够。 |
| Cargo 无法解析 `gpui-pre`，或 API 与示例不一致。 | 保留 `gpui-kit` 依赖要求，并一起更新依赖。Kit 固定一组匹配的 `gpui-pre-*` 快照；不要把其中某个 GPUI 包单独覆盖为别的版本。在本仓库中使用已提交的 `Cargo.lock`。 |

窗口打开后的功能问题，可继续阅读[Getting Started](./getting-started)以及对应功能指南。

## 提升开发模式运行性能

Rust Debug 构建下，GPUI、组件库、布局和文字渲染相关 crate 基本没有优化，因此通过 `cargo run` 启动的应用，其渲染与交互性能会明显低于 release build。下面的配置只优化这些框架依赖，应用自身代码仍保持 Debug mode，可以继续使用正常的调试流程。

这个配置**不会加快编译**。启用优化后，这些依赖的编译时间可能更长，尤其是首次构建；它改善的是开发过程中运行 GPUI 应用时的性能。Cargo 的[package profile override](https://doc.rust-lang.org/cargo/reference/profiles.html#overrides) 只在应用或 workspace 根目录的 `Cargo.toml` 中生效：

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

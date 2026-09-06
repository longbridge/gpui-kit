# GPUI Kit

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/longbridge/gpui-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/longbridge/gpui-kit/actions/workflows/ci.yml) [![Docs](https://docs.rs/gpui-kit/badge.svg)](https://docs.rs/gpui-kit/) [![Crates.io](https://img.shields.io/crates/v/gpui-kit.svg)](https://crates.io/crates/gpui-kit)

使用 Rust 和 GPUI 构建出色、高性能的桌面应用。

GPUI Kit 是一个综合性的 Rust 桌面应用开发框架。它将生产级 UI
系统、应用级数据与布局能力、编辑能力，以及可复用的行为、状态和基础设施整合在一起，
并让交付后的应用可以被 JavaScript 扩展。

文档：<https://gpui-kit.com>

```text
gpui-kit             应用唯一需要依赖的 crate
├── gpui-base        无样式的行为、状态与基础设施
├── gpui-shell       为 Rust 宿主提供 JavaScript 扩展能力
└── gpui-component   GPUI Component：完整的带样式 UI 系统
```

`gpui-kit` 会固定配套的 GPUI 版本并重新导出全部三层，应用只需声明这一个依赖，无需接触 GPUI 本身。

## 特性

- **60+ 组件**：覆盖表单、导航、浮层、反馈和布局等场景，提供成熟交互与高效默认值。
- **生产就绪**：从第一天起用于构建 Longbridge Pro，并在公开发布的商业桌面应用中持续打磨。
- **原生体验**：现代控件设计灵感来自 macOS 与 Windows，并提供语义化主题和多种尺寸。
- **120 FPS**：GPU 加速界面，在高负载下依然保持流畅。
- **数据表格**：虚拟滚动、固定列、列宽调整、排序与单元格选择，可承载数十万行数据。
- **虚拟列表**：只渲染可见区域，并支持不同尺寸的列表项。
- **代码编辑器**：20 万行规模下仍保持稳定，集成 Tree-sitter 高亮与 LSP 诊断、补全和悬浮提示。
- **Dock 布局**：可调整面板、可拖拽标签、嵌套分割、边缘停靠，以及可序列化的 Tiles 自由布局。
- **丰富内容**：原生 Markdown 与 HTML 渲染、语法高亮和内置图表。
- **设计自由**：使用完整视觉系统，或基于 `gpui-base` 的行为与基础设施构建自己的系统。
- **JavaScript 扩展**：`gpui-shell` 让已发布的 Rust 宿主以脚本方式加载面板与业务逻辑，每项能力都需显式授予。
- **跨平台**：通过一份 Rust 代码交付 macOS、Windows 和 Linux。

## 框架架构

### 三层架构，一个生态

使用 `gpui-component`，让整个应用保持统一、完整的视觉与交互风格；当产品需要创建并拥有自己的设计系统时，使用 `gpui-base`；当应用需要在交付后仍可被 JavaScript 扩展时，使用 `gpui-shell`。

| **`gpui-component`**     | **`gpui-base`**            | **`gpui-shell`**                 |
| ------------------------ | -------------------------- | -------------------------------- |
| 完整且带样式的组件       | 无预设样式的行为与基础设施 | 由 Rust 托管的 JavaScript 运行时 |
| 开箱即用，并支持主题定制 | 完全掌控结构与视觉设计     | 能力逐项授予                     |
| 适合直接构建应用         | 适合构建设计系统           | 适合插件与脚本化应用             |

```text
                             APPLICATION
                                  │
              ┌───────────────────┼───────────────────┐
              │                   │                   │
              ▼                   ▼                   ▼
    ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
    │  gpui-component  │ │ Your Design      │ │    gpui-shell    │
    │    Styled UI     │ │ System           │ │  JS extensions   │
    └────────┬─────────┘ └────────┬─────────┘ └────────┬─────────┘
             │                    │                    │
             └────────────────────┼────────────────────┘
                                  ▼
                        ┌──────────────────┐
                        │    gpui-base     │
                        │ Behavior · State │
                        │ Infrastructure   │
                        └────────┬─────────┘
                                 ▼
                               GPUI
```

> **行为属于基础层，呈现属于应用。**

如果希望使用精致、开箱即用且风格统一的控件，请选择 **`gpui-component`**。如果应用需要拥有组件源码、布局、样式和动效，同时复用复杂且可靠的交互行为，请直接构建于 **`gpui-base`**。如果希望贡献者无需 fork、也无需发新版本就能扩展产品，请加入 **`gpui-shell`**。

这种分层方式与 [shadcn](https://ui.shadcn.com) 生态的灵活性来源一致：

| GPUI Kit 生态                        | Web 生态                       |
| ------------------------------------ | ------------------------------ |
| GPUI                                 | HTML + Tailwind CSS            |
| [`gpui-base`](crates/base/README.md) | [Base UI](https://base-ui.com) |
| `gpui-component`                     | shadcn 的完整样式组件层        |

[深入了解架构 →](docs/ARCHITECTURE.md)

## Showcase

GPUI Kit 从第一天起就用于构建 [Longbridge Pro](https://longbridge.com/desktop)。
这个框架不是脱离应用场景凭空设计出来的，而是从一款公开发布的商业桌面应用中持续提炼而成。

> **GPUI 为渲染打下基础，Longbridge 为生产实践打下基础。**

<img width="1763" alt="Image" src="https://github.com/user-attachments/assets/e1ecb9c3-2dd3-431e-bd97-5a819c30e551" />

## Usage

```toml
[dependencies]
gpui-kit = "0.6"
```

`gpui-kit` 始终引入 GPUI 和 `gpui-base`；`gpui-component` 和默认图标集默认开启。只想保留部分层时关闭默认 feature 按需选择即可。`gpui-component` 的 feature（`inspector`、`decimal`、`tree-sitter` 及各 `tree-sitter-<language>`）在 `gpui-kit` 上同名可用。

### 基础示例

```rs
use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

pub struct HelloWorld;
impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    gpui_kit::application().run(move |cx| {
        // 使用任何 GPUI Component 功能之前必须先调用此函数。
        gpui_kit::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // 窗口的第一层应该是一个 Root。
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

### 图标

默认开启的 `assets` feature 会以 `gpui-kit-assets` 的形式内置 [Lucide](https://lucide.dev) 图标集，通过 `gpui_kit::application().with_assets(gpui_kit::assets::Assets)` 交给应用即可。若想使用自己的图标，去掉该 feature，并按照 [IconName](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/icon.rs#L86) 中的定义命名 SVG 文件。

## AI 编码 Agent 技能 (Skills)

为你的 AI 编码助手（Cursor, Claude Code, Gemini CLI, Codex 等）安装 GPUI Kit 技能库：

```bash
npx skills add longbridge/gpui-kit
```

| 技能                     | 描述                                                                                                          |
| ------------------------ | ------------------------------------------------------------------------------------------------------------- |
| `gpui-kit`               | 初始化、组件目录、常用使用模式、GPUI 机制（Element、Entity、异步、焦点、Actions、测试），以及 Coding Guides。 |
| `gpui-kit-design-guides` | Design Guides：布局、间距、层级、交互状态、浮层与界面文案的规范。                                             |

## Development

### 桌面 Gallery（Story）

`story` crate 是一个展示所有可用组件的画廊应用程序，通过以下命令运行：

```bash
cargo run
```

### Examples

一些较大的示例复用 `story` 画廊组件，并作为独立 package 运行：

```bash
# Dock 布局系统（面板、分割视图、标签页）
cargo run -p example-dock

# Markdown 渲染
cargo run -p example-markdown

# HTML 渲染
cargo run -p example-html
```

`examples` 目录还包含独立示例，每个示例专注于单一功能。每个示例是一个独立的 crate，使用 `cargo run -p <name>` 运行：

```bash
# 支持 LSP 和语法高亮的代码编辑器
cargo run -p example-editor

# 基础 Hello World
cargo run -p hello_world

# 系统监控器（实时 CPU/内存数据图表）
cargo run -p system_monitor

# 窗口标题自定义
cargo run -p window_title
```

查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解更多详情。

## 与其他框架对比

请查看站点上的[与 Iced、egui、Qt 6 的对比](https://gpui-kit.com/zh-CN/docs/comparison)。

## 许可证

Apache-2.0

- 基于 Zed Industries 的 [GPUI](https://github.com/zed-industries/zed) 构建，GPUI 同样采用 Apache-2.0。`gpui-pre-*` 是它的快照，发布时保留 Zed 的许可证与声明。
- UI 设计基于 [shadcn/ui](https://ui.shadcn.com)，部分来自 [Reui](https://reui.io)。
- 图标来自 [Lucide](https://lucide.dev)。

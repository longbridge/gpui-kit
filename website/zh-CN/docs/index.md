---
title: GPUI Kit
description: 基于 GPUI 构建出色高性能桌面应用的综合性 Rust 开发框架。
---

# GPUI Kit 简介

GPUI Kit 是一个基于 GPUI 的综合性 Rust 桌面应用开发框架。
依赖名 `gpui-pre` 指 GPUI Kit 固定版本使用的 GPUI 发布快照，并非另一套渲染框架；详见[安装说明](./installation#为什么依赖名是-gpui-pre)。

GPUI Kit 的核心架构包含五个层次：

- **`gpui`**：底层 UI 运行时，提供 Entity、窗口、Element、布局与渲染。
- **`gpui-kit`**：应用入口，通过一个依赖重新导出 GPUI，并整合 Base、Component 与默认资源。
- **`gpui-base`**：无样式的行为、受控状态、Focus、浮层、虚拟列表、Dock 基础设施与语义化设计 token。
- **`gpui-component`**：即 GPUI Component，完整的带样式组件库，提供 75+ 个有完整文档的组件与原语，以及主题、数据表格、Dock 布局和代码编辑器。
- **`gpui-shell`**：让 Rust 宿主可以被 JavaScript 扩展，能力逐项授予。

普通应用通过 `gpui-kit` 使用 GPUI、Base、Component 与默认资源；需要承载 JavaScript 扩展时另行依赖 `gpui-shell`。它仍是框架核心架构的一层。

使用 `gpui-component` 可以获得统一、成熟的视觉风格；基于 `gpui-base` 则可以复用可靠的行为与基础设施，同时创建并拥有自己的设计系统。本节文档介绍 GPUI Kit 的入门配置、公共设计与编码指南，以及应用开发。各层 API 请参阅 [GPUI Component](/zh-CN/component)、[GPUI Base](/zh-CN/base) 与 [GPUI Shell](/zh-CN/shell)。

先读 [Focus](./focus)，理解 `FocusHandle`、Tab 顺序与键盘目标，再读 [Action](./action) 理解命令派发；[KeyBinding](./keybinding) 介绍 Action 的绑定与当前快捷键的展示。[Event](./event) 解释类型化通知以及 Action 与 Event 的关系。

理解核心渲染模型，可先阅读 [Entity](./entity) 和 [Context](./context)，
再阅读 [Render](./render)、[RenderOnce](./render-once) 与 [ElementId](./element_id)。
[Style](./style) 介绍 GPUI 的 fluent 样式方法；[Element](./element) 与 [Paint](./paint) 解释更底层的绘制。
[Task](./task) 介绍 callback 返回后仍继续运行的工作。

## 按工作顺序学习 GPUI

可以按以下阶段学习。每一阶段都可以先完成一个小任务，再继续下一阶段：

1. **打开窗口：**按[安装](./installation)和[开始使用](./getting-started)运行按钮示例，确认点击后终端输出消息。
2. **持有并更新状态：**创建 [Entity](./entity)，通过 [Context](./context) 更新它，再用 [Render](./render) 显示新值。[Window](./window) 说明更新落在哪个窗口。
3. **绘制自定义控件：**跟着 [Element](./element)、[Geometry](./geometry) 和 [Paint](./paint) 使用现有 Brush 示例；需要稳定身份或复用时，再看 [ElementId](./element_id) 与 [View Cache](./view-cache)。
4. **处理输入和持续工作：**先用 [Focus](./focus) 建立键盘目标，再用 [Action](./action) 或 [Event](./event) 连接交互；使用 [Task](./task) 执行异步工作，使用 [Animation](./animation) 构建能正确结束的动效。
5. **检查应用：**用[无障碍](./accessibility)和[测试](./test)核对交互。谈论帧率前先看 [FPS Monitor](./fps)；应用需要相应目标时，再看 [WebAssembly](./webassembly) 或[移动端](./mobile)。

核心页面会区分代码示例与运行结果，并连接后续概念。第一个窗口跑通后，可用
[编码指南](./coding-guides)整理所有权和架构约定。

## 特性

- **75+ 组件与原语**：覆盖表单、导航、浮层、数据展示、编辑、反馈和布局等场景
- **生产就绪**：在实际桌面应用中持续打磨，并通过 GPUI Kit 的组件与示例不断验证
- **WebAssembly**：应用与组件示例可通过 `wasm32-unknown-unknown` 在 Web 中运行
- **无障碍**：交互层内置 AccessKit role、name、state、relationship 与 action
- **UI 集成测试**：在 headless window 中驱动真实鼠标、键盘、Focus、布局与无障碍行为
- **原生体验**：设计灵感来自 macOS 与 Windows 的现代桌面控件
- **高刷新率支持**：在 120 Hz 屏幕上，完整工作负载需要落在约 8.3 ms 的单帧预算内；实际流畅度取决于应用、设备与呈现链路。参阅[帧率、刷新率与渲染模式](./fps#120-hz-是帧预算不是刷新承诺)
- **数据表格**：虚拟滚动、固定列、列宽调整、排序与单元格选择，可承载数十万行数据
- **虚拟列表**：只渲染可见区域，并支持不同尺寸的列表项
- **代码编辑器**：20 万行、Tree-sitter 高亮、诊断、补全和悬浮提示
- **Dock 布局**：可调整面板、可拖拽标签、嵌套分割和边缘停靠
- **丰富内容**：原生 Markdown 与 HTML、语法高亮和图表
- **设计自由**：使用完整视觉系统，或基于 `gpui-base` 构建自己的系统
- **类型化动效**：CSS 对齐的 easing、timing、keyframes、spring、presence 与测量式展开，稳定采样路径零分配
- **跨平台**：通过一份 Rust 代码交付 macOS、Windows 和 Linux

## 最小示例

按[安装说明](./installation)准备平台依赖后，运行 `cargo new gpui-hello` 创建项目，再运行 `cd gpui-hello` 进入目录。在项目的 `Cargo.toml` 中加入：

```toml
[dependencies]
gpui-kit = "0.6"
```

将 `src/main.rs` 写成下面的完整程序，运行 `cargo run`。窗口会显示一段文字和按钮；
点击按钮后，终端输出 `Clicked!`。

```rust
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child("Hello, World!")
            .child(
                Button::new("hello")
                    .primary()
                    .label("Click me")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    application()
        .with_assets(assets::Assets)
        .run(|cx| {
            init(cx);

            open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| HelloWorld)
            })
            .expect("Failed to open window");
        });
}
```

下一步跟着[开始使用](./getting-started)理解初始化、`Render` 和持久状态，
再按上面的学习路线进入绘制与交互。

## 下一步

- 阅读 [开始使用](./getting-started)
- 浏览 [组件文档](../component/index)
- 阅读 [GPUI Base 动画与动效](/zh-CN/base/motion)

## 社区与支持

- [GitHub 仓库](https://github.com/longbridge/gpui-kit)
- [问题反馈](https://github.com/longbridge/gpui-kit/issues)
- [贡献指南](https://github.com/longbridge/gpui-kit/blob/main/CONTRIBUTING.md)

## 许可证

Apache-2.0

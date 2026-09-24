---
title: Getting Started
description: 通过一个依赖和一个视图构建首个 GPUI Kit 桌面应用。
order: -2
---

# Getting Started

本指南创建一个带 GPUI Kit 按钮的小型桌面窗口。你需要 Rust、Cargo，以及对应平台的系统库；macOS、Windows 和 Linux 的要求见[安装](./installation.md)。学习这里的视图模型后，如需浏览器目标可继续阅读 [WebAssembly](./webassembly.md)。

## 创建项目

```sh
cargo new gpui-hello
cd gpui-hello
```

在生成的 `Cargo.toml` 中加入 GPUI Kit：

```toml
[dependencies]
gpui-kit = "0.6"
```

只需这一个依赖，即可使用 GPUI、GPUI Base、带样式的 GPUI Component 和默认图标资源。应用代码通过 `use gpui_kit::*;` 使用 GPUI，通过 `gpui_kit::component` 使用组件。以后可以调整 feature 选择，详见[图标与资源](./assets.md)。

## 添加视图

将 `src/main.rs` 替换为：

```rust
use gpui_kit::component::button::Button;
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

在项目目录执行 `cargo run`。窗口会显示文字和按钮；点击按钮后，终端会输出 `Clicked!`。

启动过程分为三步：

1. `gpui_kit::application()` 创建桌面应用；`.with_assets(...)` 注册默认图标资源。
2. `gpui_kit::init(cx)` 初始化启用的 Kit 层，包括组件主题。只调用一次，并且要先于打开应用窗口或构造组件。
3. `gpui_kit::open_window(...)` 从闭包创建 `Entity<HelloWorld>`，再用 [`Root`](./window) 包裹它。`Root` 管理窗口的浮层，包括对话框、抽屉和通知。闭包返回内容视图，不要再自行返回一个 `Root`。

`HelloWorld` 实现 GPUI 的 [`Render`](./render) trait。GPUI 渲染视图时，`render` 返回[元素树](./element)：一个包含文字和 `Button` 的 `div`。按钮是在本次渲染中构建的值；如果控件需要持久状态，比如输入框文字，所属视图应保存对应的状态 `Entity`，不要在 `render` 中重新创建。

## 一个简短的心智模型

[Entity<T>](./entity) 保存跨帧状态。它可以持有不参与绘制的 model；当 `T` 实现 `Render` 且被挂载时，这个 Entity 就是持久的 **View**，每次渲染都会生成新的元素树。[RenderOnce](./render-once) 组件则把输入作为一个值，描述树中可复用的一部分。调用方提供当前状态和 handler 时适合用它；它仍可使用少量带 key 的元素状态。复杂状态、订阅和任务则需要持久的 owner。

```text
应用入口 → 功能单元（model、command、view）
             ├─ Entity<Model>       保留状态
             └─ Entity<View>        持久视图；View 实现 Render
                   └─ 元素树         每次渲染重新构建
                        └─ RenderOnce 值组成可复用部分
```

应用增长后，拥有独立流程的功能可以把 model 和 View 放在同一个 feature crate 内；只有需要真正面向整个应用的状态时，才在其中使用私有 `Global`。功能之间通过小型公开接口、event 或 `Entity` handle 协作。这样可复用部分容易接入，团队成员或 AI 代理并行修改时也有清晰边界。何时拆分以及如何确定所有权和依赖方向，详见[编码指南](./coding-guides)。

## 接下来读什么

随着应用扩展，可按顺序阅读：

1. [Element](./element.md) 和 [RenderOnce](./render-once.md)：理解每帧的元素树和按值构建的组件。
2. [Entity](./entity.md) 和 [Context](./context.md)：保存状态，并在渲染之外更新它。
3. [Window](./window.md)：打开窗口并使用 `Root` 浮层。
4. [Event](./event.md) 和 [Action](./action.md)：连接状态变化、键盘快捷键和命令。
5. [组件目录](../component/index.md)：选择控件；随后按界面需要阅读[图标与资源](./assets.md)和[字体](./fonts.md)。

如需了解带有持久输入状态与订阅的完整应用，请看[可运行应用示例](https://github.com/longbridge/gpui-kit/tree/main/examples/ai_recipes)。[编码指南](./coding-guides.md)说明了这些示例遵循的约定。

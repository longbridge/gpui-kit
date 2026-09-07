---
title: 快速开始
description: 安装、初始化并渲染第一个 gpui-base 控件。
order: 2
---

# 快速开始

## 安装

使用与 `gpui-base` 匹配的 GPUI 仓库版本：

```toml
[dependencies]
gpui-base = { git = "https://github.com/longbridge/gpui-kit" }
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
```

## 初始化

打开窗口前调用一次 `gpui_kit::base::init`。若应用已调用 `gpui_kit::component::init`，其中已经包含 Base 初始化。

```rust
use gpui_kit::AppContext as _;

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_kit::base::init(cx);
        // 在这里打开应用窗口。
    });
}
```

## 渲染控件并设置样式

Base 控件刻意不提供产品专属的内边距、颜色或圆角，请用普通 GPUI 方法设置样式：

```rust
use gpui_kit::prelude::*;
use gpui_kit::{px, rgb};
use gpui_kit::base::Button;

Button::new("save")
    .px_3().py_2().rounded(px(6.))
    .bg(rgb(0x2563eb)).text_color(rgb(0xffffff))
    .on_click(|_, _, _| println!("save"))
    .child("Save")
```

跨渲染保持每个 `ElementId` 稳定，GPUI 才能保留元素和焦点状态。Checkbox、Switch、Radio、Toggle 等受控组件会通过回调报告下一个值；把它存进视图，并在下一次渲染时传回。

## 默认颜色 Token

`gpui-base` 通过 `ColorTokens::light()` 和 `ColorTokens::dark()` 提供可直接使用的浅色、深色语义调色板。`ColorTokens::default()` 使用浅色调色板。两套颜色均使用 `Hsla`，并与 `gpui-component` 的默认浅色、深色主题保持相同的语义角色。

```rust
use gpui_kit::base::{ColorTokens, SemanticThemeTokens, Theme};

// 根据应用当前外观选择对应调色板。
let colors = if is_dark {
    ColorTokens::dark()
} else {
    ColorTokens::light()
};

Theme::global_mut(cx).tokens = SemanticThemeTokens {
    colors,
    ..Default::default()
};
```

调色板描述的是语义角色，而不是某个组件的专用颜色：`background` 与 `foreground`、`surface` 与 `surface_foreground`、`primary`、`secondary`、`muted`、`accent`、`destructive`、`border`、`input`、`ring` 和 `selection`，以及对应的前景色。能从既有角色推导的细节，Base 组件就直接推导，例如链接颜色取自 `primary`，不再为它单独加 token。`selection` 之所以自成一个角色，是因为没有别的角色能替代：选区绘制在文字下方，必须保证文字依然清晰，而 `accent` 和 `ring` 都无法保证这一点。

调用 `gpui_kit::component::init` 时，当前浅色或深色主题会自动映射到同一套 Base token。只使用 `gpui-base` 的应用应在外观模式变化时安装对应的调色板。

## 运行共享示例

```sh
cargo run -p gpui-base-examples -- button
```

将 `button` 替换为[原语目录](./primitives/index.md)中的 slug。网站会把同一份展示代码编译为 `wasm32-unknown-unknown` 并加载到各原语页面。

---
title: Popover
description: 支持受控或内部开关状态的锚定浮层。
order: 19
---

# Popover

支持受控或内部开关状态的锚定浮层。

和所有 GPUI Base 原语一样，Popover 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- popover
```

## 导入

```rust
use gpui_kit::base::{Popover};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/popover.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/popover.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

使用 `.anchor(Anchor::TopCenter).offset(px(8.))` 可让弹层在触发器下方居中，并留出八像素间距。
Base 的 offset 默认为零。`Top*` 在下方，`Bottom*` 在上方，
`LeftCenter` 在右侧，`RightCenter` 在左侧。anchor 描述弹层自身的锚点。
窗口边界限制不会翻转弹层或更改 anchor。

`on_position` 在内容 prepaint 前提供最终弹层和触发器边界，供自定义绘制使用。
Base 不绘制箭头；带样式的 Component Popover 提供 `.arrow(true)`（默认 `false`），箭头跟随 anchor 对齐。

触发器切换打开状态；点击外部或 Escape 可按配置关闭。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/popover.rs{rust}

## 可访问性

管理触发器与内容的关系和焦点，不要让关闭后焦点丢失。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

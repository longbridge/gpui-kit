---
title: OTP Input
description: 由共享文本状态驱动的多单元格一次性验证码输入。
order: 17
---

# OTP Input

由共享文本状态驱动的多单元格一次性验证码输入。

和所有 GPUI Base 原语一样，OTP Input 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- otp-input
```

## 导入

```rust
use gpui_kit::base::{OtpInput, OtpState};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/otp-input.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/otp-input.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

OtpState 保存完整验证码，各视觉单元格只是同一输入状态的投影。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/otp_input.rs{rust}

## 可访问性

提供整体标签，不要让辅助技术把每个视觉格误读为互不相关的输入。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

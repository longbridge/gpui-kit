---
title: Date Picker
description: 将日历行为与弹出层组合的焦点感知日期输入。
order: 10
---

# Date Picker

将日历行为与弹出层组合的焦点感知日期输入。

和所有 GPUI Base 原语一样，Date Picker 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- date-picker
```

## 导入

```rust
use gpui_kit::base::{DatePicker};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/date-picker.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/date-picker.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

应用持有日期与打开状态，并处理输入或日历产生的变更。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/date_picker.rs{rust}

## 可访问性

输入应有标签；弹出日历需保留键盘导航和清晰焦点。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

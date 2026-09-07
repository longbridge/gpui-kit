---
title: Alert Dialog
description: 用于需要用户明确决定之操作的模态确认界面。
order: 2
---

# Alert Dialog

用于需要用户明确决定之操作的模态确认界面。

和所有 GPUI Base 原语一样，Alert Dialog 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- alert-dialog
```

## 导入

```rust
use gpui_kit::base::{AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogDescription, AlertDialogPopup, AlertDialogTitle, AlertDialogTrigger};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/alert-dialog.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/alert-dialog.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

打开状态和确认/取消结果由应用管理。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/alert_dialog.rs{rust}

## 可访问性

将焦点限制在模态层内，提供标题与说明，并确保取消操作始终可用。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

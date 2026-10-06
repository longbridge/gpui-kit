---
title: Tooltip
description: 与触发元素关联、延迟显示且可定位的说明。
order: 34
---

# Tooltip

与触发元素关联、延迟显示且可定位的说明。

和所有 GPUI Base 原语一样，Tooltip 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- tooltip
```

## 导入

```rust
use gpui_kit::base::{Tooltip};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/tooltip.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/tooltip.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

指针悬停或键盘聚焦后延迟显示，离开或失焦后关闭。

`TooltipOverlay` 默认等待 500 ms 后显示；关闭时保留 300 ms 宽限期，期间移入另一个触发元素会立即切换。安装 `TooltipDefaults` 可在应用范围内调整这两个时长，`TooltipRequest::with_show_delay` 可覆盖单次请求的显示延迟。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/tooltip.rs{rust}

## 可访问性

Tooltip 只补充说明，不能承载完成任务所必需的信息；触发器必须可聚焦。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

---
title: Time Field
description: 分段编辑一天中的时间，提供完整的键盘模型，支持 24 或 12 小时制。
order: 30.5
---

# Time Field

分段编辑一天中的时间，提供完整的键盘模型，支持 24 或 12 小时制。

和所有 GPUI Base 原语一样，Time Field 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- time-field
```

## 导入

```rust
use gpui_kit::base::{HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision};
```

## 结构与 API

示例在 `TimeFieldState` 之上组合 `TimeField`。字段为时、分以及可选的秒和上午/下午各渲染一个 `TimeFieldSegment`，数字段之间用 `:` 分隔。用 `Styled` 布局和装饰根元素，通过 `TimeField::render_segment` 装饰每一段；该插槽会收到 `TimeFieldSegmentState`，包含段类型、当前值以及是否选中。权威实现位于 [`components/time_field.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/time_field.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

`TimeFieldState` 保存时间、精度（`TimePrecision::Minute` 或 `Second`）和小时制（默认 `HourCycle::H23`，也可以是 `H12`）。`set_time` 替换值但不触发事件；用户编辑会触发 `TimeFieldEvent::Change`。

整个字段是一个 Tab 停靠点。Up/Down 调整当前选中的段，并在段内循环、不向上一级进位；Left/Right 和 Tab/Shift-Tab 在段之间移动；输入数字时使用两位缓冲，当前段无法再容纳更多数字时自动跳到下一段；`a`/`p` 切换上午或下午；Backspace/Delete 重置当前段。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

<<< ../../../../crates/base/examples/showcase/components/time_field.rs{rust}

## 可访问性

根元素以 `Role::TimeInput` 暴露，值为格式化后的时间。请为字段提供标签，并让选中的段在视觉上与其他段明显区分。

## 注意事项

使用等宽数字或固定段宽，避免输入数字时字段宽度变化。在消费端设计系统中验证焦点、选中、禁用和高对比度状态。

---
title: GPUI Base
description: GPUI Kit Rust 桌面框架中无样式的行为与基础设施层。
order: 1
---

# GPUI Base

`gpui-base` 是 GPUI Kit 的无样式基础层。它提供交互行为、受控状态、焦点管理、无障碍语义、动画、虚拟列表和主题 token，同时将布局与视觉设计完整留给应用。

## 如何选择

| 使用 | 适用场景 |
| --- | --- |
| `gpui-base` | 需要创建自己的设计系统，并掌控每个视觉选择 |
| `gpui-component` | 需要一套具有完整视觉设计、可直接使用的组件 |

依赖始终由上层指向基础层：`gpui-component` 构建于 `gpui-base` 之上，应用也可以直接使用任意一层。

## 基本原则

- **行为内置**：控件提供一致的指针、键盘、焦点和状态行为。
- **表现由应用决定**：直接组合 GPUI 样式方法和 children，不需要覆盖默认视觉。
- **部件可组合**：primitive 暴露有意义的子部件，而不是把结构隐藏在单体组件中。
- **状态明确**：受控输入报告变化，最终状态由 view 持有。

## 开始使用

从[入门指南](./getting-started.md)开始，使用 [TextView](./text-view.md) 渲染可选择的 Markdown 与 HTML，或阅读[文本选择](./text-selection.md)为自定义 renderer 接入窗口级选择。每个页面都提供 Rust 代码和可运行的 WASM 示例。

[History](./history.md) 介绍两种用途明确不同的状态结构：`History` 是包含根条目、当前条目、后退与前进分支的导航轨迹，`UndoHistory` 则记录分组的 undo 与 redo 事务。

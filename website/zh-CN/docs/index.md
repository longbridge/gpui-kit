---
title: 介绍
description: 基于 GPUI 构建出色高性能桌面应用的综合性 Rust 开发框架。
---

# GPUI Kit 简介

GPUI Kit 是一个基于 GPUI 的综合性 Rust 桌面应用开发框架。

它将完整 UI 系统与应用级数据、布局、内容和编辑能力整合在一起，并以三个层层递进的 crate 交付，只需依赖 `gpui-kit` 一个包即可全部获得：

- **`gpui-base`**：无样式的行为、受控状态、焦点、浮层、虚拟列表、Dock 基础设施与语义化设计 token。
- **`gpui-component`**：即 GPUI Component，完整的带样式组件库，提供 60+ 控件、主题、数据表格、Dock 布局和代码编辑器。
- **`gpui-shell`**：让 Rust 宿主可以被 JavaScript 扩展，能力逐项授予。

使用 `gpui-component` 可以获得统一、成熟的视觉风格；基于 `gpui-base` 则可以复用可靠的行为与基础设施，同时创建并拥有自己的设计系统。本节文档面向 GPUI Component；另外两层请参阅 [GPUI Base](/zh-CN/base) 与 [GPUI Shell](/zh-CN/shell)。

## 特性

- **60+ 组件**：覆盖表单、导航、浮层、反馈和布局等场景
- **生产就绪**：从第一天起用于构建 Longbridge Pro，并在公开发布的商业桌面应用中持续打磨
- **原生体验**：设计灵感来自 macOS 与 Windows 的现代桌面控件
- **120 FPS**：GPU 加速界面，在高负载下依然保持流畅
- **数据表格**：虚拟滚动、固定列、列宽调整、排序与单元格选择，可承载数十万行数据
- **虚拟列表**：只渲染可见区域，并支持不同尺寸的列表项
- **代码编辑器**：20 万行、Tree-sitter 高亮、诊断、补全和悬浮提示
- **Dock 布局**：可调整面板、可拖拽标签、嵌套分割、边缘停靠和 Tiles 自由布局
- **丰富内容**：原生 Markdown 与 HTML、语法高亮和图表
- **设计自由**：使用完整视觉系统，或基于 `gpui-base` 构建自己的系统
- **类型化动效**：CSS 对齐的 easing、timing、keyframes、spring、presence 与测量式展开，稳定采样路径零分配
- **跨平台**：通过一份 Rust 代码交付 macOS、Windows 和 Linux

## 下一步

- 阅读 [开始使用](./getting-started)
- 浏览 [组件文档](./components/index)
- 阅读 [GPUI Base 动画与动效](/zh-CN/base/motion)

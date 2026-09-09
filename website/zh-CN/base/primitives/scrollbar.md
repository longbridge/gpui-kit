---
title: Scrollbar
description: 为 GPUI 滚动视图、列表和自定义视口添加带动画的可定制滚动条。
order: 24
---

# Scrollbar

`Scrollbar` 是连接 GPUI 滚动句柄的自绘滚动条，支持纵向、横向和双轴视口、轨道点击、滑块拖动、可配置可见模式、类型化绘制样式、减少动态效果，以及可反向的可见性和宽度过渡。`gpui-base` 负责交互与过渡生命周期，应用或设计系统负责颜色、几何、时序和进入编排。

## 运行示例

```bash
cargo run -p gpui-base-examples -- scrollbar
```

原生与 WASM 共用 [`scrollbar.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/scrollbar.rs)。

## 基本用法

把 `ScrollHandle` 保存在持久视图状态中，通过 `track_scroll` 连接可滚动内容，并在同一个 `relative()` 容器中叠加 `Scrollbar`。`Scrollbar::new` 启用双轴；单轴使用 `vertical`、`horizontal` 或 `.axis(...)`。滚动条是绝对定位覆盖层，进入动画不会移动内容或命中区域。

## 可见模式

- `Scrolling`：滚动或拖动后显示，离开悬停区域后重新计算空闲等待。
- `Hover`：指针进入轨道后显示。
- `Always`：始终可见并跳过可见性过渡。

默认静止滑块宽 6 px，滑块悬停或拖动目标宽度为 8 px。隐藏的轨道与滑块不响应点击。

## 全局主题与实例覆盖

使用 `ScrollbarStyles` 的 `track`、`track_hover`、`track_active`、`thumb`、`thumb_hover` 和 `thumb_active` builder 设置外观；使用 `ScrollbarMotion` 配置 `idle`、`enter`、`exit`、`expand` 以及 `ScrollbarEntrance`。再把它们装入 `Theme::global_mut(cx).scrollbar = ScrollbarTheme::new().with_mode(...).with_motion(...).with_styles(...)`。字段保持私有，可通过 reader 查询。单个实例的 `.styles(...)` 优先于全局主题。

## 动画行为

Base 不附带产品动画。默认仅有 2 秒行为性空闲等待，进入、退出和展开时长均为零。`Fade` 原地淡入；`SlideAndFade` 让纵向滚动条从右侧、横向滚动条从底部进入。被中断的过渡从当前视觉值反向；零时长立即采用目标值。GPUI 的减少动态效果偏好也会把可见性和宽度时长降为零。

## 自定义视口与句柄

默认视口来自 `ScrollbarHandle::viewport_bounds`。组合控件可用 `.viewport_bounds(bounds)` 指定绘制视口，或 `.viewport_from_layout()` 使用覆盖层布局；只有句柄无法报告完整范围时才用 `.scroll_size(...)` 覆盖内容尺寸。`ScrollHandle`、`UniformListScrollHandle` 和 `ListState` 已实现 `ScrollbarHandle`；自定义容器需实现视口、偏移、设置偏移和内容尺寸，必要时实现 `start_drag` / `end_drag`。

## 稳定身份

构造器默认从调用位置派生 ID。同一调用位置生成多个独立滚动条时，应使用 `.id(("activity-list", panel_id))` 提供稳定 ID，以跨渲染保留可见性和宽度动画状态。

## 完整示例

<<< ../../../../crates/base/examples/showcase/components/scrollbar.rs{rust}

## 可访问性与交互检查

- 保留底层视口的滚轮、触控板和键盘滚动。
- 即使绘制的滑块很窄，也保留完整轨道命中区域。
- 验证普通、悬停和活动状态对比度。
- 动画不要移动布局或命中区域。
- 在减少动态效果下分别测试三种模式以及纵向、横向和双轴溢出。

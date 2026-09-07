---
title: 动画与动效
description: gpui-base 的类型化 transition、spring、keyframes、presence、stagger 与 reduced-motion 行为。
order: 4
example: motion
exampleKind: base
---

# 动画与动效

`gpui-base` 负责确定性的动效采样与生命周期，并把视觉选择留给应用。它提供稳定 keyed state、中断与反向、animation frame 请求和 reduced-motion 处理，不强加产品级时长或样式。

运行本文配套的交互示例：

```bash
cargo run -p gpui-base-examples --bin motion
```

示例包含五个相互独立的页面，可通过顶部标签逐个查看。

## 能力一览

| 示例 | API | 演示内容 |
| --- | --- | --- |
| Sliding time | `transition` | 08:00–20:00 的四位独立滚动数字，目标会在前一次过渡完成前继续变化 |
| Spring | `spring` | 快速切换目标时仍保持速度连续的分段选择器指示块 |
| Keyframes | `Keyframes`、`Timing`、`animate_keyframes` | 持续循环的多段活动信号 |
| Stagger | `Stagger` | 无分配地为列表计算错峰时间 |
| Presence | `Presence` | 退出动画完成前继续挂载内容 |

此外还提供 `Easing`、`Discrete`、`MotionTransform` 和 `MotionReveal`，它们与同一套 primitive 组合，不需要额外动画 runtime。

## Transition

已知时长、向目标值变化时使用 `transition`。每个独立运动值都要有稳定 ID：

```rust
let opacity = transition(
    ("save-dialog", "opacity"),
    if open { 1.0 } else { 0.0 },
    Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut),
    window,
    cx,
);
```

运动中改变目标会从当前采样值继续；直接反向还会缩短返回时长。`transition_with_status` 额外返回 `Idle`、`Delayed`、`Running` 或 `Finished`。

`Easing` 支持 CSS 关键字曲线、cubic Bézier、全部 step position 和分段 `linear()` stops，无效参数会返回类型化错误。

## Spring

目标可能在运动中变化时使用 `spring`。它同时保留位置与速度，适合选择指示器和空间值回落。

```rust
let x = spring(
    "selected-indicator",
    selected_x,
    Spring::new(Duration::from_millis(420)).with_damping(0.72),
    window,
    cx,
);
```

指针直接控制数值时，不要让 spring 追赶指针；拖动中使用 `with_travel(false)`，释放后再恢复。

`with_damping` 要求有限且非负的 ratio；`with_epsilon` 要求有限且大于零，并以目标值自身的单位解释。builder 会在无效的可信常量上 panic；配置值或用户输入应使用 `try_with_damping` 和 `try_with_epsilon`。归一化值通常保留默认的 `0.001`，像素移动可以使用 `0.1` 等较粗容差。

## Keyframes 与 Timing

`Keyframes` 定义经过校验的值序列；`Timing` 按绝对 elapsed time 采样，支持正负 delay、有限或无限迭代，以及 normal、reverse 和 alternate 播放方向。

offset 必须从 `0` 开始、以 `1` 结束并保持单调。不可插值属性使用 `Discrete`。

`animate_keyframes` 会在传入的稳定 ID 下保留播放起始时间。使用相同 ID 重新渲染只会继续当前序列，不会重新开始。需要重播时，把应用持有的 generation 放进 ID，例如 `("notification-enter", generation)`，并在每次重播时递增它。

## Presence 与 Stagger

`Presence` 将逻辑可见性与实际挂载分开，阶段包括 entering、present、exiting 和 absent。`should_render()` 为 true 时继续渲染，并把 `progress` 应用到所选视觉属性。退出中重新打开会从当前进度反向。

`Stagger` 可以从首项、末项、中心或指定位置开始，为每个 index 计算 delay；它不分配时间表，也不接管列表 identity。

## 测量式展开

`MotionReveal` 按 child 的自然尺寸测量，再根据 progress 裁剪可见高度。`Collapsible::motion_id(...)` 是控件层的便捷入口；没有 motion ID 时仍保持即时挂载/卸载。

## Reduced motion 与性能

Transition、spring、keyframes、presence 和 reveal 控件都遵守 GPUI 的 reduced-motion 偏好。有限动画会直接同步目标、更新 retained state，并且不留下待处理 frame。动画不能成为表达状态的唯一方式。

benchmark 覆盖的纯稳定采样路径——timing/easing、关键帧查找、解析式 spring 积分和 stagger delay 计算——均为零分配。Keyed transition、spring、presence 和 reveal 生命周期由 GPUI retained state 与 frame-request 测试覆盖，因为这些更新属于框架生命周期，而不是纯采样器。采样使用绝对时间，关键帧查找使用二分搜索。运行 release benchmark：

```bash
cargo bench -p gpui-base --bench motion
```

选择最小且合适的 primitive：固定时长目标使用 `transition`，频繁变化的空间目标使用 `spring`，编排序列使用 keyframes，卸载前退出使用 `Presence`，列表错峰使用 `Stagger`。

## Benchmark 结果

以下数据来自 Linux x86_64 release 构建，每项运行 31 个 batch、每个 batch 迭代 200 次：

| 工作负载 | Median | P95 | Worst | 内存分配 |
| --- | ---: | ---: | ---: | ---: |
| 1,000 次 scalar timing + easing 采样 | 26.490 µs | 26.567 µs | 27.290 µs | 0 |
| 1,000 次 keyframe 采样，2 frames | 21.656 µs | 21.707 µs | 21.729 µs | 0 |
| 1,000 次 keyframe 采样，8 frames | 25.197 µs | 25.251 µs | 25.269 µs | 0 |
| 1,000 次 keyframe 采样，32 frames | 27.932 µs | 27.969 µs | 27.971 µs | 0 |
| 1,000 次解析式 spring 积分采样 | 6.042 µs | 6.106 µs | 6.216 µs | 0 |
| 1,000 次 stagger delay 计算 | 0.574 µs | 0.583 µs | 0.587 µs | 0 |

Scalar timing/easing 工作负载低于 100 µs median 预算。这些数值是可复现的开发基线，并非跨平台性能保证；对特定平台性能有要求时，应在对应目标平台重新运行 benchmark。

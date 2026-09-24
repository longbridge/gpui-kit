---
title: Animation
description: 正确选用 GPUI 元素动画、GPUI Base Motion 与 GPUI Component 动效，并处理标识、中断和减弱动态效果。
order: -3.1
---

# Animation

GPUI Kit 有三个动效层次。应按**变化中的值由谁持有**来选择，而不是只看动画的外观：

| 层次 | 适用场景 | 状态与策略 |
| --- | --- | --- |
| GPUI `Animation`、`AnimationExt` | 元素入场、循环提示，或挂载期间播放固定步骤 | GPUI 在包装元素的 [`ElementId`](./element_id) 下保存播放进度；调用方决定时长、曲线和视觉属性。 |
| [GPUI Base Motion](/zh-CN/base/motion) | 运动中会变化的目标、卸载前的退出、关键帧和测量式展开 | Base 以稳定 key 保存每个通道，活动期间通过 [Window](./window) 请求帧；调用方决定视觉结果。 |
| GPUI Component 动效 | 外观需要跟随主题的样式化控件 | `cx.theme().motion_tokens()` 提供语义化时长、曲线、弹簧和距离；组件再与 GPUI 或 Base 组合。 |

语义状态归应用所有：对话框是否打开、当前选中哪个标签、滑块指向哪里。动画只负责呈现这些状态。无论在动画两端，还是关闭动效后，结果都必须清楚。

## GPUI 元素动画

`Animation::new(duration)` 创建播放一次、线性变化的动画。`AnimationExt::with_animation(id, animation, animator)` 包装一个 `IntoElement`；回调收到元素及经过 easing 映射的进度值。GPUI 在布局阶段调用它，把返回元素的样式用于当前帧，并在结束前继续请求帧。回调可修改元素支持的属性，例如透明度或变换。

```rust
use std::time::Duration;
use gpui_kit::*;

let entering = div()
    .child("已保存")
    .with_animation(
        ("saved-notice", generation),
        Animation::new(Duration::from_millis(180)),
        |element, progress| element.opacity(progress),
    );
```

通过 `gpui_kit::*`（或显式导入 trait）让 `AnimationExt` 生效。ID 标识的是动画包装元素，而不是文字或视觉属性。同一位置、同一 ID 的包装元素再次渲染时会继续已有播放；每次渲染重新构造 `Animation::new(...)` **不会**重播。需要重新入场时，把应用维护的 generation 放入 ID。移除包装元素会结束它的生命周期。一次性动画结束后，只要同一包装元素仍挂载，就保持终值。

`with_easing(f)` 把归一化时间映射到动画进度。GPUI 提供 `ease_in_out`、`bounce` 等函数；自定义曲线必须返回有限数值。曲线允许超出 `0..1`，因此当样式属性的合法范围更窄时，应自行裁剪结果。`repeat()` 在本地循环；`repeat_synced()` 根据应用共享时钟循环，适合多个提示同步。`with_max_fps(rate)` 限制该动画的最高重绘频率；非正数和非有限值会被忽略。`with_animations(id, animations, |element, step, progress| ...)` 播放固定步骤链，并提供当前步骤索引。

GPUI 还提供 `AnimationExt::with_spring(id, SpringAnimation<T>, animator)`，以弹簧驱动一个元素。稳定 ID 使目标改变时仍保留位置和速度。新挂载的弹簧默认从目标值开始，除非用 `SpringAnimation::from(...)` 指定起点。弹簧只作用于一个元素时可直接使用它；若一个独立 keyed 值要驱动组合中的多个部分，Base 的 `spring` 更合适。

GPUI 弹簧的 `SpringPlayback` 控制运行、暂停、停止、完成或取消。减弱动态效果会让**运行中**的弹簧直接到达目标；暂停和停止状态仍按各自的播放状态处理。目标与播放状态应由应用状态决定，不能把包装元素当成语义状态的唯一来源。

### 中断时会怎样

`with_animation` 按已播放时间推进，并不是追逐可变目标的 transition。在同一 ID 下改变回调捕获的终点，原播放时钟仍会继续；改 ID 则开始新播放。这两种操作都不会自动以当前画面值作为新起点。选择指示块需要在连续点击时平滑反向，应使用目标驱动的 spring 或 Base `transition`。

GPUI 的 `AnimationExt` 遵守 `App::reduce_motion()`：一次性动画显示终点，循环动画显示起点，两者都不再请求动画帧。加载状态仍需文字或其他静态提示；停住的旋转图标不足以说明当前正在加载。

## GPUI Base Motion：keyed 值与生命周期

应用只依赖 `gpui-kit` 时，从 `gpui_kit::base` 导入这些 API：

```rust
use gpui_kit::*;
use gpui_kit::base::{Easing, Transition, transition};
use std::time::Duration;

let opacity = transition(
    ("save-panel", "opacity"),
    if open { 1.0 } else { 0.0 },
    Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut),
    window,
    cx,
);

div().opacity(opacity)
```

这里的 `Transition` 是**值的时间策略**，不是带样式的元素。`transition` 返回当前采样值；`transition_with_status` 还返回 `Idle`、`Delayed`、`Running` 或 `Finished`。目标改变时，通道从当前采样值继续。直接反向会按剩余路程缩短返回时长。Base 只在延迟或运动期间请求帧。只要 owner 仍在，即使结果暂时不可见，也要在每次渲染时采样该通道，让保留的值正确收敛。

每个独立变化的值都要有自己的稳定通道 ID。按业务对象和属性设定命名空间，例如 `(project_element_id.clone(), "opacity")` 与 `(project_element_id.clone(), "height")`，其中 `project_element_id` 是稳定的 `ElementId`。两个通道共用 ID 会覆盖 retained state；每帧换 ID 则失去连续性。可重排列表要使用条目的业务 ID，而不是行号。即使描述同一个视觉元素，GPUI 包装元素 ID 和 Base 通道 ID 也服务于不同生命周期。

目标可能在收敛前再次变化时，使用 Base `spring(id, target, Spring, window, cx)`。重新设定目标时，它同时保留**位置和速度**。指针直接拖动期间使用 `Spring::with_travel(false)`，让值跟随指针；松开后恢复 travel。弹簧的 `epsilon` 使用目标值自身的单位，所以像素偏移通常需要比归一化透明度更粗的容差。

Base 还提供以下选择：

| 需求 | API | 生命周期要点 |
| --- | --- | --- |
| 编排数值停靠点 | `Keyframes`、`Timing`、`animate_keyframes` | 相同 ID 会继续播放；需要重播时把应用维护的 generation 放入 ID。Timing 支持延迟、迭代和播放方向。 |
| 连续的独立步骤 | `Sequence` | 每一步从上一步的绝对结束时刻开始；同一 ID 只播放一次，除非有意更改。活动步骤的目标变化时，会从当前采样值重新开始；sequence 不会自动反向。 |
| 退出完成前继续挂载 | `Presence` | 逻辑关闭后继续采样，直到 `should_render()` 为 false 才停止渲染。退出中重新打开会从当前值反向。 |
| 列表条目错峰 | `Stagger` | 根据索引和起点计算延迟；不持有列表及其 ID。 |
| 展开未知高度的内容 | `MotionReveal` | 测量 child，再按调用方提供的进度裁剪可见高度；它本身不采样或推进进度。 |

[Base Motion 指南](/zh-CN/base/motion)列出完整签名、参数校验、示例与基准测试。这里的 `Transition` 不同于旧的 `gpui_kit::base::animation::EffectTransition`：后者包装 GPUI `with_animation`，直接应用预设的淡入、滑动、宽度和高度效果。新的目标驱动动效优先使用 `base::motion` primitive，再自行应用采样值。

## GPUI Component：语义化动效策略

样式化组件通过 `cx.theme().motion_tokens()` 共享策略。`MotionTokens` 包含 `duration_instant`、`duration_fast`、`duration_normal`、`duration_slow`；`easing_enter`、`easing_exit`、`easing_move`；`spring_control`、`spring_move`；以及 `distance_short`、`distance_medium`。默认值构成协调的尺度，并不意味着所有控件都必须动画。从当前主题读取 token，产品才能集中调整。

例如，[Switch 源码](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/switch.rs) 在 `(self.id.clone(), "thumb")` 通道上采样 Base spring，向选中或未选中时的滑块偏移移动，策略来自 `cx.theme().motion_tokens().spring_move`。[可调整大小手柄源码](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/resizable.rs) 分别采样长度和透明度通道，使用 `duration_fast` 与 `easing_move`；指示块淡出时，细分隔线仍保留。[Collapsible](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/collapsible.rs) 通过 `.motion_id(id)` 启用测量式、可反向的展开。没有这个 ID 时，它直接挂载或卸载。

部分组件用 GPUI 元素包装器播放固定动画：[Spinner](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/spinner.rs) 循环旋转，[Popover](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/popover.rs) 播放入场。选择依据是组件需要持续追踪目标，还是播放固定流程；两者都可以属于样式层。

## 减弱动态效果与帧请求

GPUI 把偏好保存在 `App` 中；`cx.reduce_motion()` 读取，`cx.set_reduce_motion(...)` 可设置。`gpui_kit::init(cx)` 会初始化 Base 的系统偏好处理。Base 在初始化时读取 macOS 和 Windows 的设置；Linux 桌面 portal 的设置到达后会继续跟踪变化；其他目标不修改该标志。应用显式设置后，Base 会尊重应用的选择。若要在初始化后重新读取 macOS 或 Windows，调用 `gpui_kit::base::apply_system_reduce_motion(cx)`。

GPUI 元素动画采用前述静态端点。Base 的有限 transition、spring、keyframes、presence 和 sequence 会立即到达相应目标或最终状态，并停止请求运动帧。`MotionReveal` 只消费进度：直接使用时，应在减弱动态效果下传入端点值，或由遵守该偏好的采样器驱动。子元素测量高度发生变化时，它仍可能请求一帧。无限循环的活动状态也必须有可理解的静态表示。自定义元素若自行持有时钟，应检查 `cx.reduce_motion()`，并且只在仍有必要运动时请求帧；不要在 render 中无条件调用 `cx.notify()`、`window.refresh()` 或 `window.request_animation_frame()`。

动效应用于解释出现、关闭、展开和空间连续性。当透明度或 transform 已足以说明关系时，避免大范围 layout 动画。过渡过程中也要协调键盘焦点、命中区域和语义状态；绘制位置改变本身不会向辅助技术播报新状态。

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
| [GPUI Base Motion](../base/motion.md) | 运动中会变化的目标、卸载前的退出、关键帧和测量式展开 | Base 以稳定 key 保存每个通道，活动期间通过 [Window](./window) 请求帧；调用方决定视觉结果。 |
| GPUI Component 动效 | 外观需要跟随主题的样式化控件 | `cx.theme().motion_tokens()` 提供语义化时长、曲线、弹簧和距离；组件再与 GPUI 或 Base 组合。 |

语义状态归应用所有：对话框是否打开、当前选中哪个标签、滑块指向哪里。动画只负责呈现这些状态。无论在动画两端，还是关闭动效后，结果都必须清楚。

设计第一个动画时，先判断新事件是否可能在动画结束前改变目标。如果会，先用目标驱动的 Base `transition` 或 `spring`。如果一个已挂载元素只需从头到尾播放固定过程，则用 GPUI `with_animation`。用户连续点击时，这个区别尤其重要：播放时钟与可变目标的中断行为不同。

## 从选中状态开始做动画

在仓库根目录运行已有的 [Motion 示例](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/motion/mod.rs)：

```sh
cargo run -p gpui-base-examples --bin motion
```

示例初始打开 **Sliding time**。切换到 **Spring** 标签，交替点击 **Focus** 和 **Flow**。深色指示块会移向选中的半边；还没停下就切换选项，可以看到它转向。两个选项的文字始终可见，选中文字的颜色也会立即改变；动画帮助表现状态在空间上的连续性。

### 把目标留在视图状态中

`MotionExample` 保存 `spring_selected: bool`。每个选项的按钮把自己的值写入状态，并通知视图：

```rust
.on_click(move |_, _, cx| {
    _ = entity.update(cx, |this, cx| {
        this.spring_selected = selected;
        cx.notify();
    });
})
```

这个布尔值决定选中 **Focus**（`false`）还是 **Flow**（`true`）。它是持久的状态；视图不必保存指示块当前的像素位置。再次点击已选中的选项，目标不会改变。

### 每次 render 都采样弹簧

在 `spring_demo` 中，布尔值决定目标是 0 还是 120 像素。Base 的 `spring` 返回当前帧的值：

```rust
let x = spring(
    "selector-indicator",
    if self.spring_selected { 120. } else { 0. },
    Spring::new(Duration::from_millis(420)).with_damping(0.68),
    window,
    cx,
);
```

`"selector-indicator"` 是稳定的通道 ID。后续 render 因而能找到同一个弹簧；目标改变时，它保留当前位置和速度。首次 render 时，弹簧直接采用目标值；点击改变目标后才开始移动。运动期间 Base 会请求后续帧。指示块挂载期间，每次 render 都要继续采样这个通道；点击处理器中的 `cx.notify()` 用于报告状态变化，不需要 render 循环或计时器。

### 把采样值用于指示块

轨道是宽 240 像素的相对定位父元素。指示块把采样值用作左边距：

```rust
div()
    .relative()
    .w(px(240.))
    .h_10()
    .child(
        div()
            .absolute()
            .left(px(x))
            .w(px(119.))
            .h_full(),
    )
```

源码还为轨道、指示块和选项按钮设置了样式。0 和 120 两个目标让宽 119 像素的指示块分别位于对应选项下方。示例使用固定宽度，便于理解坐标计算；产品组件应使用自己的尺寸与主题策略。

### 跟踪一次点击直到静止

| 时刻 | 发生了什么 | 谁持有它 |
| --- | --- | --- |
| 点击 **Flow** | 处理器写入 `spring_selected = true` 并调用 `cx.notify()`。 | `MotionExample` 持有选中状态。 |
| 下一次 render | `spring_demo` 将新目标 `120.` 传给名为 `"selector-indicator"` 的通道。 | Base 保留该通道采样到的位置与速度。 |
| 移动期间 | `spring` 返回新的 `x`，并请求后续动画帧。指示块将 `x` 用于 `.left(px(x))`。 | GPUI 调度请求的帧；render 代码描述每次的结果。 |
| 静止时 | `spring` 返回目标值，不再请求下一帧运动。 | 视图仍持有 `spring_selected`，下一次点击还能改变目标。 |

在 `spring_demo` 中将 `Duration::from_millis(420)` 改为 `Duration::from_millis(700)`，重新执行同一运行命令，并在指示块停下前切换 **Focus** 与 **Flow**。这会改变弹簧的响应策略，不改变点击处理器或语义选中状态。观察后恢复为 `420`。再把 **Flow** 的弹簧目标从 `120.` 改为 `60.`：指示块会停在两个标签之间，选中文字仍正确变化。恢复 `120.`；目标值是呈现用的几何位置，`spring_selected` 才回答“当前选中了哪项”。

要观察减弱动态效果，可在示例 `run` 函数中的 `gpui_base::init(cx);` 后立即加上 `cx.set_reduce_motion(true);`，然后重新运行。选中项仍会变化，弹簧则直接到达目标，不再请求动画帧。观察后移除新增的代码。若启用动画后指示块没有移动，检查目标是否变化、通道 ID 是否稳定，以及指示块挂载期间是否继续采样 `spring`。其他运动属性应使用各自的 ID；共用通道会混淆保留值。

### 退出完成后再卸载元素

在同一示例中打开 **Presence**，点击 **Remove**。示例立即把 `present` 改为 `false`，但仍渲染提示框，直到透明度降为零。render 路径先采样 presence，再决定是否包含提示框：

```rust
let sample = Presence::new("presence-notice", self.present)
    .transition(Transition::new(Duration::from_millis(360)).easing(Easing::EaseInOut))
    .sample(window, cx);

// Keep the notice mounted during its exit transition.
if sample.should_render() {
    div().opacity(sample.progress).child("Background task")
} else {
    div()
}
```

此处摘出生命周期决策；[完整示例](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/motion/mod.rs)还负责布局和样式。如果改用 `if self.present` 判断是否渲染，点击 **Remove** 后的第一次更新就会卸载提示框，退出动画也无从绘制。在退出期间点击 **Insert**，可看到 `Presence` 从当前采样值反向。减弱动态效果下，它会直接到达相应端点。`present` 仍是逻辑状态；当操作已不可用时，视觉上还在退出的提示框不应继续提供活动控件。

## GPUI 元素动画

`Animation::new(duration)` 创建播放一次、线性变化的动画。`AnimationExt::with_animation(id, animation, animator)` 包装一个 `IntoElement`；回调收到元素及经过 easing 映射的进度值。GPUI 在布局阶段调用它，把返回元素的样式用于当前帧，并在结束前继续请求帧。回调可修改元素支持的属性，例如透明度或变换。包装器的播放规则可参见 [GPUI {{gpui_pre_version}} 动画源码](https://docs.rs/gpui-pre/{{gpui_pre_version}}/src/gpui/elements/animation.rs.html)。这里的 `gpui-pre` 仅是 GPUI 的发布与版本同步包名，并不是额外的应用层。

```rust
use std::time::Duration;
use gpui_kit::*;

let entering = div()
    .child("Saved")
    .with_animation(
        ("saved-notice", generation),
        Animation::new(Duration::from_millis(180)),
        |element, progress| element.opacity(progress),
    );
```

### 并排体验单次重播与弹簧改目标

暂时将 `examples/hello_world/src/main.rs` 替换为以下练习，再运行 `cargo run -p hello_world`。它复用现有示例 package 和同一个 `gpui-kit` 依赖：

```rust
use std::time::Duration;
use gpui_kit::*;
use gpui_kit::base::{Spring, spring};
use gpui_kit::component::button::*;
use gpui_kit::prelude::FluentBuilder as _;

struct MotionProbe {
    generation: usize,
    show_notice: bool,
    selected: bool,
}

impl Render for MotionProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let offset = spring(
            "motion-probe-indicator",
            if self.selected { 120. } else { 0. },
            Spring::new(Duration::from_millis(420)).with_damping(0.68),
            window,
            cx,
        );

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .when(self.show_notice, |parent| {
                parent.child(
                    div().child("Saved").with_animation(
                        ("motion-probe-notice", self.generation),
                        Animation::new(Duration::from_millis(900)),
                        |element, progress| element.opacity(progress),
                    ),
                )
            })
            .child(
                Button::new("replay").label("Replay notice").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.generation += 1;
                        this.show_notice = true;
                        cx.notify();
                    },
                )),
            )
            .child(
                Button::new("hide").label("Hide notice").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.show_notice = false;
                        cx.notify();
                    },
                )),
            )
            .child(
                div().relative().w(px(240.)).h_10().child(
                    div()
                        .absolute()
                        .left(px(offset))
                        .w(px(119.))
                        .h_full()
                        .bg(rgb(0x3366cc)),
                ),
            )
            .child(
                Button::new("retarget").label("Switch target").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.selected = !this.selected;
                        cx.notify();
                    },
                )),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| MotionProbe {
                generation: 0,
                show_notice: true,
                selected: false,
            })
        })
        .expect("failed to open window");
    });
}
```

**Saved** 提示先淡入一次，然后保持显示。淡入结束前点击 **Hide notice**：`show_notice = false` 会卸载包装元素，因此这次播放停止。点击 **Replay notice** 会重新挂载包装元素并再次淡入。蓝色条仍在移动时再次点击 **Switch target**：它会从当前采样位置改变方向，因为 Base 弹簧的通道 ID 稳定，变化的是 `selected` 对应的目标。弹簧的 420 ms 是响应尺度，不是固定的完成期限；它达到配置的容差后才停止。点击其中一个控件不会重新开始另一个动效。验证 ID 时，把 `("motion-probe-notice", self.generation)` 精确替换成 `"motion-probe-notice"`，保留点击处理。在提示保持挂载期间，后续点击 **Replay notice** 仍会更新 view 状态，但已完成的单次动画不会再次淡入。练习后恢复 tuple ID 和原始示例文件。

要在减弱动态效果下重做这个练习，在 `gpui_kit::init(cx);` 后立即加入 `cx.set_reduce_motion(true);`，重新运行。**Saved** 会以完全不透明的状态出现，不再淡入；弹簧直接采用选中项的目标，**Hide notice** 仍会移除提示。观察后移除临时增加的代码。

蓝色条只用于观察弹簧采样值；固定的 240 像素轨道和 120 像素目标属于这个示例的局部几何。实际应用还应在没有动效时清楚表达选中状态。要对照带文字标签和交互状态的完整选择器，运行现有 [Motion 示例](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/motion/mod.rs)并打开 **Spring** 标签页。

通过 `gpui_kit::*`（或显式导入 trait）让 `AnimationExt` 生效。ID 标识的是动画包装元素，而不是文字或视觉属性。同一位置、同一 ID 的包装元素再次渲染时会继续已有播放；每次渲染重新构造 `Animation::new(...)` **不会**重播。需要重新入场时，把应用维护的 generation 放入 ID。移除包装元素会结束它的生命周期。一次性动画结束后，只要同一包装元素仍挂载，就保持终值。

`with_easing(f)` 把归一化时间映射到动画进度。GPUI 提供 `ease_in_out`、`bounce` 等函数；自定义曲线必须返回有限数值。曲线允许超出 `0..1`，因此当样式属性的合法范围更窄时，应自行裁剪结果。`repeat()` 在本地循环；`repeat_synced()` 根据应用共享时钟循环，适合多个提示同步。`with_max_fps(rate)` 限制该动画的最高重绘频率；非正数和非有限值会被忽略。实际帧表现的测量方法见 [FPS](./fps)。`with_animations(id, animations, |element, step, progress| ...)` 播放固定步骤链，并提供当前步骤索引。

GPUI 还提供 `AnimationExt::with_spring(id, SpringAnimation<T>, animator)`，以弹簧驱动一个元素。稳定 ID 使目标改变时仍保留位置和速度。新挂载的弹簧默认从目标值开始，除非用 `SpringAnimation::from(...)` 指定起点。弹簧只作用于一个元素时可直接使用它；若一个独立 keyed 值要驱动组合中的多个部分，Base 的 `spring` 更合适。

GPUI 弹簧的 `SpringPlayback` 控制运行、暂停、停止、完成或取消。减弱动态效果会让**运行中**的弹簧直接到达目标；暂停和停止状态仍按各自的播放状态处理。目标与播放状态应由应用状态决定，不能把包装元素当成语义状态的唯一来源。

### 中断时会怎样

`with_animation` 按已播放时间推进，并不是追逐可变目标的 transition。在同一 ID 下改变回调捕获的终点，原播放时钟仍会继续；改 ID 则开始新播放。这两种操作都不会自动以当前画面值作为新起点。选择指示块需要在连续点击时平滑反向，应使用目标驱动的 spring 或 Base `transition`。

GPUI 的 `AnimationExt` 遵守 `App::reduce_motion()`：一次性动画显示终点，循环动画显示起点，两者都不再请求动画帧。加载状态仍需文字或其他静态提示；停住的旋转图标不足以说明当前正在加载。

为每个新动效逐项回答：哪项应用状态改变了？哪个元素或通道 ID 在多次 render 间保持稳定？首次 render 采样到什么值？结束前再次输入会怎样？何时停止请求帧？减弱动态效果后，界面还是否能被理解？这些问题通常比先调时长曲线更容易定位“动画没有执行”的原因。

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

[Base Motion 指南](../base/motion.md)列出完整签名、参数校验、示例与基准测试。这里的 `Transition` 不同于旧的 `gpui_kit::base::animation::EffectTransition`：后者包装 GPUI `with_animation`，直接应用预设的淡入、滑动、宽度和高度效果。新的目标驱动动效优先使用 `base::motion` primitive，再自行应用采样值。

## GPUI Component：语义化动效策略

样式化组件通过 `cx.theme().motion_tokens()` 共享策略。`MotionTokens` 包含 `duration_instant`、`duration_fast`、`duration_normal`、`duration_slow`；`easing_enter`、`easing_exit`、`easing_move`；`spring_control`、`spring_move`；以及 `distance_short`、`distance_medium`。默认值构成协调的尺度，并不意味着所有控件都必须动画。从当前主题读取 token，产品才能集中调整。

运行现有样式化控件：`cargo run -p gpui-component-story -- SwitchStory`。快速点击可用的开关两次：每次点击都改变 checked 状态，滑块会在运动途中改变方向。同页的禁用开关不会响应。打开 [`crates/component/src/switch.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/switch.rs)，沿着 `checked` 找到滑块的目标偏移，并沿着 `cx.theme().motion_tokens().spring_move` 找到 Base `spring` 调用。再到 [`crates/component/src/theme/motion.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/theme/motion.rs)，暂时将默认 `spring_move` 时长从 280 ms 改为 600 ms，重新运行 story：checked 状态相同，滑块收敛更慢。练习后恢复 280 ms。story 持有 checked 布尔值，组件将其映射成视觉目标，主题提供动效策略。

例如，[Switch](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/switch.rs) 源码 在 `(self.id.clone(), "thumb")` 通道上采样 Base spring，向选中或未选中时的滑块偏移移动，策略来自 `cx.theme().motion_tokens().spring_move`。[可调整大小手柄](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/resizable.rs) 源码 分别采样长度和透明度通道，使用 `duration_fast` 与 `easing_move`；指示块淡出时，细分隔线仍保留。[Collapsible](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/collapsible.rs) 通过 `.motion_id(id)` 启用测量式、可反向的展开。没有这个 ID 时，它直接挂载或卸载。

部分组件用 GPUI 元素包装器播放固定动画：[Spinner](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/spinner.rs) 循环旋转，[Popover](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/popover.rs) 播放入场。选择依据是组件需要持续追踪目标，还是播放固定流程；两者都可以属于样式层。

## 减弱动态效果与帧请求

GPUI 把偏好保存在 `App` 中；`cx.reduce_motion()` 读取，`cx.set_reduce_motion(...)` 可设置。`gpui_kit::init(cx)` 会初始化 Base 的系统偏好处理。Base 在初始化时读取 macOS 和 Windows 的设置；Linux 桌面 portal 的设置到达后会继续跟踪变化；其他目标不修改该标志。应用显式设置后，Base 会尊重应用的选择。若要在初始化后重新读取 macOS 或 Windows，调用 `gpui_kit::base::apply_system_reduce_motion(cx)`。

GPUI 元素动画采用前述静态端点。Base 的有限 transition、spring、keyframes、presence 和 sequence 会立即到达相应目标或最终状态，并停止请求运动帧。`MotionReveal` 只消费进度：直接使用时，应在减弱动态效果下传入端点值，或由遵守该偏好的采样器驱动。子元素测量高度发生变化时，它仍可能请求一帧。无限循环的活动状态也必须有可理解的静态表示。自定义元素若自行持有时钟，应检查 `cx.reduce_motion()`，并且只在仍有必要运动时请求帧；不要在 render 中无条件调用 `cx.notify()`、`window.refresh()` 或 `window.request_animation_frame()`。

动效应用于解释出现、关闭、展开和空间连续性。当透明度或 transform 已足以说明关系时，避免大范围 layout 动画。过渡过程中也要协调键盘焦点、命中区域和语义状态；绘制位置改变本身不会向辅助技术播报新状态。界面变化的语义处理见[无障碍指南](./accessibility)。

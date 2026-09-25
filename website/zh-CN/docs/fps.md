---
title: FPS Monitor
description: 读懂 gpui-fps 的 HUD —— MAX FPS 是什么、为什么是推导而非计数，以及每一行在测什么。
order: -15
---

# FPS Monitor

`gpui-fps` 在 [Window](./window) 上叠加一个性能 HUD：一个主读数、一条滚动的帧耗时曲线，以及本进程的
CPU、GPU 与内存。它只依赖 `gpui`，任何 GPUI 应用都能用。

```rs
use gpui_fps::fps_monitor;

fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .relative()
        .size_full()
        .child(self.content.clone())
        .when(self.show_fps, |this| this.child(fps_monitor(window, cx)))
}
```

父元素必须是 `relative()`——HUD 用绝对定位；是否显示由调用方自己决定。

## 120 Hz 是帧预算，不是刷新承诺

“120 FPS”通常是 **120 Hz 屏幕上的目标**的简称：一个刷新周期是 `1 / 120 s`，
约 **8.33 ms**。它本身既不说明空闲应用多久请求一次画面，也不保证某个界面能在
每次预算内完成并呈现。60 Hz 屏幕的单次刷新约为 16.67 ms。可变刷新率和平台
调度还会影响实际情况；这两个数字都不是应用的实测 FPS。

GPUI 响应失效：输入或状态变化可将 View 标为 dirty，窗口随后绘制并呈现新场景；
多次变化可以合并。动画在进行时可以请求后续帧，结束后应停止请求动画帧。仅仅
因为屏幕是 120 Hz，GPUI 的渲染管线并不要求应用每秒重建 120 次元素树。
平台的帧回调仍可能运行；部分后端也可能在不重建元素的情况下呈现已有场景。
另外，HUD 自身的定时器、输入设备、平台合成器或应用代码，仍可能让看似空闲的
窗口产生活动。应在目标平台测量空闲时的 CPU/GPU 使用量，而不是从屏幕刷新率
推断出零工作量或持续完整重绘。

<figure class="frame-timeline" data-frame-timeline>
  <div class="frame-timeline__header"><strong>何时需要新的绘制过程</strong><button type="button" data-frame-timeline-toggle aria-pressed="false">暂停动画</button></div>
  <div class="frame-timeline__tracks" role="img" aria-label="概念性时间线：空闲时不必重建元素树；一次输入事件触发一次绘制；动画进行时连续请求帧。">
    <div class="frame-timeline__phase" data-phase="idle"><span>空闲</span><div class="frame-timeline__ticks"></div><small>无需重建元素树</small></div>
    <div class="frame-timeline__phase" data-phase="event"><span>输入</span><div class="frame-timeline__ticks"><i></i></div><small>事件 → 失效 → 绘制</small></div>
    <div class="frame-timeline__phase" data-phase="animation"><span>动画</span><div class="frame-timeline__ticks"><i></i><i></i><i></i><i></i></div><small>运动期间按需请求帧</small></div>
  </div>
  <figcaption>这是概念性的帧请求示意，不是实测 FPS，也不限制平台呈现已有场景的行为。</figcaption>
</figure>

要声称**持续显示 120 FPS**，需要在目标硬件和窗口尺寸下，用 release 构建测量
有代表性的*整个窗口*；包含真实数据、滚动或动画，并报告预热后的帧耗时分布与
present 间隔。单个小组件的 `render` 少于 8.33 ms，只测到了单帧的一部分。
layout、prepaint、paint、提交、GPU 工作、合成器和调度都可能用掉其余预算。
平均值很低，也可能被较慢的 `P95` 帧打断，产生可见卡顿。

GPUI 的工作跨越 CPU 与 GPU：应用状态、元素构造、布局和绘制准备在 CPU 上完成；
平台渲染器与合成器在支持时使用 GPU。文字、场景复杂度、缓存行为、图形后端和
硬件共同决定某项负载的瓶颈。“CPU-bound”或“GPU-bound”应是对具体负载的
分析结果，而不是所有 GPUI 应用的固定属性。

## Immediate、Retained 和 Hybrid 描述的是不同层次

这些词回答的是不同问题，仅用一个标签容易造成误解：

| 层次 | GPUI 的行为 | 跨过程保留的内容 |
| --- | --- | --- |
| View 状态 | [`Entity<T>`](./entity) 跨更新与帧持有应用状态。 | 仍有所有者时，Entity 及其订阅、Task 和子 handle。 |
| 界面描述 | 需要[渲染过程](./render)时，`Render` 或 `RenderOnce` 根据当前状态构造元素，形式上接近 immediate 风格的声明。 | 元素描述属于本次过程；`RenderOnce` **不是**每显示帧只运行一次。 |
| 复用与绘制 | GPUI 可复用符合条件的[缓存 View](./view-cache) 与带 key 的元素状态，并提交窗口绘制工作。 | 缓存及平台场景资源可比单次元素描述存活更久。 |

“Retained”适用于 Entity 状态与部分缓存，“immediate 风格”描述当前元素的
构造方式，“hybrid”则概括了两者；[Zed 的 GPUI 概览](https://github.com/zed-industries/zed/blob/bcf6582ce3500df93a8a39366640173e6786cea6/crates/gpui/README.md#the-big-picture)也采用这个名称。这些术语都不能决定空闲时的 present 频率，
也不能证明性能数字。[Render](./render)、[Element](./element) 与
[View Cache](./view-cache) 从代码层面解释所有权与失效边界。

`gpui-fps` 应与应用使用同一套 GPUI 依赖。每个窗口最多渲染一次
`fps_monitor(window, cx)`；重复调用会复用同一个 monitor，导致 HUD 画两遍。
仓库中的可运行示例用 `cargo run --release -p fps_monitor` 启动。只有比较 Debug
构建时才用 `cargo run -p fps_monitor`：未优化的框架代码会显著改变测得的帧耗时。
开发配置参见[安装](./installation#提升开发模式运行性能)。

## 从状态更新到画面显示

1. 状态改变后的 `cx.notify()`、动画请求或窗口刷新使窗口失效。多次失效可能合并为一次绘制。
2. GPUI 执行 `Window::draw`，为脏 view 构建元素，完成 layout、prepaint 和 paint。
   profiler 记录该窗口的 `draw_start`、`draw_end` 和失效次数。`FRAME` **只取**
   `draw_end - draw_start`。
3. GPUI 在单独的 present 阶段把场景提交给平台。其 `present_end` 时间戳用于
   `FPS` 和 `INTERVAL`。

因此，`FRAME` 为 5 ms 只说明 GPUI 的 draw 在 5 ms 内完成；它不表示请求只等待了
5 ms、平台提交耗时 5 ms，也不表示 GPU 和合成器在 5 ms 内把画面显示出来。
profiler 还提供 `dirty_to_draw_duration()` 和 `PresentTiming` 供自行埋点，
但它们与 HUD 的 `FRAME` 是不同的测量。参见 GPUI 的
[帧计时定义](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/profiler/struct.FrameTiming.html)和
[sampler 源码](https://github.com/longbridge/gpui-kit/blob/main/crates/fps/src/sampler.rs)。

## 主读数

那个大数字回答两个问题之一，`MAX` 标记说明当前是哪一个。**右键**切换，**左键**折叠成一个小标签。

| | 读的是 | 含义 |
| --- | --- | --- |
| `MAX FPS`（默认） | `1 / FRAME` 平均值；能获知显示器刷新率时以其为上限 | 根据取样负载估计的 draw 吞吐量，不含呈现和 GPU 完成时间 |
| `FPS` | 根据近期 present 时间戳推算的速率 | 样本足够时观察到的 present 节奏 |

这是两个不同的问题。例如，即使应用没有可见变化，HUD 也每秒请求两次读数更新。
此时窗口观察到的 `FPS` 可以很低，而 `MAX FPS` 仍很高；任何一个数字都不能
单独证明空闲时持续重绘，或复杂界面的持续性能。

### 为什么 MAX 是推导出来的，而不是数出来的

要让帧计数器读出"这个 UI 能跑多快"，最直接的做法是像游戏里的帧数器那样不停地要帧。但在
这里这件事并不免费：标脏任意一个 view 排的是一次**窗口**绘制，GPUI 会重新 render 该窗口中
除 [`Entity::cached`] 边界（见 [View Cache](./view-cache)）后面之外的所有 view —— 于是 HUD 每要一帧，代价就是应用的一次
layout 与 [paint](./paint) 工作，而下面那行 CPU 也会包含 HUD 自己制造的开销。

测得的 draw 耗时可用来估计当前取样窗口在 CPU 侧的余量。其倒数就是 `MAX FPS`，
但这不是对持续动画或完整呈现管线的测量。HUD 不会为了计算它而持续请求帧；
下文说明每半秒更新读数的时钟。

### 为什么是「问平台」而不是「测出来」

3ms 的一帧会读成 333，没有任何面板能显示这个数。数 present 的时候这个上界是免费的（帧走 vsync
交给合成器），而从帧耗时推导出来的数字没有这个天花板，所以必须显式地夹。

**它推不出来。** present 之间的间隔是面板周期的整数倍，所以只能给出刷新率的**下界**，永远给不出
上界：41.7ms 在 144Hz 屏上是 6 个刷新周期，在 24Hz 屏上是 1 个，时序本身无法区分。试过的每一种
估计法都在真实窗口上读错过——取最短间隔得到 169、取最稠密的一组得到 149、隔帧绘制的窗口得到 75、
而一个自身定时器每 41.7ms 触发一次的应用得到 24。

所以改成向平台索取。GPUI 通过 `DisplayId` 把平台自己的显示器句柄透了出来，HUD 从那里接手：

- **macOS** —— 用 `CGDirectDisplayID` 调 `CGDisplayCopyDisplayMode`。内置屏报告「没有固定速率」，
  在 ProMotion 上这是实情，按「不夹」处理。
- **Windows** —— 用显示器的设备名调 `EnumDisplaySettingsW`。
- **Wayland** —— 另开一个连接重新枚举 outputs，再按 GPUI 从名字派生的标识与它的 display 对上；
  因为对象 id 是每连接独立的，跨连接没有意义。
- **X11 及其它** —— 没有查询，也就不夹。

窗口移动到另一块显示器时会重新索取，其余时候不会。**没人能给出答案时保持不夹**，而不是按猜测夹：
夹到真值以下会把读者想看的数字藏起来。

## 各行含义

| 行 | 测的是 |
| --- | --- |
| `INTERVAL` | present 之间的平均间隔，也就是平台自带 overlay 里的 frame interval，`FPS` 的倒数。它与 `MAX` 差得越大，说明窗口越空闲，而不是越慢。 |
| `FRAME` | `Window::draw` 的平均耗时，按帧预算着色。觉得卡的时候看这一行。 |
| `P95` | 同一批帧的慢尾，着色规则相同。 |
| `DROP` | 超出预算的帧占比。 |
| `INV` | 合并进同一帧的失效次数。明显大于 1 说明窗口被要求重绘的频率超过了它能画的频率。 |
| `GPU` | 平台能提供本进程 GPU 计数器时显示其使用率；否则整行不显示。 |
| `CPU` | 本进程，采用 `top` / 活动监视器的口径：100 表示占满一个核，占用一核半约为 150。 |
| `MEM` | 归于本进程的内存：macOS 用 physical footprint，Windows 用 private usage，Linux 用常驻匿名内存；必要时回退到 RSS。各平台不是同一种计数器。 |

`FRAME`、`P95`、`DROP` 按帧预算着色：平台报得出上面那个刷新率时，预算就是窗口所在面板的一次刷新；
报不出时是一个 60Hz 帧。`frame_budget()` 可以钉一个自己的预算，之后面板不再替换它。

`FRAME`、`P95`、`DROP` 和 `INV` 默认使用最近 **120 个保留的 draw 样本**，
而不是固定时长；`capacity()` 可以调整数量。`P95` 是这批帧中最接近第 95 百分位的
实际帧耗时，`DROP` 则是 draw 耗时**严格大于**预算的比例。曲线画的也是这些 draw
耗时，最新的一帧在右侧。`FPS` 和 `INTERVAL` 使用最近一秒内的 present 时间戳。
屏幕数字每 500 ms 更新一次；CPU、GPU、内存在后台取样，显示最近三秒的平均值。
Web 上不提供资源采样。

这些统计窗口可能描述不同的时刻。一次昂贵操作后，曲线可能仍保留尖峰；连续多帧缓慢时，
`P95` 也可能保持偏高，而 `FPS` 已随窗口空闲下降。主读数不按预算着色，因为实际 FPS 低也可能只是应用
不再请求帧。

## 复现并定位卡顿

1. 在仓库中运行 `cargo run --release -p fps_monitor`。待窗口与 HUD 稳定后，
   记录显示器、窗口大小、构建配置，以及 `FRAME`、`P95`、`DROP`、`INV` 和资源行的基线。
   示例中的 `+ load`、`− load` 会改变曲线数量；跨次比较时保持数量和窗口大小一致。
   示例场景调用 `window.request_animation_frame()`，可提供持续绘制负载。
2. 加大负载，比较 `FRAME` 与显示器预算。60 Hz 的一次刷新约为 16.7 ms，
   120 Hz 约为 8.3 ms。`FRAME` 低但 `P95` 或 `DROP` 升高时，寻找间歇性工作；
   draw 耗时低但 `INV` 升高时，检查重复更新或动画请求；只有 `INTERVAL` 增大时，
   先确认窗口是否空闲、被遮挡，或在等待 present。
3. 预热后重复同一操作。构建 view、layout 或 paint 提交成本高时采集 CPU profile；
   `FRAME` 低但画面仍卡顿时使用平台 GPU／合成器工具。缩小负载范围，每次只改一个原因，
   再在相同构建配置和条件下对比。HUD 能指出症状，但不能把耗时归因到具体 view 或 GPU 命令。

若要记录应用特定事件，GPUI 的 `profiler` feature 提供
`FrameTimingCollector::new()` 与 `collect_unseen()`。它们返回
`FrameEvent::Draw` 和 `FrameEvent::Present`；按 `window_id` 筛选，并在采样期间保留
collector。trace 是进程级的，可用 `gpui::profiler::set_trace_enabled(true)` 打开；
关闭会清空缓冲区。HUD 显示期间会管理这个开关。这些原始记录可把帧与应用事件关联，
包括首次失效时间 `dirty_at` 和 present 提交，但也不直接测 GPU 完成或像素何时出现在屏幕上。
参见 GPUI 的
[collector API](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/profiler/struct.FrameTimingCollector.html)和
[GPUI Kit monitor 源码](https://github.com/longbridge/gpui-kit/blob/main/crates/fps/src/monitor.rs)。

## 最初的几帧不计入统计

窗口最初的几帧是最贵的——着色器、字形图集、图标，所有缓存都是冷的——它们并不代表应用的运行成本。
其中一帧可能是 100ms，而预算只有 16ms；只看过八帧的 HUD 会把它算成窗口十二分之一的工作量，用琥珀色
标出来，而此时读它的人什么都还没做。

所以 sampler 会丢掉两部分：HUD 挂载之前 GPUI 记录的全部帧（要么是别人的历史，要么是冷启动），
以及挂载之后的最初几帧。刚打开的窗口，默认读数就应该是健康的。

## HUD 自己的开销

每 500ms 一帧。它不驱动帧循环，但需要一个时钟——否则在一个已经停止绘制的窗口里没有任何东西能唤醒它，
读数会冻结在应用最后一次绘制的值上。这个时钟同时承担 CPU、GPU 与内存的采样。

这些帧不计入读数。对 GPUI 来说，时钟的 `notify` 和任何一次失效没有区别，都会换来一次整窗绘制；
如果留在读数里，就是每 500ms 一帧冷帧被当成应用的 `FRAME` 和 `MAX`。所以时钟每次触发都会先告知采样器，
采样器把回应它的那次绘制排除在外——除非应用自己也请求了这一帧，那这份工作本来就是应用要的，成本照算。

隐藏起来的 HUD 没有任何开销。连续两个 tick（一秒）没有被渲染，时钟就停下，资源探针随之停止，
frame trace 也会放掉（除非别处还持有）。下一次渲染再从一个空的采样器重新开始：trace 缓冲区随开关被清空了，
中间那些帧也本来就不归谁报告。

[`Entity::cached`]: https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.Entity.html#method.cached

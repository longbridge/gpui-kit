---
title: Performance
description: 当帧率不再是变量之后，JavaScript 真正的开销——失效频率乘以描述规模、每个 View 各自的 Snapshot，以及 FPS 分辨不出来的那两类问题。
order: 14
maturity: [preview]
---

# Performance

一个 View 的 `render()` 不是每帧都跑。它产出一份描述并存成 Snapshot，之后的每一帧都由 Rust 从这份 Snapshot 画出来，不再进入 JavaScript——[首页那一节](./index.md#性能-脚本不在每一帧里)讲的就是这件事。

一旦重绘不进入 JavaScript，剩下要算的就只有两样：

```text
JavaScript cost = View invalidation frequency × cost to describe the View
```

两样都不由屏幕刷新率单独决定。窗口即使以 120 Hz 重绘，也不会仅因为呈现的帧更多，就多执行这个 View 的 JavaScript；没有失效的 View 不会执行脚本 `render`。左边取决于你何时调用 `cx.notify()`，也包括主题切换等其他失效原因；右边是一次调用背后压了多少界面。屏幕刷新率只是调度上限或目标，并不表示空闲窗口会持续重绘。

两样也都在你手里：左边是你在哪里调用 `cx.notify()`，右边是一次 `notify` 背后压了多少界面。这一页剩下的内容讲的就是这两件事，以及出问题时怎么分辨是哪一个。

## 每个 View 都有自己的 Snapshot

GPUI Shell 给每一个 JavaScript View 一份属于它自己的 Snapshot：这个 View 的 `render` 产出的那份描述，保存在 Rust 一侧。

**只要 View 本身没有变化，它的 Snapshot 就一直被复用。** 当 GPUI 确实请求下一帧时，可以使用这份 Snapshot，而不执行这个 View 的 JavaScript `render`。这本身不说明空闲窗口多久呈现一帧，也不说明该帧其余 CPU 与 GPU 工作的成本。

```text
View changed    ──▶  render()  ──▶  new Snapshot       ──▶  frame
View unchanged  ─────────────────▶  existing Snapshot  ──▶  frame
```

Snapshot 是按 View 存的，不是按窗口存的。一个窗口里有一百个 View，就有一百份 Snapshot，各自独立失效：

| 发生了什么 | 会执行什么 |
| --- | --- |
| `Watchlist` 调用 `cx.notify()` | `Watchlist.render`，其余什么都不跑 |
| 父 View 调用 `cx.notify()` | 父 View 的 `render`。每个子 View 用自己的 Snapshot 回答这一帧 |
| `this.chart.set_props({ symbol })` | 那个子 View 的 `update` 与 `render`。父 View 不重建 |
| 子 View 的子 View 调用 `cx.notify()` | 那个子 View 的 `render`。失效不会向上传播 |
| 主题切换 | 每一个 View——因为 Snapshot 里烘进了它构建时的颜色 |

<img class="architecture-light" src="/shell-view-invalidation-light.svg" alt="一个窗口，画成互相嵌套的 View：侧栏、一块装着四行（每行本身也是 View）的自选清单、图表，以及装着两个子 View 的详情面板。三个阶段循环。价格跳动时，只有 MSFT 那一行被标为「render 在执行」，其余每个 View 都从自己已有的 Snapshot 画出来。列表重排时，自选清单本身执行，而它的四行不执行——父 View 记录的是每个子 View 的一个句柄，不是子 View 的描述。主题切换时所有 View 同时执行，因为 Snapshot 里烘进了它构建时的颜色。">
<img class="architecture-dark" src="/shell-view-invalidation-dark.svg" alt="一个窗口，画成互相嵌套的 View：侧栏、一块装着四行（每行本身也是 View）的自选清单、图表，以及装着两个子 View 的详情面板。三个阶段循环。价格跳动时，只有 MSFT 那一行被标为「render 在执行」，其余每个 View 都从自己已有的 Snapshot 画出来。列表重排时，自选清单本身执行，而它的四行不执行——父 View 记录的是每个子 View 的一个句柄，不是子 View 的描述。主题切换时所有 View 同时执行，因为 Snapshot 里烘进了它构建时的颜色。">

## 把大 View 拆成小 View

View 是整体重建的，内部没有局部重建：如果一个 View 的描述有四百个节点，那么任何一点变化都会把这四百个节点全部重建一遍，无论变化多小。

这就是大 View 贵的原因。它画的所有东西共用一份 Snapshot，于是变化最频繁的那部分数据，会连带让那些从不变化的部分一起失效。在一个行情终端里，一个价格动一下，图表、侧栏、盘口也会被重新描述一遍——不是因为它们变了，而是因为它们和价格在同一个 View 里。

拆分就是解法。把各自独立变化的部分用 `cx.new` 拆成各自的 View，一次变化就只会落到一份 Snapshot 上，而不是全部：

```js
import { View } from "gpui-kit";

export default class Terminal extends View {
  init(props, cx) {
    this.sidebar = cx.new(Sidebar);
    this.watchlist = cx.new(Watchlist, { symbols: props.symbols });
    this.chart = cx.new(PriceChart, { symbol: props.symbols[0] });
    this.detail = cx.new(Detail, { symbol: props.symbols[0] });
  }

  render() {
    return h_flex()
      .child(this.sidebar)
      .child(this.watchlist)
      .child(v_flex().child(this.chart).child(this.detail));
  }
}
```

在本页测量的那块 40 行看板上，描述整块面板要 **0.315 ms**，描述其中一行只要 **0.012 ms**——361 个节点对 9 个。

嵌套本身的开销和这个差距比几乎可以忽略：父 View 为每个子 View 记录的是一个句柄，不是子 View 的描述。所以界面复杂本身不是性能问题， View 太大才是。

而且「为了性能而拆」指的是拆成 **View**，不是拆成多个插件、多个应用或多个进程。需要第二个应用，是因为你想要第二份**授权**，那是 [Capabilities](./capabilities.md) 的事，而不是因为你想要第二份缓存。

## 只为用户看得见的变化 notify

`cx.notify()` 就是这里全部的依赖系统，而它只表达一件事：**我的描述过期了。** 它不是事件通知，把它当事件通知用，是让 JavaScript 变贵的最常见方式。

行情回调是典型场景：

```js
onQuote(quote, cx) {
  this.quotes.set(quote.symbol, quote);
  cx.notify();                  // Notifies even when this symbol is not visible.
}
```

如果这个 View 从两千只订阅里只画二十只，这句 `notify` 会为它根本没画的标的的每一跳，付一次完整的面板描述。解法是一个条件，不是更快的 render：

```js
onQuote(quote, cx) {
  this.quotes.set(quote.symbol, quote);
  if (this.visible.has(quote.symbol)) cx.notify();
}
```

同一个想法推出三条规则：

- **让变化的那个 View 失效。** 只属于某个子 View 的状态，就应该放在那个子 View 上、在那里 notify，而不是放在挂载它的父 View 上。
- **下一帧之前的多次通知可以共用一次脚本 render。** 见下；各个回调及其其他工作仍会执行，所以只为 View 真正显示的变化发通知。
- **在 Host 一侧，`cx.notify()` 与 `ScriptView::refresh` 是两个不同的请求。** 单纯的 `notify` 只是重绘已有的描述。如果 Rust 改的是脚本通过 [HostModule](./host-module.md) 读到的状态，那描述已经陈旧，只有 `refresh` 能说明这一点。见 [Hosting](./hosting.md#host-状态变了-怎么刷新-view)。

### notify 到底做了什么，谁在合并它

`cx.notify()` 不重建任何东西。它只是在这个 View 上置一个标志，表示「我的描述可能过期了」，然后请求 GPUI 绘制。重建发生在之后的那一帧里，而且只在标志仍然置位时才发生。

所以两帧之间的所有 notify 都会合并成一次 `render`——无论它们来自三个事件回调、一个循环里的 task，还是 Host：

```text
notify  notify  notify  ──▶  one frame  ──▶  one render()
```

一个标志置三次等于置一次。什么都没有被丢掉：三个回调都执行了，状态也都改了；它们共享的只是随后那一次重建。

**合并让每个 View 每帧最多执行一次脚本 render。** 例如，某窗口在实测的一秒里呈现了 120 帧，同时行情送来 1,000 次通知，那么该 View 在这些帧中最多执行 120 次 render。这不会让 1,000 个回调变成零成本，也不能证明实际帧率就是 120 FPS，更不表示屏幕每次刷新都会产生一帧。

运行时没有另外可调的脚本 render 节流。GPUI 的调度会把这些请求合并到它处理的下一帧中；这一帧何时呈现，仍取决于窗口、平台与负载。

### 这份缓存占多少内存

一个 View 持有**两份**描述：它已发布的那份，以及被它替换掉的那份。留着第二份，是因为事件仍可能派发到一个已经被取代的帧上，而那一帧需要的回调属于那份较旧的描述。

没有第三份。发布新描述时最旧的那份会被丢弃，丢弃它同时会退役随它注册的那些回调。所以上限就是每个存活的 View 两份描述，而且不随时间累积：一个重渲染过一百万次的 View，持有的东西和一个只渲染过两次的 View 完全一样。关掉一个面板，它的 View 就没了，两份描述一起走。

这也是「该拆大 View 而不必怕拆」的另一个理由：一百个小 View 持有的是一百对小描述，加起来仍然只是把这个界面描述了两遍，而不是一百遍。

## 帧率与呈现延迟是两类问题

一个运行中的界面可能出两种问题，而只有一种会体现在 FPS 上：

```text
Frame rate          Is motion smooth?
State → display     How long until the user sees a state change?
```

漏掉一次 `cx.notify()`，数据可能已经过期，却没有出现掉帧。如果其他活动继续触发画面，GPUI 可以沿用上一份描述，帧率读数看起来也可能正常；如果没有请求新帧，窗口则可能保持静止。直到另一次失效刷新了这个 View，过期数据才会变化。仅靠帧率指标无法发现漏掉的通知。

| 症状 | 哪个数字不对 | 常见原因 |
| --- | --- | --- |
| 脚本状态未变化，但窗口重绘时仍卡顿 | 帧时间 / 帧率 | 布局、绘制或虚拟列表按行执行的工作可能过大；见[那次实测](./engine.md#那次实测) |
| 行情在跑的时候窗口卡 | 帧率**和**失效频率 | 某个 View 重建得太频繁、太大，或两者都有 |
| 画面很流畅，但数据慢半拍 | 呈现延迟 | 某次 `notify` 被漏掉、被压在 `await` 之后，或该用 `refresh` 的地方用了 Host 侧的 `cx.notify()` |

这两件事要分开诊断。FPS 从没掉过，并不能证明失效逻辑是对的。

## 怎么读那几个计数器

运行时把这两类事件分开计数，Host 用 `runtime.read_metrics()` 读取——接口本身以及“留一个基线再相减”得到每秒速率的用法，见[观察它花了多少](./hosting.md#观察它花了多少)。

| 读数 | 它回答什么 |
| --- | --- |
| `script_renders()` | JavaScript 执行了多少次。跟着 `cx.notify()`、hot-reload 与主题切换走，永远不跟帧走 |
| `materializations()` | 脏 Snapshot 被转成元素多少次；干净的窗口帧可复用 GPUI 子树 |
| `mean_script_render()` | 一次描述要花多少，包含其中的 Host 调用 |
| `mean_native()` | 其中有多少是在 HostModule 函数里，而不是在描述界面 |
| `slowest_script_render()` | 这一段里最慢的那一次构建 |
| `frame_script_calls()` | 从帧路径进入 VM 的次数——只有[虚拟列表](./elements.md)的 item 渲染器与 [Dock](./dock.md) 的 chrome 回调会计入 |
| `structure_repeat_rate()` | 在有上一份描述可比的重建里，有多大比例产出了相同的**结构**——见下 |

一份读数的形状说明什么：

- **每秒 `script_renders` 远高于数据实际变化的频率**——`notify` 正在为用户看不见的东西触发。加条件。
- **`script_renders` 正常，但 `mean_script_render` 高**——View 太大。把它拆开。
- **`mean_native` 占了 `mean_script_render` 的大部分**——成本在描述过程中调用的那些 Host 函数上，而不在描述本身。在 `render` 之前把它们一次性读进字段，不要按节点调用。
- **`slowest_script_render` 远高于均值**——某一次构建付了其余各次没付的东西：首次渲染物化的一份集合，或一个很少走到、却描述得多得多的分支。如果是均值整体在漂，那是系统负载，不是这个。

## Snapshot 缓存止步于哪里

Snapshot 消除的是**没有变化**的成本，它不消除**变化很小**的成本。

一份 Snapshot 把结构和取值一起存着：

```text
StockRow
├── Symbol("AAPL")
├── Price("230.42")
└── Change("+1.42%")
```

当价格变成 `230.51`，结构完全一样，只有一个叶子不同——但要表达这一点，唯一的办法就是产出一份新的描述，于是整个 View 被重新描述一遍：每个 `div()`、每个 `.gap()`、每个 `.bg()`、每一次进入 Rust 的跨越。这就是 dirty render 那条路径，行情一快，跑的就是它。

<img class="architecture-light" src="/shell-change-cost-light.svg" alt="三条泳道，长条按同一比例绘制。这个 View 读到的东西没变：没有长条，不执行任何 JavaScript，这一帧从已有的 Snapshot 画出来。取值变了，也就是今天的情形：无论变化多小，整块面板都被重新描述一遍，0.315 毫秒。同样的变化，若这一行本身是一个留存的 View：0.012 毫秒，约为二十六分之一，因为描述的是 9 个节点而不是 361 个。">
<img class="architecture-dark" src="/shell-change-cost-dark.svg" alt="三条泳道，长条按同一比例绘制。这个 View 读到的东西没变：没有长条，不执行任何 JavaScript，这一帧从已有的 Snapshot 画出来。取值变了，也就是今天的情形：无论变化多小，整块面板都被重新描述一遍，0.315 毫秒。同样的变化，若这一行本身是一个留存的 View：0.012 毫秒，约为二十六分之一，因为描述的是 9 个节点而不是 361 个。">

可用的杠杆就是本页开头那一个：**把必须重建的 View 缩小。** 在上面那块看板上，描述整块面板 0.315 ms，描述其中一行 0.012 ms——361 个节点对 9 个。把这一行放进它自己的 View，就是把前一个数字变成后一个，而这是今天就能做的。

`structure_repeats()` 与 `structure_changes()` 是用来核对这条线划得对不对的。它们统计一次重建产出的**结构**与被替换那份是否相同——只有其中的取值不同。如果某块面板报出来的比例很低，这件事本身就值得知道：你以为只有一个数字在变，实际上有东西在改变结构。

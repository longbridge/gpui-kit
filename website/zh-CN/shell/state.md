---
title: 状态与 View
description: View、init 与 render、cx.notify()、留存的输入状态，以及异步工作。
order: 6
---

# State and Views

View 是这个运行时里唯一有身份、能跨帧存活、并且由 GPUI 拥有的东西。其余一切——元素、回调、传给某次调用的 `cx`——都属于产生它的那一次调用。

## 定义 View

```js
import { View } from "gpui-kit";

export default class Counter extends View {
  init(props) {
    this.count = props?.start ?? 0;
  }

  render(cx) {
    return v_flex().child(`${this.count}`);
  }
}
```

`init` 在 View 创建时执行一次。跨帧存活的状态在这里建立——普通字段，以及 View 需要的任何[留存实体](#留存状态)。

`render` **返回一个元素、留存的 `Entity` 或字符串**，并且是在 View 被置为失效时执行，而不是每帧执行——见 [`render` 什么时候执行](#render-什么时候执行)。返回其它东西会立刻失败：

```text
render(cx) must return an element, an Entity, or a string
```

`main.js` 必须 `export default` 一个 View 类。 Host 构造一个实例并把它挂载为窗口的根 View；default 导出不是类的模块会被拒绝，并说明原因。

永远不要把元素存在实例上。见 [Elements](./elements.md#元素是一次性的)。

## `cx.notify()`

没有任何东西会自动重绘。这里没有 signal、没有 observable，也没有自动依赖追踪。改完状态，然后请求重新渲染：

```js
add(cx) {
  this.items = [...this.items, { id: this.nextId, caption, done: false }];
  this.nextId += 1;
  cx.notify();
}
```

这与整个前端生态的默认假设正好相反，所以有必要直说：**这里没有 `useState`，也没有依赖数组。** 运行时不加自动追踪，有三个理由。

GPUI 本身就是显式 `notify` 的模型，两套响应式心智模型放进同一个应用会互相干扰而不是彼此配合。自动追踪意味着要把每个 View 实例包进 `Proxy`，这是渲染路径上一笔长期开销——而 QuickJS 没有 JIT 来摊薄它。而漏写 `notify` 的症状是确定的：界面不更新。找出这种问题，远比排查一个触发过多的自动系统便宜。

一次事件回调内的多次 `notify` 会合并为一次重绘——也合并为一次 `render`。

## `render` 什么时候执行

`render` **不是**每帧执行一次。GPUI 会因为你的应用完全不知情的原因重绘——指针划过一个按钮、文本光标闪烁、列表滚动、动画推进——这些都不构成执行 JavaScript 的理由。

所以一次 `render` 调用描述的不是*这一帧*。它把界面描述一次，写进运行时保留的一份 Snapshot：

```text
cx.notify()  ──▶  render()  ──▶  Snapshot  ──┬──▶  帧
                                             ├──▶  帧
                                             └──▶  帧  …
```

Snapshot 只在有东西让它失效时才重建：

- 事件回调或异步任务里的 `cx.notify()`
- [hot-reload](./getting-started.md) 替换了脚本
- 主题切换——因为 `bg(cx.theme().colors.surface)` 在 `render` 执行时记录真实颜色，已经烘进了 Snapshot
- Host 调用 `ScriptView::refresh`——Rust 用它表示“我改了脚本会读到的状态”（通过 [HostModule](./host-module.md)）。Host 侧单纯的 `cx.notify()` 只是重绘，不会跑脚本：这是两个不同的请求

其余情况都在 Rust 里复用你已经产出的那份描述，不执行任何 JavaScript。

三条值得记住的推论：

**你的 `render` 成本跟着用户走，不跟着帧率走。** 一个每秒变化十次的 View，成本就是每秒十次渲染，无论窗口是 60 FPS 还是 120 FPS 在重绘。描述一个大面板之所以负担得起，正是因为它不会为了没有变化的内容被重复描述六十次。

**hover、focus 与 active 样式永远不回调脚本。** `.hover(s => s.opacity(0.8))` 在构建 Snapshot 时就被解析成原生样式描述，之后由 GPUI 自己套用。指针在界面上移动不会执行任何 JavaScript。[`Input`](#留存状态) 的光标与选区同理。

**一次失败的 `render` 不会毁掉界面。** Snapshot 只在 `render` 成功返回后才发布，所以抛异常的脚本会让上一份描述——以及随它注册的那些回调——原封不动地留着。失败以横幅的形式**盖在**仍然可用的界面之上，说明当前画面比最新版本旧了一版，并把详情交出去供粘贴；你的滚动位置和焦点都还在。首次渲染就失败的 View 没有可保留的东西，会拿到整屏的错误界面。两种情况下，在有东西再次让 View 失效之前，运行时都不会重跑那次失败的 `render`。

## ScopePhase

每一次从 Rust 进入脚本的调用都会开启一个带 **phase** 的作用域，phase 决定这次调用的 `cx` 能做什么。

| `ScopePhase` | 时机 | 允许 | 不允许 |
| --- | --- | --- | --- |
| `render` | 构建元素树 | 读状态、构建元素、注册回调 | `notify`、打开浮层、创建留存状态 |
| `event` | 处理点击或变更 | 全部 | 阻塞 |
| `task` | 恢复异步工作 | 全部 | 阻塞 |
| `layout` | 在 GPUI 布局过程中渲染一个虚拟化项 | 读状态、构建元素 | `notify`、打开浮层、创建留存状态 |

`cx.phase()` 返回当前 phase，不在任何 Host 调用中时返回 `"none"`。

`cx.theme()` 返回这次调用中 gpui-base 当前语义主题的深度只读 Snapshot：既包含直接颜色角色，也包含 `colors`、`spacing`、`radius`、`appearance` 与 `is_dark`。优先使用它，而不是兼容用的 `theme()` 导出，因为 context 写法明确表达了调用生命周期与当前 Host 主题。

每一条拒绝都是一条具体信息，而不是未定义行为：

```text
cx.notify() is not allowed during the `render` phase;
request a re-render from an event handler instead
```

渲染中通知自己是一个死循环，所以它被拒绝而不是被延后。

## 两种 `cx`

在 GPUI 里 `&mut Window` 与 `&mut App` 是借用：它们的存活期恰好是一次调用。脚本对象比任何借用都活得久，所以脚本侧的 `cx` 不能持有它们。GPUI 为确实需要跨调用持有的代码准备了第二种——`AsyncApp`，由 `cx.spawn` 交给它的闭包——这里也一样。

**`Context`** 是 `render` 和每个事件处理器收到的那种。它持有一个 **generation 编号**，每次使用都与实时的作用域栈比对，所以把它留到调用之外得到的是一条错误，而不是一帧被破坏的画面：

```text
cx is no longer valid: it was captured during an earlier call and used later.
Use cx.spawn or take cx from the callback arguments instead.
```

**`AsyncContext`** 是 `init` 收到的那种，也是 `cx.spawn` 与 `cx.timer` 交给回调的那种。它不指名任何一次调用——用到它时才解析当时正在执行的那一次——所以 `await` 不会把它带走：

```js
async save(cx) {
  await cx.sleep(100);
  cx.notify();          // 同一个 cx，仍然是对的那个
}
```

这三处正是职责为「安排或延续比启动它的那次调用活得更久的工作」的地方。其余场合要的就是严格的那种，被告知「你留得太久了」正是它的价值所在。

`cx` 上除了函数什么都没有——`Object.keys(cx)` 只看得到方法，看不到 generation——所以脚本无法伪造一个。

没有第三种拿到它的办法。模块顶层和裸 `constructor` 不会被交给 context，也无从索取——这是设计而非缺口：GPUI 根本没有模块顶层，在那里启动的工作不属于任何 View，没有东西拥有它，也没有东西取消它。把它放进 `init`，那正是 View 被交给 context 的地方。

## 留存状态

View 自己的字段放普通数据。带有跨帧机制的东西——文本框的内容、光标位置与撤销历史——存放在 GPUI 实体里，脚本持有一个**句柄**。

```js
import { InputState, Input } from "gpui-base";

init() {
  this.draft = InputState.new({ placeholder: "What needs doing?" });
  this.draft.on("submit", (_event, cx) => this.add(cx));
}

render(cx) {
  return Input.new(this.draft)
    .flex_1()
    .h(28)
    .px(8)
    .border(1)
    .border_color(cx.theme().colors.input)
    .bg(cx.theme().colors.surface)
    .text_size(12);
}
```

| 调用 | 作用 |
| --- | --- |
| `InputState.new({ placeholder, value })` | 创建状态，两个选项都可省略 |
| `state.value()` | 当前文本 |
| `state.set_value(text)` | 替换文本 |
| `state.on(event, handler)` | 订阅，见下 |
| `state.release()` | 释放句柄 |
| `Input.new(state)` | 渲染它的元素 |

**在 `init` 或事件回调里创建，绝不要在 `render` 里创建。** 创建实体需要一个实时窗口，而 `render` 本来也是最不该做这件事的地方：

```text
InputState.new(...) cannot run during render; create state in init()
or in an event handler and keep it on the view
```

脚本持有的是句柄而不是实体——实体归 GPUI 所有。使用已释放的句柄会抛异常，而不是返回 `undefined`；因为 `undefined` 在 JavaScript 里往往飘出很远才炸，那时源头已经找不到了：

```text
this input state has been released
```

`Input` 是唯一由运行时给出默认值的元素，而且只有三条：垂直居中的一行、占满宽度、点击框内任意位置获得焦点。每一条都是脚本可以覆盖、但不该被迫记住的默认——没有第一条，文本会贴在给定高度的顶部，在屏幕上看起来像 bug 而不是缺一条样式。

### 输入事件

```js
this.draft.on("submit", (event, cx) => this.add(cx));
```

| 事件 | 触发于 |
| --- | --- |
| `change` | 文本发生变化 |
| `submit` | 按下回车；`event.secondary` 与 `event.shift` 说明按法 |
| `focus` | 获得焦点 |
| `blur` | 失去焦点 |

与渲染期注册的 `on_click` 不同，这个订阅**活得比创建它的那次渲染更久**。订阅由运行时的句柄存储持有而不是由脚本持有，因为脚本没有地方放它，而“因为某个值被回收所以处理函数不再触发”是那种没人找得到的 bug。它随句柄一起释放。

事件名拼错会列出合法值：

```text
unknown input event `changed`; expected one of: change, submit, focus, blur
```

### 日历状态

`CalendarState` 是同样的模式，留存的东西不同：正在看的是哪个月、选中的是哪一天，以及由此推出的那张日期网格。

```js
init(_props, cx) {
  this.calendar = CalendarState.new();
  this.calendar.on("change", (date, cx) => this.pick(date, cx));
}

render(cx) {
  const grid = this.calendar.month_days()[0];
  return v_flex().children(
    grid.map((week) =>
      h_flex().gap(4).children(
        week.map((day) =>
          Button.new(day)
            .selected(day === this.calendar.value())
            .on_click((_e, cx) => { this.calendar.set_value(day); cx.notify(); })
            .child(String(Number(day.slice(8)))),
        ),
      ),
    ),
  );
}
```

`month_days()` 是它存在的理由：哪些日期落在哪一周、相邻月份的日子补在哪里、这个月需要几行。格子是你自己画的——base 的 `Calendar` 元素**没有**绑定，因为它遍历同一份网格、每个格子调用一次渲染回调，一帧最多四十二次跨语言调用，而且发生在 GPUI 的 layout 过程里，为的是一批本身不带行为的格子。

日期一律是 `"YYYY-MM-DD"`，区间是 `[start, end]`，没选是 `null`。区间即使终点还没定也保持成对——`["2026-08-03", null]` 不会塌成它的起点，因为“选了一天”和“区间开了个头”对 base 是两种状态，它自己的逻辑在这上面分支。

| 方法 | 说明 |
| --- | --- |
| `CalendarState.new()` | 创建状态；和其他留存状态一样，只能在 `init` 或事件处理器里 |
| `month_days()` | 网格：按月分组的“周”，每周固定七天 |
| `year()` / `month()` / `today()` | 网格对应的年月，以及创建时读到的今天 |
| `value()` / `set_value(next)` | 选中的日期 |
| `next_month()` / `prev_month()` | 前后移一个月；在 `render` 中不合法 |
| `on("change", handler)` | 唯一的事件，报告一个日期被选中 |
| `release()` | 丢弃句柄 |

## 异步工作

脚本代码用的是普通的 JavaScript 异步方式——`async` 函数与原生 promise。运行时补上的是裸 QuickJS 没有的那部分：一个时钟、待执行工作的 owner，以及负责推动 job 队列的人。

| 导出 | 作用 |
| --- | --- |
| `cx.sleep(ms)` | 在 GPUI 的 foreground executor 上，`ms` 之后 resolve 的 promise |
| `cx.spawn(body, opts?)` | 调用 `body(cx)` 并接管它返回的 promise |
| `cx.timer.after(ms, handler, opts?)` | 调用一次 `handler(cx)` |
| `cx.timer.every(ms, handler, opts?)` | 反复调用 `handler(cx)` |

调度挂在 `cx` 上，因为 GPUI 就是这么放的——`App::spawn`，以及由 context 交出的 executor 上的 timer。不需要 import 任何东西。

它们产生的工作全部在主线程上运行。脚本可见的东西从不离开主线程：这里没有 `Worker`，VM 与 GPUI 的 `App` 都是主线程独占的。

```js
flash(cx) {
  this.saved = true;
  cx.notify();

  cx.spawn(async (cx) => {
    await cx.sleep(1500);
    this.saved = false;
    cx.notify();
  });
}
```

这段不需要任何 import：`cx` 就是处理器的第二个参数，而它的 body 收到的 `cx` 是能挺过 `await` 的异步那种。

**`cx.spawn` 会接管 promise，这正是它的意义。** 未处理的 rejection 是 JavaScript 最常见的静默失败：工作停了，界面保持原状，什么都没写到任何地方。在这里它会带着脚本自己的调用栈进入 `tracing::error!`。

### 归属与取消

每个任务都属于某个 View——`opts.owner`，或者创建它时正在运行的那个 View。任务持有弱引用，所以当发起这项工作的面板消失时，回调会被跳过，而不是写进一份再也不会被渲染的状态。

```js
const handle = cx.timer.every(1000, (cx) => this.tick(cx));
handle.cancel();
handle.is_done();
```

`owner: null` 表示退出这套归属、比任何 View 都活得久；它是今天除了当前 View 之外运行时唯一接受的值。

取消一个 `sleep` 会让它的 promise **永远 pending**。这就是取消对 promise 的含义：后续代码不执行，也不为一段主动要求停止的代码凭空发明一个错误。

`timer.every` 的间隔从上一次调用结束开始计时，所以慢的处理函数会推迟下一次 tick，而不是把 tick 堆起来。

### Timer 与标准 Host API

```text
setTimeout  -> cx.timer.after(ms, callback)
setInterval -> cx.timer.every(ms, callback)
clearTimeout / clearInterval -> 对 after / every 返回的 Task 调用 cancel()
```

`setTimeout`、`setInterval`、`clearTimeout` 与 `clearInterval` 都是会抛错的 stub。一次性工作使用 `cx.timer.after`，重复工作使用 `cx.timer.every`；要停止其中任意一种，都对返回的 `Task` 调用 `cancel()`。全局 `fetch`，以及 [Capabilities](./capabilities.md) 中记录的安全标准模块（包括 `websocket`），都是真实的异步 Host API。CommonJS `require` 仍不可用；请使用 ES module。

浏览器 DOM 与存储并不存在：没有 `document` 或 `localStorage`。全局 `window` 是 gpui-shell 用来承载 dialog、sheet 与 toast 的 overlay host，并不是浏览器 `Window`，也不提供 DOM。

## 还没有的东西

- **全局与跨 View 状态。** 除了 [Capabilities](./capabilities.md) 里的持久化层和普通模块作用域，没有别的 store。
- **Action 与快捷键。** `gpui.action` 与 `gpui.keymap` 设计了但没有绑定；今天唯一的按键处理是 `ShellRoot` 安装的那几个（Tab、Shift-Tab、Escape）。
- **多窗口。** 窗口由 Host 打开，没有 `gpui.open_window`。
- **`gpui.gc_stats()`**，以及会读取它的调试面板。

---
title: GPUI Shell
description: 让 Rust 的 GPUI 应用可以用 JavaScript 扩展，界面仍由 GPUI 自己渲染——没有 WebView，也没有 DOM。首要目标是插件，其次才是纯脚本应用。
order: 1
---

# GPUI Shell

`gpui-shell` 的存在，是为了让一个用 Rust 写的 GPUI 应用**能被 JavaScript 扩展**。

**首要目标是插件扩展。** Host 应用编译一次、发布一次，此后新增一块面板、一个侧边工具或一段业务逻辑，都以脚本的形式加载进同一个进程——不必重新编译，不必重新分发二进制，想加一块面板的人也不必 fork 整个 Host。

**次要目标是用 JavaScript 写完整的应用。** CLI 可以直接跑起一个应用目录。这本身就是一条能用的路径，同时也是插件的开发方式：先把脚本单独跑通，再挂进 Host。

**它不是 Electron，也不是 Tauri。** 没有 WebView，没有 DOM，没有 HTML 与 CSS，没有浏览器引擎，也没有 Node.js。脚本从不负责渲染，它只把界面**描述**一次，此后每一帧都由 Rust 把这份描述重放成真正的 GPUI 元素——和一个基于 `gpui-base` 的 Rust 应用所构建的，是同一套元素模型、同一个 GPU 渲染器。在这里 JavaScript 是应用层，不是渲染层：所以一次重绘完全不执行 JavaScript，而带上整个运行时也只多 [13.5 MiB 二进制](./engine.md#链接它要付多少)。

这两个目标建立在同一条分工上。`gpui-shell` 直接构建在 [`gpui-base`](/base) 之上，[QuickJS](https://github.com/quickjs-ng/quickjs) 跑在 Host 自己的线程上。由 Host 构建运行时、决定脚本能碰到什么，而脚本在同一个进程里画出真正的界面。Rust 负责渲染、布局、文本编辑、虚拟化、焦点、浮层以及全部系统能力；脚本负责界面组合、视觉呈现与业务逻辑。

```js
import { View } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Counter extends View {
  init() {
    this.count = 0;
  }

  render(cx) {
    return v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap(20)
      .bg(cx.theme().colors.background)
      .child(
        div()
          .text_3xl()
          .text_color(cx.theme().colors.foreground)
          .child(`${this.count}`),
      )
      .child(
        Button.new("increment")
          .h(32)
          .px(14)
          .items_center()
          .justify_center()
          .bg(cx.theme().colors.primary)
          .text_color(cx.theme().colors.primary_foreground)
          .rounded(6)
          .on_click((_event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child("Increment"),
      );
  }
}
```

## 为什么插件优先

`crates/base/src/dock` 已经具备了插件系统所需的一半：布局是纯数据，`PanelRegistry` 能按持久化文件里的名字重建面板，每块面板还带着一份属于自己的 `serde_json::Value`。缺的另一半是——面板的实现必须编进 Host 的二进制，因此没有人能在不 fork 的前提下贡献一块面板。`gpui-shell` 补的正是这一半。

「插件优先」不是一句定位口号。下面这些设计决策，如果只面向独立脚本，每一条都可以是另一种选择；放在插件的语境下才成为必然：

| 设计决策                                              | 为什么它由插件推导而来                                                          |
| ----------------------------------------------------- | ------------------------------------------------------------------------------- |
| `Capabilities::default()` 是空集，由 Host 授予        | 插件是别人写的代码，授权必须来自 Host，而不能由插件在自己的 manifest 里声明即得 |
| 每个插件一份独立 `Policy`，卸载即取消它名下的全部任务 | 多个插件共用同一个运行时，授权之间不能相互渗透                                  |
| 脚本出错是可恢复的异常，Host 进程存活                 | 一个插件写崩了，不该把整个应用一起带走                                          |
| 重绘只重放快照，从不进入 VM                           | 帧预算由 Host 负责，插件的 JavaScript 不能压在上面                              |
| `HostModule` 把 Host 自己的 Rust 借给脚本             | 只有脚本跑在 Host 内部时才有意义——独立应用没有 Host 可借                        |
| Dock 面板在应用被卸载后仍保留位置与状态               | 插件会被装了又卸；重新装回来时，面板还在原来的位置，状态也还在                  |
| 基座不提供任何视觉，呈现权整个交给脚本                | 插件要长得像 Host 的一部分，就必须能掌控每一个像素                              |

独立脚本应用用得上其中的很少几条。它真正获得的是迭代速度——hot-reload、`check`，以及自动生成的 `gpui-kit.d.ts`。这也是它排在第二位的原因：它是插件被开发和验证的地方，而不是这套运行时的目的本身。

文本编辑、语法高亮、LSP、虚拟化与动画采样都留在 Rust。这条线是职责划分，而不是对脚本的限制：所有必须贴着 GPU 与系统运行的部分都归 Host，插件因此不会成为应用性能与稳定性上的变量。

::: warning 插件是目标，但接口还没有全部开放
插件之下的机制已经建成并有测试覆盖——manifest 解析与发现、加载与卸载、每个插件独立的 policy 与数据目录。脚本现在已经可以贡献面板并绘制 dock 的 chrome：`DockArea`、`dock_area(...)` 与 `DockArea.register_panel` 都已公开，带着脚本面板的布局也能熬过一次重启。还缺的是贡献注册表的其余部分（`gpui.command`、`gpui.keymap`）、授权 UI，以及一个用上 `PluginManager` 的 CLI。**今天能完整跑通的是独立脚本这条路径，dock 也在其中。** 见 [Dock 与面板](./dock.md)。
:::

## 核心特点

### 架构：脚本负责描述，Host 负责渲染

脚本从不持有 GPUI 元素，它记录的是元素的**描述**——builder 链上的每一次调用都会往一块 arena 里写入一条操作，等某一帧需要时，Rust 再把这些操作重放成真实元素。布局、绘制、命中测试、滚动、IME 与文本编辑全部留在 Rust，不会回调进脚本。[一次渲染是怎么走完的](#一次渲染是怎么走完的)完整走了一遍这个过程。

引擎是这套设计的一个参数，而不是其中一部分。今天只有 QuickJS 一种，但这条分界线之上的全部模块——arena、把描述变成真实元素的 `materialize`、CallScope、样式表、主题、能力模型、浮层 Host 、hot-reload——源码里都没有出现任何 VM 的名字。见 [The Engine Seam](./engine.md)。

### 能力：一整层应用层，而不是一套控件

脚本拿到的，正是一个基于 `gpui-base` 的 Rust 应用能拿到的东西：元素与布局、链接与控件、建立在语义主题 token 之上的流式样式接口、通过 `init` / `render` / `cx.notify()` 管理的 View 状态、由 Host 留存的状态（例如文本输入的 rope 与选区）、dialog / sheet / toast、异步任务、原生 transition 与 spring，以及需要授权才能用的文件、存储、剪贴板、进程、HTTP、TCP 与 WebSocket 接口。

围绕它的还有：`--watch` 保存文件即 hot-reload，`gpui-shell.json` 在代码运行前声明身份与最小权限，自动生成的 `gpui-kit.d.ts` 把整套 API 描述给编辑器或模型，`check` 则在应用跑起来之前就报出问题。

::: tip
`gpui-kit.d.ts` 可以加进 `.gitignore`，它是自动生成的。
:::

### 性能：脚本不在每一帧里

`render` **不是**每帧跑一次。它把界面描述一次、存进一份 Snapshot；在下一次 `cx.notify()` 之前，每一次重绘都由 Rust 重放这份 Snapshot。指针划过按钮、光标闪烁、列表滚动、原生 transition 或 spring 推进，这些重绘都不执行 JavaScript。

运行时把两件事分开计数，gallery 的 Shell story（`cargo run -- shell`）把这两个数摆在界面上：

<img class="architecture-light" src="/shell-render-frequency-light.svg" alt="一秒内的一块实时面板。JavaScript 的数据没有变化时，60 帧全部触发，而 JavaScript 那一行始终是空的；价格每 50 ms 变动一次时，仍是 60 帧，JavaScript 触发约 20 次。">
<img class="architecture-dark" src="/shell-render-frequency-dark.svg" alt="一秒内的一块实时面板。JavaScript 的数据没有变化时，60 帧全部触发，而 JavaScript 那一行始终是空的；价格每 50 ms 变动一次时，仍是 60 帧，JavaScript 触发约 20 次。">

| 界面在做什么                        | 每秒画的帧 | 每秒跑的 JavaScript |
| ----------------------------------- | ---------- | ------------------- |
| 只是重绘，JavaScript 的数据没有变化 | 60         | 0                   |
| 价格每 50 ms 变动一次               | 60         | 19                  |

帧数取决于屏幕，JavaScript 的次数取决于数据。第二行里另外 41 帧重放的是已有的描述。

成本因此按用户操作计，而不是按帧计。443 节点的面板，跑一遍 `render`、把整个界面记进 Snapshot 要 1.1 ms，只在状态变化时付；之后每一帧 1.3 ms，那是渲染本身——把 Snapshot 变成元素、布局、绘制，其中没有 JavaScript。

|               | 每帧成本                                                            |
| ------------- | ------------------------------------------------------------------- |
| 没有 Snapshot | 1.1 ms (JS render) + 1.3 ms (Rust render) = **2.4 ms/frame render** |
| 有 Snapshot   | **1.3 ms**                                                          |

面板变大也不改变这条性质：[基准测试](./engine.md#那次实测)覆盖到 8,403 个节点，各档的每一帧都不执行 JavaScript，最小一档由每次 CI 运行的断言保证。

### 体积：一个脚本运行时只要 +13.5 MiB

跑一个真实脚本应用的 Host，二进制 **26.1 MiB**、常驻内存 **81 MiB**——QuickJS 和整个标准运行时都在里面。相比同一个应用不带它的版本，取这个依赖的代价是**二进制 +13.5 MiB、内存 +14 MiB**。

这个数是个常数，不是比例：组件 gallery——体量是它的五倍——增加的同样是 13.5 MiB。[链接它要付多少](./engine.md#链接它要付多少)给出了测量所用的那一对程序，以及这些兆字节都去了哪里。

以上数字都取自一台 MacBook Pro（M3，8 核，24 GB）：帧数与次数来自 Shell story，毫秒数来自 release 构建的基准测试，二进制与内存数字来自 `examples/hello_world` 和 `gpui-shell` CLI 的 release 构建。

### 安全：默认什么都没有，语言本身也一并收紧

`Capabilities::default()` 是空集——没有文件访问、没有存储、没有剪贴板、不能执行进程、没有网络。 Host 在加载 View 之前决定授权， View 随后在自己的整个生命周期里保持这份授权；`fs` 接口上的每一条路径都走**同一个**解析器，任何落在授权根之外的结果都会被拒绝。

在授权之下，沙箱还收紧了语言本身——因为一个 VM 早晚要同时承载多个插件：`eval` 与四个函数编译器全部移除，内置原型被冻结，避免一个插件改动 `Object.prototype` 波及另一个；模块解析被限制在应用目录内；堆（256 MiB）、解释器栈（1 MiB）与单次调用耗时（`render` 为 50 ms）都有上限。其中的耗时上限是一个 `catch` 无法吞掉的中断，这一点由测试保证。见 [Capabilities](./capabilities.md)。

## 一次渲染是怎么走完的

<img class="architecture-light" src="/shell-architecture-light.svg" alt="脚本如何变成界面：脚本描述元素，Rust 把它们变成真实元素，GPUI 负责绘制">
<img class="architecture-dark" src="/shell-architecture-dark.svg" alt="脚本如何变成界面：脚本描述元素，Rust 把它们变成真实元素，GPUI 负责绘制">

这张图画的是一帧的过程，而这张图的形状基本解释了本节文档的其余部分。

GPUI 的元素是**被消费**的值：`RenderOnce::render` 按值取走 `self`，`.child()` 按值取走子元素， View 每次重绘都从零重建整棵元素树。因此一个 JavaScript 对象永远不可能**就是**一个 GPUI 元素——它没有东西可以长期持有。

所以脚本不构建元素，而是**描述**元素。builder 链上的每一次调用，都会把一条操作记录进一块元素描述 arena；脚本手里的对象只带一个指向 arena 的整数下标。当 GPUI 要求 View 渲染时，Rust 把这些记录下来的操作重放成真实元素、交给 GPUI，然后整块清空 arena。布局、绘制、命中测试、滚动与 IME 全程不再回到脚本。

由此直接推出三条结论，每条对应下面一个页面：

- **元素是一次性的。** 描述在本次渲染结束时就消失了，所以被保存下来的元素在下次使用时抛出异常，而不是画出一个意料之外的东西。见 [Elements](./elements.md)。
- **`cx` 只属于产生它的那次调用。** 它带着一个 generation 编号，每次使用都与实时的调用栈比对；一个跨过 `await` 仍在使用的 `cx` 会给出明确错误，而不是去访问一个早已失效的栈帧。见 [State and Views](./state.md)。
- **回调属于注册它的那次渲染。** 下一次渲染会整体替换它们，这正是脚本闭包不会在 Host 里堆积的原因。见 [Elements](./elements.md)。

这三条都是“把脚本绑到一个会消费其值的元素模型上”必然的结果。

## 呈现权在脚本一侧

大多数脚本层的做法，是把一批做好的控件交给脚本去摆放。这里没有这样的控件可交，因为它下面那一层同样没有。

`gpui-base` 的控件完全不带视觉样式。Rust 里的 `Button::new("save")` 没有内边距、没有背景、没有圆角、没有尺寸，这就是接口约定。JavaScript 绑定原样保留了这一点：`Button.new("save")` 不写样式时，除了它的子元素之外什么都不画。

结论才是重点：**因为基础层不提供任何呈现，呈现权就完整地落在脚本一侧**——颜色、间距、hover 状态、圆角，全部由脚本决定。这与 Rust 应用选择基于 `gpui-base` 而不是 `gpui-component` 时做的取舍完全一样；区别在于，这里的取舍写在一个存盘就能立刻看到结果的文件里，中间不需要 `cargo build`。

多打的字换来的是整个应用层。改一个按钮的圆角，不必再回到 Rust。

## 适用场景

- **为已有的 GPUI 应用增加插件能力——首要场景。** 插件跑在 Host 进程内，能力由 Host 一项一项授予，起点是什么都没有。扩展产品不再意味着 fork 或者发一个新版本：界面与业务逻辑以脚本形式交付，改动不需要重新编译、也不需要重新分发二进制；插件出错会呈现为一个可恢复的错误，而不是把 Host 一起带走。
- **基于 `gpui-shell` 编写纯 JavaScript 的应用——次要场景。** 整个应用层——元素、样式、 View 状态、浮层与系统接口——都在 JavaScript 一侧，而渲染、文本编辑、虚拟化与每一个动画帧仍留在 Rust。这里也是一个插件在挂进 Host 之前被写出来、被验证的地方。

## 它在架构中的位置

```text
  JavaScript 应用            main.js · views · 样式 · 业务逻辑
            │  import { … } from "gpui-kit"
            ▼
  gpui-shell                 引擎分界线 · 元素描述 · CallScope
                             样式表 · 主题 token · 能力模型
                             ShellRoot（dialog / sheet / toast）· 调度器
            │
            ▼
  gpui-base                  行为 · 状态 · 基础设施（无样式）
            │
            ▼
  gpui                       元素 · 样式 · 渲染 · GPU · 平台
```

`gpui-shell` 与 `gpui-component` 是并列关系，而不是在它下游：两者都是 `gpui-base` 的使用者，都补上了 Base 不提供的那一层呈现。`gpui-component` 用 Rust 提供了一套成品且统一的呈现；`gpui-shell` 提供的是让脚本自己去提供呈现的那套机制。

## 接着读

| 页面                                    | 内容                                                                               |
| --------------------------------------- | ---------------------------------------------------------------------------------- |
| [Getting Started](./getting-started.md) | 运行示例、最小应用、`check` 与 `types`                                             |
| [Examples](./examples.md)               | 仓库里的独立应用、 Host 状态与原生动画示例                                         |
| [Elements](./elements.md)               | 构造器、`child` / `children` / `when`，以及元素为什么是一次性的                    |
| [Styling](./styling.md)                 | 流式样式接口、长度与颜色、语义 token、状态样式                                     |
| [State and Views](./state.md)           | `init` / `render`、`cx.notify()`、留存状态、异步                                   |
| [Overlays](./overlays.md)               | dialog、sheet、toast，以及 phase 规则                                              |
| [Capabilities](./capabilities.md)       | `gpui-shell.json`、默认拒绝、文件、存储、进程与网络 API                            |
| [依赖](./dependencies.md)               | shell package：什么样的仓库算一个，manifest 如何命名与钉住它，以及编辑器拿到的类型 |
| [Hosting](./hosting.md)                 | Rust 这一侧的全貌：挂载、刷新、指标、退出、hot-reload                              |
| [HostModule](./host-module.md)          | 把 Host 自己的 Rust 借给脚本，以及那条纯数据边界                                   |
| [Dock 与面板](./dock.md)                | 把脚本 View 变成可停靠面板、为它绘制 chrome，以及重启后什么会留下                  |
| [Performance](./performance.md)         | 脚本的成本：失效频率乘以描述规模、 View 这条边界，以及那几个计数器                 |
| [The Engine Seam](./engine.md)          | QuickJS、这条分界线存在的理由，以及把脚本成本与帧成本分开的三项实测                |

## 当前状态

该 crate 处于 **M0** 里程碑：一条可行性基线，而不是稳定接口。它没有发布到 crates.io，脚本 API 预计还会变化。本节文档写到的都是已经实现并可用的部分；缺失的部分，会写在你最可能去找它的那一页上。

设计详见 [GPUI Shell 设计文档](https://github.com/longbridge/gpui-kit/blob/main/docs/gpui-shell.md)，代码位于 [`crates/shell`](https://github.com/longbridge/gpui-kit/tree/main/crates/shell)。

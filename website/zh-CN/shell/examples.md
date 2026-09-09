---
title: 示例
description: 独立与嵌入式完整应用，包括留存状态、 HostModule 与原生动画。
order: 3
pageClass: shell-examples-page
---

# Examples

仓库自带四个示例，合起来覆盖独立应用、可停靠布局、由 Host 状态驱动的脚本，以及动画帧完全不进入 JavaScript 的原生 motion。

| | 怎么跑 | 展示了什么 |
| --- | --- | --- |
| [Todo list](#todo-list) | 一个独立应用 | 脚本这一侧的全部：留存输入、dialog、toast、受授权约束的存储、资源、类型 |
| [工作区](#工作区) | 一个独立应用 | 可停靠布局：熬过重启的面板，以及全部由脚本绘制的 chrome |
| [报价面板](#报价面板) | gallery 里的一块面板 | Host 那一半：HostModule 、一个实体被两种语言读取、实时的成本读数 |
| [原生动画](#原生动画) | gallery 内独立的脚本 View | 由 GPUI 保留并采样的像素目标 transition 与 spring |

## 一个完整的应用

这里的示例每个都只讲一件事。想看一个完整产品放在一个仓库里——OAuth、实时
WebSocket 报价流、虚拟化自选列表、用留存嵌套 View 承载的价格图表，以及它自己的
Rust Host 二进制——见
[**longbridge/longbridge-lite**](https://github.com/longbridge/longbridge-lite)。
它是一个只读的 Longbridge 桌面客户端，几千行 JavaScript，也是目前基于这个运行时
写出来的最大的东西。

## Todo list

```bash
cargo run -p gpui-shell -- examples/js_todolist
```

`examples/js_todolist/` 的目的是把整个运行时都跑一遍，而不是做到最小——`gpui-shell` 哪里坏了，通常先在这里露出来。

```text
main.js                View：状态、筛选、所有事件处理
ui.js                  呈现层，以函数形式导出
storage.js             持久化，以及没拿到授权时怎么办
confirm.js             确认 dialog，它自己也是一个 View
icons/                 四个 SVG，相对应用根目录解析
gpui-kit.d.ts              自动生成；jsconfig.json 与 types.d.ts 把类型接上
```

其中有四件事值得照抄。

**`ui.js` 是一个由函数构成的组件库。** 它导出 `label`、`muted`、`title`、`button`、`iconButton`、`checkbox`、`field`、`row`、`surface`、`rule` 与 `emptyState`，于是 `main.js` 读起来就像在用一个组件库：

```js
export const label = (value, cx) =>
  div().text_size(12).line_height(1).text_color(cx.theme().colors.foreground).child(value);

export const surface = (cx) =>
  v_flex().flex_1().bg(cx.theme().colors.surface).border(1).border_color(cx.theme().colors.border).overflow_hidden();
```

`main.js` 把当前 `cx` 传给这些 helper，由 helper 直接通过 `cx.theme()` 读取 token。这样做没有额外代价，因为[一次函数调用产出的正是一份新的描述](./elements.md)。这也是对“基础层不提供任何带样式的控件”的回答——带样式的那一层你在自己的文件里写一次，从此不必重复。

**存储是把拒绝吸收掉，而不是先去问有没有权限。** Host 没授予存储时 `store` 会抛异常，而这是关于 Host 的一个事实，不是这个应用的错误：

```js
export function load() {
  try {
    const saved = store.get(KEY);
    return Array.isArray(saved) ? saved : [];
  } catch (error) {
    console.warn(`todolist: storage unavailable, starting empty (${error.message})`);
    return [];
  }
}
```

`save()` 会返回这次写入有没有落盘，页脚则把它显示在界面上——“Not saved — this host did not grant storage, so the list lasts for this run only”。在边界上把拒绝吸收掉，然后如实告诉用户。

**dialog 是一个函数，不是一个元素。** `confirm.js` default 导出一个返回内容函数的函数；`main.js` 用 `window.open_dialog(confirmClear(count, onConfirm))` 打开它。数量和回调是闭包捕获的，不是交接过去的。见 [Overlays](./overlays.md)。

**类型是配好的，一共三个文件。** `jsconfig.json` 打开 `checkJs`，`gpui-kit.d.ts` 由 `gpui-shell types` 生成，`types.d.ts` 放这个应用自己的形状——`Todo`、`Filter`。编辑器补全与 `checkJs` 报错从此就能用，不需要任何构建步骤。

## 工作区

```bash
cargo run -p gpui-shell -- examples/js_dock
```

`examples/js_dock/` 是一个可停靠的工作区——左边是文件列表，中间是文档，布局会以你离开时的样子回来。

```text
main.js                   工作区本体：面板、dock 与持久化
ui.js                     chrome：标签页、dock 外框、落点提示
```

其中三件事是重点。

**base 不画 chrome，所以这些全在 `ui.js` 里。** 标签栏、dock 的标题条、折叠控件、缩放把手与落点提示都是用普通样式接口写出来的普通元素。一个这些都没有的 area 照样能停靠、拖动、调整大小、持久化，只是除了面板本身之外什么都不画。

**标签页带的是命令，不是处理器。** chrome 描述会缓存到原生状态改变为止，所以其中的脚本事件处理器没有可靠的生命周期。`select_tab(group, tab.index)` 与 `close_panel(group, tab.id)` 完全不携带脚本值——它们只是指名某个容器、以及要请它做什么。

**面板就是多了两个方法的 View。** `Document.serialize()` 返回它的标题与编辑次数；重启之后 `deserialize(data)` 把它们收回来。关于这块面板的其他一切——它在哪、是否正在显示——都是布局的事，永远不会传到脚本。

完整接口见 [Dock 与面板](./dock.md)。

## 报价面板

```bash
cargo run -- shell
```

gallery 的 Shell story 并排跑着两块面板：左边那块由 Rust 的 `shell_story.rs` 画，右边那块由 JavaScript 的 `crates/story/js/quotes/main.js` 画，两边读的是同一份数据。

脚本自己不持有任何状态。这块行情板是一个 Rust 的 `Entity<Market>`，从 story 在运行时启动前注册的 [HostModule](./host-module.md) import 进来：

```text
import { quotes, ticks, watch, watch_all } from "market";
```

主题值来自调用作用域内的 `cx.theme()` Snapshot，而不是第二个 HostModule 。

因为两块面板读的是同一个实体，两边一旦对不上就会立刻看出来——这也是它算一个测试而不只是演示的原因。改 `main.js` 就能改变右边那块面板，中间不需要 `cargo build`；面板旁边有一个 “Reload script” 按钮。

底下就是这套文档反复引用的那组计数读数：每秒的脚本次数对每秒的帧数，还有一个 feed 选择器，可以让其中一个动而另一个不动。这就是[那条性能主张](./index.md#性能-脚本不在每一帧里)在一个运行着的窗口里的样子。

## 原生动画

`crates/story/js/motion/main.js` 刻意使用与 quote benchmark 分离的 `ScriptView`，避免动画活动污染 render-frequency 测量。它可以在 `.transition(...)` 与 `.spring(...)` 之间切换，再重新设定 opacity 以及像素值的 width、height、left 与 top。

脚本只运行一次来发布新目标。后续每一个动画帧都由 GPUI 原生调度与采样，不会重新进入 JavaScript。示例只使用数值像素目标——没有 `rem`、百分比或 `auto`——并使用稳定 id，让留存通道能够跨 description 重建继续存在。

## 从哪儿开始

把 `examples/js_todolist` 复制到你自己的目录里跑起来——它是一个类型都配好了的完整应用。把 `main.js` 削回到一个只有 `init` 与 `render` 的 `View`，留着 `ui.js`，再从那里往上加。

要写 Host 的话，`crates/story/src/stories/shell_story.rs` 是另一侧的可用参考：它构建运行时、注册 HostModule 、挂载一个 `ScriptView`，并按需重载它。[Hosting](./hosting.md) 走的是同样这几个调用。

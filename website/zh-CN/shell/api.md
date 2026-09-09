---
title: API 参考
description: 脚本能 import 或触及的每个名字——四个内置模块、cx 与 window 全局对象，以及那些不是样式的元素方法。
order: 10
---

# API Reference

脚本接口的一份清单：有什么，以及它来自哪个模块。其余页面解释每样东西为什么是这个样子——这一页是用来查名字的。

权威不在这一页。runtime 会为自己的版本生成 `gpui-kit.d.ts`，并在应用加载时尽力刷新到源码旁；`gpui-shell types <directory>` 执行同一次写入，并会明确报告失败。生成文件的头部带有 `gpui-shell` 版本，也包含该应用注册的 HostModule 。请忽略这个文件而不要提交，并在脚本顶部写上 `// @ts-check` 让编辑器照着它检查。manifest 声明的 Git 依赖同样不在这一页：它们由同一次刷新链接进 `node_modules`，名字、签名与文档都来自 package 自身。见[依赖](./dependencies.md)。

## 模块

每个内建模块都以它所暴露的公开 Rust 层命名，所以一条 import 能说明脚本依赖哪一层。`gpui-kit` 还包含从 JavaScript 使用 GPUI 所需的 shell 桥接： View、留存实体、调度与共享类型。一个名字只属于一个模块，这里不为了方便做 re-export。

```js
import { View, div } from "gpui-kit";
import { Button, v_flex } from "gpui-base";
import { fps_monitor } from "gpui-fps";
```

| 模块         | 提供                                                             |
| ------------ | ---------------------------------------------------------------- |
| `gpui-kit`   | GPUI 自己的元素，加上这个运行时补上的部分： View、样式接口与调度 |
| `gpui-base`  | 布局辅助函数、组件与主题                                         |
| `gpui-shell` | shell 桥接层自有的纯类型概念；没有运行时导出                     |
| `gpui-fps`   | 性能 HUD                                                         |

有两个名字从不需要 import，但原因不同。`window` 是真正的全局：没有谁把它交给你，它本来就在作用域里。`cx` 恰恰相反——它从来不是全局的，只会作为参数到达：`render(cx)`、`init(props, cx)`、每个处理器的第二个参数、`cx.spawn` body 的形参。标准运行时模块——`fs/promises`、`path`、`crypto`、`process`、`net`、`websocket` 等等——受 Host 授权门控，记录在 [Capabilities](./capabilities.md)。

API 形态跟随 Rust 原型：`App` 上的方法放在 `cx`，`Window` 上的方法放在 `window` 全局对象，关联构造器写成 `Type.new(...)`，自由函数保持小写。没有直接 GPUI 或 Base 原型的名字，属于实现它的公开层。表中也会列出仅存在于类型系统的名字，但它们不是运行时可调用的值。

## `gpui-kit` 模块

### 元素

| 名称              | 说明                                                                                |
| ----------------- | ----------------------------------------------------------------------------------- |
| `Element`         | 通过链式方法构建、只属于当前 render pass 的描述                                     |
| `div()`           | 自身不带布局的元素                                                                  |
| `svg(path)`       | 来自应用根目录的矢量图，按周围的文字颜色着色                                        |
| `image(path)`     | 来自应用根目录的全彩图片，保留原色                                                  |
| `list(…)`         | GPUI 的惰性列表：行高任意，边画边测量                                               |
| `uniform_list(…)` | GPUI 的等高列表：测量一行，其余按它排布                                             |
| `PathBuilder`     | GPUI 的路径构建器类型及其工厂；`fill()` 与 `stroke(width)` 都返回一个 `PathBuilder` |
| `Background`      | `solid`、`stop`、`linear_gradient`、`pattern_slash`、`checkerboard`                 |

`PathBuilder.fill()` 与 `.stroke(width)` 返回一个句柄，可链式调用 `move_to`、`line_to`、`curve_to`、`cubic_bezier_to`、`arc_to`、`add_polygon`、`close` 与 `dash_array`，最后以 `build()` 收尾。用 `window.paint_path(path, background)` 把结果画出来——它是唯一一个通过对象取到的元素构造器，因为它镜像的东西在 Rust 侧就是窗口上的一个方法。

`list` 和 `uniform_list` 是 GPUI 自己的惰性列表，参数是 `(id, item_count, get_key, render)`，即 `gpui-base` 的 `v_virtual_list` 去掉 `item_sizes` 的形状：没有尺寸表，因为 GPUI 自己测量各项。`uniform_list` 测量一行，其余各行按它排布，`render(range, cx)` 和虚拟列表一样按区间返回元素数组。`list` 会测量画出的每一项并记住尺寸，`render(index, cx)` 为一项返回一个元素，所以高度不等的行或面板不必事先说明有多高。两者都只绘制屏幕内的内容外加折叠线下方的一小段，自己处理滚动，并按名字与 `Scrollbar` 配对；都不接受 `VirtualListScrollHandle`。

字符串本身也是元素，和 GPUI 里 `&str` 实现 `IntoElement` 完全一样：`.child("hello")` 就是写文本的方式，样式来自持有它的那个元素。

### View

| 名称        | 说明                                                          |
| ----------- | ------------------------------------------------------------- |
| `View`      | 每个 View 的基类；继承它，并把子类作为 default export         |
| `ViewClass` | 一个具体的 `View` 子类，也就是 `cx.new` 接受的东西            |
| `Entity`    | 对一个嵌套 View 的留存所有权：`set_props(props)`、`release()` |

子类定义只执行一次的 `init?(props, cx)`，以及返回一个 `Element`、`Entity` 或字符串、在 View 被置为失效时执行的 `render(cx)`。可选的 `update(props)` 在父 View 改变嵌套 View 的 props 时执行。

### 调度

| 名称    | 说明                                                       |
| ------- | ---------------------------------------------------------- |
| `Task`  | 一个正在运行的任务：`cancel()`、`is_done()`                |
| `Timer` | `after(ms, handler, opts?)` 与 `every(ms, handler, opts?)` |

### 焦点

| 名称          | 说明                                             |
| ------------- | ------------------------------------------------ |
| `FocusHandle` | 脚本自己持有的焦点目标；[它的成员](#focushandle) |

### 共享类型

| 名称               | 说明                                                                                                     |
| ------------------ | -------------------------------------------------------------------------------------------------------- |
| `Length`           | 数字（像素）、`"12px"`、`"1.5rem"`、`"50%"` 或 `"auto"`                                                  |
| `DefiniteLength`   | 同上，但不含 `"auto"`                                                                                    |
| `AbsoluteLength`   | 只有像素或 rem                                                                                           |
| `Axis`             | `"horizontal"` 或 `"vertical"`，镜像 `gpui_kit::Axis`                                                    |
| `Color`            | 一个 `gpui-base` 的 `ColorToken`，或 `#rgb` / `#rrggbb` / `#rrggbbaa` 字面量                             |
| `Role`             | 一个无障碍 role，镜像 `gpui_kit::Role` 的 snake_case 拼写                                                |
| `Anchor`           | 锚定浮层的哪个角固定在它的触发元素上                                                                     |
| `MouseButton`      | `"left"`、`"right"` 或 `"middle"`                                                                        |
| `ClickEvent`       | `click_count`、`modifiers`                                                                               |
| `MouseMoveEvent`   | `position`、`local_position`、`bounds`、`modifiers`                                                      |
| `MouseButtonEvent` | `button`、`click_count`、`position`、`modifiers`，以及元素绘制之后才有的局部几何                         |
| `ScrollWheelEvent` | 以像素表示的 `delta`；设备按行上报时还有 `delta_lines`；以及 `touch_phase`                               |
| `KeyEvent`         | `keystroke`（整个组合键，平台修饰键在所有平台上都拼作 `cmd`）、`key`、`key_char`、`modifiers`、`is_held` |
| `ActionEvent`      | `action`——脚本给这个 action 起的名字                                                                     |
| `KeyBinding`       | `cx.bind_keys` 的一项：`keystroke`、`action`、可选的 `context`                                           |
| `Modifiers`        | `shift`、`control`、`alt`、`platform`                                                                    |
| `Point`            | `x`、`y`                                                                                                 |
| `Size`             | `width`、`height`                                                                                        |
| `Path`             | 由 `PathBuilder.build()` 产出的不可变原生几何                                                            |
| `Background`       | 由 `Background.solid(...)` 等工厂创建的可复用原生背景：`opacity(factor)`、`color_space(space)`           |
| `BackgroundStop`   | 一个渐变色标，来自 `Background.stop(color, percentage)`                                                  |

#### `FocusHandle`

由 `cx.focus_handle()` 创建，用 `track_focus(handle)` 交给元素，并用 `release()` 释放。

| 方法                    | 说明                             |
| ----------------------- | -------------------------------- |
| `focus(): void`         | 把键盘移到跟踪它的那个元素上     |
| `is_focused(): boolean` | 那个元素当前是否持有键盘         |
| `release(): boolean`    | 释放句柄，并返回它当时是否还活着 |

## `gpui-shell` 模块

这些是 JavaScript 桥接层自身引入的纯类型概念。它们只用于类型检查；这个模块没有运行时值。

| 名称               | 说明                                                                                                                                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LengthString`     | shell 长度桥接接受的字符串形式                                                                                                                                                                          |
| `PathCoordinate`   | 像素，或所绘元素边界的百分比                                                                                                                                                                            |
| `Props`            | 跨 JavaScript View 桥接传递的属性包                                                                                                                                                                     |
| `ElementBounds`    | shell 事件使用的、带 `width` 与 `height` 的 `Point`                                                                                                                                                     |
| `ScopePhase`       | `"render"`、`"event"`、`"task"`、`"layout"` 或 `"none"`                                                                                                                                                 |
| `TaskOptions`      | `{ owner?: View \| null }`——任务随之取消的 View。默认是当前运行的 View；`null` 比任何 View 都活得久                                                                                                     |
| `DialogOptions`    | `{ escape_dismissable?: boolean, backdrop_dismissable?: boolean }`，两者默认都是 `true`                                                                                                                 |
| `ToastOptions`     | `{ title: string, description?: string, level?: "info" \| "success" \| "warning" \| "error", timeout?: number \| null, id?: string }`。`level` 默认 `"info"`；`timeout` 默认五秒，`null` 表示留到被关掉 |
| `MotionProperty`   | `"opacity"`、`"width"`、`"height"`、`"left"`、`"top"`                                                                                                                                                   |
| `MotionEasing`     | `"linear"`、`"ease-in"`、`"ease-out"`、`"ease-in-out"`                                                                                                                                                  |
| `TransitionPolicy` | `duration`、`delay`、`easing`                                                                                                                                                                           |
| `SpringPolicy`     | `response`、`damping`、`epsilon`                                                                                                                                                                        |

`ScopePhase` 描述当前 `Context` 属于哪一种 shell 调用。它和 GPUI 的 `DispatchPhase` 无关；后者控制事件分发时 capture 与 bubble 的顺序。

## `cx` 上下文

两种 context 的成员相同，但生命周期不同。`render` 与事件处理器收到的 `Context` 只属于那次 Host 调用；把它留到调用之后，包括跨越 `await`，都会得到 stale-context 错误。下面的 `AsyncContext` 才是为跨越 `await` 准备的那一种。

| 成员                       | 说明                                                                 |
| -------------------------- | -------------------------------------------------------------------- |
| `notify()`                 | 请求重新渲染；在 `render` 期间抛异常，因为渲染中通知自己是一个死循环 |
| `bind_keys(bindings)`      | 安装键绑定并返回安装了几条；对应 `App::bind_keys`                    |
| `stop_propagation()`       | 让这次事件不再向上传到外层的处理器；对应 `App::stop_propagation`     |
| `propagate()`              | 在同一次分发中撤销上面那一步；对应 `App::propagate`                  |
| `phase()`                  | 这次调用处于哪个 `ScopePhase`                                        |
| `theme()`                  | 当前 `gpui_kit::base::Theme` 的语义 token 投影                       |
| `open_url(url)`            | 把一个绝对的 `http`/`https` URL 交给系统处理器                       |
| `read_from_clipboard()`    | 剪贴板里的文本，没有文本时是 `undefined`                             |
| `write_to_clipboard(text)` | 替换剪贴板里的文本                                                   |
| `focus_handle()`           | 一个新的 `FocusHandle`；属于 `init` 或事件处理器，绝不属于 `render`  |
| `new(Class, props?)`       | 创建一个留存的嵌套 View，并返回拥有它的 `Entity`                     |
| `spawn(body, opts?)`       | 执行 `body(cx)` 并接管它返回的 promise，让 rejection 得到上报        |
| `sleep(ms?)`               | 在 GPUI 的 foreground executor 上，`ms` 之后 resolve                 |
| `timer`                    | `Timer`：`after` 与 `every`                                          |

其中好几个都指名了它所镜像的 GPUI 方法：`open_url` 是 `App::open_url`，`read_from_clipboard` 与 `write_to_clipboard` 是 `App::read_from_clipboard` 与 `App::write_to_clipboard`，`focus_handle` 是 `App::focus_handle`（GPUI 没有 `FocusHandle::new`，这里同样没有），`new` 是 `AppContext::new`，`spawn` 是 `App::spawn`。

### `AsyncContext`

`AsyncContext` 继承 `Context`，不增加任何成员。区别在生命周期，不在接口：普通的 `Context` 只为一次 Host 调用发言，一旦那次调用返回就明确报错；而 `AsyncContext` 不指名任何一次调用——用到它时才解析当时正在执行的那一次，只有在一次都没有时才拒绝。它对应 GPUI 的 `AsyncApp`。

有三处会交出一个：`init`、`cx.spawn` 的 body，以及 `cx.timer` 的回调。这三处的职责正是「安排或延续比启动它的那次调用活得更久的工作」。

## `window` 全局对象

这个全局对象的类型是 `gpui-kit` 导出的 `Window`。调用处不需要 import，也没有谁把它交给你。每次调用都读取当前正在跑的那次 Host 调用，不在任何调用中时抛异常，所以没有句柄要持有，也没有东西会过期。浮层属于窗口，而不属于打开它的那个 View——这就是这些方法在这里、而不在 `Context` 上的原因。

| 成员                                                                                | 说明                                                              |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `open_dialog(content, options?)`                                                    | 打开一个 dialog，并返回栈的新深度                                 |
| `close_dialog()`                                                                    | 关闭最上层的 dialog，并回答有没有找到                             |
| `close_all_dialogs()`                                                               | 关闭所有 dialog，并回答关掉了几个                                 |
| `has_active_dialog()`                                                               | 是否有 dialog 打开；与其余方法不同，它在 `render` 中合法          |
| `open_sheet(content)`                                                               | 在右侧打开 sheet，替换掉原本在那里的内容                          |
| `open_sheet_at(placement, content)`                                                 | 同上，贴靠你指定的 `gpui-base` `Placement`                        |
| `close_sheet()`                                                                     | 关闭 sheet，并回答原本有没有打开                                  |
| `has_active_sheet()`                                                                | sheet 是否打开；在 `render` 中合法                                |
| `push_toast(options)`                                                               | 弹出一个 toast，并返回它的 id                                     |
| `remove_toast(id)`                                                                  | 撤回一个 toast，并回答它当时是否还在显示                          |
| `clear_toasts()`                                                                    | 撤回所有 toast，并回答撤回了几个                                  |
| `paint_path(path, background)`                                                      | 用原生背景绘制不可变几何；对应 `Window::paint_path`               |
| `dispatch_action(action)`                                                           | 沿本窗口的焦点路径派发一个 action；对应 `Window::dispatch_action` |
| `rem_size()` / `line_height()`                                                      | 窗口的排版度量，单位是像素                                        |
| `viewport_size()` / `bounds()`                                                      | 可绘制区域，以及窗口在屏幕上的位置                                |
| `mouse_position()`                                                                  | 指针位置，窗口坐标                                                |
| `appearance()`                                                                      | `"light"` 或 `"dark"`                                             |
| `is_window_active()` / `is_fullscreen()` / `is_maximized()`                         | 平台窗口的状态                                                    |
| `set_rem_size(size)`                                                                | 重新缩放所有以 rem 表达的尺寸                                     |
| `refresh()`                                                                         | 重绘窗口里的每一个 View                                           |
| `focus_next()` / `focus_prev()`                                                     | 把键盘移到相邻的一个 tab stop                                     |
| `activate_window()` / `minimize_window()` / `zoom_window()` / `toggle_fullscreen()` | 平台窗口控制                                                      |
| `localStorage`                                                                      | Web Storage，背后是 Host 放好的一个文件，跨重启存活               |
| `sessionStorage`                                                                    | Web Storage，只在内存里，随进程一起消失                           |

上面这些度量——从 `rem_size()` 一直到 `is_maximized()`——在 `render` 中都是合法的：一个要按窗口尺寸决定自身大小的 View，只能在绘制它的那一趟里问。而所有*改变*窗口的调用在 `render` 中都会被拒绝，理由和 `cx.notify()` 一样：一帧去改自己正在绘制的窗口，就是这一帧在和自己较劲。

`open_dialog`、`open_sheet` 与 `open_sheet_at` 接受的是**一个返回元素的函数**，而不是元素：dialog 活得比打开它的那次调用久，每次重绘时这个函数都会再执行一次。除了两个 `has_active_*` 查询与 `paint_path`，这里的一切在 `render` 中都不合法。见 [Overlays](./overlays.md)。

### 存储

[Web Storage API](https://developer.mozilla.org/zh-CN/docs/Web/API/Web_Storage_API)，原样照搬。两个 store 同时也是裸的全局变量——`localStorage.getItem(k)` 与 `window.localStorage.getItem(k)` 是同一次调用——因为在浏览器里也是如此。

| 成员                  | 说明                        |
| --------------------- | --------------------------- |
| `length`              | 已存的键数量                |
| `key(index)`          | 该位置上的键，越界为 `null` |
| `getItem(key)`        | 值，键不存在时为 `null`     |
| `setItem(key, value)` | 存入，值会被转成字符串      |
| `removeItem(key)`     | 忘掉一个键                  |
| `clear()`             | 全部忘掉                    |
| `flush()`             | 写入落盘后 resolve          |

值是字符串，所以有结构的东西照 web 上的写法走 `JSON.stringify` 与 `JSON.parse`。`flush()` 是唯一多出来的成员：浏览器不需要它，因为它的存储从头到尾都是同步的。`localStorage` 受 capability 管辖，Host 没授权时抛异常；`sessionStorage` 不受管辖，因为它持有的东西从不离开进程。见 [Capabilities](./capabilities.md#storage)。

## `gpui-base` 模块

这里的组件拥有行为、焦点，以及屏幕阅读器听到的内容，而自身几乎什么都不画。画面归脚本所有，用[样式接口](./styling.md)写出来。每个名字都链接到它在 [gpui-base 文档](../../base/index.md)里的页面，那里描述了它完整的 Rust 接口与行为。

### 布局

| 名称                                                      | 说明                                               |
| --------------------------------------------------------- | -------------------------------------------------- |
| `h_flex()`                                                | 一行                                               |
| `v_flex()`                                                | 一列                                               |
| [`h_resizable(id)`](../../base/primitives/resizable.md)   | 一行带可拖拽分隔条的窗格；尺寸按这个 id 存在窗口里 |
| [`v_resizable(id)`](../../base/primitives/resizable.md)   | 同上，纵向堆叠                                     |
| [`resizable_panel()`](../../base/primitives/resizable.md) | 可调整组里的一个窗格，用在别处都不合法             |

### 控件

| 名称                                                     | 说明                                                                                     |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| [`Button`](../../base/primitives/button.md)              | 激活、焦点、disabled 与 selected 状态                                                    |
| [`Link`](../../base/primitives/link.md)                  | 通过系统浏览器打开的外部 HTTP(S) 资源                                                    |
| [`Checkbox`](../../base/primitives/checkbox.md)          | 受控的勾选；勾选标记自己画                                                               |
| [`Switch`](../../base/primitives/switch.md)              | 受控的 switch                                                                            |
| [`Radio`](../../base/primitives/radio.md)                | 一组中的一个选项；只报告 `true`，从不报告取消选中                                        |
| [`Toggle`](../../base/primitives/toggle.md)              | 一个会保持按下的按钮                                                                     |
| [`RadioGroup`](../../base/primitives/radio-group.md)     | 被报读为一组的一批 radio；自身不持有选中项                                               |
| [`ToggleGroup`](../../base/primitives/toggle-group.md)   | 被报读为 toolbar 的一批 toggle                                                           |
| [`Tabs`](../../base/primitives/tabs.md)                  | 自身不持有选中项的 tab 列表                                                              |
| [`Tab`](../../base/primitives/tabs.md)                   | 一个 tab：`selected(...)` 进，`on_click(...)` 出                                         |
| [`Progress`](../../base/primitives/progress.md)          | 只有报读，没有进度条；单独的 `Progress.new(...)` 什么都不画                              |
| [`ProgressTrack`](../../base/primitives/progress.md)     | 凹槽：一个由你设定尺寸与颜色的普通元素                                                   |
| [`ProgressIndicator`](../../base/primitives/progress.md) | 已填充的部分；按你报读的百分比设置它的宽度                                               |
| [`Avatar`](../../base/primitives/avatar.md)              | 渲染它的 `image` 槽；没有图片时渲染 `fallback`。它自己不画圆形、尺寸或背景               |
| [`AvatarImage`](../../base/primitives/avatar.md)         | 图片槽：`AvatarImage.new(path)`，用在别处无效                                            |
| [`AvatarFallback`](../../base/primitives/avatar.md)      | 兜底槽：一个普通盒子，放首字母、图形或 `svg`                                             |
| [`Pagination`](../../base/primitives/pagination.md)      | 一个 navigation landmark，带报读的标签；页码按钮由脚本自己画                             |
| `pagination_items(current, total, visible?)`             | 该画哪些页码、省略号落在哪。`visible` 默认 7，最小 5；总页数 ≤ 1 时返回空                |
| [`Accordion`](../../base/primitives/accordion.md)        | 一个 group，装 item                                                                      |
| [`AccordionItem`](../../base/primitives/accordion.md)    | 一个条目：`open(...)` 进，trigger 的 `on_change(...)` 出；它把自己的 `open` 传给下面两半 |
| [`AccordionHeader`](../../base/primitives/accordion.md)  | 标题：`AccordionHeader.new(trigger)`，`aria_level(n)` 报读层级（默认 3）                 |
| [`AccordionPanel`](../../base/primitives/accordion.md)   | 展开的区域。关闭时不在树里，除非 `keep_mounted(true)`                                    |
| [`AccordionTrigger`](../../base/primitives/accordion.md) | 按钮：报读展开状态，`on_change` 请求相反的那个                                           |
| [`CalendarState`](../../base/primitives/calendar.md)     | 留存的日历状态：月网格、当前月份、选中的日期                                             |
| [`SliderState`](../../base/primitives/slider.md)         | 留存的 slider 状态，也是一次拖拽写入的地方                                               |
| [`Slider`](../../base/primitives/slider.md)              | 根：报读数值，并拥有 release                                                             |
| [`SliderTrack`](../../base/primitives/slider.md)         | 按下与拖拽的表面                                                                         |
| [`SliderIndicator`](../../base/primitives/slider.md)     | 凹槽，也是每个指针位置据以测量的那个盒子                                                 |
| [`SliderThumb`](../../base/primitives/slider.md)         | 滑块；shell 给它位置，你给它外观                                                         |

slider 的四个部件接受同一个 `SliderState`，而且四个都不能少——没有 `SliderIndicator` 的 slider 根本拖不动。

### 文本编辑

| 名称                                                   | 说明                                                        |
| ------------------------------------------------------ | ----------------------------------------------------------- |
| [`InputState`](../../base/primitives/input.md)         | 留存的文本状态：`InputState.new({ placeholder, value })`    |
| [`Input`](../../base/primitives/input.md)              | 包住留存文本状态的框                                        |
| [`NumberInput`](../../base/primitives/number-input.md) | 建立在同一个 `InputState` 上的 spinbutton，三个插槽都有分量 |
| [`TextareaState`](../../base/primitives/textarea.md)   | 留存的多行文本状态；`rows` 是一个选项                       |
| [`Textarea`](../../base/primitives/textarea.md)        | 包住留存多行状态的框                                        |
| [`OtpState`](../../base/primitives/otp-input.md)       | 留存的一次性验证码状态；长度在创建时固定                    |
| [`OtpInput`](../../base/primitives/otp-input.md)       | 定长验证码，格子由 shell 画、由脚本设定样式                 |

没有专门的数字状态类型：给 `InputState` 设上 `set_step`、`set_min` 与 `set_max`，它就成了数字状态。

### 容器与浮层

| 名称                                                  | 说明                                                              |
| ----------------------------------------------------- | ----------------------------------------------------------------- |
| [`Collapsible`](../../base/primitives/collapsible.md) | 仅在 `open` 时渲染它的 `content` 插槽；不带 role、箭头或触发器    |
| [`Popover`](../../base/primitives/popover.md)         | 锚定在触发元素上、由按下打开的浮层                                |
| [`HoverCard`](../../base/primitives/hover-card.md)    | 同上，但由指针停留打开，并有自己的打开状态                        |
| [`Popup`](../../base/primitives/popup.md)             | 光秃秃的锚定浮层：`Popup.new(id, trigger)`，填入 `content` 即打开 |
| [`Select`](../../base/primitives/select.md)           | combobox 的根：role、报读的打开状态、键盘——但不含任何画面         |
| [`Combobox`](../../base/primitives/combobox.md)       | 同一个根，被报读为一个触发器是可编辑输入框的 combobox             |
| [`DatePicker`](../../base/primitives/date-picker.md)  | 日期选择器的根：`DatePicker.new(id, focus_handle)`；它不持有日期  |

在这些之上动手之前，有两处缺口值得先知道：打开的 `Select` 或 `Combobox` 列表的方向键导航要你自己接（零件都在，见下），而 Enter 与 Escape 到不了 `DatePicker`。两者都写在各自类型的声明里，也就是它们真正咬人的地方。

### 表格与列表

| 名称                                                    | 说明                                                                  |
| ------------------------------------------------------- | --------------------------------------------------------------------- |
| [`Table`](../../base/primitives/table.md)               | 语义表格的根，组合方式与 HTML 组合表格一致                            |
| [`TableHeader`](../../base/primitives/table.md)         | 表头行组                                                              |
| [`TableBody`](../../base/primitives/table.md)           | 表体行组                                                              |
| [`TableRow`](../../base/primitives/table.md)            | 一行：`.new(id, row_index)`，从 1 开始                                |
| [`TableHead`](../../base/primitives/table.md)           | 一个列头：`.new(id, column_index)`，从 1 开始                         |
| [`TableCell`](../../base/primitives/table.md)           | 一个数据单元格：`.new(id, column_index)`，从 1 开始                   |
| [`TableCaption`](../../base/primitives/table.md)        | caption 该在的视觉位置；它不带 caption role                           |
| [`v_virtual_list(…)`](../../base/virtual-list.md)       | 只描述屏幕内内容的纵向列表                                            |
| [`h_virtual_list(…)`](../../base/virtual-list.md)       | 另一个轴上的同一件事；`item_sizes` 是宽度                             |
| [`VirtualListScrollHandle`](../../base/virtual-list.md) | 虚拟列表的滚动位置，跨帧保留                                          |
| [`Scrollbar`](../../base/primitives/scrollbar.md)       | `new(id)`、`horizontal(id)`、`vertical(id)`——一条由你自己摆放的滚动条 |

两种虚拟列表都接受 `(id, item_count, item_sizes, get_key, render)`。`render(range, cx)` 是这套接口里唯一由 Host 在一帧*进行中*调用的回调，所以在它内部注册处理器、创建留存状态与调用 `cx.notify()` 都会被拒绝。

### Dock

| 名称                                   | 是什么                                                            |
| -------------------------------------- | ----------------------------------------------------------------- |
| `DockArea.new(id, options?)`           | 一个可停靠布局，retained；`options` 为 `{ version?: number }`     |
| `DockArea.register_panel(name, Class)` | 教会运行时用 `Class` 重建 `name` 这块面板；返回加了命名空间的名字 |
| `dock_area(area)`                      | 画出它，并承载六个 chrome handler                                 |
| `dock_content()`                       | 一侧 dock 自己的面板，在你画的 chrome 里应该出现的位置            |

area 上的方法是 `add_panel(view, options)`、`remove_panel(id)`、`panels()`、`dump()`、`load(state)`、`has_dock`、`is_dock_open`、`toggle_dock`、`remove_dock`、`dock_size`、`set_dock_size`、`set_dock_collapsible`、`is_locked`、`set_locked`、`is_zoomed`、`zoom_out`、`on("layout_changed", handler)` 与 `release()`。

**每一次编辑都在发起它的那次调用返回之后按调用顺序应用**——面板的主体来自 `cx.new(Class)`，那时它自己还在构造中——所以 `panels()` 与 `dump()` 读到的是本轮编辑之前的布局。见 [Dock 与面板](./dock.md)。

### 留存句柄

每一个都只创建一次——在 `init` 或事件处理器里，绝不在 `render` 里——并且每一个都有 `release(): boolean`，返回它当时是否还活着。释放之后再用会抛异常。

`on(...)` 是替换该事件的处理器，而不是再加一个，返回值表示之前是否已经有一个。

#### `InputState`

来自 `InputState.new(options?)`，其中 `options` 是 `{ placeholder?: string, value?: string }`。

| 方法                                   | 说明                                                                               |
| -------------------------------------- | ---------------------------------------------------------------------------------- |
| `value(): string`                      | 当前文本                                                                           |
| `set_value(next: string): void`        | 替换它                                                                             |
| `on(event, handler): boolean`          | `event` 为 `"change"`、`"submit"`、`"focus"` 或 `"blur"`；handler 收 `(event, cx)` |
| `set_step(step: number \| null): void` | `NumberInput` 的步长，`null` 表示没有                                              |
| `set_min(min: number \| null): void`   | 数值下界，或 `null`                                                                |
| `set_max(max: number \| null): void`   | 数值上界，或 `null`                                                                |
| `set_masked(masked: boolean): void`    | 文本是否按密码绘制                                                                 |
| `set_loading(loading: boolean): void`  | 是否显示加载状态                                                                   |

#### `TextareaState`

来自 `TextareaState.new(options?)`，其中 `options` 是 `{ placeholder?: string, value?: string, rows?: number }`。

| 方法                                                      | 说明                                                                    |
| --------------------------------------------------------- | ----------------------------------------------------------------------- |
| `value(): string`                                         | 当前文本                                                                |
| `set_value(next: string): void`                           | 替换它                                                                  |
| `on(event, handler): boolean`                             | `"change"`、`"submit"`、`"focus"` 或 `"blur"`，handler 收 `(event, cx)` |
| `set_rows(rows: number): void`                            | 可见行数                                                                |
| `set_auto_grow(min_rows: number, max_rows: number): void` | 在这两者之间随内容增高                                                  |
| `set_soft_wrap(wrap: boolean): void`                      | 长行是否折行                                                            |

#### `SliderState`

来自 `SliderState.new(options?)`，其中 `options` 是 `{ min?, max?, step?, scale?: "linear" | "logarithmic", value?: SliderValue }`。默认是 `0..100`、步长 `1`、从 `min` 起。`"logarithmic"` 需要 `min` 大于零。

| 方法                                 | 说明                                                                 |
| ------------------------------------ | -------------------------------------------------------------------- |
| `value(): SliderValue`               | 当前值：一个数字，区间滑块则是 `[start, end]`                        |
| `set_value(next: SliderValue): void` | 替换它                                                               |
| `min_value(): number`                | 创建时的下界                                                         |
| `max_value(): number`                | 上界                                                                 |
| `step_value(): number`               | 步长                                                                 |
| `on(event, handler): boolean`        | 拖动中的 `"change"` 或结束时的 `"release"`；handler 收 `(value, cx)` |

#### `OtpState`

来自 `OtpState.new(length, options?)`，其中 `options` 是 `{ value?: string, masked?: boolean }`。长度在创建时就固定了。

| 方法                                | 说明                                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------------- |
| `value(): string`                   | 目前已输入的数字                                                                                  |
| `set_value(next: string): void`     | 替换它们                                                                                          |
| `len(): number`                     | 它持有几位                                                                                        |
| `is_masked(): boolean`              | 是否遮蔽绘制                                                                                      |
| `set_masked(masked: boolean): void` | 改变这一点                                                                                        |
| `focus(): void`                     | 把键盘移进去                                                                                      |
| `on(event, handler): boolean`       | 每次编辑后的 `"change"`、填满时的 `"complete"`，或 `"focus"` / `"blur"`；handler 收 `(event, cx)` |

#### `VirtualListScrollHandle`

来自 `VirtualListScrollHandle.new()`，用 `track_scroll(handle)` 交给列表。

| 方法                                             | 说明                                                                       |
| ------------------------------------------------ | -------------------------------------------------------------------------- |
| `scroll_to_item(index: number, strategy?): void` | 在下一帧之前把某一项带到屏幕上；`strategy` 是 `"top"`（默认）或 `"center"` |
| `scroll_to_bottom(): void`                       | 滚到末尾                                                                   |

### 日历

`CalendarState` 存在的理由是 `month_days()`——哪些日期落在哪一周、相邻月份的日子补在哪里、这个月需要几行。格子由脚本自己画。

```js
const grid = this.calendar.month_days()[0];
v_flex().children(
  grid.map((week) =>
    h_flex().children(
      week.map((day) =>
        Button.new(day)
          .selected(day === this.calendar.value())
          .on_click((_, cx) => {
            this.calendar.set_value(day);
            cx.notify();
          })
          .child(String(Number(day.slice(8)))),
      ),
    ),
  ),
);
```

base 的 `Calendar` 元素**没有**绑定，这是个决定而不是遗漏：它遍历同一份网格，每个格子调用一次渲染回调——一帧最多四十二次跨语言调用，而且发生在 GPUI 的 layout 过程里，为的是一批本身不带任何行为的格子。在这里读到网格自己画，是同样的活，少了那四十二次穿越。

日期一律是 `"YYYY-MM-DD"`：按文本排序即是按时间排序，`new Date(s)` 能直接读——需要星期名或本地化月份名时用它。

| 方法                            | 说明                                                         |
| ------------------------------- | ------------------------------------------------------------ |
| `month_days()`                  | 网格：按月分组的“周”，每周固定七天，首尾两周带相邻月份的日子 |
| `year()` / `month()`            | 网格对应的年份与月份（1–12）                                 |
| `today()`                       | 状态创建时读到的今天                                         |
| `value()` / `set_value(next)`   | 选中的日期：一天、`[start, end]` 区间，或 `null`             |
| `next_month()` / `prev_month()` | 把网格前后移一个月；在 `render` 中不合法                     |
| `on("change", handler)`         | 唯一的事件，报告一个日期被选中                               |

### 主题

| 名称                  | 说明                                                                |
| --------------------- | ------------------------------------------------------------------- |
| `set_theme(theme)`    | 用应用自己的主题替换 `gpui-base` 当前生效的语义 token               |
| `ColorToken`          | 已安装调色板定义的语义颜色名称                                      |
| `Theme`               | `cx.theme()` 返回的东西：语义 token，加上 `appearance` 与 `is_dark` |
| `SemanticThemeTokens` | `colors`、`spacing`、`radius`                                       |
| `ColorTokens`         | 每个语义角色一个 `Color`                                            |
| `SpacingTokens`       | `xxs` `xs` `sm` `md` `lg` `xl` `xxl`                                |
| `RadiusTokens`        | `none` `sm` `md` `lg` `xl` `full`                                   |

读主题用 `cx.theme()`。`set_theme` 留在 `gpui-base`，因为主题属于这一层；但修改仍然要求当前存在一次 Host 调用，只能从事件处理器或 task 调用，不能在 `render` 或 layout 中调用。

### 其他类型

| 名称                    | 说明                                                                                         |
| ----------------------- | -------------------------------------------------------------------------------------------- |
| `ScrollbarMode`         | `"scrolling"`、`"hover"` 或 `"always"`                                                       |
| `ItemRange`             | 虚拟列表的可见项，写作半开区间 `[start, end)`                                                |
| `SliderValue`           | 一个数字，或区间 slider 的 `[start, end]`                                                    |
| `InputEvent`            | 文本状态的事件 payload；submit 事件带可选的 `secondary` 与 `shift` 标志                      |
| `OtpEvent`              | 当前为空的 OTP 事件 payload；值从 `OtpState` 读取                                            |
| `PartType`              | `gpui-base` 中没有自身身份的子部件共同使用的 `new()` 形态                                    |
| `Placement`             | `"top"`、`"bottom"`、`"left"` 或 `"right"`，镜像 `gpui_kit::base::Placement`                 |
| `ComponentType`         | `gpui-base` 中带身份的组件构造器共同使用的 `new(id)` 形态                                    |
| `DockPlacement`         | `"center"`、`"left"`、`"right"` 或 `"bottom"`                                                |
| `DockPanel`             | `panels()` 报告的一块面板：`id`、`name`、`placement`、`node`、`index`、`active` 与三个标志位 |
| `DockGroup` / `DockTab` | 一个标签组与它的一个标签页，也就是 `tab_bar` 与 `empty_group` 拿到的东西                     |
| `DockRegion`            | 一侧 dock，也就是 `dock` handler 拿到的东西                                                  |
| `DockTile`              | 一个 tile，bounds 已经解析好                                                                 |
| `DockDrop`              | 被拖动的面板会落在哪里                                                                       |
| `TileResizeSide`        | `"left"`、`"right"`、`"top"`、`"bottom"` 或 `"bottom_right"`                                 |

### 组合模式

其中五个组件不是一个元素，而是一种搭法，上面的表格说不出这件事。下面每段都是能跑起来的最小写法，并且都经过运行时校验。

**受控控件。** `Checkbox`、`Switch`、`Radio` 与 `Toggle` 自身不持有状态：值由你读进去、再写回来。它们什么都不画，所以勾选标记是一个子元素。

```js
Checkbox.new("done")
  .checked(this.checked)
  .on_change((checked, cx) => {
    this.checked = checked;
    cx.notify();
  })
  .child(this.checked ? "done" : "not done");
```

**`Progress` 负责报读，进度条是你的。** root 带的是 role 和屏幕阅读器要念的 `0..=100`，它自己什么都不画。

```js
Progress.new("upload")
  .value(62)
  .child(
    ProgressTrack.new()
      .w(200)
      .h(6)
      .bg(cx.theme().colors.muted)
      .child(ProgressIndicator.new().w(124).h(6).bg(cx.theme().colors.primary)),
  );
```

**滑块是四个部件，四个都不能少**——没有 `SliderIndicator` 的滑块根本拖不动，因为每一个指针位置都是相对它的盒子测量的。四个部件收的是同一份状态。

```js
Slider.new(this.volume).child(
  SliderTrack.new(this.volume)
    .w(200)
    .h(16)
    .child(SliderIndicator.new(this.volume).h(4).bg(cx.theme().colors.primary))
    .child(
      SliderThumb.new(this.volume).w(12).h(12).bg(cx.theme().colors.background),
    ),
);
```

**`Select` 管键盘，`Popup` 管那块面。** root 持有 combobox 的语义与展开状态；列表是放在它里面的一个 `Popup`。它需要两个 focus handle——一个给触发元素，一个给内容——没有第一个，屏幕上就没有任何东西持有键盘。

```js
Select.new("mode")
  .accessibility_label("Mode")
  .open(this.open)
  .track_focus(this.trigger)
  .content_focus_handle(this.list)
  .on_open_change((open, cx) => {
    this.open = open;
    cx.notify();
  })
  .child(
    Popup.new("mode-list", trigger)
      .anchor("bottom_left")
      .when(this.open, (el) => el.content(list)),
  );
```

展开后用方向键移动高亮这件事要你自己写：base 期待里面的东西用自己的按键绑定来跑高亮，而它不会替你跑。零件都在——把 `on_key_down` 放在键盘被移过去的那个 content 元素上，自己移动高亮；或者在自己的 `key_context` 下把 ↑ / ↓ 绑到 action。开箱状态是：指针可用，Escape 关闭，Enter 与 ↓ 展开，高亮不动。

**虚拟列表和它的滚动条按名字配对。** 列表自己不画滚动条，而且配对在运行前不做任何校验，所以两半都要写。

```js
v_flex()
  .relative()
  .h(200)
  .child(
    v_virtual_list(
      "rows",
      rows.length,
      28,
      (index) => rows[index].id,
      (range) =>
        rows.slice(range.start, range.end).map((row) => div().child(row.name)),
    ).size_full(),
  )
  .child(Scrollbar.vertical("rows").absolute().inset_0());
```

**嵌套 View 创建一次，然后作为子元素挂上。** `cx.new` 属于 `init` 或事件处理器；实体在任何接受子元素的位置都能当子元素。

```js
init(props, cx) {
  this.chart = cx.new(PriceChart, { symbol });
}
render() {
  return v_flex().child(this.chart);
}
```

## `gpui-fps` 模块

| 名称            | 说明                                                |
| --------------- | --------------------------------------------------- |
| `fps_monitor()` | 原生 `gpui-fps` HUD，每个窗口共享一个，固定在右上角 |

它的父元素必须设置 `relative()`。HUD 自己拥有完整外观；普通样式与子元素对它不起作用。

## 元素方法

所有元素共享同一个 prototype，所以下面每个方法在任何元素上都能通过类型检查——某个方法实际适合哪个组件，类型并不表达。交给一个不承接它的组件的行为 builder 会被写进日志，而不是被悄悄丢掉。

元素 builder 方法都返回同一个元素，所以一条链就是一个表达式。`map` 是例外：与 GPUI 的 `FluentBuilder.map` 一样，它原样返回回调的结果。元素被用作子元素时即被消费，并且属于构建它的那一趟渲染。

### 组合

| 方法                      | 作用                                                                         |
| ------------------------- | ---------------------------------------------------------------------------- |
| `map(transform)`          | 把当前元素交给 `transform`，并返回其结果；对应 GPUI 的 fluent builder helper |
| `child(value)`            | 添加一个子元素：元素、`Entity`，或字符串、数字、布尔值                       |
| `children(iterable)`      | 按顺序添加多个                                                               |
| `when(condition, branch)` | `condition` 为真时应用 `branch`，让链保持完整                                |
| `id(name)`                | 给这个元素一个稳定的名字，作为它的身份                                       |

### 插槽

插槽不是子元素：元素被组件消费，渲染在组件决定的位置。

| 方法                        | 作用                                                               |
| --------------------------- | ------------------------------------------------------------------ |
| `content(element)`          | `Collapsible`、`Popover`、`HoverCard` 或 `Popup` 的内容            |
| `image(element)`            | `Avatar` 的图片槽，接受一个 `AvatarImage`                          |
| `fallback(element)`         | `Avatar` 的兜底槽，接受一个 `AvatarFallback`                       |
| `header(element)`           | `AccordionItem` 的 header 槽，接受一个 `AccordionHeader`           |
| `panel(element)`            | `AccordionItem` 的 panel 槽，接受一个 `AccordionPanel`             |
| `trigger(element)`          | `Popover` 或 `HoverCard` 的触发器                                  |
| `input(element)`            | `NumberInput` 的编辑器插槽；留空则画出裸编辑器                     |
| `decrement_button(element)` | `NumberInput` 减少按钮的外观——重放到 base 的按钮上，而不是直接渲染 |
| `increment_button(element)` | 增加按钮，重放方式相同                                             |
| `controls_right()`          | 把两个步进按钮叠放在文本右侧                                       |

### 事件

| 方法                             | 交付什么                                                                    |
| -------------------------------- | --------------------------------------------------------------------------- |
| `on_click(handler)`              | 激活时的 `(ClickEvent, cx)`                                                 |
| `on_mouse_move(handler)`         | 指针悬停在元素上时的 `(MouseMoveEvent, cx)`                                 |
| `on_hover(handler)`              | 指针进入与离开时的 `(hovered, cx)`                                          |
| `on_key_down(handler)`           | 该元素持有键盘时按下按键的 `(KeyEvent, cx)`                                 |
| `on_key_up(handler)`             | 同一条焦点路径上松开按键的 `(KeyEvent, cx)`                                 |
| `on_mouse_down(button, handler)` | 按下该按钮时的 `(MouseButtonEvent, cx)`                                     |
| `on_mouse_up(button, handler)`   | 松开时的 `(MouseButtonEvent, cx)`                                           |
| `on_mouse_down_out(handler)`     | 在该元素之外任意位置按下时的 `(MouseButtonEvent, cx)`                       |
| `on_scroll_wheel(handler)`       | 滚轮或触控板滚动时的 `(ScrollWheelEvent, cx)`                               |
| `on_action(action, handler)`     | 该命名 action 被派发到此元素或其内部时的 `(ActionEvent, cx)`                |
| `on_change(handler)`             | 开关变化时的 `(checked, cx)`；新值由脚本保存                                |
| `on_step(handler)`               | `("increment" \| "decrement", cx)`，并且它会**取代**内置的步进              |
| `on_item_click(handler)`         | 虚拟列表某一行被点击时的 `(key, cx)`，按 key 而不是按下标                   |
| `on_open_change(handler)`        | 脚本之外的东西改变了 `Popover` 的打开状态时的 `(open, cx)`                  |
| `on_confirm(handler)`            | 在打开的 `Select` 或 `Combobox` 中按下回车；无参数                          |
| `on_dismiss(handler)`            | 在打开的 `Select` 或 `Combobox` 中按下 Escape，早于 `on_open_change(false)` |
| `on_resize(handler)`             | 可调整组的拖拽结束后的 `(sizes, cx)`                                        |

### Actions 与键绑定

一个 action 是比按键高一层的东西。`cx.bind_keys` 说哪个组合键在什么上下文里意味着 `"save"`，元素上的 `on_action("save", ...)` 说 `"save"` 做什么；菜单项或工具栏按钮用 `window.dispatch_action("save")` 派发同一个名字，就能走到同一个处理器，而两边都不必知道对方存在。

```js
init(_props, cx) {
  cx.bind_keys([{ keystroke: "cmd-s", action: "save", context: "Editor" }]);
}

render(_cx) {
  return div()
    .key_context("Editor")
    .track_focus(this.handle)
    .on_action("save", (event, cx) => this.save(cx));
}
```

`context` 是一个匹配元素 `key_context(...)` 的谓词表达式，所以同一个组合键可以在列表里是一个意思、在编辑器里是另一个意思。同一个元素上注册多个 `on_action` 是可以的，彼此独立；一个它们都没认领的 action 会继续往外层元素传。

上面这组——`on_key_down`、`on_key_up`、四个指针事件、`on_action` 与 `key_context`——接线在 `div`、`h_flex`、`v_flex`、`Button`、`Link`、`Checkbox`、`Switch`、`Radio`、`Toggle`、`Tabs` 与 `Tab` 上。写在其余组件上的处理器会被记录下来但永远到不了 GPUI，日志里会说明——把它包一层，写在外层元素上。

接线了不等于收得到。按键沿焦点路径传递，所以一个不接受脚本焦点句柄的组件——比如 `Tab`——听得到按下、永远听不到按键，无论两者接线得多好。

### 控件状态

| 方法                   | 设置什么                                                                                           |
| ---------------------- | -------------------------------------------------------------------------------------------------- |
| `disabled(value)`      | 阻止激活并报告该状态；外观自己画                                                                   |
| `selected(value)`      | `Button` 的 selected 状态                                                                          |
| `checked(value)`       | `Checkbox`、`Switch` 或 `Radio` 的受控值                                                           |
| `pressed(value)`       | `Toggle` 的受控状态                                                                                |
| `value(percent)`       | 报读的进度百分比，钳制在 `0..=100`；它不会让屏幕上任何东西移动                                     |
| `indeterminate(value)` | 把 `Progress` 的数值从无障碍树里撤下                                                               |
| `open(value)`          | `Collapsible` 是否渲染内容，或浮层是否正在显示                                                     |
| `default_open(value)`  | 非受控的 `Popover` 是否以打开状态开始                                                              |
| `keep_mounted(value)`  | 关闭的 `AccordionPanel` 是否留在树里。默认关；开启后它的内容能跨越一次关闭保住滚动位置或半填的输入 |
| `start(value)`         | `SliderThumb` 是区间 slider 的哪一个滑块                                                           |
| `href(url)`            | `Link` 的绝对 HTTP(S) 目标                                                                         |

### 无障碍

| 方法                           | 报读什么                                                       |
| ------------------------------ | -------------------------------------------------------------- |
| `accessibility_label(text)`    | 屏幕阅读器读出的内容；纯图标控件没有它就什么都不会被读出       |
| `role(name)`                   | 这个元素把自己报读成什么——仅限朴素元素、`Button` 与 `Checkbox` |
| `aria_selected(value)`         | 脚本自己搭的列表里某一项的选中状态                             |
| `aria_active_descendant()`     | 在祖先持有键盘时，把本元素报读为当前焦点项                     |
| `set_position(position, size)` | 从 1 开始的位置与总数——“第 2 个 tab，共 5 个”                  |
| `row_count(count)`             | `Table` 的总行数，包含未渲染的行                               |
| `column_count(count)`          | `Table` 的总列数                                               |
| `aria_level(level)`            | `AccordionHeader` 报读的标题层级，默认 3；只报读，不改字号     |
| `axis(value)`                  | `RadioGroup` 或 `ToggleGroup` 的方向；只有语义，不做任何布局   |
| `tooltip(text)`                | 只对指针有效的悬停说明，不能替代 `accessibility_label`         |

### 焦点与键盘

| 方法                           | 作用                                                     |
| ------------------------------ | -------------------------------------------------------- |
| `track_focus(handle)`          | 让这个元素成为该 handle 所指的对象                       |
| `content_focus_handle(handle)` | `Select` 或 `Combobox` 打开时把键盘移到哪里              |
| `tab_index(index)`             | 这个元素在 Tab 顺序中的位置；同时也把它变成一个 tab stop |
| `tab_stop(value)`              | Tab 能否落到这里，不改变它在顺序中的位置                 |

### 滚动与面板

| 方法                                                | 作用                                           |
| --------------------------------------------------- | ---------------------------------------------- |
| `overflow_scroll()`                                 | 接管双轴的滚轮与触控滚动                       |
| `overflow_x_scroll()` / `overflow_y_scroll()`       | 单轴上的同一件事                               |
| `overflow_scrollbar()`                              | 双轴滚动并绘制基础层的滚动条                   |
| `overflow_x_scrollbar()` / `overflow_y_scrollbar()` | 单轴上的同一件事                               |
| `mode(value)`                                       | `Scrollbar` 的显示策略；不写则跟随主题         |
| `scroll_size(width, height)`                        | `Scrollbar` 据以计算滑块的内容尺寸             |
| `viewport_from_layout()`                            | 让 `Scrollbar` 从自身的盒子取 viewport         |
| `track_scroll(handle)`                              | 给虚拟列表一个脚本可以驱动的滚动位置           |
| `with_item_to_measure_index(index)`                 | 虚拟列表在它滚动的那个轴上测量哪一项           |
| `size_range(min, max?)`                             | `resizable_panel()` 可被拖拽的范围，单位为像素 |

### 锚定浮层

| 方法                      | 设置什么                                        |
| ------------------------- | ----------------------------------------------- |
| `anchor(value)`           | 哪个角固定在触发元素上；无论怎样都会被钳进窗口  |
| `mouse_button(value)`     | 哪个指针按键打开 `Popover`                      |
| `open_delay(ms)`          | 指针要在 `HoverCard` 触发器上停留多久；默认 600 |
| `close_delay(ms)`         | `HoverCard` 关闭前等待多久；默认 300            |
| `overlay_closable(value)` | 在打开的 `Popover` 之外按下是否将其关闭         |

### Dock 命令

dock 的 chrome 画出来的元素*做什么*。缓存的 chrome 描述没有脚本事件处理器的生命周期，所以其中不能注册事件处理器——取而代之的是不携带任何脚本值的命令，由 base 完成实际动作。每一个的第一个参数都是它所在 handler 拿到的那个对象；它们只能挂在 `div`、`h_flex` 或 `v_flex` 上。

| 方法                           | 触发 | 作用                                          |
| ------------------------------ | ---- | --------------------------------------------- |
| `select_tab(group, index)`     | 点击 | 显示那个标签页                                |
| `close_panel(group, panel_id)` | 点击 | 关闭该面板（如果它所在的 group 允许）         |
| `toggle_zoom(group)`           | 点击 | 放大 group，或还原                            |
| `drag_tab(group, index)`       | 拖动 | 让该元素成为这个标签页的拖动源                |
| `drop_tab(group, index?)`      | 放下 | 在此接收被拖来的面板；不给 index 就追加到末尾 |
| `toggle_dock(dock)`            | 点击 | 展开或收起这侧 dock                           |
| `resize_dock(dock)`            | 拖动 | 拖动 dock 的边；每个位置都由 base 钳制        |
| `move_tile(tile)`              | 拖动 | 在画布上移动这个 tile                         |
| `resize_tile(tile, side)`      | 拖动 | 拖动某条边或某个角                            |
| `raise_tile(tile)`             | 按下 | 把这个 tile 提到最上层                        |
| `toggle_tile_zoom(tile)`       | 点击 | 让 tile 放大占满所在 dock                     |
| `close_tile(tile)`             | 点击 | 关闭这个 tile                                 |

### Dock chrome

六个 handler，全都可选，且只能挂在 `dock_area(...)` 上。每一个都会先在 GPUI 的 layout pass 内部被调用，拿到的是 base 已经解析好的状态；描述会缓存到该状态或 handler 改变为止。

| 方法                           | 画什么                                               |
| ------------------------------ | ---------------------------------------------------- |
| `tab_bar(handler)`             | 一个 group 当前显示面板上方的标签栏                  |
| `empty_group(handler)`         | 没有可显示面板的 group 显示什么                      |
| `drop_indicator(handler)`      | 被拖动的面板会落在哪里                               |
| `dock(handler)`                | 一侧 dock 包住内容的外框；把 `dock_content()` 放进去 |
| `tile_drag_bar(handler)`       | 拖动 tile 用的那条拖拽条                             |
| `tile_resize_handles(handler)` | tile 的缩放把手                                      |

### 动效

| 方法                           | 作用                                           |
| ------------------------------ | ---------------------------------------------- |
| `transition(property, policy)` | 完全在原生 GPUI 代码里，对之后的目标变化做动画 |
| `spring(property, policy?)`    | 改用弹簧                                       |

property 取 `"opacity"`、`"width"`、`"height"`、`"left"`、`"top"` 之一，每一帧都不会进入 JavaScript。

### 样式模板

每一个都接受一个函数，函数收到一个游离的元素用来收集样式；返回值会被忽略，所以写成一条链或写成块状函数体都可以。

| 方法                         | 作用于什么                                                     |
| ---------------------------- | -------------------------------------------------------------- |
| `hover(declare)`             | 指针悬停在元素上时                                             |
| `active(declare)`            | 元素被按下时                                                   |
| `focus(declare)`             | 元素持有焦点时                                                 |
| `range_style(declare)`       | `SliderIndicator` 已填充的部分——只管它长什么样，从不管它在哪里 |
| `cell_style(declare)`        | `OtpInput` 的每个格子；没有它屏幕上什么都没有                  |
| `cell_active_style(declare)` | 叠在上面一层，用于下一个数字将落入的那个格子                   |
| `caret_style(declare)`       | 那个格子为空时，里面闪烁的光标                                 |

### 样式方法

元素上其余的一切都是样式。它们分成两族，而且从不重叠：

- **带参数的方法**，手工绑定：size、padding、margin、position、flex、border、radius 与 paint 各族。每个方法接受哪种长度类型跟随它的 Rust 签名，所以 `.p("auto")` 是类型错误，理由与它在运行时抛异常完全相同。
- **无参方法**，从 GPUI 的反射表生成：`flex_col`、`items_center`、`gap_2`、`rounded_md`、`text_sm`、`size_full`、`truncate` 以及这一族的其余成员。生成的声明就是当前构建所用 GPUI 版本的完整清单。

两者都记录在 [Styling](./styling.md) 里，还有长度与颜色的语法，以及调色板定义的 token。

## HostModule

Host 在 Rust 侧注册的模块，按名字 import，和其它模块没有区别：

```js
import { quotes } from "market";
```

它不属于任何内建模块。生成的类型声明里每个注册过的模块各有一段 `declare module`，所以模块名和每一个导出名都会被检查。见 [HostModule](./host-module.md)。

---
title: 元素
description: 构造器、用 child / children / when 组合，以及元素描述为什么只能使用一次。
order: 4
---

# Elements

`gpui-shell` 里的元素是一段**描述**，不是一个对象。它只在一次渲染中存在，被使用时即被消费。本页讲能构建什么、怎么组合，以及一段描述被用了两次时运行时会做什么。

## 构造器

每个模块只装它自己那个包提供的东西：

```js
import { div, svg, image } from "gpui-kit";
import {
  h_flex,
  v_flex,
  Button,
  Link,
  Checkbox,
  Switch,
  Input,
  InputState,
} from "gpui-base";
import { fps_monitor } from "gpui-fps";
```

函数是小写的，组件类型首字母大写并通过 `.new` 构造。这与 Rust 侧一一对应：那边 `div()` 同样是自由函数，`Button::new(id)` 同样是类型上的关联函数。

| 构造器             | 来自        | 产出                                                            |
| ------------------ | ----------- | --------------------------------------------------------------- |
| `div()`            | `gpui-kit`  | 自身不带布局的元素                                              |
| `value`            | `gpui-kit`  | 文本元素，参数会被转成字符串                                    |
| `svg(path)`        | `gpui-kit`  | 来自应用自身目录、跟随主题着色的矢量图标                        |
| `image(path)`      | `gpui-kit`  | 来自应用自身目录的全彩图片                                      |
| `h_flex()`         | `gpui-base` | 一行                                                            |
| `v_flex()`         | `gpui-base` | 一列                                                            |
| `Button.new(id)`   | `gpui-base` | base 的 `Button`：激活、焦点、disabled 与 selected 状态，无样式 |
| `Link.new(id)`     | `gpui-base` | 可聚焦的外部 HTTP(S) 链接；用 `.href(url)` 设置目标             |
| `Checkbox.new(id)` | `gpui-base` | base 的受控 checkbox，无样式也无勾选标记                        |
| `Switch.new(id)`   | `gpui-base` | base 的受控 switch，无样式                                      |
| `Input.new(state)` | `gpui-base` | 由 [`InputState`](./state.md#留存状态) 支撑的文本框             |
| `fps_monitor()`    | `gpui-fps`  | 原生 `gpui-fps` 性能 HUD，每个窗口共享一个 monitor              |

这是入门够用的一组，不是全部。base 绑定的组件——`Select`、`Combobox`、`Tabs`、`Table`、`VirtualList`、`Slider`、`Popover`、`Avatar`、`Accordion`、`Pagination`、`CalendarState` 等等——完整清单在 [API 参考](./api.md#gpui-base-模块)里。

### 性能监视器

`fps_monitor()` 直接公开原生 `gpui-fps` HUD，不会把采样或绘制搬进 JavaScript。monitor 在首次使用时创建，并按窗口复用。一个窗口最多渲染一次，并把它放在设置了 `relative()` 的父元素中：

```js
div().relative().size_full().child(content).child(fps_monitor());
```

默认固定在右上角。可以沿用已有的 anchor 取值调整位置，例如 `fps_monitor().anchor("bottom_left")`。HUD 自己拥有完整外观，普通元素样式、children 和交互状态不会作用于它。

### 为什么是 `.new(id)` 而不是 `new Button(id)`

JavaScript 的习惯写法是 `new Button(id)`。运行时不提供它，理由正是本页的主题：`new` 承诺的是一个有身份的对象——可以保存、可以挂在实例上、可以再次使用。而描述恰恰不是这种东西。`Button.new(id)` 读起来是“构造一段描述”，它做的也正是这件事，并且与 Rust 侧一字不差。

View 是相反的情形，用的就是标准写法：`class Counter extends View`。 View 确实有身份、有跨帧状态，并且由 GPUI 拥有。同一份文件里出现两种构造形态，是因为这两类东西的生命周期本来就不同。

### id

`Button`、`Link`、`Checkbox` 与 `Switch` 的 `id` 用于跨渲染标识元素，GPUI 据此保留焦点与元素状态。请保持它稳定，并在兄弟节点之间唯一——用 `` `item-${item.id}` ``，而不是一个会在列表被筛选时移位的数组下标。

其余元素——`div`、`h_flex`——的身份是**它在这次渲染所构建的树里所处的位置**。只要树的形状不变，这就够用；而一旦上方多出一个条件子节点，它下面的每个元素都会移位，按下状态、焦点以及其他按身份记录的东西都跟着移位。

`.id(name)` 用来说明“这是哪个元素”，而不是“它落在了哪里”：

```js
div()
  .id("toolbar")
  .active((el) => el.opacity(0.7));
```

凡是身份必须扛得住邻居变化的元素，都给它取个名字。`Button`、`Link`、`Checkbox` 与 `Switch` 已经从 `new(id)` 拿到了身份，会忽略这里的名字——并且是给出警告，而不是默不作声。

### 文本

**字符串本身就是元素。** GPUI 为 `&str`、`String` 与 `SharedString` 实现了 `IntoElement`，所以文本的写法就是把字符串交给承载它的元素，没有 `text()` 可调：

```js
v_flex().child(`${this.remaining} of ${this.items.length} remaining`).child(42);
```

样式由承载它的元素带，和 Rust 那边完全一致：

```js
div().text_size(12).font_semibold().child("AAPL");
```

字符串子元素最终变成一个包含它的 `div`——这正是 `div().child(s)` 已经说明的事。

### 图片

```js
svg("icons/check.svg").w(14).h(14).flex_none();
image("images/brand.png").w(120).h(40);
```

两种路径都相对于**应用根目录**——也就是交给 `gpui-shell` 的那个目录——而不是相对于调用构造器的文件。这个不对称常常让人意外，所以值得直说：`import "./ui.js"` 相对于发起 import 的文件解析，和所有 JavaScript 模块系统一样；而 `svg("icons/check.svg")` 与 `image("images/brand.png")` 相对于应用根目录解析，和 Web 应用的 public 目录一样。运行时无法知道是哪个模块调用了 asset 构造器，因此按文件解析的资源路径对它并不可得。

越出应用目录的路径会被拒绝。缺失的文件会按路径去重报告一次，并附上查找位置，而不是安静地什么都不画。

单个 asset 最多 16 MiB。列举 asset tree 时最多接受 10,000 个 entry 与累计 1 MiB 的 UTF-8 文件名，避免 asset discovery 无界增长内存。

单色图标应使用 `svg()`：它会继承周围的文字颜色，所以深色按钮里的图标不用脚本说第二遍就是浅色的。Logo、照片或插画等需要保留源文件颜色的内容应使用 `image()`。

```js
renderIcon(cx) {
  return div()
    .bg(cx.theme().colors.foreground)
    .text_color(cx.theme().colors.surface)
    .child(svg("icons/check.svg").w(11).h(11));  // 以 surface 绘制
}
```

## 组合

| 方法                       | 作用                                 |
| -------------------------- | ------------------------------------ |
| `.child(element)`          | 添加一个子元素，该子元素随即被消费   |
| `.children(iterable)`      | 按顺序添加多个                       |
| `.when(condition, branch)` | 仅当 `condition` 为真时应用 `branch` |

```js
v_flex()
  .gap(8)
  .child(this.header())
  .children(this.visible().map((item) => this.row(item)))
  .when(this.items.length === 0, (el) => el.child("Nothing yet"));
```

`.when` 的存在是为了不让一个条件把链断成两截。`branch` **必须返回该元素**——不返回的分支会立刻抛异常，而不是悄悄丢掉它构建的一切：

```text
when(...) must return the element
```

这与 GPUI 自己的 `FluentBuilder`，以及本仓库 Rust 侧“元素构建保持一条流式链”的风格规则同源。

如果条件是在两个元素之间二选一，普通三元表达式比 `when` 更清楚：

```js
.child(
  visible.length === 0
    ? emptyState("No items yet", "Type above and press Add.")
    : v_flex().children(visible.map((item) => this.row(item))),
)
```

## 行为方法

这些不是样式。它们把状态报告给基础层，由基础层处理交互，外观仍然交给你。

| 方法                              | 用于                                | 作用                                               |
| --------------------------------- | ----------------------------------- | -------------------------------------------------- |
| `.on_click(handler)`              | `Button`                            | `handler(event, cx)`，点击**以及**键盘激活都会触发 |
| `.on_change(handler)`             | `Checkbox`、`Switch`                | `handler(checked, cx)`，由脚本保存新值             |
| `.disabled(value)`                | `Button`、`Checkbox`、`Switch`      | 阻止激活并报告该状态                               |
| `.selected(value)`                | `Button`                            | 报告 selected 状态                                 |
| `.checked(value)`                 | `Checkbox`、`Switch`                | 受控值                                             |
| `.accessibility_label(text)`      | `Button`、`Checkbox`                | 屏幕阅读器读出的内容                               |
| `.tooltip(text)`                  | `div`、`h_flex`、`v_flex`、`Button` | 指针停留后显示的说明文字                           |
| `.id(name)`                       | `div`、`h_flex`、`v_flex`           | 一个稳定的身份，取代“在树中的位置”                 |
| `.overflow_scrollbar()`           | `div`、`h_flex`、`v_flex`           | 双轴滚动并绘制原生 scrollbar                       |
| `.overflow_x_scrollbar()`         | `div`、`h_flex`、`v_flex`           | 水平滚动并绘制原生 scrollbar                       |
| `.overflow_y_scrollbar()`         | `div`、`h_flex`、`v_flex`           | 垂直滚动并绘制原生 scrollbar                       |
| `.on_key_down(handler)`           | [可接输入的元素](#哪些元素接了输入) | 该元素持有键盘时的 `handler(event, cx)`            |
| `.on_key_up(handler)`             | [可接输入的元素](#哪些元素接了输入) | 松开时同上                                         |
| `.on_mouse_down(button, handler)` | [可接输入的元素](#哪些元素接了输入) | 按下 `"left"`、`"right"` 或 `"middle"`             |
| `.on_mouse_up(button, handler)`   | [可接输入的元素](#哪些元素接了输入) | 松开                                               |
| `.on_mouse_down_out(handler)`     | [可接输入的元素](#哪些元素接了输入) | 在该元素**之外**任意位置按下                       |
| `.on_scroll_wheel(handler)`       | [可接输入的元素](#哪些元素接了输入) | 滚轮与触控板滚动                                   |
| `.on_action(action, handler)`     | [可接输入的元素](#哪些元素接了输入) | 命名 action 被派发到它或它内部                     |
| `.key_context(name)`              | [可接输入的元素](#哪些元素接了输入) | 该元素与其子树所处的按键绑定上下文                 |

disabled、selected 与 checked 的**外观**由你来画。基础层只报告状态，脚本不说就什么都不会变：

```js
Button.new("clear")
  .disabled(this.completed === 0)
  .when(this.completed === 0, (el) => el.opacity(0.4))
  .child("Clear completed");
```

`.accessibility_label` 对纯图标控件最重要——没有它，这类控件什么都不会被读出来：

```js
Button.new(`remove-${item.id}`)
  .accessibility_label(`Remove “${item.caption}”`)
  .child(svg("icons/trash.svg").w(14).h(14));
```

### 受控值只报告意图

base 的 checkbox 不会自己改状态。它只报告用户的请求，由脚本决定：

```js
Checkbox.new(`item-${item.id}`)
  .checked(item.done) // 值来自脚本状态
  .on_change((done, cx) => {
    // 回调只是一个请求
    this.toggle(item.id, done, cx);
  })
  .child(indicator(item.done))
  .child(label(item.caption));
```

运行时绝不会替脚本悄悄维护一个 checked 标志。如果它这么做，脚本作者与 Rust 作者会对同一个控件持有不同的心智模型，而这两类作者共存于同一个应用里。

### 事件对象

`on_click` 的处理函数收到的是一个普通对象，字段名与 Rust 结构一致：

```js
.on_click((event, cx) => {
  // event.click_count === 1
  // event.modifiers === { shift, control, alt, platform }
});
```

`platform` 在 macOS 上是 Command，其他平台是 Windows 键。这里只暴露基础层已经归一化过的语义——Base 把“回车激活按钮”与“点击按钮”归为同一个回调，脚本看不到这个差别。

按键处理器拿到的组合键有两种形态。`keystroke` 是整串，拼法和写绑定时一致；`key` 与 `modifiers` 是同一个组合键拆开的样子，只关心其中一半时用它：

```js
.on_key_down((event, cx) => {
  if (event.keystroke === "cmd-s") {
    this.save();
    cx.stop_propagation();
  }
});
```

**平台修饰键在所有平台上都拼作 `cmd`**，Linux 与 Windows 也一样。GPUI 会按编译目标平台来拼——`cmd-`、`super-`、`win-`——这对给人读的 keymap 是对的，对给程序比较的字符串是错的：同一份脚本要在三个平台上跑，`event.keystroke === "cmd-s"` 必须在三个平台上是同一件事。

指针处理器拿到按钮、当前连击次数以及落点。`local_position` 与 `bounds` 在元素第一次绘制之前是没有的：

```js
.on_mouse_down("right", (event, cx) => {
  // event.button === "right"
  // event.click_count === 1
  // event.local_position?.x  —— 相对这个元素
  this.openMenuAt(event.position, cx);
});
```

滚动处理器拿到的一律是像素；设备按行上报时，原始行数也在：

```js
.on_scroll_wheel((event, cx) => {
  this.offset += event.delta.y;      // 一律是像素
  // event.delta_lines?.y            —— 只有设备按行上报时才有
  cx.notify();
});
```

### 哪些元素接了输入

上面这八个方法是 GPUI 自己的 `InteractiveElement` 构建器，shell 把它们装在 `div`、`h_flex`、`v_flex`、`Button`、`Link`、`Checkbox`、`Switch`、`Radio`、`Toggle`、`Tabs` 与 `Tab` 上。

其余组件各自构建自己的 base 类型、挂自己的监听器，所以写在它们上面的处理器会被记进描述、但永远到不了 GPUI。日志里会说明，而不是留给你自己去发现：

```text
`on_key_down` is not wired on a Select: the shell installs GPUI's input
listeners on the element it owns outright, which is a plain `div`, `h_flex`
or `v_flex`. Wrap it and write `on_key_down` on the wrapper
```

**接线了不等于收得到。** 按键沿焦点路径传递，指针沿 hitbox 传递，所以一个不接受焦点句柄的组件——`Tab` 就是——听得到按下、永远听不到按键，无论两者接线得多好。哪些组件接受焦点句柄，见[焦点与无障碍](#焦点与无障碍)。

### Actions 与按键绑定

action 是比按键高一层的东西。`cx.bind_keys` 说哪个组合键在什么上下文里意味着 `"save"`；`on_action` 说 `"save"` 做什么。菜单项或工具栏按钮派发同一个名字就会走到同一个处理器，而两边都不必知道对方存在：

```js
init(_props, cx) {
  cx.bind_keys([
    { keystroke: "cmd-s", action: "save", context: "Editor" },
    { keystroke: "ctrl-k ctrl-c", action: "comment", context: "Editor" },
  ]);
}

render(_cx) {
  return div()
    .key_context("Editor")
    .track_focus(this.handle)
    .on_action("save", (_event, cx) => this.save(cx))
    .child(
      Button.new("save")
        .on_click(() => window.dispatch_action("save"))
        .child("Save"),
    );
}
```

`context` 是一个匹配元素 `key_context(...)` 的谓词，所以同一个组合键可以在列表里是一个意思、在编辑器里是另一个意思。keymap 属于应用而不属于某个窗口，所以在一个 View 里绑的组合键，在它的谓词匹配的任何地方都生效。

同一个元素上注册多个 `on_action` 是可以的，彼此独立。一个它们都没认领的 action 会继续往外层传——这正是内层面板处理 Save、外层窗口处理 Quit 的做法。

整份绑定列表会在安装任何一条之前先校验完：因为第四条有拼写错误而只装了一半的 keymap，比一条都没装更糟，而脚本没有办法知道装进去的是哪一半。

::: tip 事件处理器请用箭头函数
箭头函数不绑定自己的 `this`，所以处理函数里的 `this` 仍然是 View 实例。用 `function () {}` 写会拿到错误的 `this`。这是为本运行时写脚本时最常见的一处错误，人和模型都一样。
:::

## 焦点与无障碍

焦点目标由脚本自己持有。`cx.focus_handle()` 创建一个——对应 GPUI 的 `App::focus_handle`，那边并没有 `FocusHandle::new` 可供镜像——它像 [`InputState`](./state.md#留存状态) 一样挂在 View 上，再用 `.track_focus(handle)` 交给某个元素：

```js
init(props, cx) {
  this.search = cx.focus_handle();
}

render() {
  return Button.new("search")
    .tab_index(1)
    .track_focus(this.search)
    .child("Search");
}
```

`cx.focus_handle()` 需要一次活的 Host 调用；而在 `render` 里创建的 handle 每一帧都是新的，它所跟踪的焦点会被下一次重绘丢掉。所以它属于 `init` 或事件处理器，在 `render` 里调用会抛错。

| handle 上的方法       | 回答什么                     |
| --------------------- | ---------------------------- |
| `handle.focus()`      | 把键盘移到跟踪它的那个元素上 |
| `handle.is_focused()` | 那个元素此刻是否持有键盘     |
| `handle.release()`    | 释放这个 handle              |

`Tab` 与 `Shift-Tab` 由窗口根 View 处理：它按下表的顺序双向行走，并遵守已打开的 dialog 或 sheet 的 focus trap。

| 方法                        | 作用于                                                             | 效果                                                                                       |
| --------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| `.track_focus(handle)`      | `div`、`h_flex`、`v_flex`、`Button`、`Checkbox`、`Radio`、`Toggle` | 把元素绑定到脚本持有的 handle                                                              |
| `.tab_index(n)`             | 上述这些，外加 `Link`、`Switch`                                    | 元素在窗口 Tab 顺序中的位置；同时也把它变成一个 tab stop                                   |
| `.tab_stop(value)`          | 与 `tab_index` 相同                                                | Tab 是否能落到它上面。`false` 保留它在顺序中的位置但不可达                                 |
| `.role(name)`               | `div`、`h_flex`、`v_flex`、`Button`、`Checkbox`                    | 屏幕阅读器把这个元素读作什么                                                               |
| `.aria_selected(value)`     | `div`、`h_flex`、`v_flex`                                          | 脚本自己搭的列表里某一项的选中状态                                                         |
| `.aria_active_descendant()` | `div`、`h_flex`、`v_flex`                                          | 在祖先持有键盘时，把本元素报读为当前焦点项——比如输入框保持焦点的 combobox 中被高亮的那一项 |

三张表的范围不同，是因为组件本身不同。`Button`、`Checkbox`、`Radio`、`Toggle` 的焦点 handle 由一个你可以替换的值构建；`Link` 与 `Switch` 自己构建 handle，且没有可替换它的 builder。除 `Button` 与 `Checkbox` 之外的每个组件都自带 role——`Tab` 就是 tab，`Radio` 就是 radio——只有这两个把 role 当作可覆盖项，这正是「让一个按钮被读作菜单项」得以成立的原因。组件无法承接的调用会**写进日志**，而不是被悄悄丢掉：

```text
`role` is not wired on a Tab: base's Tab owns this part of its own focus and
accessibility. Put it on an element around it
```

朴素元素六个方法全都接受，脚本正是靠它们搭出 base 没有对应组件的 listbox、toolbar 或 dialog：

```js
div()
  .id(`cadence-${index}`)
  .role("list_box_option")
  .aria_selected(index === this.chosen)
  .when(index === this.chosen, (el) => el.aria_active_descendant())
  .child(name);
```

role 的取值逐字镜像 `gpui_kit::Role` 的 snake_case 拼写——`list_box`、`list_box_option`、`combo_box`、`menu_item`——整套取值以 `Role` 联合类型写在 `gpui-kit.d.ts` 里，编辑器能补全；不在其中的名字会在调用处失败：

```text
unknown accessibility role `listbox`; the names mirror gpui_kit::Role in snake_case
— see the Role type in gpui-kit.d.ts
```

## 元素是一次性的

这条规则最容易让新读者意外，所以下面写清它长什么样、以及为什么成立。

```js
const row = h_flex().child("hello");

v_flex().child(row).child(row); // 抛异常
```

```text
element `h_flex` was already added to a parent; elements are single-use values
```

跨帧保存也是同样的失败：

```js
init() {
  this.header = h_flex().child("Todo");   // 错误
}

render() {
  return v_flex().child("Todo list").child(this.header);
}
```

```text
this element belongs to a previous render pass; elements are single-use values
and must be rebuilt each time render runs
```

有一处毛刺值得知道：arena 每一趟都会清空并复用下标，所以一个过期元素偶尔会正好持有运行时刚分配给“它要挂上去的那个节点”的下标。误用仍然会被抓到，但信息变成 `an element cannot be added to itself`。两者含义相同——这个元素属于一趟已经结束的渲染。

### 为什么

这条限制来自 GPUI 本身：`RenderOnce::render` **按值**取走 `self`，`.child()` 也按值取走子元素。Rust 里编译器用移动语义强制这一点：使用已移动的值是编译错误。JavaScript 既没有移动语义也没有编译器，于是运行时在运行期强制同一条规则——而描述 arena 本来就有做这件事所需的记录，因为节点被挂载的那一刻就会被标记为已有父节点。

另一种做法是在重复使用时复制描述。这一条被否决了：它会让同一段脚本在 Rust 与 JavaScript 里含义不同，而重复使用几乎总是错误而非本意。

### 可行的写法

在 `render` 里构建，把重复部分抽成**每次返回新元素的函数**：

```js
const label = (value, cx) => div().text_size(12).text_color(cx.theme().colors.foreground).child(value);

render(cx) {
  return v_flex()
    .child(label("first", cx))
    .child(label("second", cx));
}
```

[示例应用](https://github.com/longbridge/gpui-kit/tree/main/examples/js_todolist)就是这样写的：`ui.js` 把 `button`、`label`、`icon`、`checkbox` 等导出为函数，`main.js` 调用它们。读起来像一个组件库，而且不花什么代价——一次函数调用就是一段新描述的来源。

## 回调属于它所在的那次渲染

传给 `.on_click` 的处理函数属于那次渲染产出的那份描述——而不是属于某一帧。那份描述会[被之后的每一帧复用，直到有东西让它失效](./state.md#render-什么时候执行)，处理函数在这期间一直可调用。描述里只记录一个 id；Rust 装配的闭包持有对运行时的弱引用加上这个 id。

被替换掉的那份描述会多保留一代，因为事件可能针对一个已经被取代的帧派发。再晚到达的事件会被丢弃并记一条 `debug` 日志，而不是报错——作者没有做错什么，也没有什么可修。

实际后果是：渲染期注册的回调不是订阅。需要活得比本次渲染更久的东西——比如响应输入框的 `change` 事件——见 [State and Views](./state.md#输入事件)。

## 未知方法是错误

既不是样式也不属于上面那批行为方法的调用，会在调用点失败；如果有相近的名字，会给出建议：

```text
unknown element method `items_centre` (did you mean `items_center`?)
```

```text
unknown element method `on_clicked`; it is neither a style method nor one of
child, children, when, on_click, on_change, disabled, selected, checked, id
```

这件事比看上去重要。拼错的样式名不会改变画面——它只是没起作用——没有诊断的话完全不可见。运行时如何在不给每次渲染加负担的前提下产生这条信息，见 [Styling](./styling.md#未知方法)。

## 还没有的东西

元素接口现在已经包含 Tabs、Table、Progress、表单控件、Popover/HoverCard
锚定浮层、Textarea、Scrollbar、PathBuilder、VirtualList，以及一个由脚本绘制
chrome 的 [dock area](./dock.md)。仍刻意缺少：

- 更高层的 List、Tree 系统，以及尚未接入的其他 `gpui-base` 组件；
- `gpui.memo`——它能让未变化的子树跳过重建描述的那部分脚本工作。

焦点现在归脚本所有，但还不完整。仍然缺少的部分：

- **复合控件内部的键盘导航，需要自己写。** Tab 与 Shift-Tab 能在控件之间移动；在 listbox、菜单或 tab list *内部*移动的方向键不会自动出现。零件现在都有了——`on_key_down`、`cx.bind_keys` 与 `key_context`——但把 ↑ / ↓ 变成高亮移动这件事仍然是脚本的活。
- **窗口尚无焦点时的第一次 Tab。** 只要还没有任何元素持有焦点，根 View 的 Tab 绑定就没有可达的分发路径；焦点必须先以别的方式进入——点击，或者 `handle.focus()`。
- **`Tab`、`Tabs`，以及 table、group、progress 的各个部件**不在 Tab 顺序里。base 本身就把它们排除在键盘焦点之外，对它们调用 `tab_index` 会被记录而不是被承接。
- **`Link` 与 `Switch` 上的 `track_focus`**，原因相同：它们自己构建 handle，且不暴露替换它的 builder。

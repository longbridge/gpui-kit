---
title: 样式
description: 流式样式接口、长度与颜色语法、语义主题 token，以及 hover / active / focus 状态样式。
order: 5
---

# Styling

呈现权在脚本，所以一个应用的大部分代码都写在这里。所有元素接受同一套样式接口，写成一条流式链——与 Rust 侧写的一模一样：

```js
render(cx) {
  return v_flex().size_full().bg(cx.theme().colors.surface).p(12).gap(8).rounded(6);
}
```

```rust
// 同一件事，Rust 侧、基于 gpui-base。
v_flex().size_full().bg(surface).p(px(12.)).gap(px(8.)).rounded(px(6.))
```

## 统一的样式 API

所有样式方法都通过同一套链式 API 调用。根据 GPUI 是否能够自动导出方法信息，实现分为以下两类。

**无参方法来自 GPUI 的反射表。** `flex_col`、`items_center`、`gap_2`、`rounded_md`、`text_sm`、`size_full`、`font_semibold`、`truncate`、`cursor_pointer`——整个家族都取自 `gpui_kit::base::styled_ext_reflection_methods` 与 `gpui_kit::styled_reflection::methods`，零维护成本。这些名字没有一个写在运行时的任何地方。上游 GPUI 新增一个样式方法，脚本接口就有了，生成的 `gpui-kit.d.ts` 也有了。

本文写作时的这次构建里有 **3,148** 个。这个数字就是 GPUI 当前有多少个 `fn(self) -> Self` 形态的样式方法，GPUI 变它就变。`gpui-shell types` 会打印你这次构建的准确数字。

**有参方法无法被反射**，所以有 **57** 个是手工绑定的。这份列表是样式层里唯一手工维护的表，而且刻意保持很小。

两类方法不会重名。测试会检查每个名字只出现一次，发现冲突时直接让构建失败。

## 长度

裸数字是像素。字符串自带单位。

```js
.p(12)          // 12px
.w("50%")       // 父容器的一半
.h("auto")
.gap("0.5rem")
```

某个方法接受其中哪些，取决于**它的 Rust 签名**——因为正是那个签名在拒绝不合法的形式。GPUI 有三种互相嵌套的长度类型，运行时保留了这个区分而没有把它拍平：

| 类型             | 接受                                          | 拒绝             |
| ---------------- | --------------------------------------------- | ---------------- |
| `Length`         | 数字、`"12px"`、`"1.5rem"`、`"50%"`、`"auto"` | —                |
| `DefiniteLength` | 数字、`"12px"`、`"1.5rem"`、`"50%"`           | `"auto"`         |
| `AbsoluteLength` | 数字、`"12px"`、`"1.5rem"`                    | 百分比、`"auto"` |

```text
`p` cannot be "auto"; it expects a definite length such as 12 or "50%"
```

```text
`rounded` expects an absolute length such as 8 or "0.5rem";
percentages and "auto" are not allowed here
```

`"auto"` 的内边距和百分比的圆角，在底层布局引擎里没有含义；接受它们的运行时就必须自己发明一个含义。

### 有参方法一览

| 家族   | 方法                                                                       | 参数             |
| ------ | -------------------------------------------------------------------------- | ---------------- |
| 尺寸   | `w` `h` `size` `min_w` `min_h` `min_size` `max_w` `max_h` `max_size`       | `Length`         |
| 内边距 | `p` `px` `py` `pt` `pb` `pl` `pr`                                          | `DefiniteLength` |
| 外边距 | `m` `mx` `my` `mt` `mb` `ml` `mr`                                          | `Length`         |
| 定位   | `inset` `top` `bottom` `left` `right`                                      | `Length`         |
| Flex   | `gap` `gap_x` `gap_y`                                                      | `DefiniteLength` |
| Flex   | `flex_basis`                                                               | `Length`         |
| Flex   | `flex_grow` `flex_shrink`                                                  | 数字             |
| 边框   | `border` `border_t` `border_b` `border_l` `border_r` `border_x` `border_y` | `AbsoluteLength` |
| 圆角   | `rounded` 及 `_t` `_b` `_l` `_r` `_tl` `_tr` `_bl` `_br` 各形式            | `AbsoluteLength` |
| 绘制   | `bg` `text_color` `text_bg` `border_color`                                 | 颜色             |
| 绘制   | `text_size`                                                                | `AbsoluteLength` |
| 绘制   | `line_height`                                                              | `DefiniteLength` |
| 字体   | `font_family`                                                              | 字符串           |
| 绘制   | `opacity`                                                                  | 数字             |

`line_height` 是唯一值得专门记住的例外：**裸数字是倍数，不是像素**。`line_height(1.45)` 表示字号的 1.45 倍，因为业界其他地方都是这个含义，而 1.45px 从来不是任何人的意思。字符串仍然走普通的长度语法。

### 刻意没有绑定的

`shadow`、`cursor`、`text_align`、`text_overflow`、`font_weight` 与 `scrollbar_width` 接受的是 Rust 结构体或枚举而不是标量，因此没有作为有参方法暴露。它们每一个都有一个被反射到、今天就能用的无参形式：`shadow_sm`、`cursor_pointer`、`text_center`、`truncate`、`font_bold`。真正的 shadow API 应当与 token 工作一起做，而不是做成一串位置参数。

## 颜色

颜色通常从调用期主题读取。语义 token 名字符串仍为兼容性保留，固定颜色也可以使用十六进制字面量：

```js
render(cx) {
  return element
    .bg(cx.theme().colors.surface)         // 跟随主题
    .text_color("#1e88e5");    // 不跟随
}
```

调色板定义了十七个 token：

|      |                                                                      |
| ---- | -------------------------------------------------------------------- |
| 基底 | `background`、`foreground`                                           |
| 表面 | `surface`、`surface_foreground`                                      |
| 强调 | `primary`、`primary_foreground`、`secondary`、`secondary_foreground` |
| 弱化 | `muted`、`muted_foreground`                                          |
| 高亮 | `accent`、`accent_foreground`、`selection`                           |
| 危险 | `destructive`、`destructive_foreground`                              |
| 框架 | `border`、`input`、`ring`                                            |

十六进制字面量接受 `#rgb`、`#rrggbb` 与 `#rrggbbaa`。

**优先使用 `cx.theme().colors` 中的值。** 字面量绕开了主题，切换主题时不会波及到它。示例应用恰好说明了这一点：它沿用 `crates/base/examples/showcase` 的视觉语言——那份 Rust 示例只能写死颜色，因为 Base 不带调色板——而示例应用读的是语义 token，因此同一份代码能跟随主题，Rust 版的 showcase 做不到。

拼错 token 会列出整个集合，而不是含糊地失败：

```text
unknown color token `surfacee`; expected one of: background, foreground, surface, … —
or a #rrggbb literal
```

### 当前 token 来自哪里

gpui-shell 不拥有调色板或主题文件格式，而是读取 Host 提供的
`gpui_kit::base::Theme`。JavaScript 应用也可以通过
`set_theme({ appearance, tokens })` 替换同一份 Base Snapshot；主题名称和
registry 始终属于应用状态。

## 状态样式

`hover`、`active` 与 `focus` 接受一个函数，函数收到一个用于收集声明的游离元素：

```js
renderSave(cx) {
  return Button.new("save")
    .bg(cx.theme().colors.surface)
    .border(1)
    .border_color(cx.theme().colors.border)
    .hover((style) => style.bg(cx.theme().colors.muted).border_color(cx.theme().colors.foreground))
    .active((style) => style.bg(cx.theme().colors.border))
    .focus((style) => style.border_color(cx.theme().colors.ring))
    .child("Save");
}
```

函数的返回值会被忽略，所以链式写法和块状写法都能用。里面写的就是**普通的样式方法**——“什么是样式”没有第二套语法，上面所有长度与颜色规则原样适用。

有两处实现细节会泄漏到使用者这边，值得知道：

- **`active` 与 `focus` 需要稳定的元素身份。** 普通 `div` 会按需获得一个，由它在描述中的位置推出；只要树是稳定的，这个身份跨渲染就是稳定的。`Button`、`Checkbox` 与 `Input` 本来就有。
- **`Switch` 会忽略状态样式。** switch 的根节点不是可交互元素——它的 track 才是——所以挂在根上的状态样式无处落地。运行时会记一条警告，提示改为给它外面那一行加样式，而不是不声不响地丢掉这条声明。

## 滚动溢出内容

滚动属于元素行为，不是样式声明。先为 viewport 设置有限的宽度或高度，再指定它负责的滚动方向：

```js
v_flex()
  .id("activity")
  .h(240)
  .overflow_y_scroll()
  .children(this.rows.map((row) => row));
```

`.overflow_scroll()` 同时启用两个方向，`.overflow_x_scroll()` 只启用横向滚动，`.overflow_y_scroll()` 只启用纵向滚动。稳定的 `.id(...)` 会让原生滚动位置在多次脚本 render 之间始终归属于同一个 viewport。

对应的 `.overflow_scrollbar()`、`.overflow_x_scrollbar()` 与 `.overflow_y_scrollbar()` 保持相同的滚动行为，同时绘制 gpui-component 的原生 scrollbar。它们需要稳定的 `.id(...)`，确保每个 viewport 分别保留自己的 scrollbar 与滚动位置状态。

## 主题值

从正在 render 或处理事件的 context 读取语义值：

```js
render(cx) {
  return v_flex()
    .gap(cx.theme().spacing.md)
    .rounded(cx.theme().radius.lg)
    .bg(cx.theme().colors.surface)
    .child(`${cx.theme().appearance}: ${cx.theme().is_dark ? "dark" : "light"}`);
}
```

这个 Snapshot 是深度只读的。`theme()` 仍作为兼容入口保留，但优先使用 `cx.theme()`。应用可以从 event 或 task 调用 `set_theme({ appearance, tokens })`，传入自己管理的完整颜色、间距与圆角 token Snapshot。gpui-shell 只把它写入 gpui-base 并重建使用 token 的脚本 View，不拥有主题名称、palette 或文件格式。

## 原生动画

`.transition(property, policy)` 与 `.spring(property, policy?)` 会为 `opacity`、`width`、`height`、`left` 和 `top` 的后续目标变化制作动画。动画由 GPUI 原生保留并逐帧推进：脚本改变目标并调用 `cx.notify()` 后，动画帧**不会重新进入 JavaScript**。

```js
div()
  .id("drawer")
  .left(this.open ? 320 : 16)
  .opacity(this.open ? 1 : 0.5)
  .transition("left", { duration: 220, easing: "ease-out" })
  .spring("opacity", { response: 260, damping: 0.85 });
```

参与动画的长度目标**只能是数值像素**。`"50%"`、`"1rem"` 与 `"auto"` 之类相对值无法采样成稳定的原生通道，因此会被拒绝。请给元素稳定的 `.id(...)`（控件已使用构造器 id），否则树位置变化会改变动画 identity。

## 未知方法

```text
unknown style method `text_colour` (did you mean: text_color?)
```

建议来自对完整名字表的 Levenshtein 匹配，阈值卡得很紧——两次编辑，对较长标识符放宽到名字长度的三分之一。给错的建议比不给更糟。

这条信息背后有一处漂亮的机制，也解释了源码里的一个数字。QuickJS 报告缺失方法时只给一句 `TypeError: not a function`，**不带属性名**，所以拼错的样式名本来会毫无线索地到达使用者。用 `Proxy` 包住元素原型可以解决这一点——代价是实测占整个描述过程的约 30%（443 个节点从 1.09 ms 涨到 1.42 ms）。

于是运行时默认使用快速的普通原型，只有当一次渲染以 “not a function” 失败时，才**用带诊断 `Proxy` 的原型把这次渲染重跑一遍**，纯粹为了产出那条信息。出错是罕见的，每次渲染多付 30% 不是。

## 还没有的东西

- **语义状态样式。** `gpui-base` 有一层 `state_style`，为 checked、selected、disabled 定义了优先级顺序。它还没有被绑定；今天请用 `.when(condition, …)` 表达这些状态。
- **Keyframe 动画。** 已有目标值 transition 与 spring；任意 keyframe 和逐帧 JavaScript callback 仍不存在。
- **样式中的 spacing 与 radius token。** 调色板带有 spacing 与 radius 标尺，但样式方法接受的是长度而不是 token 名——只有颜色会去查 token。应用自己定义一份标尺常量即可，示例里的 `SPACE` 对象就是这么做的。

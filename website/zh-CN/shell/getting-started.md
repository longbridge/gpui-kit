---
title: 开始使用
description: 把运行时接进一个 Rust 应用、写它要加载的脚本，并在不开窗口的情况下检查这个脚本。
order: 2
---

# Getting Started

`gpui-shell` 首先是给一个 Rust GPUI 应用加上 JavaScript 扩展点的办法：由 Host 构建运行时、决定脚本能碰到什么，并把脚本 View 挂在它想挂的位置。直接运行一个脚本目录——也就是下面那个 `gpui-shell` 二进制——是随之而来的开发便利，而不是它的定位。

## 把运行时接进 Rust 应用

`gpui-shell` 二进制本身是一个很薄的 Host：解析命令行、装上日志 sink、建一个运行时、开一个窗口。任何嵌入这个库的 Host 做的也是同样四件事。

```rust
use gpui_kit::shell::{Capabilities, ShellRuntime};

gpui_platform::application()
    .with_assets(gpui_kit::shell::AppAssets::new(root.clone()))
    .run(move |cx| {
        // 初始化 gpui-base、shell 的默认 token 调色板，以及样式反射表。
        gpui_kit::shell::init(cx);

        let runtime = ShellRuntime::new(cx).expect("script runtime");

        // 在 Host 开口之前，什么都不允许。
        gpui_kit::shell::set_store_path(store_directory.join("store.json"));
        gpui_kit::shell::set_capabilities(
            Capabilities::new()
                .read_roots([root.clone()])
                .write_roots([store_directory.clone()])
                .store(true),
        );

        cx.open_window(Default::default(), move |window, cx| {
            runtime.load(&root, window, cx)
        })
        .expect("window");
    });
```

其中两行承载的是规则而不是机制。

**`runtime.load(...)` 返回窗口的 `ShellRoot`**，作用与 `gpui-component` 窗口中的 `Root` 相同。它持有 dialog 栈、sheet、toast 栈、焦点恢复与 Tab 导航。manifest 负责选择应用入口并记录能力请求，但不会自行批准这些请求。带 manifest 与不带 manifest 的目录都使用 Host 当前的默认 policy；后者采用 `main.js`。

**能力默认为空。** `Capabilities::default()` 什么都不授予——没有文件、没有存储、没有剪贴板、没有进程。由 Host 决定，因为只有 Host 知道它对即将运行的这段代码信任到什么程度。见 [Capabilities](./capabilities.md)。

同时也要装上 `tracing` subscriber。运行时通过 `tracing` 报告脚本错误、未处理的 promise rejection 以及 phase 非法的调用；没有 subscriber 时，这些全部被丢弃，症状是一个安静地不再响应的界面。

## 它加载的那个脚本

一个文件就够了。新建一个目录，放一个 `main.js`：

```js
// hello/main.js
import { View } from "gpui-kit";
import { v_flex, Button } from "gpui-base";

export default class Hello extends View {
  init() {
    this.clicks = 0;
  }

  render(cx) {
    return v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap(12)
      .bg(cx.theme().colors.background)
      .child(
        div()
          .text_color(cx.theme().colors.foreground)
          .child(`Clicked ${this.clicks} times`),
      )
      .child(
        Button.new("click")
          .h(28)
          .px(12)
          .items_center()
          .justify_center()
          .border(1)
          .border_color(cx.theme().colors.border)
          .bg(cx.theme().colors.surface)
          .text_color(cx.theme().colors.foreground)
          .on_click((_event, cx) => {
            this.clicks += 1;
            cx.notify();
          })
          .child("Click me"),
      );
  }
}
```

```bash
cargo run -p gpui-shell -- hello
```

这个文件里有四件事值得现在就点明，因为后面所有内容都建立在它们之上。

**能力由哪个包提供，就从哪个模块导入。** `"gpui-kit"` 是 GPUI 自身的元素和运行时补上的部分——`View`、`div`、`text`、存储、调度。`"gpui-base"` 是 gpui-base 的布局辅助、组件和主题——`v_flex`、`Button`、`InputState`。`"gpui-fps"` 是它的性能浮层。一个名字只属于其中一个模块，所以一行 import 就说清了脚本依赖的是哪一层。运行时还提供一层刻意收窄的 JavaScript 标准能力：`buffer`、`path`、`url`、`crypto`、`zlib`、`console`、`process`、`os`、`fs/promises`、`net`、`websocket`，以及全局 `fetch`。应用相对导入仍被限制在应用目录内。`node:fs` 这类 `node:` 别名、包查找和 CommonJS `require` 不属于契约。

**`main.js` 必须 `export default` 一个继承 `View` 的类。** `init` 在 View 创建时只执行一次；`render` 返回一个元素、留存的 `Entity` 或字符串，并且是在 View 失效时执行，而不是每帧执行——见 [`render` 什么时候执行](./state.md#render-什么时候执行)。

**样式方法是 snake_case，你自己写的代码是 camelCase。** `items_center`、`on_click`、`text_color`、`gap_2` 保留了 Rust 的拼写，因为无参样式接口是从 GPUI 的反射表生成的，而不是手写的。应用自己声明的一切——变量、方法、对象的键——都是普通的 JavaScript camelCase。这个对比是刻意的：snake_case 的调用是 Host 接口，camelCase 的是你的代码。

**没有任何东西会自动重绘。** 没有 signal，没有 `useState`，也没有依赖数组。改完状态，自己调用 `cx.notify()`。

## 单独运行一个脚本

一个脚本目录也可以直接跑起来，不必先写 Host 。自带的示例就是这么运行的；一段脚本在被它将来所属的那个应用加载之前，通常也是这样开发的。`gpui-shell` 没有发布到 crates.io，所以先克隆仓库，再在仓库根目录运行：

```bash
cargo run -p gpui-shell -- examples/js_todolist
```

窗口里会出现一个可用的 todo list：带留存状态的输入框、受控 checkbox、一个确认 dialog、一个 toast、从应用自身目录加载的图标，以及在未获授权时退化为内存存储的持久化。它的目的是把整个运行时都跑一遍，而不是做到最小——哪里坏了，通常先在这里露出来。

参数是一个**目录**，不是文件。运行时解析该目录，默认读取其中的 `main.js`，取出该模块 default 导出的类、构造一个实例，并把它挂载为窗口的根 View。如果目录中存在 `gpui-shell.json`，二进制会先验证它，并采用其中声明的 `entry` 与 capabilities。

## 不运行也能检查脚本

JavaScript 没有编译器，这个运行时也不打算造一个。它补上的是编译器本该替你做的那件事：

```bash
cargo run -p gpui-shell -- check hello
```

`check` 会加载应用，并向一个永远不显示的窗口渲染一帧，成功退出 `0`，失败退出 `1`。因为脚本接口是动态的——未知的样式方法、类型不对的参数、被重复使用的元素，都是运行期事实——所以“构建并渲染一次”是唯一诚实的检查方式。它能报出：

- 语法错误，并带上脚本自身的调用栈；
- 无法解析的 import，以及越出应用目录的 import；
- 缺失或形态不对的 default 导出；
- 未知的样式方法，并给出 `did you mean` 建议；
- 类型不对的样式参数，例如 `.p("auto")`；
- 被使用了两次的元素。

它不开窗口，因此可以放进编辑器、CI，或者一个 agent 的循环里。

加上 `--print-spec` 可以顺带打印构建出的元素描述：

```bash
cargo run -p gpui-shell -- check hello --print-spec
```

这份输出是 arena 自己的调试输出——在它变成真实元素之前，由组件与记录操作构成的那棵树。当问题是“我这条链到底记录了什么”时，它很有用。

## 生成 TypeScript 声明

```bash
cargo run -p gpui-shell -- types hello
```

它会在应用旁边写出 `gpui-kit.d.ts`。在脚本顶部加上 `// @ts-check`，编辑器就会补全整套 API，并在运行之前、在调用点上直接拒绝拼错的样式方法、不存在的颜色 token，或者 `.p("auto")`。

它同时会把编辑器需要的其余部分一并配好：manifest 声明的每个 Git 依赖都会被抓取并按声明的名字链接进 `node_modules`，于是 `import { style } from "omarchy-ui"` 解析到的正是运行时将要执行的那批文件，连同该 package 自己的类型、参数与 JSDoc；若目录里既没有 `jsconfig.json` 也没有 `tsconfig.json`，还会生成一份 `jsconfig.json`。详见[依赖](./dependencies.md)。

这份声明可信，是因为它**从运行时实际派发所依据的那几张表生成**，而不是照着文档抄的：

- 样式方法名来自 JavaScript prelude 构建元素原型时遍历的同一份列表；
- 每个有参方法的参数类型是**探测**出来的——生成器逐一询问运行时该方法接受哪些字面量，所以 length、definite length、absolute length、颜色与裸数字之间的区别，由真正做校验的那段代码决定；
- 颜色的联合类型来自已安装调色板的 token 名。

有三件事声明刻意不表达，因为没有类型能表达：能力是否被**授权**（被拒绝的 `fs.readFile` 一样能通过类型检查）；元素与 `cx` 的**生命周期**（TypeScript 没有仿射类型，重复使用元素照样能通过类型检查，也照样会抛异常）；以及**某个方法适用于哪个组件**（所有元素共用一个原型，所以 `.checked(true)` 声明在全部元素上，在 `div` 上只是不起作用）。

升级运行时之后重新生成即可；输出是确定性的，所以 diff 是可以审阅的。

## Hot-reload

```bash
cargo run -p gpui-shell -- hello --watch
cargo run -p gpui-shell -- hello --dev      # 隐含 --watch
```

`--watch` 每秒轮询应用目录四次，对一串连续写入去抖 200 ms，然后重载。一次重载会重新读取**每一个**模块，入口也在内——一个悄悄用了旧 import 的 hot-reload 比没有更糟，因为它看起来是成功的。

重载会在碰到实时 View 之前，先把所有可能失败的工作做完。如果新代码加载失败，之前的 View 继续运行，错误输出到 stderr，同时窗口里出现一个带固定 id 的 toast；下一次成功重载会把它撤回。存了一份坏代码，不会因此丢掉窗口。

`--dev` 隐含 `--watch`，并在构造运行时之前开启 development mode。它恢复动态代码构造器并让内建原型保持可写，但 capability 检查完全不变。见 [Capabilities](./capabilities.md#沙箱)。

## 命令一览

```text
gpui-shell <directory> [--watch] [--dev]
gpui-shell check <directory> [--print-spec]
gpui-shell types <directory>
gpui-shell --help | --version
```

| 参数           | 含义                                               |
| -------------- | -------------------------------------------------- |
| `<directory>`  | 应用根目录，或其中的 `main.js`                     |
| `check`        | 不开窗口地加载并渲染一次，退出码 `0` 或 `1`        |
| `types`        | 写出 `gpui-kit.d.ts`、链接 manifest 依赖、生成配置 |
| `--watch`      | 源码变化时重载                                     |
| `--dev`        | 开发模式，隐含 `--watch`                           |
| `--print-spec` | 配合 `check`，额外打印构建出的元素描述             |

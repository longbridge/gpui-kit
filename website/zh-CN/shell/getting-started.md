---
title: Getting Started
description: 安装 gpui-shell，用 JavaScript 编写并运行 gpui-shell Standalone App，在不开窗口的情况下检查它，并在需要更多能力时把它迁移到 Rust Host 下。
order: 2
maturity: [preview]
---

# Getting Started

应用先写成 gpui-shell Standalone App：用 JavaScript 编写，带热重载运行，不开窗口就能检查，也能直接打包发布，全程不需要 Rust，也没有构建步骤。产品前期需要的绝大多数功能都在这里。之后需要更多——原生库、繁重的计算、已经用 Rust 写好的逻辑——同一套脚本可以原样搬到一个嵌入了这个 crate 的 Rust 程序下面；本页末尾的[迁移到 Rust Host](#迁移到-rust-host) 讲怎么做。一个想要 JavaScript 扩展点的 Rust GPUI 应用，则是从另一侧走同样的最后一步。

## 安装 gpui-shell

无论以哪种方式使用运行时，都先安装 `gpui-shell` 命令。完全用 JavaScript 写的应用直接跑在它上面；嵌入这个库的 Rust Host 也要用它来 `check` 脚本、用 `types` 生成 TypeScript 声明，以及用 `agents` 生成给编程 Agent 的说明。

### 从 Release 安装

每个 GitHub Release 都附带预编译的 `gpui-shell`，覆盖 macOS（Apple 芯片与 Intel）、Linux x86-64 和 Windows x86-64。它内置了 [`gpui-component`](./examples.md) 组件目录，所以应用除了 `"gpui-base"`，也可以从 `"gpui-component"` 导入带样式的组件。

```bash
curl -fsSL https://gpui-kit.com/shell/install.sh | sh
```

每个 release 也附带同一个脚本，所以 `https://github.com/longbridge/gpui-kit/releases/latest/download/install.sh` 同样可用。

脚本会把二进制装到 `~/.local/bin`，并创建 `~/.gpui-shell/apps`。`GPUI_SHELL_VERSION=v0.7.1` 可以固定某个版本。`GPUI_SHELL_HOME` 会替换 `~/.gpui-shell` 目录——已安装的应用和 Git 依赖缓存都随之移动；`gpui-shell` 运行时也读取这个变量，所以要保持导出。产物名不带版本号（`gpui-shell-aarch64-apple-darwin.tar.gz`，旁边附 `.sha256`），所以 Homebrew formula 或 mise 的 `ubi` backend 也可以直接下载 `releases/latest/download/<asset>`。

### 从 Git 安装最新版

想在发版之前跟上 `main`——比如一个已经合并但还没发布的修复，或者 Rust Host 依赖的是仓库而不是某个 tag——就用 Cargo 从 Git 构建：

从 crates.io 安装本次发布的版本：

```bash
cargo install gpui-shell --version 0.8.1 --locked
```

也可以从 Git 构建开发版本：

```bash
cargo install --git https://github.com/longbridge/gpui-kit gpui-shell --locked
```

它会装到 `~/.cargo/bin`，和 Release 一样内置 `gpui-component` 组件目录。再次运行同一条命令即可更新，`main` 有新提交时 Cargo 会重新构建。用 `--branch <name>`、`--tag <tag>` 或 `--rev <commit>` 可以构建指定的版本。这需要 Rust 工具链；在 Linux 上还需要 GPUI 链接的系统库，仓库里的 `script/install-linux.sh` 列出了 Ubuntu 上需要的包。

如果 `PATH` 里 `~/.local/bin` 排在 `~/.cargo/bin` 前面，脚本装的 Release 版会盖过 Git 版；用 `command -v gpui-shell` 确认实际运行的是哪一个。

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
gpui-shell run ./hello
```

这个文件里有四件事值得现在就点明，因为后面所有内容都建立在它们之上。

**能力由哪个包提供，就从哪个模块导入。** `"gpui-kit"` 是 GPUI 自身的元素和运行时补上的部分——`View`、`div`、`text`、存储、调度。`"gpui-base"` 是 gpui-base 的布局辅助、组件和主题——`v_flex`、`Button`、`InputState`。`"gpui-fps"` 是它的性能浮层。一个名字只属于其中一个模块，所以一行 import 就说清了脚本依赖的是哪一层。运行时还提供一层刻意收窄的 JavaScript 标准能力：`buffer`、`path`、`url`、`crypto`、`zlib`、`console`、`process`、`os`、`fs/promises`、`net`、`websocket`，以及全局的 `fetch`、`TextEncoder` 与 `TextDecoder`。应用相对导入仍被限制在应用目录内。`node:fs` 这类 `node:` 别名、包查找和 CommonJS `require` 不属于契约。

**`main.js` 必须 `export default` 一个继承 `View` 的类。** `init` 在 View 创建时只执行一次；`render` 返回一个元素、留存的 `Entity` 或字符串，并且是在 View 失效时执行，而不是每帧执行——见 [`render` 什么时候执行](./state.md#render-什么时候执行)。

**样式方法是 snake_case，你自己写的代码是 camelCase。** `items_center`、`on_click`、`text_color`、`gap_2` 保留了 Rust 的拼写，因为无参样式接口是从 GPUI 的反射表生成的，而不是手写的。应用自己声明的一切——变量、方法、对象的键——都是普通的 JavaScript camelCase。这个对比是刻意的：snake_case 的调用是 Host 接口，camelCase 的是你的代码。

**没有任何东西会自动重绘。** 没有 signal，没有 `useState`，也没有依赖数组。改完状态，自己调用 `cx.notify()`。

## 单独运行一个脚本

运行一个目录不需要你自己的 Host。自带的示例就是这么运行的；无论应用之后是否迁移到 Rust Host 下，开发时也都是这样运行。在仓库的克隆里运行：

```bash
gpui-shell run examples/js_todolist
```

窗口里会出现一个可用的 todo list：带留存状态的输入框、受控 checkbox、一个确认 dialog、一个 toast、从应用自身目录加载的图标，以及在未获授权时退化为内存存储的持久化。它的目的是把整个运行时都跑一遍，而不是做到最小——哪里坏了，通常先在这里露出来。

参数是一个**目录**，不是文件。运行时解析该目录，默认读取其中的 `main.js`，取出该模块 default 导出的类、构造一个实例，并把它挂载为窗口的根 View。如果目录中存在 `gpui-shell.json`，`gpui-shell` 会先验证它，并采用其中声明的 `entry` 与 capabilities。

包含路径分隔符的参数——`./hello`、`examples/js_todolist`——是一个目录；不含分隔符的裸名字则表示一个**已安装**的应用，见下文。`-d` / `--dir <directory>` 永远表示目录。

## 已安装与随包的应用

`gpui-shell` 一个进程只运行一个应用，自身不带任何界面。运行哪个应用由命令行决定：

| 命令                            | 运行的应用                                  |
| ------------------------------- | ------------------------------------------- |
| `gpui-shell run clock`          | 已安装的应用 `~/.gpui-shell/apps/clock/`    |
| `gpui-shell run -d ~/dev/clock` | 指定的那个目录                              |
| `gpui-shell`                    | 与可执行文件一起打包的应用；没有则显示 help |

安装应用就是把它的目录复制或链接到 `~/.gpui-shell/apps/`，目录名就是运行它时用的名字。`gpui-shell list` 列出所有已安装的应用及其 manifest 申请的 capabilities，一个都不会执行。无论以哪种方式运行——已安装、打包，还是用 `--dir`——应用都按 manifest 的 `id` 保存数据，移动或重装都不会丢，正在编辑的副本也和已安装的副本共用数据。只有没有 manifest 的目录才按路径区分。

不带参数时，可执行文件会在自己旁边找应用：如果它是 macOS bundle 里的 `Contents/MacOS/gpui-shell`，就找 `Contents/Resources/app`；否则找可执行文件所在目录下的 `app`。找不到时就显示 help，所以刚安装好的 `gpui-shell` 在终端里直接运行会先介绍自己的用法。于是一个独立的桌面应用可以就是 `gpui-shell` 二进制加一个脚本目录，不需要额外的启动脚本来传路径：

```text
Clock.app/Contents/
├── Info.plist          # CFBundleExecutable = gpui-shell
├── MacOS/gpui-shell
└── Resources/app/
    ├── gpui-shell.json
    └── main.js
```

`gpui-shell bundle -d clock` 会替你生成桌面应用。在 macOS 和 Linux 上不带二进制，生成 `.app`，或者 `.tar.gz` 及与之并列的 `install.sh`：在用户的机器上，它们使用已安装的、不低于打包时版本的 `gpui-shell`，没有就下载那个 release，再用 `run --packaged` 运行应用，像已安装的应用一样按 `id` 保存数据。在 Windows 上生成便携 `.zip`，其中的可执行文件就是 `gpui-shell.exe` 本身，旁边是 `app\`。[Packaging](./packaging.md) 介绍了这两种方式、各个选项，以及签名与公证。

## 新建应用

```bash
gpui-shell new clock
gpui-shell dev -d clock
```

`new` 和 `cargo new` 一样，在命令运行的位置创建目录——这里的 `clock` 指 `./clock`，不是已安装的应用。它会写入 `gpui-shell.json`（`id` 由目录名推出）、一个带标题栏和按钮的 `main.js`、`.gitignore`（其中也忽略了 `bundle` 的输出目录 `dist/`）、`jsconfig.json`、`gpui-kit.d.ts` 和 `AGENTS.md`；目标目录里已有文件时会拒绝创建。

## 给 coding agent 的开发说明

```bash
gpui-shell agents                 # 打印出来
gpui-shell agents -d ./hello      # 写入 ./hello/AGENTS.md
```

`agents` 会打印 coding agent 为这个运行时编写应用所需的规则，以及用来验证结果的 `check` 流程：没有 DOM、状态在 `init` 中赋值、改完状态调用 `cx.notify()`、元素只能用一次、capabilities 默认拒绝。其中的模块列表和组件目录都来自打印它的这个 `gpui-shell`。指定应用时，它会在那里写入 `AGENTS.md`，并且只替换自己生成过的文件（首行有标记），手写的说明不会被覆盖。

## 不运行也能检查脚本

JavaScript 没有编译器，这个运行时也不打算造一个。它补上的是编译器本该替你做的那件事：

```bash
gpui-shell check ./hello
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
gpui-shell check ./hello --print-spec
```

这份输出是 arena 自己的调试输出——在它变成真实元素之前，由组件与记录操作构成的那棵树。当问题是“我这条链到底记录了什么”时，它很有用。

## 生成 TypeScript 声明

```bash
gpui-shell types ./hello
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
gpui-shell dev ./hello           # 重载，并输出 debug 日志
gpui-shell run ./hello --watch    # 只重载
```

开发应用时用 `dev`：它像 `run --watch` 一样在源码变化时重载，日志级别为 `DEBUG`。监听时每秒轮询应用目录四次，对一串连续写入去抖 200 ms，然后重载。一次重载会重新读取**每一个**模块，入口也在内——一个悄悄用了旧 import 的 hot-reload 比没有更糟，因为它看起来是成功的。

重载会在碰到实时 View 之前，先把所有可能失败的工作做完。如果新代码加载失败，之前的 View 继续运行，错误输出到 stderr，同时窗口里出现一个带固定 id 的 toast；下一次成功重载会把它撤回。存了一份坏代码，不会因此丢掉窗口。

两者都不放宽沙箱。`--dev` 可配合 `run` 或 `dev`，在构造运行时之前开启 development mode：恢复动态代码构造器并让内建原型保持可写，但 capability 检查完全不变。它不会重载，也不改变日志级别。见 [Capabilities](./capabilities.md#沙箱)。

## 迁移到 Rust Host

当应用需要脚本够不到的东西时，就给它一个自己的 Rust 程序。JavaScript 一行都不用重写：脚本、manifest、import 与已存储的数据都保持原样。这一步有两种深度，通常第一种就够了。

### 保留命令，加上 Rust

`gpui-shell` 命令本身就在库里，所以你自己的 Rust 程序可以就是这个命令：换成自己的名字，再给脚本注册一些 Rust 函数。依赖已发布的 crate：

```toml
[dependencies]
gpui-shell = "0.8.1"
```

```rust
use gpui_shell::{HostModule, HostValue, host::HostBrand};

fn main() {
    gpui_shell::export_module(
        HostModule::new("native")
            .declarations("export function greet(name: string): string;")
            .function("greet", |arguments| {
                let name = arguments.string(0)?;
                Ok(HostValue::from(format!("Hello, {name}")))
            }),
    )
    .expect("register the native module");

    let components = gpui_shell::components::components().expect("component catalog");
    gpui_shell::host::main_with_brand_and_components(
        HostBrand::new("clock", env!("CARGO_PKG_VERSION")),
        components,
    );
}
```

```js
import { greet } from "native";

greet("world"); // "Hello, world"
```

在调用 `main_with_brand_and_components` 之前注册模块：import 在应用加载时就完成链接。其余部分就是发布的那个命令——`clock dev -d ./clock`、`clock check`、`clock types`（现在也会声明 `native`）和 `clock agents` 的行为与 `gpui-shell` 相同；应用读取同一份 manifest，得到同样的授权，数据也仍在同一个 `id` 下。不带参数运行 `clock` 会运行可执行文件旁边的 `app` 目录，所以一个[独立包](./packaging.md#独立包)就是这个可执行文件加上应用目录。唯一被拒绝的命令是 `bundle`：它生成的启动器会安装 `gpui-shell` 的 Release，而那里面没有你的模块。哪些值能跨过边界、函数怎样访问 Host 的状态，见 [HostModule](./host-module.md)。

### 自己构建运行时

需要掌控进程里更多东西的 Host——自己的窗口、和 Rust View 并排的脚本 View、多个授权不同的应用——自己构建运行时。它要做四件事：初始化库、建一个运行时、授予它愿意授予的能力，并把脚本 View 挂在 `ShellRoot` 下。`gpui-shell` 命令本身就是一个只做这四件事的很薄的 Host。

```rust
use gpui_shell::{Capabilities, ShellRuntime};

gpui_platform::application()
    .with_assets(gpui_shell::AppAssets::new(root.clone()))
    .run(move |cx| {
        // 初始化 gpui-base、shell 的默认 token 调色板，以及样式反射表。
        gpui_shell::init(cx);

        let runtime = ShellRuntime::new(cx).expect("script runtime");

        // 沿用 gpui-shell 命令的数据目录：manifest 里的 `id`。
        let data = gpui_shell::set_bundle_id("com.example.clock").expect("data directory");

        // 在 Host 开口之前，什么都不允许。
        gpui_shell::set_capabilities(
            Capabilities::new()
                .read_roots([root.clone()])
                .write_roots([data.clone()])
                .storage(true),
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

## 命令一览

```text
gpui-shell run [<name> | -d <directory>] [--watch] [--dev] [--packaged]
gpui-shell dev [<name> | -d <directory>] [--dev]
gpui-shell
gpui-shell new <directory>
gpui-shell list
gpui-shell check <name> | -d <directory> [--print-spec]
gpui-shell types <name> | -d <directory>
gpui-shell agents [<name> | -d <directory>]
gpui-shell bundle [<name> | -d <directory>] [--platform <platform>] [--icon <png>]
                  [--out <directory>] [--download-url <url>]
gpui-shell --help | --version
```

| 参数                | 含义                                                                    |
| ------------------- | ----------------------------------------------------------------------- |
| `<name>`            | 已安装的应用，`~/.gpui-shell/apps/<name>`                               |
| `-d`, `--dir <dir>` | 应用根目录，或其中的 `main.js`                                          |
| _（无）_            | 与可执行文件一起打包的应用，没有则显示 help                             |
| `run`               | 在窗口中运行一个应用                                                    |
| `dev`               | 开发时运行：源码变化时重载，日志级别 `DEBUG`                            |
| `new`               | 在新目录中创建一个空白应用                                              |
| `list`              | 列出已安装的应用及其申请的 capabilities                                 |
| `check`             | 不开窗口地加载并渲染一次，退出码 `0` 或 `1`                             |
| `types`             | 写出 `gpui-kit.d.ts`、链接 manifest 依赖、生成配置                      |
| `agents`            | 打印给 coding agent 的开发说明，或写成 `AGENTS.md`                      |
| `bundle`            | 在 `dist/` 中生成桌面应用，见 [Packaging](./packaging.md)               |
| `--packaged`        | 配合 `run`，像包的启动脚本那样运行目录：必须有 manifest，不写编辑器文件 |
| `--watch`           | 配合 `run`，源码变化时重载；`dev` 总会重载                              |
| `--dev`             | 配合 `run` 或 `dev`，开启开发模式：放宽沙箱                             |
| `--print-spec`      | 配合 `check`，额外打印构建出的元素描述                                  |

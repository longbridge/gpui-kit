---
title: WebAssembly
description: 使用仓库中的 WebAssembly 示例，在浏览器中构建和运行 GPUI Kit 应用。
order: -3.5
maturity: [preview]
---

# WebAssembly

:::warning 当前适用范围
GPUI 和 GPUI Kit 的 WebAssembly 支持目前主要用于**在浏览器中展示、体验组件**。组件画廊与 Base 示例可以构建和运行，但本仓库尚未验证它是成熟的完整应用分发路径。浏览器兼容性、无障碍、输入、启动成本和部署方式，都需要针对具体产品逐项验证。
:::

GPUI Kit 可以在浏览器中渲染与桌面端共用的 Rust 视图和组件。Web 目标是 `wasm32-unknown-unknown`：Rust 生成 WebAssembly 模块，`wasm-bindgen` 生成 JavaScript 绑定，网页负责加载并启动应用。浏览器提供 canvas、输入和网络环境，因此只有桌面端的 `main` 函数还不足以作为 Web 入口。

在当前工作区中，[`gpui_web` 是 Cargo 包 `gpui-pre-web` {{gpui_pre_version}} 的别名](https://github.com/longbridge/gpui-kit/blob/main/Cargo.toml)。[`gpui-kit` 将它作为仅用于 WASM 的依赖](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/Cargo.toml)，并以 `gpui_kit::web` 重新导出；[画廊 crate](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/Cargo.toml)直接依赖的是 `gpui-kit`，并没有另行直接依赖 `gpui-pre-web`。画廊的 `cdylib`、导出的 `run(...)`、Web 平台初始化和 JavaScript 加载器，共同提供桌面端 `main` 所没有的浏览器入口。

[组件画廊](https://gpui-kit.com/gallery/)是最快可运行的示例。它的 [Rust 入口](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs)、[构建脚本](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/scripts/build-wasm.sh)和 [JavaScript 加载器](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/www/src/main.js)展示了从 GPUI Kit 视图到浏览器页面的完整路径。

## 在本地运行画廊

准备好 Rust、Bun 和 `make`，然后从仓库根目录（包含工作区 `Cargo.toml` 的目录）执行：

```sh
cd crates/story-web
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.121
make dev
```

保持 `make dev` 运行，打开 **http://localhost:3000/gallery/**。成功后应看到画廊的组件列表和渲染视图；仅看到加载提示并不能说明图形初始化已经完成。首次 Rust 构建可能比后续构建慢。`make dev` 以 debug 模式构建 WASM，在 `www/src/wasm/` 生成绑定，安装 Web 依赖并启动 Vite。

按下面的顺序验收：

1. 终端显示 Vite 的本地地址，且 Rust 编译和 `wasm-bindgen` 均未报错。
2. 浏览器 Network 面板中，生成的 JavaScript 与 `.wasm` 请求均成功。使用上面的 `/gallery/` 地址，而不是 Vite 服务器根路径。
3. 加载提示消失后出现画廊。打开一个组件故事并操作按钮等控件，确认输入确实传到了 Rust 视图。加载提示消失本身只说明 JavaScript 加载器调用了 `run(...)`。

修改 Rust 视图时，可在一个终端保持 Vite 运行，另一个终端进入 `crates/story-web` 执行 `make build-wasm-dev`，再刷新页面。修改加载器或其他 `www/` 文件由 Vite 处理。Vite 不会自行重新编译 Rust crate，这两种修改不能混为一谈。

[本地工具链文件](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/rust-toolchain.toml)选用 nightly；上述 `wasm-bindgen-cli` 版本与当前检出的 [`Cargo.lock`](https://github.com/longbridge/gpui-kit/blob/main/Cargo.lock) 一致。如果锁文件更新，请让 CLI 版本与锁定的 `wasm-bindgen` crate 版本保持一致。如果已安装其他版本，可运行 `wasm-bindgen --version` 检查，再用 `cargo install -f wasm-bindgen-cli --version 0.2.121` 安装锁定的版本。[构建脚本](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/scripts/build-wasm.sh)为画廊较大的渲染树设置 8 MiB 链接栈，debug 构建也会使用该设置。

要构建生产版画廊，在同一目录运行 `make build-prod`。网页产物位于 `www/dist/`，基础路径为 `/gallery/`。请部署到这个路径；如果部署在其他路径，需要同时调整 [Vite 基础路径](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/www/vite.config.js)和 [Rust 资源端点](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs)。

## Web 入口的工作方式

画廊使用 `#[wasm_bindgen]` 导出 `run(story, dark, theme_name, theme_json)`。[加载器](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/www/src/main.js)导入生成的 JS 模块，等待默认 WASM 初始化完成，读取可选的 `?story=` 参数及宿主主题，再调用 `run(...)`。Rust 端先调用 `gpui_kit::platform::web_init()`，创建 Web `Application`、注册资源，并将启动闭包传给 `run_embedded`。图形初始化成功后，该闭包才调用 `gpui_component_story::init(cx)`（其中会调用 `gpui_kit::init(cx)`）、加载字体、应用主题，最后调用 `gpui_kit::open_window`。`run_embedded` 返回的 `ApplicationHandle` 保存在 thread-local 状态中。图形初始化是异步的，因此该函数返回时，启动闭包可能尚未运行，首帧也可能尚未绘制。改造自己的应用时，也要像[快速开始](./getting-started.md)那样先初始化 Kit，并在页面使用视图期间保留 handle。

画廊通过 `WebPlatform::new_with_backend_and_font_fallback` 选择 Web 平台，同时接入 fetch HTTP client。`Auto` 先尝试 WebGPU，失败后再尝试 WebGL2。当前 Web 平台使用一个文档级 canvas，且只支持一个顶层窗口；不能再打开第二个顶层窗口，也不能在关闭后重新打开。对话框应通过 Kit 的 `Root` 在该窗口内渲染。可以参照[画廊入口](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs)改造应用：共用视图留在 Rust 中，网页负责加载 WASM 模块并提供宿主页面。

### 跟踪一次启动

| 阶段 | 文件或 API | 完成的工作 |
| --- | --- | --- |
| 编译 | `scripts/build-wasm.sh` | `cargo rustc` 将画廊的 `cdylib` 编译为 `wasm32-unknown-unknown` 目标；随后 `wasm-bindgen --target web` 将 JavaScript 绑定和可供浏览器加载的模块写入 `www/src/wasm/`。 |
| 加载 | `www/index.html` 与 `www/src/main.js` | 页面先显示加载状态；加载器导入生成的绑定并等待默认初始化函数。这一步跨越 JavaScript 与 Rust 的边界。 |
| 启动 | `src/lib.rs` 中的 `run(...)` | Rust 创建 Web 平台，提供资源并保留 `ApplicationHandle`；`run_embedded` 异步启动图形初始化。 |
| 开窗 | `src/lib.rs` 中的启动闭包 | 图形初始化成功后才初始化 Kit、注册字体、应用主题并打开唯一的 GPUI 窗口；之后首个视图才能绘制。 |

将桌面应用移到浏览器时，可保留目标平台支持的 GPUI 视图与状态代码，但还需提供 Web 入口、页面加载器和浏览器专用资源。Rust 目标编译成功并不等于页面加载器或图形初始化已通过测试。

## 字体与中日韩文本

GPUI Web 平台启动时的字体数据库是**空的**。画廊使用 `include_bytes!` 将四个 TTF 子集嵌入 WASM：Inter 用于界面文字，JetBrains Mono 用于代码，Noto Sans SC 包含故事中用到的汉字，IBM Plex Sans 对应 GPUI 的 `.SystemUIFont` 别名。最后这个字体必须先于首个窗口加载：初始文本测量也可能沿用窗口默认样式，如果对应字体不存在，文本系统就会报错。画廊先调用 `cx.text_system().add_fonts(...)`，再应用主题，并把界面字体与等宽字体强制设回已打包的字体；选中的主题可能指定仅桌面端存在的字体。详见[字体初始化与主题代码](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs)和[字体](./fonts.md)。

[子集脚本](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/scripts/subset-fonts.py)扫描故事源码，只保留其中出现的字符。当前检出中，Noto Sans SC 子集为 **25,484 字节**，仓库内对应的原始 TTF 为 **1,213,236 字节**。这是字体文件大小，不能当作压缩后 WASM 传输量的差值。访客输入的新文字不一定包含在该子集中。

对于符合条件、打包字体缺少的 emoji，以及横排汉字、假名和现代谚文字素，`CanvasFontFallback::EmojiAndCjk` 可让浏览器用本机字体测量并绘制。GPUI 已加载的字体仍优先。回退按单个字素工作，因此字形覆盖、字距和排版特性取决于浏览器，不能代替完整的中日韩字体。默认策略仅覆盖 emoji，`Disabled` 则只使用已加载字体。策略在构造 `WebPlatform` 时确定，之后不能更改。

GPUI 也支持**启动后加载字体**。在本仓库固定的 GPUI 版本（`gpui-pre` {{gpui_pre_version}}）中，`TextSystem::add_fonts` 可以通过 `Cow::Owned` 接收下载的字体字节，并使字体解析及行布局缓存失效。异步请求得到并检查过有效的原始字体文件后，在应用上下文注册并刷新窗口：

```rust
use std::borrow::Cow;

// cx: &mut App; font_bytes: Vec<u8>, downloaded and validated by the application.
cx.text_system().add_fonts(vec![Cow::Owned(font_bytes)])?;
cx.refresh_windows();
```

使用 GPUI Web `examples/hello_web/dynamic_fonts.rs` 所示的原始 TTF 等受支持字体文件，不要把字体服务的 CSS 响应直接传给文本系统；CSS 可能引用 WOFF2 子集。该示例通过 `cx.on_missing_glyphs(...) -> Subscription` 在缺字时请求字体，保留订阅与下载 task，检查 HTTP 状态是否成功、响应体是否非空，再调用 `add_fonts` 与 `refresh_windows`。字体是否能解析由 `add_fonts` 检查；仅有 HTTP 200 不能证明响应是可用字体。缺字报告会去重，队列有长度限制；报告丢弃后不会自动安排重试。当前画廊**没有**按需下载字体，而是打包子集并使用 Canvas 回退。GPUI Web 先应用 Canvas 回退，再报告缺字；成功由 Canvas 绘制的中日韩文字**不会**触发 `on_missing_glyphs`。如果需要准确的中日韩排版，应根据内容或语言选择决定要下载哪种字体，提供加载和失败状态，并在安装后检查布局。跨域字体下载还必须符合对方服务器的 CORS 策略。

## 更小的 GPUI Base 示例

[GPUI Base WASM 示例](https://github.com/longbridge/gpui-kit/tree/main/crates/base/examples/wasm)与原生示例共用同一套[展示视图](https://github.com/longbridge/gpui-kit/tree/main/crates/base/examples/showcase)。从仓库根目录单独运行时，如果尚未安装匹配版本的 `wasm-bindgen-cli`，请一并安装：

```sh
cd crates/base/examples/wasm
rustup target add wasm32-unknown-unknown --toolchain nightly
cargo install wasm-bindgen-cli --version 0.2.121
make dev
```

保持服务器运行，打开 **http://localhost:3001/examples/base/**。它使用 `gpui_platform::single_threaded_web()`，通过 `wasm-bindgen` 生成绑定，并由 Vite 提供示例。导出的 `run(component)` 选择展示视图；JavaScript 加载器从 URL 读取 `?component=...`。CLI 版本同样以工作区 `Cargo.lock` 为准。详见[构建脚本](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/wasm/scripts/build.sh)和[加载器](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/wasm/www/src/main.js)。要从有主题的 GPUI Kit 组件入手，请使用画廊；此示例主要用于了解 Base 原语。

## 加载失败或画布空白时如何排查

先打开浏览器的 **Console 与 Network** 面板。画廊加载器会捕获导入绑定、初始化 WASM 或调用 `run` 时的异常，并用“Failed to load the application”替换加载提示。图形初始化发生在 Web 平台稍后的异步任务中；如果失败，平台会记录错误，并向页面追加“Failed to initialize browser graphics”。因此，加载提示消失并不代表首帧已经绘制。

| 现象 | 检查方式 |
| --- | --- |
| Rust 编译找不到 WASM 目标，或找不到 `wasm-bindgen` | 在 `crates/story-web` 下执行 `rustup target add wasm32-unknown-unknown`，让其选择的 nightly 工具链安装目标。按 `Cargo.lock` 的版本安装 CLI，再执行 `make build-wasm-dev`。必须先成功完成 Rust 构建，Vite 页面才有模块可加载。 |
| JS 或 `.wasm` 请求返回 404 | 打开 `/gallery/`，而不是站点根路径。让 Vite 的 `base: '/gallery/'`、部署目录和生成文件保持一致。Rust 修改后重新执行 `make build-wasm-dev`，确认 Web 构建包含 `www/src/wasm/` 中的生成文件。 |
| `wasm-bindgen` 导入或实例化失败 | 让 `wasm-bindgen-cli` 版本与锁定的 crate 一致，并重新生成绑定。以 `application/wasm` MIME 类型提供 `.wasm`；MIME 错误时生成的加载器可能退回较慢的非流式实例化。检查 Network 响应是否真的是模块。 |
| 加载提示消失，但 canvas 仍为空白 | 在 `run` 返回之后继续查看 Console。`Auto` 先尝试 WebGPU，再尝试 WebGL2；若都失败，检查浏览器 GPU 支持、策略和硬件加速。Web 运行时不能打开第二个顶层窗口。 |
| 启动时 panic 或文字缺失 | 先于窗口注册 IBM Plex Sans 和初始主题所用的每种字体。画廊的 `add_fonts(...).expect(...)` 与 `open_window(...).expect(...)` 会把错误变成 panic；panic hook 会将其输出到 Console。 |
| 中日韩文字显示方块或字形不对 | 确认字符是否在打包子集中，再检查 Canvas 回退策略和浏览器实际字体覆盖。若需要准确整行排版，应下载合适的原始字体。 |
| 图标空白 | 检查实际请求 URL、HTTP 状态及 CORS。`Assets::load` 会直接拼接 endpoint 和 `/assets/icons/...`：如果 endpoint 像画廊一样以 `/` 结尾，URL 中会出现 `//assets/`。自己的应用应使用不带末尾斜杠的 endpoint。即使本地开发，画廊固定的资源端点仍指向已发布站点。资源加载器会缓存成功下载的图标，但本身不会请求刷新窗口；如果图标仍为空白，可以触发重新渲染并查看 Console 中的资源错误。 |

## 包体大小与网络分发

`include_bytes!` 把画廊字体子集**放进 WASM 载荷**，访客需要在首帧前随模块一起下载。画廊还包含组件故事与渲染栈；图标 SVG 则另行按需请求。Vite 将 JavaScript 加载器和 WASM 打包到 `/gallery/`，但仅修改 Vite 基础路径不会改变 Rust 中的图标端点。运行 `make build-prod` 后，`ls -lh www/dist/assets/*.wasm` 可查看未压缩的模块大小。应在浏览器 Network 面板比较该模块与 JS 的**压缩传输量**，并用限速网络记录首次加载时间。本地文件大小、CDN 压缩、浏览器缓存、编译时间和 GPU 初始化分别是不同的成本。

发布画廊副本前，应从目标 `/gallery/` 路径提供 `www/dist/`，对部署后的页面重新执行上面的三步浏览器验收。确认生成的 `.wasm` 和 JS 地址指向部署主机，而图标、主题、字体请求指向预期主机。示例将资源端点固定为已发布的 GPUI Kit 网站，因此本地画廊运行成功**不能**证明另外托管的副本会提供自己的图标。分别记录首次加载与缓存后的加载结果，浏览器缓存会改变测量值。仓库的[发布流程](https://github.com/longbridge/gpui-kit/blob/main/.github/workflows/release-website.yml)构建两套 WASM 示例并将其 `dist/` 文件复制到网站对应路径，但仍需在最终主机上做浏览器验证。

例如，**如果**把类似长桥 Pro / Longbridge Pro 的完整应用及全部功能和字体覆盖一并编进单个 WASM 模块，首次下载和启动成本可能显著增加。这是需要测量的分发风险，并非该应用的实测包体，也不表示它目前在 Web 上发布。把字体或功能放到后续请求中可以降低首次传输量，但也会增加网络、缓存、CORS 和加载状态处理。画廊的字体子集与按需图标体现了这些取舍，不能直接作为完整应用的包体预算。

## 需要规划的 Web 能力

| 方面 | 示例中的做法 | 在自己的应用中需要检查 |
| --- | --- | --- |
| 资源 | WASM 上的 `Assets::new(endpoint)` 通过拼接 endpoint 和 `/assets/icons/...` 按需下载 SVG。画廊的 endpoint 指向已发布站点，Vite 构建则把图标复制到 `/gallery/assets/`。 | 托管对应路径，并让 endpoint 与部署路径一致，且不要在末尾加斜杠。如果不修改 endpoint，本地画廊也会从已发布站点请求图标。文件缺失或请求失败会使图标无法显示。详见[图标与资源](./assets.md)。 |
| 字体 | 画廊在首帧前嵌入 Inter、JetBrains Mono、Noto Sans SC 子集和 IBM Plex Sans，应用主题时重新设置字体。 | 打包初始字体；对于更大的文字集合，可考虑运行时下载字体，并验证回退与布局。详见[字体](./fonts.md)。 |
| 键盘与输入法 | Web 平台通过一个很小的隐藏 HTML input 接收键盘与组合输入事件。画廊加载器在嵌入页面时管理焦点，并在纯触屏设备上禁用文字输入，避免 canvas 交互意外弹出键盘。 | 在目标浏览器和设备上检查焦点、Tab 顺序、组合输入和屏幕键盘。画廊的纯触屏策略只适合展示用途，不能当作通用文本输入方案。 |
| [无障碍](./accessibility.md) | 组件可以在 GPUI 中声明 role 和 label，而此 Web 示例最终绘制在 canvas 中。 | 在浏览器中实际验证屏幕阅读器与键盘操作。不要推断原生无障碍桥接或 GPUI 属性在 Web 上有等价语义。对于目标浏览器无法暴露的内容或操作，提供可访问的 HTML 替代界面。 |
| 原生服务 | 浏览器提供 fetch 及自身的输入、渲染 API；桌面设施的可用性和权限不同。 | 将文件对话框、剪贴板、通知等功能放在按目标平台区分的能力接口后面，并测试浏览器路径。详见[编码指南](./coding-guides.md#平台与能力边界)。 |

发布流程会[构建两套 WASM 示例](https://github.com/longbridge/gpui-kit/blob/main/.github/workflows/release-website.yml)，因此这些入口也是持续维护的构建参考。WASM 构建成功只说明能够编译；交互、字形覆盖和无障碍仍需在浏览器里验证。

---
title: WebView
description: 在 GPUI Kit 窗口中嵌入 Wry 原生 WebView，并了解当前的平台与 overlay 限制。
order: -9.125
maturity: [experimental, platform-dependent]
---

# WebView

[`gpui-wry`](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) 是 GPUI Kit 基于 [Wry](https://github.com/tauri-apps/wry) 的**实验性**集成。需要浏览器行为时可以使用它；[TextView HTML](../component/text-view.md#html) 用于渲染文档内容，并不是浏览器。要在默认外部浏览器中打开 URL，使用 [`cx.open_url`](./context)。当前集成支持 macOS 和 Windows。仓库示例中的 Linux 路径尚未完成。

## 运行示例

在仓库根目录运行：

```sh
cargo run -p webview
```

[完整示例](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/src/main.rs)是可运行的起点：它在 `open_window` 回调中创建原生子视图，把它包装为 `Entity<WebView>`，再放在地址输入框下方渲染。在输入框按 Enter 会调用 `load_url`；示例还包含返回上一页的处理函数。从仓库根目录运行上面的命令。其他应用请参照[示例的依赖配置](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/Cargo.toml)：这条集成路径直接依赖 `gpui-kit`、`gpui-wry`、`wry`（package 名为 `lb-wry`）和 `raw-window-handle`。

```rust
use gpui_kit::*;
use gpui_wry::WebView;

let webview = cx.new(|cx| {
    use raw_window_handle::HasWindowHandle;

    let handle = window.window_handle().expect("No window handle");
    let native = wry::WebViewBuilder::new()
        .build_as_child(&handle)
        .expect("Failed to create WebView");
    WebView::new(native, window, cx)
});

webview.update(cx, |view, _| view.load_url("https://gpui-kit.com"));

// In the owning View's render method:
div().flex_1().child(webview.clone())
```

这段代码展示 macOS 和 Windows 的子视图路径。要等 GPUI 提供有效的 `Window` 后才创建原生视图；所属 View 保存 `Entity<WebView>`，让它在多次 render 之间持续存在。完整示例先调用 `gpui_kit::init(cx)` 初始化组件，再通过 `gpui_kit::open_window(...)` 创建窗口。完整的应用启动代码请以链接的源码为准；此片段不是独立的 `main` 函数。

## 视图归属与布局

`WebView::new` 起初把原生 bounds 设为空矩形。渲染 Entity 会安装 GPUI layout element；在 `prepaint` 阶段，该 element 把算出的 bounds 以**逻辑坐标**传给 Wry，并插入 GPUI hitbox。容器必须有实际尺寸。示例使用 `div().flex_1().h(px(400.)).child(self.webview.clone())`。浏览器像素由操作系统绘制，所以 GPUI clipping、hitbox 和 content mask 都不能让 GPUI 内容盖过这个子视图。

每个原生视图保存一个 `Entity<WebView>`，不要在每次 `render` 时重新构建 Wry 视图。包装层的 `visible()` 和 `bounds()` 分别返回保存的可见状态及上次 layout 的 bounds；`show()` 和 `hide()` 改变原生可见性。如果页面或标签不再渲染该 Entity，应在需要消失时明确调用 `hide()`，恢复时调用 `show()`。隐藏期间 `prepaint` 不更新 bounds。

在 GPUI UI context 中通过 `webview.update(cx, |view, _| ...)` 调用包装层方法。`load_url(&str)` 请求导航，但丢弃 Wry 的 `Result`，也不表示加载完成。`back()` 执行 JavaScript `history.back()`，其 `Result` 只反映脚本提交，不表示历史导航成功。需要返回值时，通过 `view.raw()` 使用 Wry 的 `reload()`、`url()` 或 `evaluate_script()` 等方法。短期回调需要持有 Wry 对象时可使用 `view.handle().raw()`，但这只适用于 UI 线程；不要仅为了避免更新 Entity 而长期保留 handle。

## 加载、导航与页面消息

页面策略与回调应在 `build_as_child` **之前**配置到 `wry::WebViewBuilder` 上。它们属于 Wry，不是 `gpui-wry` 事件。仓库固定的 `lb-wry` 版本提供：

| 需求 | Wry API | 含义 |
| --- | --- | --- |
| 初始内容 | `with_url(...)` 或 `with_html(...)` | 构建前选择内容；仓库示例则在创建后调用包装层的 `load_url`。 |
| 加载状态 | `with_on_page_load_handler(|event, url| ...)` | 观察 `Started` 和 `Finished`；`Finished` 不表示加载成功，也不提供 HTTP 状态。这与 `load_url` 是否接受请求是两件事。 |
| 导航策略 | `with_navigation_handler(|url| -> bool { ... })` | 返回 `true` 允许导航，返回 `false` 取消。初始 URL 之外还要考虑链接和重定向。 |
| 新窗口请求 | `with_new_window_req_handler(...)` | 决定如何处理 `window.open`；其回调有平台相关的线程约束。 |
| Rust 向页面发送 | 在原始 Wry 视图上调用 `evaluate_script(...)` | 提交 JavaScript 并返回 Wry `Result`；需要序列化返回值时使用 `evaluate_script_with_callback`。 |
| 页面向 Rust 发送 | `with_ipc_handler(|request| ...)` | 接收 `window.ipc.postMessage(...)` 发送的字符串。执行操作前解析、验证消息。 |

例如，在构建 builder 时可附加一个只允许固定 URL 的导航策略：

```rust
let builder = wry::WebViewBuilder::new()
    .with_navigation_handler(|url| url == "https://gpui-kit.com/")
    .with_on_page_load_handler(|event, url| {
        // Send a brief status message to the owning GPUI View if the UI must update.
        // Do not capture &mut Window, &mut App, or &mut Context here.
        let phase = match event {
            wry::PageLoadEvent::Started => "started",
            wry::PageLoadEvent::Finished => "finished",
        };
        eprintln!("{phase}: {url}");
    });
```

此精确匹配仅用于展示回调形式；实际白名单应解析 URL，并明确校验 scheme 与 host。回调中的工作应保持简短。通过安全的调度或 channel 把状态和 IPC 消息送到应用持有的状态，再在 GPUI context 中更新 Entity；界面变化时调用 `cx.notify()`。Wry 回调不是 GPUI Entity 事件；Windows 的新窗口回调还会在另一线程运行。应处理回调到达时页面已导航或所属窗口已关闭的情况。

## 安全与失败边界

把页面内容视为不可信输入，包括来自自己控制的 URL 的内容。IPC 是应用能力入口：仅接受预期的消息类型与长度；平台提供发送方 origin 时应检查；执行文件、网络或账户操作前还要检查应用层授权。Wry 在 Linux/Android 上对 iframe 的 IPC request URL 使用主 frame URL，因此不能仅凭它确认 iframe 身份。初始化脚本会在新页面执行；固定版本的 Wry 文档指出，即使请求只注入主 frame，Windows 仍会注入子 frame。不要把秘密放进脚本或页面全局变量。

展示远程内容前，应决定下载、外部链接和 `window.open` 的处理策略。Wry 默认的下载开始回调允许下载；如果这不符合应用要求，应配置下载策略。在所属 GPUI View 中维护用户可见的加载和错误状态：`gpui-wry::WebView::load_url` 忽略即时错误，而成功提交请求也可能最终加载失败。分别使用原始 Wry 返回值与页面加载回调作为信号；单靠 `PageLoadEvent::Finished` 或 `load_url` 都不能证明内容成功加载。应为应用能够检测到的失败提供重试入口。

## Focus 与生命周期

WebView 是**原生子视图**，不是 GPUI 绘制的 Element。它占据 GPUI layout 节点的 bounds，并接收浏览器原生输入。`WebView` 实现了 `Focusable`，包装层跟踪一个 `FocusHandle`。调用 `hide()` 时会先把 Focus 交回父视图；点击其 bounds 之外也会请求父视图取得 Focus。界面同时使用 GPUI input 和浏览器 input 时，应验证实际键盘与 Focus 行为。

WebView 及其 handle 应在父窗口销毁前结束生命周期。销毁所属 Entity 会隐藏子视图，但克隆的 `WebViewHandle` 或帧内持有的克隆可能推迟原生视图销毁。销毁父窗口前应释放这些 handle。状态归属参见 [Entity](./entity)，窗口 handle 参见 [Window](./window)。

## 调试与验证

示例在 debug 构建或启用其 `inspector` feature 时请求启用 Wry devtools，但不会自动打开。在 macOS release 构建中，Wry 还要求启用它自己的 `devtools` feature，这项请求才会生效。Wry 的 `open_devtools()` 仅在 debug 或该依赖 feature 下可用。通过页面检查工具区分 JavaScript／网络问题与 GPUI layout 问题。遇到空白子视图，先确认 GPUI 容器的 bounds 非零、`visible()` 为 true、原生视图来自当前有效窗口。在 Windows 上还应核对下文示例的 DirectComposition 设置。导航与 IPC 需在真实原生窗口中验证；GPUI 的 headless UI 测试不能证明原生浏览器像素、系统 Focus 或 compositor 层级。

在每个受支持的操作系统上做应用级 smoke test：初次加载、链接与重定向、返回与刷新、新窗口和下载策略、加载失败／离线、IPC 输入验证、窗口尺寸变化、显示／隐藏、键盘 Focus 转移，以及关闭父窗口。纯策略解析可以单独测试。仓库示例展示加载与布局，但没有完整的导航、bridge 或错误状态测试套件。

## 当前限制

| 范围 | 当前行为 |
| --- | --- |
| 平台 | macOS 和 Windows 实验性支持。Linux 示例的 GTK hosting 路径尚未完成，不能视为已支持。 |
| Overlay 层级 | 原生 WebView 位于 GPUI surface 上方，会遮住同一矩形范围内的 GPUI 内容，包括 popover、dialog、menu 和 tooltip。GPUI overlay 无法可靠地显示在它上方。 |
| Windows renderer | 仓库示例在启动 GPUI 前设置 `GPUI_DISABLE_DIRECT_COMPOSITION=true`，以便当前子视图方案正常渲染。这是此示例的要求，不是 GPUI 的通用设置建议。 |

需要显示 overlay 时，可以将 WebView 放在单独窗口，或安排布局使 overlay 不跨过 WebView 的 bounds。当前实现不支持把普通 GPUI overlay 作为可依赖的交互显示在 WebView 上方。

## 尚未合并的 Composition 尝试

以下 PR 探索 overlay composition。**它们都不属于上文所述的当前 `gpui-wry` 行为。**应用采用实验分支前，应重新核对 PR 状态和实现：

- [GPUI Kit #2626](https://github.com/longbridge/gpui-kit/pull/2626) 尝试将 GPUI overlay 绘制在原生 WebView 上方。当前分支依赖 [Zed/GPUI #61945](https://github.com/zed-industries/zed/pull/61945)，后者为 deferred GPUI overlay 提供可选的分层 scene。GPUI Kit 这项 PR 验证了 macOS 路径；Windows composition 和 Linux hosting 在该 PR 中仍属于后续工作。
- [Zed/GPUI #62379](https://github.com/zed-industries/zed/pull/62379) 提出另一套范围更广、可选启用的 `CompositionTree`，用于编排 GPUI 与原生 surface，并带有 macOS 和 Windows 示例。它是 #61945 的替代方案，**不是** GPUI Kit #2626 的依赖。Linux composition 不在这项 PR 的范围内。

这些实验在合并前仍可能变化。它们说明了探索方向，并未消除当前的平台、overlay 和 Focus 限制。

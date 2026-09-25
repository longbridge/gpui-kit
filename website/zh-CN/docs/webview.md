---
title: WebView
description: 在 GPUI Kit 窗口中嵌入 Wry 原生 WebView，并了解当前的平台与 overlay 限制。
order: -2.32
---

# WebView

[`gpui-wry`](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) 是 GPUI Kit 基于 [Wry](https://github.com/tauri-apps/wry) 的**实验性**集成。需要浏览器行为时可以使用它；[TextView HTML](/zh-CN/component/text-view#html) 用于渲染文档内容，并不是浏览器。要在默认外部浏览器中打开 URL，使用 [`cx.open_url`](./context)。当前集成支持 macOS 和 Windows。仓库示例中的 Linux 路径尚未完成。

## 运行示例

在仓库根目录运行：

```sh
cargo run -p webview
```

[完整示例](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/src/main.rs)从 GPUI 的原生 window handle 创建 Wry 子视图，将其包装成 `Entity<WebView>`，再放进普通 GPUI layout。应用要使用同一集成时，除 `gpui-kit` 外，还需要 `gpui-wry`、`wry`（仓库使用 `lb-wry` package）和 `raw-window-handle`；版本配置可参考示例的 `Cargo.toml`。

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

// 在所属 View 的 render 方法中：
div().flex_1().child(webview.clone())
```

这段代码展示 macOS 和 Windows 的子视图路径；所属 View 应保存 `Entity<WebView>`，让它在多次 render 之间持续存在。`gpui-wry` 会根据 GPUI layout 更新原生视图的 bounds。通过 `webview.update(...)` 调用 `load_url`、`back`、`show` 和 `hide`。需要其他 Wry API 时，在 UI 线程使用 `raw()` 或 `handle().raw()`。

## Layout、Focus 与生命周期

WebView 是**原生子视图**，不是 GPUI 绘制的 Element。它占据 GPUI layout 节点的 bounds，并接收浏览器原生输入。`WebView` 实现了 `Focusable`，包装层跟踪一个 `FocusHandle`。调用 `hide()` 时会先把 Focus 交回父视图；点击其 bounds 之外也会请求父视图取得 Focus。界面同时使用 GPUI input 和浏览器 input 时，应验证实际键盘与 Focus 行为。

WebView 及其 handle 应在父窗口销毁前结束生命周期。销毁所属 Entity 会隐藏子视图，但克隆的 `WebViewHandle` 或帧内持有的克隆可能推迟原生视图销毁。销毁父窗口前应释放这些 handle。状态归属参见 [Entity](./entity)，窗口 handle 参见 [Window](./window)。

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

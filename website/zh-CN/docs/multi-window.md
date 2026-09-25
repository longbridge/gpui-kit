---
title: Multi Window
description: 在 GPUI Kit 中打开多个窗口，划分共享与窗口局部状态，并处理路由、关闭和窗口位置恢复。
order: -2.301
---

# Multi Window

一个 GPUI 应用可以同时拥有多个窗口。每个窗口都有独立的 `Window` 上下文、焦点、输入派发、几何信息和 GPUI Kit `Root`；应用数据则可以由多个窗口共享。[Window](./window) 介绍单窗口 API，本指南从第二个窗口开始，说明各窗口应持有哪些状态。

## 从空项目开始

先按[安装指南](./installation.md)准备平台依赖，然后创建应用：

```sh
cargo new gpui-multi-window
cd gpui-multi-window
```

在 `Cargo.toml` 中加入与[快速开始](./getting-started.md)相同的单一依赖：

```toml
[dependencies]
gpui-kit = "0.6"
```

用下面的完整示例替换 `src/main.rs`。如果已有单窗口应用，保留 `application().run(...)` 和 `init(cx)`，在启动闭包中创建一次共享 model，再调用第二次 `open_window`，为新窗口创建单独的内容 view。下面的循环会对两个名称各调用一次。

## 基于同一份 model 打开两个窗口

打开两个窗口前只调用一次 `gpui_kit::init`。每次调用 `gpui_kit::open_window` 都会创建新窗口，并为 builder 返回的 view 包上一层独立的 Base `Root`。返回值是 `(AnyWindowHandle, Entity<V>)`，其中 `V` 是应用传入的内容 view。builder 应返回内容 Entity，不要再包一层 `Root`。`Root` 负责该窗口的浮层、菜单、通知与焦点协调。

下面的例子让两个窗口共享一个计数器 Entity，同时为每个窗口建立自己的 `Workspace` Entity。observer 让共享数据的变化在两个窗口中显示出来；`local_clicks` 则各自独立。

```text
Application
  SharedCounter (one Entity)
    ├── First window  → Root → Workspace (first Entity, first observer)
    └── Second window → Root → Workspace (second Entity, second observer)
```

每个 `Workspace` 都持有同一个 Entity handle 的克隆。克隆 `Entity<T>` handle 不会复制其中的 `T` 值。两个 observer 观察同一份 model，但各自归属于一个窗口的内容 view。

```rust
use gpui_kit::component::button::Button;
use gpui_kit::*;

struct SharedCounter {
    count: usize,
}

struct Workspace {
    shared: Entity<SharedCounter>,
    _shared_observer: Subscription,
    local_clicks: usize,
    name: &'static str,
}

impl Workspace {
    fn new(shared: Entity<SharedCounter>, name: &'static str, cx: &mut Context<Self>) -> Self {
        let _shared_observer = cx.observe(&shared, |_, _, cx| cx.notify());
        Self { shared, _shared_observer, local_clicks: 0, name }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.shared.read(cx).count;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(format!("{}: shared count {count}", self.name))
            .child(format!("Clicks in this window: {}", self.local_clicks))
            .child(
                Button::new("increment")
                    .label("Increment")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.local_clicks += 1;
                        this.shared.update(cx, |shared, cx| {
                            shared.count += 1;
                            cx.notify();
                        });
                        cx.notify();
                    })),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        let shared = cx.new(|_| SharedCounter { count: 0 });

        for name in ["First window", "Second window"] {
            let shared = shared.clone();
            open_window(WindowOptions::default(), cx, move |_, cx| {
                cx.new(|cx| Workspace::new(shared, name, cx))
            })
            .expect("open workspace window");
        }
    });
}
```

在新项目中运行 `cargo run`，应看到两个窗口。点击任一窗口的 **Increment**：共享计数会在两个窗口中变化，局部点击数只在点击按钮的窗口中变化。关闭其中一个窗口后，另一个窗口的计数和按钮仍可使用。`SharedCounter` 在循环外只创建一次，`Workspace::new` 则为每个窗口执行一次。

按以下顺序操作，检查各处状态：

| 操作 | 第一个窗口 | 第二个窗口 |
| --- | --- | --- |
| 启动 | 共享 0；局部 0 | 共享 0；局部 0 |
| 在第一个窗口点击 **Increment** | 共享 1；局部 1 | 共享 1；局部 0 |
| 在第二个窗口点击 **Increment** | 共享 2；局部 1 | 共享 2；局部 1 |
| 关闭第一个窗口，再在第二个窗口点击 | 已关闭 | 共享 3；局部 2 |

沿着代码追踪第一次点击：按钮 listener 增加第一个 `Workspace.local_clicks`；`shared.update` 修改唯一的 `SharedCounter` 并调用它的 `cx.notify()`；两个已保存的 observer 都收到通知，并各自为所在 view 调用 `cx.notify()`。随后两个 view 重新渲染，读到相同的计数。第一个 view 修改自身的局部计数后，也调用 `cx.notify()`。`render` 中的 `self.shared.read(cx)` 只提供本次渲染使用的值；model 变化后由 observer 安排后续渲染。

要让第三个独立 workspace 共享同一计数器，可在 `for name in [...]` 数组中加入 `"Third window"` 后重新运行。点击任一窗口，三个共享计数都应增加，只有被点击窗口的局部计数增加。如果把 `SharedCounter` 的创建移到循环内，则每个窗口会拥有自己的 model，观察结果也会改变。

文档或会话数据应放在 feature 持有、需要它的窗口共同引用的 Entity 中。窗口选中项、焦点 handle、浮层状态和窗口专属 task 应放在该窗口的 view 中。只有真正全应用共享的设置和服务才使用 [Global](./global)，不要用一个 Global 收纳所有窗口的 UI 状态。共享 Entity 的通知只会到达观察它的 view；仅在 `render` 中读取它，不会自动订阅更新。应将返回的 `Subscription` 保存在观察它的 view 上：直接丢弃会停止更新，放在应用级 owner 则会不必要地延长其生命周期。

## 找到正确的窗口

`gpui_kit::open_window` 返回 `AnyWindowHandle`，因为窗口实际的根 view 是 GPUI Kit 的 `Root`，而不是应用的内容 view。需要读写内容状态时保留返回的 `Entity<V>`；需要在以后操作窗口焦点、Action 派发、激活或几何信息时保留 window handle。在 callback 内可以通过 `window.window_handle()` 取得当前窗口的 handle。启动示例中的按钮只操作各自的 view，因此可以丢弃两个返回值；文档切换器或窗口 registry 则应保留它们。

`AnyWindowHandle` 提供 `window_id()` 和 `update(...)`。`cx.windows()` 列出已打开的窗口，`cx.active_window()` 在平台支持时返回当前获得系统 Focus 的窗口。已知目标窗口时，应保留它的 handle，不要依赖遍历顺序选窗口。如果需要强类型的根 handle，可将它 downcast 为 `WindowHandle<gpui_kit::base::Root>`；downcast 为 `WindowHandle<Workspace>` 会失败，因为 `Workspace` 位于 `Root` 内。

```rust
// target is the AnyWindowHandle returned by open_window.
target.update(cx, |_, window, _cx| {
    window.activate_window();
    window.set_window_title("Document");
})?;
```

必须处理 `update` 的结果，因为目标窗口可能已经关闭。Focus 与 Action 派发使用所选窗口的上下文。如果更新 Entity 时还需要它所属的窗口，可用 `cx.update_window(target, |_, window, cx| { ... })`，在 callback 中更新内容 Entity。绑定某个窗口的异步任务可用 [`cx.spawn_in`](./task)，并在窗口或 Entity 消失后处理 `update_in` 的失败结果。

## 关闭与清理

`window.remove_window()` 请求移除当前窗口；何时退出整个应用由应用决定。若未保存内容需要拦截系统关闭请求，在对应窗口注册 `window.on_window_should_close(cx, |window, cx| { ... })`；返回 `false` 可取消关闭。直接调用 `remove_window()` 是应用决定移除窗口，因此应在调用前完成确认。窗口关闭前应读取或保存其局部状态：`cx.on_window_closed(...)` 执行时已无法访问该 `Window`，callback 只收到用于清理 registry 的 `WindowId`。

如果桌面应用希望关闭最后一个窗口时退出：

```rust
cx.on_window_closed(|cx, _closed_id| {
    if cx.windows().is_empty() {
        cx.quit();
    }
})
.detach(); // Keep this app-level observer until the application exits.
```

也可以将返回的 `Subscription` 存在应用 owner 中，随 owner 一起释放。基础示例无需关闭 observer：关闭任一窗口，另一窗口仍可使用。若应用可以从 dock 或系统托盘重新打开窗口，则应选择相应的生命周期策略。用 `WindowId` 作 key 的 registry 可以在这个 callback 中移除已关闭的 handle。窗口专属的 `Task` 和 `Subscription` 字段应随对应 view 一起释放；不要让应用级集合意外延长已关闭窗口的 view 生命周期。

关闭策略有两个不同的决策点。系统关闭请求到达 `on_window_should_close` 时，窗口仍可访问；若用户需要先处理未保存内容，应返回 `false`。窗口实际关闭后，`on_window_closed` 可以按 `WindowId` 清理 registry，或决定是否退出应用。如果按钮直接调用 `remove_window()`，应在调用之前完成确认；should-close callback 不能代替按钮自己的确认流程。

## 恢复窗口位置

窗口关闭前，读取 `window.window_bounds()`，得到可恢复的 `WindowBounds`（`Windowed`、`Maximized` 或 `Fullscreen`）。由应用设置保存这个值，下次再传给 `WindowOptions`：

```rust
let options = WindowOptions {
    window_bounds: Some(saved_bounds),
    ..Default::default()
};
open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(shared, "Restored", cx)))?;
```

这里的 `saved_bounds` 是之前窗口取得的 `WindowBounds`；如何持久化、恢复由应用负责。使用前还应检查位置是否仍落在当前连接的显示器内，因为显示器配置和缩放比可能改变。[Window 几何信息](./window#几何信息与缩放)解释全局窗口边界与窗口局部 viewport 的区别。

## 测试窗口边界

在 [`TestAppContext` 测试](./test)中，通过 `gpui_kit::open_window` 打开两个窗口，修改共享 Entity，断言两个 view 都更新，同时窗口局部状态保持独立。对第一个窗口调用 `window.remove_window()` 后，它的 handle 更新应返回错误；第二个窗口仍应能渲染和接收输入。运行 `cargo test -p gpui-kit --features test-support --test lifecycle closing_one_window_preserves_other_window_and_owned_snapshot` 可执行仓库现有的[多窗口生命周期测试](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/tests/lifecycle.rs)，覆盖关闭边界。对于本示例，还应在新项目中运行 `cargo run`，目视检查两个计数和关闭行为。

## 排查示例问题

| 现象 | 检查点 |
| --- | --- |
| 只打开一个窗口 | 检查循环中每次都调用 `open_window` 且调用成功；`.expect(...)` 的 panic 会指出打开失败。 |
| 被点击的窗口变化，但另一个窗口的共享计数未更新 | 在每个 `Workspace` 中保存 `cx.observe` 返回的 `Subscription`，并在更新 `SharedCounter` 时调用 `cx.notify()`。只读取 Entity 不会自动注册 observer。 |
| 两个窗口的局部点击数一起变化 | 每个窗口都应通过 `cx.new(...)` 创建不同的 `Workspace`；只有 `SharedCounter` 在循环外创建。 |
| Action 更新了错误窗口，或关闭后更新失败 | 保存目标窗口返回的 `AnyWindowHandle`，将它用于窗口相关操作，并处理关闭后的 `update` 错误。不要依赖 `cx.windows()` 顺序。 |
| 关闭一个窗口就退出整个应用 | 检查应用的最后一个窗口退出策略及 `on_window_closed` callback。本双窗口示例没有注册该 callback。 |

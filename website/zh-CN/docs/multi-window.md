---
title: Multi Window
description: 在 GPUI Kit 中打开多个窗口，划分共享与窗口局部状态，并处理路由、关闭和窗口位置恢复。
order: -2.301
---

# Multi Window

一个 GPUI 应用可以同时拥有多个窗口。每个窗口都有独立的 `Window` 上下文、Focus、输入派发、几何信息和 GPUI Kit `Root`；应用数据则可以由多个窗口共享。[Window](./window) 介绍单窗口 API，这里重点说明多窗口下的所有权和生命周期。

## 基于同一份 model 打开两个窗口

`gpui_kit::init` 只需调用一次。每次调用 `gpui_kit::open_window` 都会创建新窗口，并为 builder 返回的 view 包上一层独立的 Base `Root`。返回值是 `(AnyWindowHandle, Entity<V>)`，其中 `V` 是应用传入的内容 view；不要再为它包一层 `Root`。

下面的例子让两个窗口共享一个计数器 Entity，同时为每个窗口建立自己的 `Workspace` Entity。observer 让共享数据的变化在两个窗口中显示出来；`local_clicks` 则各自独立。

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

文档或会话数据应放在 feature 持有、需要它的窗口共同引用的 Entity 中。窗口选中项、Focus handle、overlay 状态和窗口专属 task 应放在该窗口的 view 中。只有真正全应用共享的设置和服务才使用 [Global](./global)，不要用一个 Global 收纳所有窗口的 UI 状态。共享 Entity 的通知只会到达观察它的 view；仅在 `render` 中读取它，不会自动订阅更新。

## 找到正确的窗口

`gpui_kit::open_window` 返回 `AnyWindowHandle`，因为窗口实际的根 view 是 GPUI Kit 的 `Root`，而不是应用的内容 view。需要读写内容状态时保留返回的 `Entity<V>`；需要在以后操作窗口 Focus、Action 派发、激活或几何信息时保留 window handle。在 callback 内可以通过 `window.window_handle()` 取得当前窗口的 handle。

`AnyWindowHandle` 提供 `window_id()` 和 `update(...)`。`cx.windows()` 列出已打开的窗口，`cx.active_window()` 在平台支持时返回当前获得系统 Focus 的窗口。已知目标窗口时，应保留它的 handle，不要依赖遍历顺序选窗口。如果需要强类型的根 handle，可将它 downcast 为 `WindowHandle<gpui_kit::base::Root>`；downcast 为 `WindowHandle<Workspace>` 会失败，因为 `Workspace` 位于 `Root` 内。

```rust
// target 是 open_window 返回的 AnyWindowHandle。
target.update(cx, |_, window, _cx| {
    window.activate_window();
    window.set_window_title("Document");
})?;
```

必须处理 `update` 的结果，因为目标窗口可能已经关闭。Focus 与 Action 派发使用所选窗口的上下文。如果更新 Entity 时还需要它所属的窗口，可用 `cx.update_window(target, |_, window, cx| { ... })`，在 callback 中更新内容 Entity。绑定某个窗口的异步任务可用 [`cx.spawn_in`](./task)，并在窗口或 Entity 消失后处理 `update_in` 的失败结果。

## 关闭与清理

`window.remove_window()` 关闭当前窗口；何时退出整个应用由应用决定。若未保存内容需要拦截系统关闭请求，在该窗口注册 `window.on_window_should_close(cx, ...)`。窗口关闭前应读取或保存其局部状态：`cx.on_window_closed(...)` 执行时已无法访问该 `Window`，callback 只收到用于清理 registry 的 `WindowId`。

如果桌面应用希望关闭最后一个窗口时退出：

```rust
cx.on_window_closed(|cx, _closed_id| {
    if cx.windows().is_empty() {
        cx.quit();
    }
})
.detach(); // 此 app 级 observer 有意持续到应用结束。
```

也可以将返回的 `Subscription` 存在应用 owner 中，随 owner 一起释放。若应用可以从 dock 或系统托盘重新打开窗口，则应选择相应的生命周期策略。用 `WindowId` 作 key 的 registry 可以在这个 callback 中移除已关闭的 handle。窗口专属的 `Task` 和 `Subscription` 字段应随对应 view 一起释放；不要让应用级集合意外延长已关闭窗口的 view 生命周期。

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

在 [`TestAppContext` 测试](./test)中，通过 `gpui_kit::open_window` 打开两个窗口，修改共享 Entity，断言两个 view 都更新，同时窗口局部状态保持独立。对第一个窗口调用 `window.remove_window()` 后，它的 handle 更新应返回错误；第二个窗口仍应能渲染和接收输入。仓库现有的[多窗口生命周期测试](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/tests/lifecycle.rs)覆盖了关闭行为。

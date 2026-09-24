---
title: Global
description: 使用 GPUI Global 共享应用级状态，并将变化传递给 View 和窗口。
order: -2.632
---

# Global

`Global` 是标记 trait，让 GPUI 按具体 Rust 类型在每个 [`App`](./context) 中保存一份值。多个功能和窗口共同使用的设置或服务适合放在这里。`AppSettings` 与 `Theme` 使用不同的存储位置。`Global` 本身只有 `'static` 约束，不会让值成为 View，也不会自动建立事件流。

```rust
use gpui_kit::*;

struct AppSettings {
    notifications_enabled: bool,
}

impl Global for AppSettings {}
```

这份值属于当前 `App`，不属于某个 `Window` 或 `Entity`。应在应用启动时、View 读取之前初始化。对相同类型再次调用 `set_global` 会替换旧值，不会合并字段。

## 读取与修改全局值

| API | 结果 | 用途 |
| --- | --- | --- |
| `cx.has_global::<T>()` | `bool` | 检查是否已安装该类型。 |
| `cx.global::<T>()` | `&T` | 读取必需的值；不存在时 panic。 |
| `cx.try_global::<T>()` | `Option<&T>` | 读取可选值。 |
| `cx.set_global(value)` | `()` | 安装或替换值，并通知全局观察者。 |
| `cx.global_mut::<T>()` | `&mut T` | 修改已安装的值，并通知全局观察者。 |
| `cx.update_global::<T, _>(\|value, cx\| …)` | 闭包结果 | 修改已安装的值，同时使用 GPUI context；修改结束时通知观察者。 |
| `cx.default_global::<T>()` | `&mut T` | 读取或安装 `T::default()`，并获得可变引用；要求 `T: Default`。 |
| `cx.update_default_global::<T, _>(\|value, cx\| …)` | 闭包结果 | 修改值；若尚未安装，先安装默认值。 |
| `cx.remove_global::<T>()` | `T` | 移除已安装的值，并通知全局观察者。 |

`global_mut`、`update_global` 和 `remove_global` 都要求值已经存在。读取不会通知观察者。可变及默认值 API 即使没有真正改动值，也会安排一次通知；当重复工作有成本时，先比较值再写入。`remove_global` 也会通知：移除后读取该位置的观察者应使用 `try_global`，或以其他方式处理值不存在的情况。`global`、`global_mut` 和 `default_global` 返回的引用只在当前 GPUI 调用期间有效；之后需要数据时应复制或 clone。

`update_global` 暂时取出全局值，让闭包同时获得 `&mut T` 和 `&mut cx`。直接使用传给闭包的 `value`，不要在闭包内部再次读取或更新同一份全局值。

```rust
cx.update_global::<AppSettings, _>(|settings, _cx| {
    settings.notifications_enabled = false;
});
```

`Context<T>` 可以调用这些应用级 API，因为它会解引用到 `App`。应用初始化等不属于某个 Entity 的代码直接接收 `&mut App`。

## App 范围与 Window 范围

当所有窗口都应看到同一个值时，使用 Global，例如应用偏好、主题或共享服务的句柄。Focus、输入派发、窗口尺寸等行为应交给 [Window](./window)。Global 在整个应用中只有一份；把各窗口独立的选择状态放入同一个 Global，会增加所有权与清理的难度。

业务功能的状态应保存在由该功能的 crate 或 View 拥有的 [Entity](./entity) 中。`Global` 只适合真正由整个应用共享的服务、设置与协调状态；不能因为跨模块传值不便，就把庞大的业务数据集合搬进应用级存储。功能之间需要协作时，若所有权边界允许，可以传递轻量的 Entity 句柄；否则使用明确的接口、command 或 event。[编码指南](./coding-guides) 进一步说明如何把每项功能的 model 与 workflow 留在其模块边界内。

GPUI Kit 的 `Theme` 展示了应用级所有权。调用 `gpui_kit::init(cx)` 后，组件通过 `cx.theme()` 读取当前主题。GPUI Kit 还需要同步供底层使用的主题数据，并在主题变化后刷新窗口。切换模式使用 `Theme::change(...)`，编辑主题使用 `Theme::update(cx, |theme| { … })`。直接通过 `Theme::global_mut(cx)` 编辑字段，不会完成这些同步，也不会刷新所有窗口。这是 GPUI Kit 主题在普通 GPUI `Global` 行为之上的专门规则。

## 让变化显示到界面上

修改 Global 会通知**全局观察者**。GPUI 把通知放进 effect 队列；同一类型在通知尚未处理时的重复改动会合并。回调读取的是当前值，而不是某个中间快照或逐字段变化。Global 变化不会自动对所有曾经读取它的 Entity 调用 `cx.notify()`。如果一个 View 的渲染结果依赖 Global，可以注册 `cx.observe_global::<T>(...)`，并在回调里通知当前 View。把返回的 `Subscription` 保存在 View 中，让观察者与 View 一起存活。

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::component::button::Button;

struct AppSettings {
    notifications_enabled: bool,
}

impl Global for AppSettings {}

struct SettingsView {
    _settings_observer: Subscription,
}

impl SettingsView {
    fn new(cx: &mut Context<Self>) -> Self {
        let _settings_observer = cx.observe_global::<AppSettings>(|_this, cx| {
            cx.notify();
        });
        Self { _settings_observer }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = cx.global::<AppSettings>().notifications_enabled;

        div().child(
            Button::new("toggle-notifications")
                .label(if enabled { "关闭通知" } else { "开启通知" })
                .on_click(|_, _window, cx| {
                    cx.update_global::<AppSettings, _>(|settings, _cx| {
                        settings.notifications_enabled = !settings.notifications_enabled;
                    });
                }),
        )
    }
}

fn main() {
    application()
        .with_assets(Assets)
        .run(|cx| {
            init(cx);
            cx.set_global(AppSettings {
                notifications_enabled: true,
            });

            open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(SettingsView::new)
            })
            .expect("failed to open window");
        });
}
```

按钮修改应用级设置，观察者把这个 View 标记为需要更新；下一次渲染会读到新值。如果 View 使用了[缓存](./view-cache)，这次通知尤为必要：仅重绘父级仍可能重放子级的旧内容。其他窗口若各自创建了 `SettingsView`，也能观察同一份 Global 并更新。`SettingsView` 释放时，它保存的 `Subscription` 一起释放，回调随之断开。`observe_global` 只表示该类型的值被触及，不携带具体变化的类型化数据；消费者需要知道操作或数据时，使用 Entity 发出的 [Event](./event)。

如果回调还需要窗口，使用 `cx.observe_global_in::<T>(window, ...)`，并保存其 `Subscription`。如果没有拥有者 Entity，也可以用 `window.observe_global::<T>(cx, ...)` 注册窗口级观察者；回调会收到 `&mut Window` 和 `&mut App`。同样需要将订阅保存在合适的 owner 中。`window.refresh()` 或 `cx.refresh_windows()` 可显式请求渲染；上面的 View 已在观察者中调用 `cx.notify()`，因此不需要它们。

## 常见错误

- 安装 `T` 之前就调用 `cx.global::<T>()`：这会 panic。应先初始化，或者对可选值使用 `try_global`。
- 认为 `set_global` 或 `update_global` 会自动重绘所有读取者：应注册全局观察者并通知依赖它的 View，或者使用 GPUI Kit 主题更新等会明确刷新窗口的 API。
- 让 `observe_global` 返回的 `Subscription` 在构造函数结束时被 drop：观察会立即停止。
- 把窗口独立的 Focus、选择状态或文档状态放进唯一的应用级 Global：让 Window 或 Entity 拥有它；确实需要应用级协调时，再明确按窗口或文档 ID 保存。
- 通过 `global_mut` 修改 GPUI Kit 主题后，期待所有主题映射和窗口自动更新：应使用主题专用 API。
- 在 `render` 中修改 Global：GPUI 可能因为很多原因重新渲染。应在输入或其他副作用回调中修改状态，再通过观察者让 View 更新。

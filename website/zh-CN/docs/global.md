---
title: Global
description: 使用 GPUI Global 共享应用级状态，并将变化传递给 View 和窗口。
order: -2.632
---

# Global

`Global` 是标记 trait，让 GPUI 按具体 Rust 类型在每个 [`App`](./context) 中保存一份值。多个功能和窗口共同使用的设置或服务适合放在这里。`AppSettings` 与 [`Theme`](../component/theme) 使用不同的存储位置。`Global` 本身只有 `'static` 约束，不会让值成为 View，也不会自动建立事件流。

```rust
use gpui_kit::*;

struct AppSettings {
    notifications_enabled: bool,
}

impl Global for AppSettings {}
```

这份值属于当前 `App`，不属于某个 [`Window`](./window) 或 [`Entity`](./entity)。应在应用启动时、View 读取之前初始化。对相同类型再次调用 `set_global` 会替换旧值，不会合并字段。

## 选择状态的所有者

| 所有者 | 适用状态 | 变化如何传递给 View |
| --- | --- | --- |
| `App` 中的 `Global` | 整个应用共享的一份设置或服务，包括多个窗口。 | 观察该 Global 类型并通知依赖它的 View，或使用会刷新窗口的领域 API。 |
| `Entity<T>` | 有自身生命周期和行为的文档、功能 model 或 View 状态。 | 更新 Entity；其渲染状态变化时调用 `cx.notify()`。其他所有者可以观察它或订阅其事件。 |
| `Window` | 属于单个窗口的 Focus、输入派发、尺寸等状态或操作。 | 根据需要使用 Window 自身的失效或刷新 API。 |

值的*作用范围*并不决定哪些界面会重绘。在 `render` 中读取 Global 不会自动建立依赖。反过来，即使 View 不负责修改某个 Global，也可以观察它。如果多个窗口中的 View 都渲染这项设置，每个 View 都需要保留自己的观察者。

可以用两个编辑器窗口来判断归属：通知偏好对两个窗口都生效，适合放在唯一的 `AppSettings` Global 中；各窗口选中的标签和点击次数属于各自的 View Entity；同时在两个窗口打开的文档可以由一个共享的 `Entity<Document>` 保存，再让两个 View 观察它。这分别对应应用、窗口 View 和文档的生命周期。仅为方便访问而把值搬进 Global，不会自动获得正确的生命周期或响应式渲染。共享 model 的练习见[多窗口](./multi-window)。

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

`global_mut`、`update_global` 和 `remove_global` 都要求值已经存在。读取不会通知观察者。可变及默认值 API 即使没有真正改动值，也会安排一次通知。若重复工作有成本，应在调用可变 API **之前**用 `global::<T>()` 检查；进入 `update_global` 后再比较，已无法避免这次通知。`remove_global` 也会通知：移除后读取该位置的观察者应使用 `try_global`，或以其他方式处理值不存在的情况。`global`、`global_mut` 和 `default_global` 返回的引用只在当前 GPUI 调用期间有效；之后需要数据时应复制或 clone。

`update_global` 暂时取出全局值，让闭包同时获得 `&mut T` 和 `&mut cx`。直接使用传给闭包的 `value`，不要在闭包内部再次读取或更新同一份全局值。

```rust
cx.update_global::<AppSettings, _>(|settings, _cx| {
    settings.notifications_enabled = false;
});
```

[`Context<T>`](./context) 可以调用这些应用级 API，因为它会解引用到 `App`。应用初始化等不属于某个 Entity 的代码直接接收 `&mut App`。

## App 范围与 Window 范围

当所有窗口都应看到同一个值时，使用 Global，例如应用偏好、主题或共享服务的句柄。Focus、输入派发、窗口尺寸等行为应交给 [Window](./window)。Global 在整个应用中只有一份；把各窗口独立的选择状态放入同一个 Global，会增加所有权与清理的难度。

业务功能的状态应保存在由该功能的 crate 或 View 拥有的 Entity 中。`Global` 只适合真正由整个应用共享的服务、设置与协调状态；不能因为跨模块传值不便，就把庞大的业务数据集合搬进应用级存储。功能之间需要协作时，若所有权边界允许，可以传递轻量的 Entity 句柄；否则使用明确的接口、command 或 event。[编码指南](./coding-guides) 进一步说明如何把每项功能的 model 与 workflow 留在其模块边界内。

GPUI Kit 的 `Theme` 展示了应用级所有权。调用 `gpui_kit::init(cx)` 后，组件通过 `cx.theme()` 读取当前主题。GPUI Kit 还需要同步供底层使用的主题数据，并在主题变化后刷新窗口。切换模式使用 `Theme::change(...)`，编辑主题使用 `Theme::update(cx, |theme| { … })`。直接通过 `Theme::global_mut(cx)` 编辑字段，不会完成这些同步，也不会刷新所有窗口。这是 GPUI Kit 主题在普通 GPUI `Global` 行为之上的专门规则。

## 动手构建双窗口示例

修改 Global 会通知**全局观察者**。GPUI 把通知放进 effect 队列；同一类型在通知尚未处理时的重复改动会合并。回调读取的是当前值，而不是某个中间快照或逐字段变化。Global 变化不会自动对所有曾经读取它的 Entity 调用 `cx.notify()`。如果一个 View 的渲染结果依赖 Global，可以注册 `cx.observe_global::<T>(...)`，并在回调里通知当前 View。把返回的 `Subscription` 保存在 View 中，让观察者与 View 一起存活。

使用仓库现有的 `examples/hello_world` workspace package。将下面的完整代码放入 `examples/hello_world/src/main.rs`，在仓库根目录运行 `cargo run -p hello_world --bin hello_world`；不需要增加 package 或依赖。示例先创建一份 Global，再打开两个窗口，每个窗口创建一个 `SettingsView` Entity。Global 保存共享偏好；`local_clicks` 只属于所在窗口的 Entity。两个按钮都在 `render` 之外修改状态。

共享状态的完整路径是：启动时安装值 → 创建每个 View 并保存观察者 → 在 `render` 中读取值 → 在输入回调中修改值 → 每个观察者调用所属 Entity 的 `cx.notify()` → 两个 View 重新渲染并读取当前值。设置不会被复制到 View 的另一个字段里。

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
    local_clicks: usize,
    name: &'static str,
}

impl SettingsView {
    fn new(name: &'static str, cx: &mut Context<Self>) -> Self {
        let _settings_observer = cx.observe_global::<AppSettings>(|_this, cx| {
            cx.notify();
        });
        Self {
            _settings_observer,
            local_clicks: 0,
            name,
        }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = cx.global::<AppSettings>().notifications_enabled;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(format!("{}: notifications {}", self.name, if enabled { "on" } else { "off" }))
            .child(format!("Clicks in this window: {}", self.local_clicks))
            .child(
                Button::new("toggle-notifications")
                    .label("Toggle shared setting")
                    .on_click(|_, _window, cx| {
                        cx.update_global::<AppSettings, _>(|settings, _cx| {
                            settings.notifications_enabled = !settings.notifications_enabled;
                        });
                    }),
            )
            .child(
                Button::new("local-click")
                    .label("Increment local clicks")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.local_clicks += 1;
                        cx.notify();
                    })),
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

            for name in ["First window", "Second window"] {
                open_window(WindowOptions::default(), cx, move |_, cx| {
                    cx.new(|cx| SettingsView::new(name, cx))
                })
                .expect("failed to open window");
            }
        });
}
```

在任一窗口点击 **Toggle shared setting**，两个窗口都应在 `notifications on` 和 `notifications off` 之间切换。在一个窗口点击 **Increment local clicks**，只有该窗口的数字变化。关闭其中一个窗口后，另一个窗口的两个按钮仍能使用：它自己的 Entity 和应用 Global 仍在。第一个按钮通过 `update_global` 修改 Global；两个已保存的观察者分别通知所属 Entity，两个 View 都会在渲染时读到新值。第二个按钮只修改所在窗口的 Entity，并调用该 Entity 的 `cx.notify()`；它不会修改 Global。若移除观察者字段，订阅随即释放，共享标签不会再跟随之后的 Global 更新。若移除本地按钮里的 `cx.notify()`，计数在内存中已改变，但画面可能在下次其他原因触发的渲染前一直显示旧值。

当 View 使用了[缓存](./view-cache)，观察者通知尤为必要：仅重绘父级仍可能重放子级的旧内容。一个 `SettingsView` 释放时，它保存的 `Subscription` 也释放，那个窗口的回调断开；另一个窗口的观察者继续工作。`observe_global` 只表示该类型的值被触及，不携带具体变化的类型化数据；消费者需要知道操作或数据时，使用 Entity 发出的 [Event](./event)。

可以做一个排错练习：暂时注释观察者里的 `cx.notify()`，运行程序并切换设置。Global 已变化，但两个标签不一定立即更新。继续前恢复调用。这样可以把“状态没有修改”和“缺少失效通知”区分开。恢复调用后若标签仍不更新，检查每个 View 是否保存了自己的 `Subscription`，以及修改是否经过 GPUI 的 Global API。

如果回调还需要窗口，使用 `cx.observe_global_in::<T>(window, ...)`，并保存其 `Subscription`。如果没有拥有者 Entity，也可以用 `window.observe_global::<T>(cx, ...)` 注册窗口级观察者；回调会收到 `&mut Window` 和 `&mut App`。同样需要将订阅保存在合适的 owner 中。`window.refresh()` 或 `cx.refresh_windows()` 可显式请求渲染；上面的 View 已在观察者中调用 `cx.notify()`，因此不需要它们。

## 常见错误

- 安装 `T` 之前就调用 `cx.global::<T>()`：这会 panic。应先初始化，或者对可选值使用 `try_global`。
- 认为 `set_global` 或 `update_global` 会自动重绘所有读取者：应注册全局观察者并通知依赖它的 View，或者使用 GPUI Kit 主题更新等会明确刷新窗口的 API。
- 让 `observe_global` 返回的 `Subscription` 在构造函数结束时被 drop：观察会立即停止。
- 把窗口独立的 Focus、选择状态或文档状态放进唯一的应用级 Global：让 Window 或 Entity 拥有它；确实需要应用级协调时，再明确按窗口或文档 ID 保存。
- 通过 `global_mut` 修改 GPUI Kit 主题后，期待所有主题映射和窗口自动更新：应使用主题专用 API。
- 在 `render` 中修改 Global：GPUI 可能因为很多原因重新渲染。应在输入或其他副作用回调中修改状态，再通过观察者让 View 更新。
- 通过 Global 内的 `Arc`、锁、原子变量或其他内部可变句柄修改数据，却期待 Global 观察者运行：GPUI 只能感知通过 Global 修改 API 进行的操作。应明确通知依赖该数据的所有者；频繁变化的数据可交给可观察的 Entity。

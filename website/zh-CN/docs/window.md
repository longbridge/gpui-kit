---
title: Window
description: 使用 GPUI Window 处理窗口内的输入、Focus、绘制与异步任务。
order: -2.3
---

# Window

GPUI 提供 [`Window`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Window.html) 作为单个平台窗口的上下文。它把渲染后的 Element 树与[平台输入](./event#鼠标与键盘输入事件)、Focus、Action 派发、绘制和窗口控制连接起来。GPUI 只会在更新或渲染这个窗口时，把它传给 View：

```rust
impl Render for Chat {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = window.is_window_active();

        div()
            .track_focus(&self.focus_handle)
            .when(!active, |this| this.opacity(0.8))
            .child("Chat")
    }
}
```

应用状态应保存在 [Entity](./entity) 中。一个操作属于当前窗口，或者依赖窗口当前的交互状态时，才使用 `Window`。

`App` 提供应用级服务、[全局状态](./global)和 Entity 访问。[`Context<Self>`](./context) 增加当前 Entity 的操作，包括 `cx.notify()`、listener、event 和 task。`Window` 则保存**单个**窗口的 Focus、派发、输入、[元素 keyed state](./element_id)、测量与绘制。这些都是 callback 执行期间临时提供的上下文；稍后需要继续工作时，保存 `Entity`、`FocusHandle`、task、subscription 或 window handle，不要保存 `&mut Window` 或 `&mut Context<_>`。

## 打开并持有窗口

打开窗口前调用 `gpui_kit::init(cx)`。`gpui_kit::open_window` 创建 GPUI 窗口，并在 builder 返回的 View 外包一层 Base `Root`。它同时返回 window handle 和应用内容 Entity，供应用持有其需要的部分：

```rust
use gpui_kit::*;

application().run(|cx| {
    init(cx);
    let (window_handle, workspace) = open_window(
        WindowOptions::default(),
        cx,
        |window, cx| cx.new(|cx| Workspace::new(window, cx)),
    )
    .expect("open workspace window");

    // Keep these handles in the application owner for later use.
});
```

[`WindowOptions`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.WindowOptions.html) 控制初始位置和尺寸、Focus、是否显示、窗口类型、最小尺寸等平台选项。builder 只在构造期间获得 `Window`。window handle 可供稍后的代码发起更新，但窗口关闭后，基于 handle 的更新可能失败。多窗口应用要使用目标窗口的 handle 来操作其 Focus 或几何信息；只有 Entity handle 并不能确定窗口。

`open_window` 返回 `AnyWindowHandle`，因为实际的 GPUI root 是 `gpui_kit::base::Root`，不是 `Workspace`。后续拿到 `&mut App` 的 callback 可通过 handle 进入对应窗口，使用前要检查窗口是否仍然存在：

```rust
if window_handle
    .update(cx, |_, window, _| window.activate_window())
    .is_err()
{
    // The window has closed.
}
```

callback 的第一个参数是 Base `Root` view；应用内容的更新应使用 `open_window` 返回的 `workspace` Entity。window handle 选择窗口，Entity 则选择要更新的状态。

## 动手练习：更新并关闭单个窗口

这个练习复用现有的 `hello_world` 包。将 `examples/hello_world/src/main.rs` 暂时替换为以下代码，然后在仓库根目录运行 `cargo run -p hello_world`：

```rust
use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

struct WindowPractice {
    renamed: bool,
}

impl Render for WindowPractice {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = if self.renamed {
            "Title: Second title"
        } else {
            "Title: First title"
        };

        div()
            .v_flex()
            .gap_2()
            .p_4()
            .child(status)
            .child(
                Button::new("rename")
                    .label("Change title")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.renamed = !this.renamed;
                        let title = if this.renamed {
                            "Second title"
                        } else {
                            "First title"
                        };
                        window.set_window_title(title);
                        cx.notify();
                    })),
            )
            .child(
                Button::new("close")
                    .label("Close window")
                    .on_click(|_, window, _| window.remove_window()),
            )
    }
}

fn main() {
    application().run(|cx| {
        gpui_kit::init(cx);
        open_window(WindowOptions::default(), cx, |window, cx| {
            window.set_window_title("First title");
            cx.new(|_| WindowPractice { renamed: false })
        })
        .expect("open practice window");
    });
}
```

连续点击两次 **Change title**：系统窗口标题与 View 内的文字应同步在两个标题间切换。`renamed` 是 Entity 中持续保存的状态；`cx.notify()` 让更新后的文字显示出来。`window.set_window_title(...)` 直接修改当前原生窗口的标题，不依赖这次 Entity 通知。点击 **Close window** 会请求移除当前窗口。按钮 callback 中的 `&mut Window` 只在这次调用期间有效；窗口被移除后，之前保存的 window handle 再执行更新可能返回错误。

如果内容文字变了但标题没变，检查桌面环境是否显示原生窗口标题，以及 `set_window_title` 是否在按钮 callback 中执行。如果两者都没变，确认 `init(cx)` 先于 `open_window` 执行、按钮有 `on_click` listener，且修改 `renamed` 后调用了 `cx.notify()`。练习结束后恢复原来的 `main.rs`。多个窗口之间的持有关系继续阅读 [Multi Window](./multi-window)。

## Window 负责什么

常见的窗口级操作包括：

| 需求 | API |
| --- | --- |
| 读取尺寸和状态 | `bounds`、`viewport_size`、`scale_factor`、`is_window_active` |
| 管理 Focus | `focused`、`focus`、`blur`、`focus_next`、`focus_prev` |
| 从代码下达命令 | `dispatch_action` |
| 重绘或安排下一帧 callback | `refresh`、`request_animation_frame`、`on_next_frame` |
| 控制原生窗口 | `set_window_title`、`activate_window`、`remove_window` |
| 稍后继续工作 | `defer`、`spawn` |

`Window` 内部还承载布局、文本、hit testing、输入和绘制状态。大多数 View 不需要直接操作这些系统；Element 和 GPUI 会在渲染期间使用它们。

## 几何信息与缩放

`window.bounds()` 返回系统窗口在**全局**坐标空间的矩形，可能横跨多个显示器。`window.viewport_size()` 返回可绘制内容区域在窗口局部坐标中的尺寸，单位是逻辑 `Pixels`。让局部 overlay 保持在内容区域内时，应使用 viewport 尺寸；保存窗口位置时，应使用包含可恢复窗口状态的 `window.window_bounds()`。在支持的平台上，`window.inner_window_bounds()` 会排除系统 inset。

`window.scale_factor()` 把逻辑像素换算为物理显示像素：系数 `2.0` 表示每个轴向上一个逻辑像素占两个设备像素。窗口移到另一块屏幕时，它可能变化。GPUI 布局尺寸不需要乘这个系数；只在原生平台集成等真正需要设备像素的边界使用。移动设备键盘出现时，`visual_viewport_bounds()` 可能缩小或移动，而 `viewport_size()` 仍表示布局区域。

这些值的坐标原点不同：`bounds().origin` 属于全局显示坐标，而指针事件和 `visual_viewport_bounds()` 使用窗口局部的逻辑坐标。不要直接拿指针位置与保存的全局窗口原点比较。若 overlay 必须避开系统 inset 或软键盘，可使用 `window.fully_visible_bounds()` 获得保守的窗口局部矩形；平台未报告的遮挡区域不在其计算范围内。

[Dialog 实现](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/dialog/dialog.rs) 使用 `window.viewport_size()` 和窗口边框 padding，让弹层保持在可用内容内。[原生菜单集成](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/windows.rs) 在平台坐标边界读取 `scale_factor()`。标准组件已经持有 overlay 或原生控件时，应让它们完成这些计算。

## Focus 与 Action 派发

Focus 属于某个具体的 Window。`window.focus(...)` 选择一个 `FocusHandle`，`window.focused(cx)` 返回当前 handle。还需通过 `.track_focus(&handle)` 将它附着到渲染后的 Element，才能在 Dispatch Path 中找到对应节点；仅创建 handle 并不会形成键盘目标。被跟踪的 handle 也不会自动进入 Tab 顺序，创建时需通过 `cx.focus_handle().tab_stop(true)` 显式加入。键盘输入随后从获得 Focus 的 Element 建立 Dispatch Path，用它匹配 [KeyBinding](./keybinding) 并派发 Action。

添加快捷键前，可跟着 [Focus 教程](./focus)建立并验证键盘目标、Tab 顺序和浮层关闭后的焦点恢复。

```rust
fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    window.focus(&self.composer_focus, cx);
}

fn on_action_open_conversation(
    &mut self,
    action: &OpenConversation,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    self.open(action.id, cx);
    window.focus(&self.composer_focus, cx);
}
```

当按钮、命令面板、原生菜单或其他代码要下达与 KeyBinding 相同的命令时，使用 `window.dispatch_action(action.boxed_clone(), cx)`。GPUI 会记录当前 Focus，并把真正的派发推迟到当前 effect cycle 结束以后。

```rust
Button::new("open-conversation")
    .label("Open")
    .on_click(|_, window, cx| {
        window.dispatch_action(Box::new(OpenConversation { id }), cx);
    })
```

这个 Action 仍沿当前 Focus 对应的 Dispatch Path 传递。它的 `on_action_*` handler 应位于这条路径上，通常挂在当前 region 或共同 owner 上。完整的路由机制见 [Action](./action)。

## 在当前更新结束后执行

当一个操作必须等当前正在更新的 Entity 被释放以后才能执行时，使用 `window.defer`。关闭 overlay 后调整 Focus，或继续修改同一棵 UI 树的其他部分，通常都需要这样处理。

```rust
fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let composer = self.composer.clone();

    window.defer(cx, move |window, cx| {
        let focus_handle = composer.read(cx).focus_handle(cx);
        window.focus(&focus_handle, cx);
    });
}
```

在 Entity 内部，`cx.defer_in(window, ...)` 通常更方便，因为 GPUI 会再次把这个 Entity 传给 callback：

```rust
cx.defer_in(window, |this, window, cx| {
    this.rebuild_results(window, cx);
});
```

callback 已经获得 `&mut Self`，不要在里面通过 handle 对同一个 Entity 调用 `update`，否则会再次更新一个正在更新中的 Entity。可以用 `defer` 和 `defer_in` 安排不能在当前 callback 中安全完成的 Focus 变化和 UI 树修改。

只有操作明确属于下一次帧 callback 时才使用 `window.on_next_frame(...)`，例如推进一帧动画。`defer` 表示“当前 effect cycle 结束后”，两者不是同一个时机。

## 重绘与帧生命周期

修改 Entity 后调用 `cx.notify()`，会把该 Entity 标记为需要渲染。`window.refresh()` 则把**整个窗口**标记为 dirty，供下一次绘制使用；窗口级状态变化无法通过 Entity 通知覆盖时可用它，例如平台或 overlay 状态变化。两者都不应该无条件写在 render 路径中。

`window.on_next_frame(callback)` 在下一次平台帧 tick 中运行 callback，时机在该 tick 可能进行的绘制之前。它会产生帧需求，但本身不把窗口标记为 dirty。`window.request_animation_frame()` 会记录当前正在渲染的 View，在下一次 tick 通知它。当前锁定的 `gpui-pre` 0.3.6 实现会立即调用 `current_view()`，因此只应在 GPUI 有当前 View 的渲染路径中使用；在该路径之外，使用 `on_next_frame`，并显式通知 Entity 或调用 `window.refresh()`。只有运动还需要下一个采样时才调用。[GPUI 动画与 Base Motion](./animation) 已为各自动画处理帧请求和减弱动态效果。

帧 callback 若修改了窗口外部状态，应在 callback 中调用 `window.refresh()`，让变化进入绘制。若修改 Entity 状态，则在更新该 Entity 的 callback 中调用 `cx.notify()`。仅有帧 tick 不会重绘没有被标记为 dirty 的窗口。

渲染会根据保留的 Entity 状态生成新 Element 树，随后 GPUI 计算布局、在 prepaint 阶段确定输入几何，最后绘制场景。`Window` 含有各阶段的方法，但应用 View 通常只需在 `render` 中描述元素；只有需要已解析边界的自定义 Element 才使用后续阶段的 hook。在 `render` 中无条件调用 `cx.notify()`、`window.refresh()` 或 `window.request_animation_frame()`，即使 UI 空闲也会持续消耗资源。参见 [Render](./render) 与 [Element](./element)。

## 需要 Window 的异步任务

当 [Task](./task) 属于当前 Entity，并且完成后需要同时访问 Entity 与 Window 时，使用 `cx.spawn_in(window, ...)`：

```rust
struct Chat {
    load_task: Option<Task<()>>,
}

fn load_conversation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.load_task = Some(cx.spawn_in(window, async move |this, cx| {
        let Ok(messages) = fetch_messages().await else { return };

        this.update_in(cx, |this, _window, cx| {
            this.messages = messages;
            cx.notify();
        })
        .ok();
    }));
}
```

`cx.spawn_in` 提供 `WeakEntity<Self>` 与 `AsyncWindowContext`。如果 Entity 或 Window 已经消失，`update_in` 会返回错误；应传播或处理错误，不能假定它们仍然存在。

任务需要 Window、但不属于某个 Entity 时，使用 `window.spawn(cx, ...)`；不需要 Window 时使用 `cx.spawn(...)`；CPU 密集型工作使用 `cx.background_spawn(...)`。`Task` 被 drop 时任务会取消，因此当任务生命周期应跟随 View 时，把它存进 View；只有任务确实应该独立继续时才调用 `.detach()`。

## 订阅中访问 Window

Event callback 需要 `&mut Window` 时使用 `cx.subscribe_in`。例如，child 完成操作后让 owner 恢复 Focus：

```rust
struct Workspace {
    chat: Entity<Chat>,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    fn new(chat: Entity<Chat>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions = vec![
            cx.subscribe_in(&chat, window, |_this, chat, event, window, cx| {
                if let ChatEvent::ConversationOpened = event {
                    window.focus(&chat.read(cx).focus_handle(cx), cx);
                }
            }),
        ];

        Self { chat, _subscriptions }
    }
}
```

返回的 `Subscription` 应保存在订阅它的 View 上。只存在局部变量会立刻 drop，订阅也会取消；把它存在生命周期更长的全局 owner 上，则可能在 View 消失后仍然保留 callback 与捕获的资源，造成内存泄漏。订阅所有权和多个订阅者见 [Event](./event)。

## Window 生命周期

不要保存 `&mut Window`；它是 GPUI 临时提供的上下文。需要稍后执行时，使用 `defer`、`spawn_in`，或者取得 `window.window_handle()` 后通过 GPUI 更新。handle 不会让已经关闭的窗口继续存活，因此通过 handle 执行的更新可能失败，代码应正确处理这一情况。

`window.remove_window()` 会在当前更新中请求移除窗口。需要决定平台关闭请求是否可以继续时，注册 `window.on_window_should_close(cx, callback)`；返回 `false` 可取消关闭，未保存内容的确认流程由应用负责。需要观察窗口已经关闭的事件时，使用 `cx.on_window_closed(callback)`，并将返回的 `Subscription` 保存在应用 owner 中。其 callback 依次接收 `&mut App` 与 `WindowId`，此时已无法访问关闭的 `Window`；需要的窗口状态应在关闭前读取。

在仍可访问窗口时注册关闭拦截，通常放在 `open_window` 的 builder 中。它处理平台发出的关闭请求；`remove_window()` 则是程序主动移除。若最后一个窗口关闭时应用也应退出，可在关闭完成的 callback 中检查 `cx.windows().is_empty()`，再调用 `cx.quit()`。[FPS monitor 示例](https://github.com/longbridge/gpui-kit/blob/main/examples/fps_monitor/src/main.rs) 展示了这种单窗口退出流程，也展示了 View 如何请求动画帧。在本仓库执行 `cargo run -p fps_monitor` 即可运行；这个示例使用 GPUI Kit，但未启用可选的 Component 层。

如果窗口动作似乎没有效果，先检查 handle 是否仍指向打开的窗口，以及当前获得 Focus 的 Element 的 Dispatch Path 是否包含 Action handler。如果 Entity 状态变化未显示，检查更新时是否调用了 `cx.notify()`；如果窗口级变化未显示，则调用 `window.refresh()`。如果帧 callback 已执行却没有绘制，请记住 `on_next_frame` 产生帧需求但不会标记窗口为 dirty。如果 Event callback 不再执行，检查其 `Subscription` 是否被保存。

可以用下面几条规则判断所有权：

- 持久 UI 状态属于 Entity；
- 窗口级工作只在 callback 执行期间获得 `&mut Window`；
- 把 `Task` 和 `Subscription` 存在 View 上，让后台工作和订阅跟随 owner 生命周期；
- Focus 与 Action 派发始终使用当前这个 Window 的状态。

应用有多个窗口时，继续阅读 [Multi Window](./multi-window)。[Native Extensions](./native-extension) 介绍窗口边界上的系统 API；[WebView](./webview) 说明原生子视图集成及其限制。需要由操作系统投递通知时，参见 [SystemNotification](./system-notification)。

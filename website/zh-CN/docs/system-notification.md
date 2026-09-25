---
title: SystemNotification
description: 使用 GPUI 发送系统通知、处理点击，并了解 GPUI Kit Notification 的集成方式和平台限制。
order: -9.05
---

# SystemNotification

`SystemNotification` 把消息送到**操作系统通知中心**。它是应用级平台服务，不是在 [Window](./window) 中绘制的 Element。GPUI Kit 的 [Notification 组件](../component/notification) 默认显示应用内 toast；使用 `.system()` 可发送到系统，使用 `.in_app_and_system()` 可同时显示在两处。

| 需求 | 使用方式 |
| --- | --- |
| 用户正在当前窗口操作时给予即时反馈 | `window.push_notification(Notification::info(...), cx)` |
| 后台任务完成，用户可能已切换到其他应用 | `window.push_notification(Notification::info(...).system(), cx)` |
| 同一状态既显示在窗口中，也发送到通知中心 | `window.push_notification(Notification::info(...).in_app_and_system(), cx)` |
| 自行设置 GPUI payload、tag 或系统通知的操作按钮 | `cx.show_system_notification(SystemNotification { ... })` |

用户正在查看结果时，使用应用内 toast；用户可能已离开应用时，使用系统通知。重要状态仍应保留在应用内：操作系统可能禁用、抑制或延迟通知。需要用户处理的错误，应在返回应用时显示应用内错误信息或对话框，不能只依赖一条短暂的系统通知。

## 先运行现有示例

在本仓库运行通知示例，然后点击 **System only** 或 **In-app and system**：

```sh
cargo run -p gpui-component-story -- NotificationStory
```

按钮代码在 `crates/story/src/stories/notification_story.rs`；`crates/story/src/main.rs` 在启动时设置应用标识。这样可以直接验证现有 GPUI Kit 路径，无需新建 crate。macOS 上，`cargo run` 启动的程序不是打包的 `.app`，系统通知会被跳过；验证真实投递时应运行打包后的应用。**In-app and system** 的应用内部分仍可显示。

在 **System notification** 区域按以下顺序操作：

| 操作 | 预期现象 |
| --- | --- |
| 点击 **System only**。 | 示例窗口内没有 toast。在支持系统通知且已获授权的桌面环境中，系统会收到标题为 **Build finished**、正文为 **Delivered straight to the notification center.** 的通知。 |
| 点击 **In-app and system**。 | 示例窗口内出现不会自动消失的 toast，因为此按钮设置了 `autohide(false)`。系统通知的标题仍为 **Build finished**，正文为 **Shown as a toast and in the notification center.**。 |
| 切换到其他应用，再点击系统通知。 | GPUI Kit 请求激活应用及原示例窗口。运行示例的终端打印 `[notification] system notification clicked`；若有对应的应用内 toast，它会关闭。实际窗口激活及通知显示还受操作系统策略影响。 |

两个按钮使用同一个 `SystemNotificationKind` ID。应用内相同 ID 的通知会替换原 toast；系统内相同 tag 的新通知只有在平台支持时才会替换旧通知。这组操作验证的是**组件路径**，并未测试底层 `SystemNotification` 的系统操作按钮。在 macOS 上，`cargo run` 只能检验应用内部分；要检验系统投递与点击激活，应使用已获授权的打包应用。如果桌面禁用了弹出横幅，也应先查看系统通知中心，再判断通知是否未送达。

## 初始化并设置应用标识

启动时调用一次 `gpui_kit::init(cx)`，并在打开窗口或发送通知前设置稳定的标识符与显示名称。未打包的 Windows 应用需要应用标识；Linux 可使用其显示名称。GPUI 测试平台也要求设置标识，因此通知测试中也应设置。

```rust
gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);
    cx.set_app_identity("com.example.exporter", "Exporter");
    // Open the application window here.
});
```

| 平台 | 投递条件与行为 |
| --- | --- |
| macOS | 从真实的 `.app` 包运行，并放在系统信任的位置，例如 `/Applications`。直接 `cargo run` 不能投递。首次发送可能请求通知权限；拒绝授权会抑制投递。 |
| Windows | 尽早设置应用标识，未打包的应用尤其需要。通知是否显示还取决于系统通知设置。 |
| Linux | 需要正常工作的会话 D-Bus 与 XDG 通知守护进程。无法发送时，当前适配器会记录警告；它既不按 tag 替换，也不支持撤回。 |

GPUI 的 `show_system_notification` 返回 `()`，不返回投递结果。因此调用成功仅表示请求已交给平台适配器。没有看到系统通知，并不代表应用的后台任务失败。

把权限视为产品流程的一部分：任务结果应保存在应用状态中，并在用户返回时可再次找到。系统通知只是引导用户查看状态，不能充当状态本身。权限被拒、应用通知被静音、免打扰模式、缺少通知守护进程或原窗口关闭，都不应抹掉已完成的导出结果及其错误。

## 使用底层 GPUI API 发送

本项目固定使用 `gpui-pre 0.3.6`。其 [`App::show_system_notification`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.App.html#method.show_system_notification) 接收包含 `tag`、`title`、`body` 和 `actions` 的 `SystemNotification`：

```rust
use gpui_kit::SystemNotification;

// In code with an &mut App named cx:
cx.show_system_notification(SystemNotification {
    tag: "export/report-42".into(),
    title: "Export complete".into(),
    body: "report.pdf is ready".into(),
    actions: Vec::new(),
});
```

为对应的任务或条目选择 tag。再次发送同一 tag 时，**支持替换的平台**会更新先前的通知。`cx.dismiss_system_notification("export/report-42")` 请求撤回待投递或已投递的通知，同样只提供尽力而为的保证。当前 Linux 适配器不实现按 tag 替换或撤回，因此重复或过期的通知可能保留到守护进程使其自然过期。

如需系统通知操作按钮，在 `actions` 中放入 `SystemNotificationAction { id, label }`。点击按钮后，`id` 通过 `SystemNotificationResponse::action_id` 返回；点击通知主体时，该字段为 `None`。部分平台可能显示通知却不显示按钮。只注册一次 `cx.on_system_notification_response(|response, cx| { ... })`，依据 `response.tag` 和 `response.action_id` 分发底层响应。只有用户激活已投递的通知时才会调用 handler；发送失败或普通关闭不会触发。底层 GPUI 通知不会自动关联原窗口，也不会自动调用应用业务回调；应用需要自己处理路由，并确认目标窗口仍存在。

底层响应处理器应将 tag 对应到持久的任务标识，在收到响应时重新查询任务，并分别处理通知主体点击（`None`）和每个操作按钮 ID。响应到达时，任务或窗口可能已经不存在。不要把这个 handler 当作后台任务的完成回调。

## 使用 GPUI Kit 的窗口集成

需要在点击后返回原窗口时，通过该窗口的 `WindowExt` 推送 `Notification`。用 `gpui_kit::open_window` 创建窗口，或用 `Root::new` 包裹视图，以挂载通知 overlay。

```rust
use gpui_kit::component::{notification::Notification, WindowExt};

struct ExportNotice;

window.push_notification(
    Notification::success("report.pdf is ready")
        .title("Export complete")
        .id::<ExportNotice>()
        .system()
        .on_click(|_, window, cx| {
            // Open the completed export task in its owning window.
        }),
    cx,
);
```

组件把 ID 映射成带命名空间的系统 tag，并记住发送通知的窗口。不同任务需要不同身份时，可用 `.id1::<ExportNotice>(task_key)`。组件将标题和消息作为系统通知文字；只有消息时，消息会成为系统标题。既没有标题也没有消息的纯自定义内容通知不会发送到系统。自定义内容和组件的 `.action(...)` 按钮只属于应用内 toast；组件发送的系统通知不包含操作按钮。

收到可识别的点击后，GPUI Kit 请求撤回通知、激活应用，再激活原窗口、关闭对应的应用内 toast（如果存在），并调用 `on_click`。回调需要原窗口仍然存活。若窗口已关闭，应用仍可能被激活，但不会调用窗口回调。应用重启后，或组件有限的路由记录被清理后，点击也可能只激活应用而不调用 `on_click`；重要任务状态应存放在通知回调之外。组件对一条响应只处理一次，并忽略其命名空间以外的底层 tag。

`.system()` 不创建应用内 toast，因此不会调用 `on_close`。`.in_app_and_system()` 同时创建两者；toast 自动超时**不会**撤回系统副本。显式调用 `window.remove_notification::<ExportNotice>(cx)` 或 `window.clear_notifications(cx)` 会请求撤回该窗口拥有的系统通知，包括仅发送到系统的通知。若另一个窗口随后用同一组件 ID 发过通知，第一个窗口的移除操作不会撤回较新的通知。操作系统是否真正撤回仍取决于平台。

## 保持单一响应 handler

GPUI 只保留一个 `App::on_system_notification_response` handler；再次注册会替换之前的 handler。启用默认组件 feature 时，`gpui_kit::init(cx)` 会安装 GPUI Kit 的 handler。如果组件系统通知需要处理点击，就不要覆盖它。底层 API 仍可发送通知，但该 handler 会忽略底层通知的 tag，因此应用收不到它们的操作按钮响应。GPUI Kit 没有公开的接口将底层 handler 串接到组件 handler；应用应选择一种响应所有权路径。

## 测试与排查

`TestAppContext` 提供 `shown_system_notifications()`、`delivered_system_notifications()`、`dismissed_system_notifications()` 和 `simulate_system_notification_response(...)`。测试中先设置应用标识，再检查发送的标题、正文和 tag，模拟点击，并断言预期的窗口回调或底层 handler 结果。测试平台会模拟替换和撤回，但**不能**证明真实操作系统的守护进程、授权请求或窗口激活正常；每个目标系统都需要实际检查。

没有看到通知时，检查应用标识、macOS 的打包与权限状态、Windows 通知设置，或 Linux 会话中的通知守护进程。点击没有反应时，检查通知是由组件还是底层 GPUI 发送、是否有其他 handler 替换了 GPUI Kit 的 handler，以及原窗口是否仍存在。toast 样式和生命周期见 [Notification 组件](../component/notification)；平台边界设计见 [Native Extensions](./native-extension)。

| 现象 | 下一步检查 |
| --- | --- |
| **In-app and system** 有应用内 toast，却没有系统通知。 | 窗口集成已经生效；检查操作系统权限、免打扰模式及上面的平台投递条件。在 macOS 上直接执行 `cargo run` 时，这是预期结果。 |
| **System only** 看起来没有反应。 | 此模式本来就没有应用内 toast。检查通知中心及平台投递条件；API 不提供投递结果回调。 |
| 点击系统通知后只激活了应用，没有进入目标视图。 | 检查原窗口是否仍打开、组件的内存路由记录是否仍存在。用户返回后应从应用状态恢复目标视图。 |
| 新通知发送或显式移除后，旧通知仍存在。 | 检查平台是否支持按 tag 替换与撤回；当前 Linux 适配器两者都不支持。 |
| 底层通知的操作按钮没有触发应用行为。 | 检查唯一的应用级响应 handler。`gpui_kit::init(cx)` 安装的 handler 只路由 GPUI Kit 组件 tag，会忽略底层 tag。 |

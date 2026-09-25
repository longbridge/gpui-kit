---
title: SystemNotification
description: 使用 GPUI 发送系统通知、处理点击，并了解 GPUI Kit Notification 的集成方式和平台限制。
order: -9.05
---

# SystemNotification

`SystemNotification` 把消息送到**操作系统通知中心**。它是应用级平台服务，不是在 [Window](./window) 内绘制的 Element。GPUI Kit 的 [Notification 组件](../component/notification) 默认显示应用内 toast，也可以用 `.system()` 送到系统，或用 `.in_app_and_system()` 同时送到两处。

| 需求 | 使用方式 |
| --- | --- |
| 在当前窗口内反馈 | `window.push_notification(Notification::info(...), cx)` |
| 需要 GPUI Kit 自动激活窗口并处理 `on_click` 的系统通知 | `window.push_notification(Notification::info(...).system(), cx)` |
| 直接控制 GPUI payload、tag 或 action button | `cx.show_system_notification(SystemNotification { ... })` |

本项目固定的 `gpui-pre 0.3.6` 提供 [`App::show_system_notification`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.App.html#method.show_system_notification)、`App::dismiss_system_notification` 和 `App::on_system_notification_response`。payload 包含 `tag`、`title`、`body`、`actions`。`SystemNotificationAction` 含有 `id` 和面向用户的 `label`；`SystemNotificationResponse` 返回 `tag`，以及所点击 action 的 `id`；点击通知主体时 `action_id` 为 `None`。

## 设置应用 identity

启动早期、打开窗口或发送通知之前，设置一次稳定的 identifier 和显示名称。Windows 系统通知要求这一步；其他平台也可能使用显示名称：

```rust
gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);
    cx.set_app_identity("com.example.exporter", "Exporter");
    // 在这里打开应用窗口。
});
```

macOS 必须从可信位置（如 `/Applications`）运行打包后的 `.app`；直接 `cargo run` 无法投递系统通知。首次投递可能出现系统授权请求。Linux 需要 XDG 通知守护进程。系统设置或授权状态可能使通知无法送达，而 `show_system_notification` 不返回错误。

## 发送与替换通知

在持有 `&mut App`（通常命名为 `cx`）的代码中，使用稳定的 tag 发送：

```rust
use gpui_kit::SystemNotification;

cx.show_system_notification(SystemNotification {
    tag: "export/report-42".into(),
    title: "Export complete".into(),
    body: "report.pdf is ready".into(),
    actions: Vec::new(),
});
```

用相同 `tag` 再次发送时，**支持替换的平台**会更新原通知。`cx.dismiss_system_notification("export/report-42")` 请求撤回待投递或已投递的通知，但撤回是 best-effort。当前 Linux adapter 不支持撤回，通知只能自然过期；它的 XDG 实现也没有按 tag 替换的逻辑。因此，不能把 tag 当成跨平台“只显示一条”的保证。

底层 `actions` 字段接收 `Vec<SystemNotificationAction>`。部分平台可能不显示 action button；如果用户点击了显示出的 action，其 `id` 会通过 `SystemNotificationResponse::action_id` 返回。

## 在 GPUI Kit 中处理点击

如果点击系统通知需要激活正确的窗口或执行应用逻辑，GPUI Kit 应使用组件入口：

```rust
use gpui_kit::component::{WindowExt, notification::Notification};

struct ExportNotice;

window.push_notification(
    Notification::success("report.pdf is ready")
        .title("Export complete")
        .id::<ExportNotice>()
        .system(),
    cx,
);
```

组件将 ID 映射成系统 tag；点击后，它会激活应用和原窗口。需要执行应用逻辑时，接上 `.on_click(...)`。希望同一消息也显示为应用内 toast 时，使用 `.in_app_and_system()`。显式移除组件通知会请求撤回系统通知；toast 的自动超时不会撤回系统副本。`.system()` 不创建 toast，因此不会调用 `on_close`。

**回调只能有一个 owner：**GPUI 只保留一个 `App::on_system_notification_response` handler；重新注册会替换之前的 handler。`gpui_kit::init(cx)` 会安装组件的 handler。如果使用组件系统通知，就不要再注册另一个 handler，否则组件的点击行为会失效。直接调用 `cx.show_system_notification(...)` 仍可投递消息，但组件 handler 会有意忽略这些原始 tag。GPUI Kit 目前没有在该 handler 之外再注册原始响应 listener 的公开入口。需要处理原始 action button 响应时，必须由一个统一 handler 负责，并与组件集成协调；或者只使用其中一条路径。

## 验证平台边界

GPUI 的 `TestAppContext` 提供 `shown_system_notifications`、`delivered_system_notifications`、`dismissed_system_notifications` 和 `simulate_system_notification_response`，可验证 tag 与 response 路由；真实投递还应在各目标系统上检查。[Native Extensions](./native-extension) 介绍同类的平台边界做法；[Notification 组件](../component/notification) 说明 toast 的呈现和投递模式。

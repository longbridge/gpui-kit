---
title: Native Extensions
description: 说明如何将系统菜单和原生子视图接入 GPUI Kit，包括 handle、布局、输入、生命周期与平台限制。
order: -9.1
---

# Native Extensions

Native extension 是把操作系统控件或视图接入 GPUI window。本仓库有两条具体路径：[NativeMenu](https://github.com/longbridge/gpui-kit/tree/main/crates/component/src/native_menu) 使用系统菜单 API，[gpui-wry](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) 嵌入原生 WebView。两者都跨越 GPUI 与系统的边界；启动另一个应用不属于这里讨论的原生控件接入。

当弹出菜单需要超出小窗口边界，或应遵循系统菜单外观时，使用 `NativeMenu`；若菜单应参与 GPUI 自身的 overlay 合成，使用 GPUI `PopupMenu`。只有屏幕确实需要浏览器引擎，并能为原生子视图留出一块矩形区域时，才使用 `gpui-wry`。[TextView HTML](/zh-CN/component/text-view#html) 用于显示文档内容，[`cx.open_url`](./context#在默认浏览器中打开-url) 会打开用户的默认浏览器。`gpui-wry` 仍属实验性功能。

| 平台 | NativeMenu | 本仓库的嵌入式 WebView |
| --- | --- | --- |
| macOS | AppKit `NSMenu`，依附于 `NSView` | Wry 原生子视图；现有例子可用 |
| Windows | Win32 `TrackPopupMenuEx`，使用 `HWND` | Wry 原生子视图；现有例子需要指定 renderer 设置 |
| Linux | GPUI 绘制的 `PopupMenu` 回退实现，受窗口边界裁剪 | GTK 宿主代码尚未完成，目前没有受支持的接入路径 |

Linux 的菜单回退实现保持 `NativeMenu` API，但内容由 GPUI 绘制，并非系统菜单。Linux 原生子视图需要分别处理 GTK、Wayland、X11 的宿主集成与测试。当前平台要求见 [WebView](./webview)。

## 源码与构建路径

工作区根目录的 `Cargo.toml` 把 `gpui-pre` 固定为 `=0.3.6`。把 adapter 移植到其他 GPUI 版本前，请先核对当前 checkout 的源码。相关路径如下：

| 内容 | 本仓库源码 | 在仓库根目录构建或运行 |
| --- | --- | --- |
| 菜单公共 API 与平台选择 | `gpui-component` crate 中的 `crates/component/src/native_menu/mod.rs` | `cargo check -p gpui-component`；`cargo test -p gpui-component native_menu` 检查 builder 和图标测试 |
| AppKit、Win32 菜单 adapter 与 GPUI 回退实现 | `crates/component/src/native_menu/{macos,windows,fallback}.rs`；回退 overlay 由 `crates/component/src/root.rs` 持有 | `cargo run -p gpui-component-story`，然后在目标系统中打开 **NativeMenu** story |
| Wry wrapper 与自定义 element | `gpui-wry` crate 中的 `crates/webview/src/lib.rs` | `cargo check -p gpui-wry` 检查当前目标平台的 wrapper |
| 子视图创建与 renderer 设置 | `examples/webview/src/main.rs`，依赖定义在 `examples/webview/Cargo.toml` | 在 macOS 或 Windows 上运行 `cargo run -p webview` |

Cargo 只检查当前目标平台的 `#[cfg]` 分支；在 Linux 上构建不会验证 AppKit 或 Win32 adapter。story 和 WebView 例子需要图形桌面环境及平台原生库。Linux WebView 分支即使编译成功，也不是可用的 GPUI 宿主实现。

## 1. 确定接入边界

对于**平台菜单集成**，应将共享的 GPUI API 与平台 adapter 分开。`NativeMenu::show(position, window, cx)` 在[共享 module](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/mod.rs)中选择 macOS、Windows 或 Linux 回退实现。原生 adapter 创建和操作系统菜单，再通过 GPUI 返回所选 Action。

对于**嵌入式子视图**，原生内容会持续占据 GPUI layout 区域。像 [`gpui-wry::WebView`](https://github.com/longbridge/gpui-kit/blob/main/crates/webview/src/lib.rs) 一样，把它交给 `Entity` 持有。owner 负责 show/hide、Focus、bounds，并确保父窗口关闭前完成销毁。

## 2. 取得正确的 window handle

`Window::window_handle(window)` 得到 GPUI 的 `AnyWindowHandle`，供之后更新**同一个**窗口。调用系统 API 则应使用 `raw-window-handle` trait。两个方法名字相近，用途不同：

```rust
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

let gpui_handle = Window::window_handle(window); // Update the GPUI window later.
let native_handle = HasWindowHandle::window_handle(window)?; // Platform adapter.
match native_handle.as_raw() {
    RawWindowHandle::AppKit(handle) => { /* macOS: handle.ns_view */ }
    RawWindowHandle::Win32(handle) => { /* Windows: handle.hwnd */ }
    RawWindowHandle::Xcb(handle) => { /* Linux X11: handle.window */ }
    RawWindowHandle::Wayland(handle) => { /* Linux Wayland: handle.surface */ }
    _ => { /* Unsupported backend. */ }
}
```

这段是返回 `Result` 的 adapter 函数节选，并非完整的跨平台函数。应在对应目标的 `#[cfg]` 下匹配实际 handle 变体。`HasWindowHandle` 返回的 handle 借用有效窗口；raw 指针或 ID 并不拥有原生对象。菜单 adapter 将 `NSView` 指针或 `HWND` 复制进 foreground task，并依赖父窗口在同步 tracking 调用期间保持有效。窗口销毁后不能继续缓存或使用它们。当前固定版本的 GPUI Linux 平台在 X11 下提供 XCB window ID，在 Wayland 下提供 Wayland surface。单凭 raw surface 指针，无法得到嵌入 GTK 控件所需的 widget 层级或 compositor 集成。handle 提取可参见仓库的 [AppKit](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/macos.rs)与 [Win32](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/windows.rs) adapter。

## 3. 平台菜单实例：NativeMenu

adapter 通过目标平台的 Rust crate 直接调用系统 UI 框架：

| Backend | 这里用到的 Rust crate 与 API | 系统对象或操作 |
| --- | --- | --- |
| macOS | `objc2`、`objc2-app-kit`、`objc2-foundation`；`MainThreadMarker`、`NSMenu`、`NSMenuItem`、`define_class` | 构建 AppKit 菜单项、接收 Objective-C 选择事件、从 `NSView` 弹出菜单 |
| Windows | 带 Win32 UI features 的 `windows` crate；`CreatePopupMenu`、`TrackPopupMenuEx`、`DestroyMenu` | 构建 `HMENU`，依附 `HWND` 跟踪选择，释放原生资源 |
| Linux | GPUI 的 X11 backend 使用 `x11rb`，Wayland backend 使用 `wayland-client`；Wry 未完成的路径使用 `gtk` 和 `WebViewBuilderExtUnix` | NativeMenu 当前在 `Root` overlay 中绘制 GPUI `PopupMenu` 回退实现；原生 widget 宿主尚未实现 |

下面是实际 adapter 中调用系统 API 的节选，省略了菜单项构建及错误处理：

```rust
// macOS: objc2-app-kit, running on the AppKit main thread.
let mtm = MainThreadMarker::new()?;
let menu = NSMenu::new(mtm);
menu.popUpMenuPositioningItem_atLocation_inView(None, point, Some(view));

// Windows：windows::Win32::UI::WindowsAndMessaging。
let menu = unsafe { CreatePopupMenu() }.ok()?;
let selected = unsafe { TrackPopupMenuEx(menu, flags.0, x, y, hwnd, None) };
let _ = unsafe { DestroyMenu(menu) };
```

macOS adapter 取得 AppKit `NSView`，以它为 `NSMenu` anchor，把 GPUI 左上角原点的逻辑坐标换算为 AppKit view 坐标，再运行系统菜单的 tracking loop。Windows adapter 取得 `HWND`，把逻辑像素换成物理 client 坐标，再换成 screen 坐标，调用 `TrackPopupMenuEx`。两者都在**没有持有 GPUI 可变借用时**运行阻塞的系统 tracking loop，随后通过 foreground task、`cx.update(...)`、保存的 `AnyWindowHandle::update(...)`，用 `Window::dispatch_action` 派发所选 `Action`。窗口已关闭或用户取消时，不派发 Action。

调用方只需提供语义化菜单项和位置；下例的 `Copy`、`Paste` 是应用定义的 GPUI Action：

```rust
NativeMenu::new()
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
    .show(position, window, cx);
```

从 `gpui_kit::component::native_menu::NativeMenu` 导入该类型。`position` 是窗口相对的逻辑像素 `Point<Pixels>`，例如 `MouseDownEvent::position`。builder 还支持分隔线、子菜单、禁用和勾选状态及图标；`NativeMenu::from(gpui_kit::Menu)` 可复用已有的 GPUI 菜单定义。`show` 消耗菜单并立即返回；空菜单不会显示任何内容。打开菜单前，先让目标 Action 处理器获得 Focus：选择结果会派发到窗口当前的焦点上下文。在 Linux 上，回退实现向 `Root` 获取 overlay；窗口没有这个根节点，就没有可显示的回退菜单。

[story](https://github.com/longbridge/gpui-kit/blob/main/crates/story/src/stories/native_menu_story.rs)展示 Focus 设置和 Action 处理。Linux 上，[回退实现](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/native_menu/fallback.rs)在 `Root` overlay 中构建 GPUI `PopupMenu`，仍受 GPUI 窗口边界限制。

## 4. 嵌入式子视图实例：WebView

[WebView 例子](https://github.com/longbridge/gpui-kit/blob/main/examples/webview/src/main.rs)从有效的 GPUI window 创建 Wry 原生子视图，再把 wrapper 存入 `Entity`：

```rust
let webview = cx.new(|cx| {
    use raw_window_handle::HasWindowHandle;

    let handle = window.window_handle().expect("No window handle");
    let native = wry::WebViewBuilder::new()
        .build_as_child(&handle)
        .expect("Failed to create WebView");
    gpui_wry::WebView::new(native, window, cx)
});
```

wrapper 在普通 GPUI element tree 中渲染一个自定义 `Element`。`request_layout` 占据 layout 区域；`prepaint` 得到计算好的 `Bounds<Pixels>`，以逻辑坐标调用 Wry 的 `set_bounds`，并插入 GPUI hitbox。`paint` 注册点击外部时的 Focus 行为。**WebView 像素由系统子视图绘制，不由 GPUI 绘制。** GPUI 的 `ContentMask` 或 hitbox 不会改变它在系统 compositor 中的层级。owner drop 时隐藏原生视图；克隆的 `WebViewHandle` 应在父窗口销毁前释放。

当前 WebView 位于同区域 GPUI 内容之上，因此 GPUI popover、dialog、tooltip 无法可靠覆盖它。Windows 例子在 `gpui_kit::application()` 之前设置 `GPUI_DISABLE_DIRECT_COMPOSITION=true`，以支持这条子视图路径。Linux 例子中的 GTK 路径标记为未完成。用法与限制详见 [WebView](./webview)。

在 Linux 上，例子引入 `gtk` 和 Wry 的 `WebViewBuilderExtUnix`，创建 `gtk::Fixed`，再调用 `build_gtk(&fixed)`。源码本身标记宿主初始化未完成：创建 GTK widget **不会**自动将它附着到 GPUI 的 XCB 或 Wayland window。当前固定版本的 GPUI Linux backend 在 X11 使用 `x11rb` 的 `configure_window` 等调用，在 Wayland 使用 `wayland-client` 的 `WlSurface::commit`。Linux adapter 需要接入对应 backend 的 connection 与 event loop，管理 surface 生命周期，并安排输入和 compositor 顺序；只有 raw `XcbWindowHandle` 或 `WaylandWindowHandle` 无法完成这些工作。当前 `gpui-wry` 例子无法作为最后这一步的可运行模板。

## 5. 实现其他原生控件

1. 定义一个面向 GPUI 的操作，并明确各平台的返回结果语义。说明 Linux 实现是系统原生、GPUI 绘制，还是不支持。
2. 把 AppKit、Win32 和 Linux backend 放进各自的 adapter；raw handle 与系统类型留在 adapter 内部。
3. 嵌入式视图由 `Entity` 管理生命周期；在 `prepaint` 中把 GPUI bounds 同步给系统视图，并测试缩放、Focus、输入、窗口关闭。NativeMenu 这类系统 UI 则应在不持有 GPUI 可变借用时运行 tracking loop，再把结果送回 GPUI。
4. 分别测试实际 backend。GPUI headless 测试能检查状态和 Action，但无法证明系统定位、原生 Focus 或 compositor 层级正确。

## 调试与验证原生边界

先在目标机器运行 `cargo check -p gpui-component` 或 `cargo check -p gpui-wry`。再运行上面的 NativeMenu story 或 WebView 例子，分别检查普通与高缩放倍率下的行为。Cargo 检查成功仅说明当前目标平台通过类型检查；它不会链接或运行应用，也不能证明原生输入和层级正确。

| 现象 | 在本仓库中检查 |
| --- | --- |
| 菜单没有出现 | 确认菜单非空、位置使用窗口相对的逻辑像素，并核对实际 `RawWindowHandle` 变体是否匹配 adapter。Linux 还须确认窗口使用 GPUI Kit 的 `Root`，使 `WindowState::native_menu_overlay` 可用。 |
| 菜单出现但选择后没有效果 | 在 `show` 之前让目标 view 获得 Focus，注册其 `on_action` 处理器，并检查是否取消了选择，或窗口是否在保存的 `AnyWindowHandle` 更新前关闭。 |
| 高缩放倍率下菜单位置偏移 | 核对 AppKit 的 Y 坐标翻转，或 Win32 的逻辑到物理坐标转换及随后的 `ClientToScreen`；把窗口移到其他屏幕后再测试。 |
| Windows 上 WebView 空白 | 检查例子的 DirectComposition 设置是否在 GPUI 初始化前生效，以及 WebView 是否作为有效窗口的子视图创建。 |
| WebView bounds 未更新或意外拦截输入 | 检查 `prepaint` 中的 bounds 更新、可见状态、`hide` 时的焦点归还，以及原生 compositor 层级。GPUI hitbox 或 content mask 不会裁剪原生像素，也不会改变其层级。 |

添加 adapter 时，要在各受支持 backend 上测试打开、取消、选择、调整大小、缩放、隐藏和关闭父窗口。确保原生对象及所有克隆 handle 的生命周期不超过父窗口。对尚不支持的行为明确说明，不要把 Rust 构建成功当作平台支持。

### 合成边界

当前 checkout 的 `gpui-wry` wrapper 没有提供让 GPUI overlay 覆盖其原生子视图的 API。应把 overlay 交互放在 WebView bounds 之外，或使用单独的窗口。当前行为见 [WebView](./webview)；若要依赖实验性 GPUI 合成分支，须另行验证。

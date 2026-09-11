---
title: 移动端
description: 使用实验性的 gpui-pre-mobile 平台构建 iOS 应用，或将 GPUI Kit 嵌入 Swift UIKit 容器。
order: -2.4
---

# 移动端

移动端支持基于 [gpui-mobile](https://github.com/itsbalamurali/gpui-mobile)，由 [itsbalamurali](https://github.com/itsbalamurali) 创建并与社区共同开发。原始移动平台的成果归功于该项目的作者和贡献者。移动平台负责窗口、触摸输入、文本系统和 GPU 渲染表面，GPUI 与 GPUI Kit 继续管理 Rust 视图树和组件。

GPUI Kit 目前使用 `gpui-pre-mobile`，这是在 [Longbridge fork](https://github.com/longbridge/gpui-mobile) 中维护的临时兼容包。它基于原项目进行打包适配，用于配合 `gpui-pre` 发布 crate，并持续跟进最新的 GPUI 版本、保持集成兼容。待社区 `gpui-mobile` 完成接入、GPUI 也发布 crate 后，我们计划将本文及相关依赖更新为社区的 `gpui-mobile`。

目前该集成仍处于实验阶段。Swift 托管的 iOS 示例已在 iOS 模拟器中构建并运行。仓库中也有 Android 平台实现，但本文介绍的 GPUI Kit 集成尚未在 Android 或实体 iPhone 上验证。

## 运行 iOS 示例

从兼容 fork 中的 [Swift 容器示例](https://github.com/longbridge/gpui-mobile/tree/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example) 开始。它使用 `Message`、`Bubble`、`TextView`、`Input`、思考摘要和复制操作组成聊天界面。回复来自本地示例数据，没有接入 AI 服务。

在 Apple Silicon Mac 上安装 Xcode、iOS 模拟器运行时、Rust 和 XcodeGen：

```sh
brew install xcodegen
rustup target add aarch64-apple-ios-sim

git clone https://github.com/longbridge/gpui-mobile.git
cd gpui-mobile
git checkout 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20
cd example
./build.sh ios --simulator
```

脚本会构建 Rust 静态库、生成 Xcode 工程，并在模拟器中安装和启动应用。添加 `--no-run` 可以只构建。示例的最低部署版本为 iOS 16；这项配置不代表所有支持的系统版本都经过测试。

真机开发还需要安装 `aarch64-apple-ios` Rust target，并在 `example/ios/project.yml` 中设置自己的开发团队和签名信息。修改后重新生成工程。模拟器运行结果不能代替真机性能测试或发布验证。

## 依赖配置

`gpui-pre-mobile` 是 Cargo 包名，Rust 库名为 `gpui_mobile`。评估阶段使用 Git 依赖；清单中的 `0.1.0` 版本号不代表已经发布到 crates.io。

```toml
[lib]
crate-type = ["staticlib", "rlib"]

[dependencies]
gpui-mobile = { package = "gpui-pre-mobile", git = "https://github.com/longbridge/gpui-mobile", rev = "0b882efdac7f524e0bb0b1d4c886b2aa752f9f20" }
gpui = { package = "gpui-pre", version = "=0.3.4", default-features = false }
gpui-kit = { git = "https://github.com/longbridge/gpui-kit", rev = "7d9efcd2069f9eaa6eb3ba6345aac4aa7d87c9f7", default-features = false, features = ["component"] }
```

这些提交固定了示例的依赖基线。Kit 提交包含移动平台条件编译支持，但尚未包含移动端 tooltip 禁用逻辑。要使用本地 GPUI Kit 检出，可以替换 Kit 依赖：

```toml
gpui-kit = { path = "../gpui-kit/crates/kit", default-features = false, features = ["component"] }
```

路径相对于应用的 Cargo 清单，请按实际目录调整。GPUI 核心与渲染器应使用同一版本：上述移动平台固定使用 `0.3.4` 的 `gpui-pre` 和 `gpui-pre-wgpu`。

与桌面端[快速开始](/zh-CN/docs/getting-started)不同，移动端不使用 `gpui_kit::application()` 或 `gpui_kit::platform`。这些桌面平台导出在 iOS 和 Android 上被排除。移动宿主负责初始化 GPUI、调用 `gpui_kit::init(cx)`，并在应用内容外挂载一个 `component::Root`。

## 嵌入 UIKit 视图

UIKit 管理原生窗口、导航、安全区域和键盘布局。示例中的 `GPUITextView` 是一个 Swift `UIView` 包装器，内部托管 GPUI 平台的子 `UIViewController`。虽然名字叫 `GPUITextView`，它承载的是完整的 Rust 聊天视图，而不只是一个 `TextView` 元素。

集成时请一起参考以下文件：

| 文件 | 职责 |
| --- | --- |
| [App.swift](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/ios/App.swift) | 原生窗口、视图包装、子控制器容纳、布局与帧调度 |
| [Embedding.h](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/ios/Embedding.h) | Swift 调用 Rust 所需的桥接声明 |
| [src/lib.rs](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/src/lib.rs) | 应用回调、Kit 初始化与 Rust 根视图 |
| [project.yml](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/ios/project.yml) | Rust 构建步骤、静态库链接、系统框架与桥接头文件 |

启动顺序如下：

1. 在创建 GPUI 应用前调用 `gpui_ios_set_embedded()`，避免平台再创建一个原生窗口。
2. 调用示例定义的 `gpui_ios_register_app()`。它通过 `gpui_mobile::ios::ffi::set_app_callback` 注册 Rust 回调，在回调中初始化 Kit 并打开 GPUI 根视图。
3. 调用 `gpui_ios_run_demo()` 启动嵌入式应用，然后通过 `gpui_ios_get_window()` 和 `gpui_ios_view_controller()` 获取窗口与子控制器。
4. 按 UIKit 的容纳规则调用 `addChild`、添加子视图，再调用 `didMove(toParent:)`。

`gpui_ios_register_app()` 属于示例，不是平台库提供的函数。请修改它的回调来创建自己的 Rust 视图。`run_demo` 是当前桥接入口的名称，实际执行的是已注册的应用回调。

将示例的 `GPUITextView` 包装器加入项目后，原生控制器可以像布局其他视图一样设置约束：

```swift
let content = GPUITextView(frame: .zero)
content.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(content)
content.attach(to: self)

NSLayoutConstraint.activate([
    content.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor),
    content.leadingAnchor.constraint(equalTo: view.leadingAnchor),
    content.trailingAnchor.constraint(equalTo: view.trailingAnchor),
    content.bottomAnchor.constraint(equalTo: view.keyboardLayoutGuide.topAnchor),
])
```

这段代码依赖示例包装器，`GPUITextView` 并不是 SDK 提供的 UIKit 类。移植时应保留它的子控制器容纳和布局逻辑，以及配套声明和构建配置，而不只是复制约束。

### 生命周期、尺寸与帧调度

平台持有 `Application::run_embedded` 返回的 `ApplicationHandle`，确保其生命周期覆盖回调和渲染视图。目前桥接支持一个随应用存活的 GPUI 视图，尚未提供独立销毁、多实例或集合视图单元格复用接口。

在 `layoutSubviews` 中，仅当非零边界尺寸发生变化时更新子控制器的 frame，调用 `gpui_ios_layout_view`，再请求渲染一帧。示例将这些操作放在禁用隐式动画的 Core Animation 事务内，使布局变化时 Metal 表面与 GPUI 视口保持同步。

宿主在界面可见时使用 `CADisplayLink` 驱动 `gpui_ios_request_frame`，在控制器消失时停止 display link。同时按 `App.swift` 转发应用激活与失活事件。UIKit 和桥接调用均应在主线程执行。

## 平台判断

`gpui_kit::is_mobile()` 是带 `#[inline]` 的 `const fn`，在 iOS 和 Android 目标上返回 `true`。它判断编译目标，不判断窗口宽度或是否连接鼠标。

```rust
if gpui_kit::is_mobile() {
    // 使用适合触摸的交互。
}
```

## 移动界面设计

可以与桌面端共享组件行为和内容，但应针对触摸操作与窄屏调整界面：

- 由原生容器处理导航、安全区域和键盘避让。不要在 Rust 内容中重复添加标题栏或安全区域内边距。
- 一段对话只由一个容器负责纵向滚动。位于该容器内的 `TextView` 使用 `.w_full().min_w_0().scrollable(false)`，使文字与图片适应可用宽度。
- 输入为空时保持紧凑。如果不需要多行输入，就使用单行输入框，并确保键盘不会遮挡发送操作。
- iOS 和 Android 上点击 HoverCard 的触发元素切换开关，点击外部关闭；移动手指不会打开卡片。
- 让操作可以通过触摸发现。复制按钮与回复正文对齐，复制成功后短暂显示对勾，不依赖悬停提示解释操作。
- 使用短段落和有意义的标题。代码、表格和图片应服务于对话，不必在每条回复中罗列所有 Markdown 格式。
- 一致使用 Kit 的主题颜色、字号和间距。在真实设备宽度下检查长回复、宽代码、图片加载和中文等不同文字。

GPUI Base 在 iOS 和 Android 上禁用其 tooltip overlay。这只覆盖通过该 overlay 显示的 Kit 提示，不影响直接调用 GPUI `.tooltip()` 的代码。上述固定依赖基线尚不包含这一修改。移动视图中不要添加 GPUI 原生悬停提示。

## 验证与当前限制

集成到应用后，应检查启动和后台恢复、键盘显示与隐藏、视口尺寸变化、文本选择与复制、滚动及触摸反馈。除了 Rust 编译检查，也应观察实际渲染界面。

在做出性能结论前，使用实体设备、Release 构建和 Xcode Instruments 测量。模拟器适合验证布局与交互，但它的结果不是设备帧耗时。

Android 使用独立的 Activity 与渲染表面生命周期。仓库包含 Android 示例，但本文不代表 Android Kit 兼容性或嵌入原生 Android `View` 的能力已经得到验证。采用这些路径前需要单独评估。

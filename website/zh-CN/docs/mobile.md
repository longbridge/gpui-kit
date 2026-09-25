---
title: Mobile
description: 使用实验性的 gpui-pre-mobile 平台构建 iOS 应用，或将 GPUI Kit 嵌入 Swift UIKit 容器。
order: -10
---

# 移动端

移动端支持基于 [gpui-mobile](https://github.com/itsbalamurali/gpui-mobile)，由 [itsbalamurali](https://github.com/itsbalamurali) 创建并与社区共同开发。原始移动平台的成果归功于该项目的作者和贡献者。移动平台负责 [Window](./window)、触摸输入、[TextSystem](./text-system) 和 GPU 渲染表面，GPUI 与 GPUI Kit 继续管理 Rust 视图树和组件。

GPUI Kit 目前使用 `gpui-pre-mobile`，这是在[兼容性 fork](https://github.com/longbridge/gpui-mobile) 中维护的临时兼容包。它基于原项目进行打包适配，用于配合 `gpui-pre` 发布 crate，并持续跟进最新的 GPUI 版本、保持集成兼容。待社区 `gpui-mobile` 完成接入、GPUI 也发布 crate 后，我们计划将本文及相关依赖更新为社区的 `gpui-mobile`。

目前该集成仍处于实验阶段。Swift 托管的 iOS 示例已在 iOS 模拟器中构建并运行。兼容性 fork 还提供 Android Activity 示例与构建脚本，但它使用不同的宿主路径；GPUI Kit 集成尚未在 Android 或实体 iPhone 上验证。下文的 iOS 模拟器路径是已演示的目标，不能据此推断移动平台均已获得支持。

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

脚本会构建 Rust 静态库、生成 Xcode 工程，并在模拟器中安装和启动应用。添加 `--no-run` 可以只构建，添加 `--release` 可构建 release 版本。此固定提交的脚本按名称选择运行 iOS 18.6 的 `iPhone 16 Pro` 作为构建目标；如果本机没有对应运行时或设备，请把脚本中的 Xcode destination 改为本机已安装的目标。示例的最低部署版本为 iOS 16；这项配置不代表所有支持的系统版本都经过测试。

还要单独核对**安装与启动**使用的模拟器。[固定版本的构建脚本](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/build.sh)在 `build_ios` 中设置 Xcode destination，但随后 `_ios_run_simulator` 会从 `xcrun simctl list devices available` 中选择第一个可用的 iPhone。安装了多个模拟器时，只修改构建目标可能导致应用在另一台模拟器上启动。运行 `xcrun simctl list devices available`，对照列表中的首个 iPhone 与构建目标；若两者不同，请把脚本中的 `sim_id` 选择改为目标模拟器的 UUID，再运行 `./build.sh ios --simulator`。

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

这些提交固定了示例的依赖基线。Kit 提交包含移动平台条件编译支持，但尚未包含移动端 tooltip 禁用逻辑。当前 GPUI Kit 工作树使用 `gpui-pre 0.3.6`，而固定的移动平台及渲染器使用 `0.3.4`。Cargo 可能同时选出两个版本，导致 GPUI 类型不兼容；**只把 Kit 依赖替换为本地路径并不能完成升级**。需要先将移动平台及渲染器更新到与 Kit 相同的 GPUI 版本，并验证这一组合，之后才可使用如下路径依赖：

```toml
gpui-kit = { path = "../gpui-kit/crates/kit", default-features = false, features = ["component"] }
```

路径相对于应用的 Cargo 清单，请按实际目录调整。GPUI 核心、渲染器、平台和 Kit 必须使用相容的同一版本。

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

### 输入、字体与资源

UIKit 决定嵌入视图的安全区域与键盘避让。GPUI 平台把原生触摸和文字输入转换为 Rust 窗口事件，Kit 则负责视图内部的组件交互。请在模拟器中检查焦点、选区、编辑、粘贴、滚动和屏幕键盘变化；构建成功不能证明所有输入法均正常工作。键盘出现时，`window.visual_viewport_bounds()` 可能变化，而布局视口保持不变。不要因同一次键盘变化同时手动缩小 UIKit 容器和 Rust 内容。

示例的嵌入式回调会初始化 Kit 并切换主题，但不会安装应用字体或图标 `AssetSource`。桌面端使用的字体在 iOS 上可能不存在，中文、其他 CJK 文字或 emoji 的字形覆盖也可能不同。如果应用依赖指定字体，请打包字体文件，在打开 GPUI 窗口前通过 `cx.text_system().add_fonts(...)` 注册，再在主题中选择对应字体。需要在目标设备上检查实际排版和缺失字形；参见[字体](./fonts.md)及 [TextSystem](./text-system.md)。

依赖示例只开启 Kit 的 `component` feature。如果应用使用命名的 Kit 图标，还需要开启 `assets` feature，并在创建第一个窗口前为移动 GPUI 应用注册合适的 `AssetSource`。原生嵌入资源与运行时文件路径对应不同的部署方式：外部图片文件必须打包到应用并使用正确路径；远程图片则需要 HTTP 客户端。不能假设桌面端的文件路径在 iOS 沙盒中也存在。参见[图标与资源](./assets.md)。

## 平台判断

`gpui_kit::is_mobile()` 是带 `#[inline]` 的 `const fn`，在 iOS 和 Android 目标上返回 `true`。它判断编译目标，不判断窗口宽度或是否连接鼠标。

```rust
if gpui_kit::is_mobile() {
    // Use interactions designed for touch input.
}
```

## 移动界面设计

可以与桌面端共享组件行为和内容，但应针对触摸操作与窄屏调整界面：

- 由原生容器处理导航、安全区域和键盘避让。不要在 Rust 内容中重复添加标题栏或安全区域内边距。
- 一段对话只由一个容器负责纵向滚动。位于该容器内的 `TextView` 使用 `.w_full().min_w_0().scrollable(false)`，使文字与图片适应可用宽度。
- 输入为空时保持紧凑。如果不需要多行输入，就使用单行输入框，并确保键盘不会遮挡发送操作。
- iOS 和 Android 上点击 HoverCard 的触发元素切换开关，点击外部关闭；移动手指不会打开卡片。
- 在 `Input` 或 `Textarea` 中长按或双击可选中单词，并显示触摸拖动手柄与编辑菜单。对于可选择的只读 `TextView` 内容，请使用长按；触摸双击在这里不会选中单词。这些是 Kit 的交互路径，仍需通过移动平台的事件转换进行实机验证。
- 让操作可以通过触摸发现。复制按钮与回复正文对齐，复制成功后短暂显示对勾，不依赖悬停提示解释操作。
- 使用短段落和有意义的标题。代码、表格和图片应服务于对话，不必在每条回复中罗列所有 Markdown 格式。
- 一致使用 Kit 的主题颜色、字号和间距。在真实设备宽度下检查长回复、宽代码、图片加载和中文等不同文字。

GPUI Base 在 iOS 和 Android 上禁用其 tooltip overlay。这只覆盖通过该 overlay 显示的 Kit 提示，不影响直接调用 GPUI `.tooltip()` 的代码。上述固定依赖基线尚不包含这一修改。移动视图中不要添加 GPUI 悬停提示。

## 验证与当前限制

集成到应用后，应检查启动和后台恢复、键盘显示与隐藏、视口尺寸变化、文本选择与复制、滚动及触摸反馈。除了 Rust 编译检查，也应观察实际渲染界面。

在做出性能结论前，使用实体设备、Release 构建和 Xcode Instruments 测量。模拟器适合验证布局与交互，但它的结果不是设备帧耗时。

Android 使用独立的 Activity 与渲染表面生命周期。仓库包含 Android 示例，但本文不代表 Android Kit 兼容性或嵌入原生 Android `View` 的能力已经得到验证。采用这些路径前需要单独评估。

## 排查问题

| 现象 | 检查项 |
| --- | --- |
| Xcode 找不到模拟器目标，或应用在另一台模拟器上启动 | 固定版本的 `build.sh` 默认面向 iOS 18.6 的 iPhone 16 Pro 构建。安装该运行时，或对照 `xcodebuild -showdestinations` 修改脚本中的 Xcode destination。随后运行 `xcrun simctl list devices available`：`_ios_run_simulator` 会独立选择第一个可用的 iPhone 安装应用。若它不是构建目标，请把 `sim_id` 选择改为目标模拟器的 UUID。 |
| 真机构建签名或安装失败 | 替换 `example/ios/project.yml` 中的示例开发团队，重新生成 Xcode 工程，并确认 Xcode 能识别设备。脚本默认面向真机；使用本文路径时要显式传入 `--simulator`。 |
| Rust 出现两个 GPUI 版本，或 `App`、`Window` 类型不匹配 | 检查解析出的 `gpui-pre` 包。固定的移动 fork 使用 `0.3.4`，当前工作树使用 `0.3.6`；使用本地 Kit 路径前须统一整套移动平台与 Kit 依赖。 |
| 应用启动后 GPUI 区域空白或画面停滞 | 检查 Rust 回调是否打开窗口、子控制器是否已加入容器、`layoutSubviews` 是否传递非零尺寸，以及界面可见时是否持续请求帧。查看 Xcode 控制台；示例将 Rust 日志与 panic 输出到 `NSLog`。 |
| 文字、图标或图片缺失 | 核对字体及字形覆盖、已注册的 `AssetSource` 与图标键名，以及图片的打包路径或 HTTP 客户端。桌面端的字体与资源配置不会自动进入移动宿主。 |

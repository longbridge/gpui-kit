---
title: UI 自动化测试
description: 为 GPUI Kit 应用编写完整的无头 UI 测试，涵盖依赖配置、真实输入事件、布局断言和 CI 接入。
order: -2.3
example: false
---

# UI 自动化测试

使用 `gpui_kit::test`，可以通过 GPUI 的真实事件分发操作应用视图，再用普通 Rust 断言验证原生无障碍属性、布局和业务结果。测试会创建无头窗口，通过 `ElementId` 定位控件，点击、输入文本，并检查焦点、值和布局。

本指南介绍进程内的行为与布局自动化。元素快照不会检查像素，也不会启动打包后的应用。像素验证使用下文单独介绍的 GPUI 离屏渲染器。如果需要验证原生窗口、平台集成或视觉效果，应另外保留相应测试。

## 配置测试项目

UI 测试直接集成在 `gpui-kit` 中，通过 `test-support` feature 启用。下面示例使用包含这些辅助方法的 Kit 源码检出目录，不需要额外测试 crate、GPUI fork 或 Cargo 补丁。

先按照[安装说明](./installation.md)准备平台依赖。无头测试仍然需要编译 GPUI 的原生依赖。可以在源码目录旁创建独立测试项目：

```text
workspace/
  gpui-kit/
  ui-tests/
    Cargo.toml
    tests/ui.rs
```

在 `ui-tests/Cargo.toml` 中写入：

```toml
[package]
name = "ui-tests"
version = "0.1.0"
edition = "2024"
publish = false

[dev-dependencies]
gpui-kit = { path = "../gpui-kit/crates/kit", features = ["test-support"] }
```

已有应用可以在自己的 package 中添加这个开发依赖。普通 `gpui-kit` 依赖必须解析到相同来源和版本，测试时 feature 才能合并。将 `test-support` 放在开发依赖中，让普通应用构建不启用观察功能。直接使用组件 crate 的应用也可以启用 `gpui-component/test-support`。

## 一个完整测试

把下面代码复制到 `tests/ui.rs`。示例使用 GPUI Kit 的统一入口，初始化组件库，用 `Root` 包装视图，并像真实应用一样将输入状态保存在视图上。

测试会输入 Unicode 姓名，通过 Backspace 编辑，点击 Save，检查状态文本与布局，最后验证保存的业务值。以下代码直接引用仓库集成测试的源码，会实际编译运行。

<<< ../../../crates/kit/tests/ui.rs{rust}

在自己的应用中，应从 library crate 导入生产视图及其构造函数。不要在测试中另写一份视图实现，否则测试与应用可能逐渐不一致。本例内联定义视图，是为了让整个示例可以直接复制到新项目。

在 `ui-tests/` 中运行：

```sh
cargo generate-lockfile
cargo test --test ui --locked
```

将 `Cargo.lock` 一起提交。在 GPUI Kit 源码目录中，可以直接运行同一个示例：

```sh
cargo test -p gpui-kit --features test-support --test ui --locked
```

## 选择稳定的测试目标

启用 `test-support` 后，以下控件在已有原生元素上注册，不增加布局容器：

| 控件 | 除几何与可见性以外报告的状态 |
| --- | --- |
| Button | 无障碍名称、焦点作用域 |
| Input | 非敏感无障碍值、名称、焦点作用域 |
| Checkbox | 勾选、半选、名称、焦点作用域 |
| Switch / Toggle | 勾选、名称、焦点作用域 |
| Radio | 勾选、选中、名称、焦点作用域 |
| Tab | 选中、名称 |
| Select | 无障碍值（包含标题前缀）、展开、焦点作用域 |
| ListItem / SidebarMenuItem | 几何；其他状态仅在原生无障碍属性提供时可读 |

优先使用构造函数 ID。Input 和 Select 支持 `.id("name")`，默认 ID 包含状态 entity ID。
TabBar 内的 Tab 使用下标 ID。Select 已有的 `"input"` 子元素是触发区域：
`window.within("language").click("input", cx)`。

原生 div 只注册观察，不再填写另一份测试状态：

```rust
use gpui_kit::ObserveElement as _;

let target = div().id("details").observe().child(content);
```

`ObserveElement` 始终可用。关闭 `test-support` 时，`.observe()` 直接返回原生元素，
保留其准确类型；启用后保留身份、布局、事件与无障碍接口，不增加布局容器。
重复调用只保留一个注册项。先调用 `.observe()`，再调用 `.track_focus(&handle)`，
让包装器观察实际焦点绑定；Kit 控件在内部完成这件事。`focused()` 检查焦点作用域内
是否存在键盘焦点，也包括 Input 外框中的编辑器；没有观察到焦点绑定时返回 `None`。

快照直接读取 `role`、`aria_toggled`、`aria_selected`、`aria_expanded`、
`aria_label` 和 `aria_value`。没有 `TestProps` 或手填的备用值。Input 在测试中启用
已有的无障碍值生成路径，沿用相同的遮蔽与敏感内容限制。Select 的 `value()` 是包含
标题前缀的无障碍值，不是选中项 ID。

`label()` 表示无障碍名称，不是屏幕文字；`value()` 表示无障碍值，不是像素。
组件仍然可能把这些属性写错。不要仅为让视觉断言通过而添加 `aria_label` 或
`aria_value`。完整示例中的 Status 角色和名称服务于生产环境的无障碍播报。
框架不自动发现任意子元素文字，也不提供用模型字符串冒充绘制文本的 `text()`。

`disabled()` 仅在原生节点暴露禁用标志时返回 `Some(true)`，否则返回 `None`。
当前 GPUI 的 div 接口不能据此提供确定的启用状态。验证禁用行为时，应尝试交互并
检查应用结果没有变化；不能把 `None` 当作启用。

ID 只需在 GPUI 身份作用域内唯一。窗口级查询遇到重复 ID 会报歧义；可以直接使用已有父级作用域，无须添加测试容器：

```rust
window.within("toolbar").click("save", cx);
window.within("dialog").click("save", cx);
let save = window.within("dialog").within("footer").find("save");
assert!(save.visible());
```

父级本身不必被观察：它的 ID 已经包含在被观察子元素的 GPUI 路径中。
`within` 要求当前已绘制路径唯一。列表使用 `("row", record_id)` 等复合 ID，可保持重排后的记录身份。

## 操作与断言

导入 `gpui_kit::test::TestWindowExt` 后使用以下方法：

| API | 行为 |
| --- | --- |
| `window.find(id)` | 严格返回最近完成帧的 `ElementSnapshot`；缺失时 panic，列出注册路径与排查提示。 |
| `window.try_find(id)` | 缺失时返回 `None`，歧义仍会 panic。 |
| `window.click(id, cx)` | 在中心发送原生鼠标移动、按下与释放。 |
| `window.click_at(id, offset, cx)` | 相对于目标左上角的像素偏移点击，适合部分裁剪。 |
| `window.right_click(id, cx)` / `double_click(id, cx)` | 原生右键或两次点击序列。 |
| `window.hover(id, cx)` | 移动指针，不按键。 |
| `window.scroll(id, delta, cx)` | 原生滚轮事件，`ScrollDelta` 保留 GPUI 的方向与单位。 |
| `window.drag(from, to, cx)` | 窗口坐标之间的左键拖拽，经过真实拖拽创建与放置命中测试。 |
| `window.press("backspace", cx)` | 使用 GPUI 按键解析器发送特殊键或快捷键。 |
| `window.input(text, cx)` | 向当前焦点逐字符输入，不自动聚焦或替换整个值。 |

作用域支持 `find`、`try_find`、嵌套 `within`、`click` 和 `click_at`。
拖拽时可查询不同作用域的目标，将它们的 `bounds().center()` 传给 `window.drag`。

`ElementSnapshot` 是某次完成绘制的独立、不可变记录。它提供 `role()`、`path()`、`bounds()`、
`visible()`、`focused()`、`disabled()`、`label()`、`value()`、`checked()`、
`indeterminate()`、`selected()` 和 `expanded()`。焦点、禁用、勾选、半选、选中、展开状态返回 `Option<bool>`：
`None` 表示无法取得，不等于 false。名称与值也可能无法取得。交互后重新查询：

```rust
let before = window.find("agree");
window.click("agree", cx);
assert_eq!(before.checked(), Some(false)); // 原来的帧。
assert_eq!(window.find("agree").checked(), Some(true)); // 新的一帧。
```

同时断言界面状态与业务结果。验证保存的模型或发出的事件也是集成测试的一部分，
但不能取代相关控件可见状态的验证。文本输入不模拟完整的系统 IME 组合输入；
密码输入框不报告值，需要时通过应用状态验证结果。

## 查询前完成一帧

第一次查询、外部直接修改状态或焦点、调整尺寸后，调用 `window.render_frame(cx)`。
交互方法会在同步派发过程中刷新，包括 `press`。但外层 window update 尚未返回时，
它们不能完成需要释放该借用的延迟回调。

```rust
cx.update_window(handle.into(), |_, window, cx| {
    window.render_frame(cx);
    window.click("name", cx);
    window.input("Ada", cx);
    window.press("backspace", cx);
    assert_eq!(window.find("name").value(), Some("Ad"));
}).unwrap();
```

使用 `TestAppContext::update_window`。带类型的 `WindowHandle::update` 已经借用根 entity，
不能在同一个回调中安全地重绘它。

异步工作或 Select 的延迟提交，应在 async `#[gpui_kit::test]` 中、window update **外部**等待：

```rust
use gpui_kit::test::TestAppContextExt;
use std::time::Duration;

cx.wait_for(handle.into(), Duration::from_millis(200), |window, _| {
    window.try_find("result").is_some_and(|snapshot| snapshot.visible())
}).await;
```

`wait_for` 按 GPUI 测试执行器时钟每 10ms 刷新并检查条件，超时报错列出注册路径。
它是有界的条件等待，不模拟操作系统事件循环或网络服务；外部依赖需要受控响应。
执行器停驻本身不代表定时器或延迟工作已经完成。

快照永不原地更新。缓存视图保留绘制事实，直到被失效并重绘。卸载目标在释放其
element state 的帧完成后消失；虚拟列表行则在滚动后实际绘制时进入查询结果。

## 覆盖范围与失败排查

这是逐步扩展的无头交互 API，并未自动覆盖所有 Kit 组件。Table、Menu、Dialog、Dock
尚未拥有全面的自动语义观察与专门的端到端流程测试。原生滚动与拖放已有测试，
包括真实虚拟列表与 HoverCard 延迟显示/关闭，但不能据此宣称所有 Table 或 Dock 行为已验证。
自定义视图需要时可以观察已有原生元素，不支持的属性保持不可用，不提供手填测试值的覆盖入口。

目标缺失或不可见时点击会 panic。禁用控件仍接收原生事件，由控件自己决定是否响应。
可见性结合几何、视口与内容裁剪、目标计算样式，不判断像素遮挡；覆盖层仍会拦截点击。
`click_at(id, point(px(10.), px(10.)), cx)` 可以选择裁剪后可见的部分，不会绕过命中测试。

观察依赖 feature，因此被测制品与生产制品并非逐字节相同。透明包装器不增加布局盒子，
但可见性检查会额外计算一次样式；style/drag 谓词不能依赖调用次数。
GPUI 没有公开未观察祖先的继承绘制透明度，因此无法推断该情况。
实现没有使用 GPUI fork 或 Cargo patch 绕过这些限制。

失败时按具体情况检查注册路径、观察配置、完成帧、键盘焦点、裁剪与覆盖层、异步完成条件。

## 独立验证绘制结果

值或勾选标志正确，不代表控件正确绘制。GPUI 提供
`HeadlessAppContext::with_platform`、`Window::render_to_image` 和
`HeadlessAppContext::capture_screenshot`，可以生成真实离屏图片。当前锁定版本的
平台 crate 仅在 macOS 提供 Metal 离屏渲染器。在支持 Metal 的 Mac 上执行：

```sh
cargo test -p gpui-kit --features test-support --test rendering --locked
```

该目标设置了 `test = false`，普通 CI 运行可跨平台执行的交互测试；在支持 Metal 的
runner 上用 `--test rendering` 显式运行像素测试。这使用 Cargo 的
[显式目标选择](https://doc.rust-lang.org/cargo/commands/cargo-test.html#target-selection)。
目标还使用 `harness = false`，因为 AppKit 必须在主线程初始化；普通 Rust
测试即使指定 `--test-threads=1` 仍运行在工作线程。其他平台明确报告跳过像素验证；
macOS 缺少渲染能力时测试失败，不用假图片替代。

测试向真实 Kit 控件注入两种故障：`checked()` 仍为 true，但勾号资源丢失；
`value()` 仍正确，但输入文字变透明。故障图片必须与正常控件不同，重复绘制正常
Checkbox 的图片必须一致。另一个原生事件测试断开 Checkbox 的状态更新处理器，
验证点击不会凭空产生已勾选结果。

这些测试验证能否发现特定错误，不是完整的基准图片回归测试。应用的视觉回归应在
固定字体、尺寸、主题、焦点和动画状态下，将图片与已审查的预期结果比较。状态与
图片断言能发现不同的故障；两者都不能证明打包应用或完整 IME 行为正确。
可执行示例见
[`crates/kit/tests/rendering.rs`](https://github.com/longbridge/gpui-kit/blob/testing/crates/kit/tests/rendering.rs)。

## 接入 CI

Kit 仓库在 macOS、Linux 和 Windows 矩阵中运行无头测试。以下是用于 Kit 检出目录的最小 macOS workflow：

```yaml
name: UI tests
on: [push, pull_request]
jobs:
  test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: ./script/bootstrap
      - run: cargo test -p gpui-kit --features test-support --locked
```

应用仓库需要安装自身的平台依赖，并改为在测试 package 中运行 `cargo test --test ui --locked`。将锁定版本的 Kit 源码放到 manifest 声明的路径，再按照普通原生构建的环境配置增加 Linux 和 Windows job。

仓库测试还覆盖只读与禁用输入、焦点变化、缓存视图、挂载与卸载、多窗口隔离、原生命中测试，以及列表从 1,000 个元素缩减后的清理。大列表用例验证正确性，不是渲染性能基准。

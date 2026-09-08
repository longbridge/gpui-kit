---
title: UI 自动化测试
description: 为 GPUI Kit 应用编写完整的无头 UI 测试，涵盖依赖配置、真实输入事件、布局断言和 CI 接入。
order: -2.3
example: false
---

# UI 自动化测试

使用 `gpui_kit::test`，可以通过 GPUI 的真实事件分发操作应用视图，再用普通 Rust 断言验证渲染状态和业务结果。测试会创建无头窗口，通过 `ElementId` 定位控件，点击、输入文本，并检查焦点、值和布局。

本指南介绍进程内的行为与布局自动化。测试框架不会启动打包后的应用，也不会检查像素。如果需要验证原生窗口、平台集成或视觉效果，应另外保留相应测试。

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
| Button | 标签文本、焦点、禁用、选中 |
| Input | 未掩码的文本与值、焦点、禁用 |
| Checkbox | 勾选、半选、焦点、禁用 |
| Switch / Toggle | 勾选、焦点、禁用 |
| Radio | 勾选、选中、焦点、禁用 |
| Tab | 选中、禁用 |
| Select | 选中项标题作为值、展开、焦点、禁用 |
| ListItem | 选中、禁用 |
| SidebarMenuItem | 标签文本、选中、展开、禁用 |

优先使用构造函数 ID。Input 和 Select 支持 `.id("name")`，默认 ID 包含状态 entity ID。
TabBar 内的 Tab 使用下标 ID。Select 已有的 `"input"` 子元素是触发区域：
`window.within("language").click("input", cx)`。

原生 div 通过 fluent API `.test_props(...)` 参与观察，额外测试属性在同一个闭包中配置：

```rust
use gpui_kit::TestPropsExt as _;

let status = div()
    .id("status")
    .test_props(|props| props.text(message.clone()))
    .child(message);
```

这里 `message` 是 `SharedString`。`TestPropsExt` 始终可用，不需要在渲染代码中写
`#[cfg]`、另建临时元素或克隆临时焦点。关闭 `test-support` 时不调用闭包，直接返回
原生元素，内部状态存储为零大小。把文本转换等测试专用计算放在闭包内；Rust
仍会正常创建和释放闭包捕获的值。

闭包接收 `TestProps`，只补充无法从 GPUI 读取的信息。它提供 `text`、`focus`、`disabled`、`checked`、`indeterminate`、
`selected`、`expanded` 和 `value`。这些方法只报告事实，不改变控件行为；
例如 `props.disabled(true)` 不会禁用事件处理器。`TestProps` 描述可观察的事实，
不是 Entity，也不是控件模型的另一份 State。连续调用 `.test_props(...)` 会完善同一份
观察信息，保留已提供的事实，不会嵌套额外包装器。

角色、勾选／半选、选中、展开状态自动从现有 `role`、`aria_toggled`、
`aria_selected`、`aria_expanded` 读取。已有属性优先，闭包中的对应值只用于缺失时补充，
避免控件报告两份互相矛盾的状态。焦点关联、未公开的禁用信息，以及准确的逻辑文本和值
仍可在闭包中提供。无障碍名称不自动当作显示文本，带占位符或前缀的无障碍值也不自动
当作逻辑值；密码值不会因此被读取出来。快照另提供 `role()`。

观察保留已有 div 的身份、布局、事件与无障碍接口，不自动发现任意子元素的文字或 ID。
只观察几何与可见性时写 `.test_props(|props| props)`。

ID 只需在 GPUI 身份作用域内唯一。窗口级查询遇到重复 ID 会报歧义；可以直接使用已有父级作用域，无须添加测试容器：

```rust
window.within("toolbar").click("save", cx);
window.within("dialog").click("save", cx);
let save = window.within("dialog").within("footer").find("save");
assert!(!save.disabled());
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

`ElementSnapshot` 是某次完成绘制的独立、不可变记录。它提供 `path()`、`bounds()`、
`visible()`、`focused()`、`disabled()`、`text()`、`value()`、`checked()`、
`indeterminate()`、`selected()` 和 `expanded()`。最后四种状态返回 `Option<bool>`：
`None` 表示未报告，不等于 false。文本与值也可以未报告。交互后重新查询：

```rust
let before = window.find("agree");
window.click("agree", cx);
assert_eq!(before.checked(), Some(false)); // 原来的帧。
assert_eq!(window.find("agree").checked(), Some(true)); // 新的一帧。
```

同时断言界面状态与业务结果。验证保存的模型或发出的事件也是集成测试的一部分，
但不能取代相关控件可见状态的验证。文本输入不模拟完整的系统 IME 组合输入；
密码输入框既不报告文本，也不报告值，需要时通过应用状态验证结果。

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
自定义视图需要时可以观察已有原生元素，并报告真实状态，而非测试专用常量。

目标缺失或不可见时点击会 panic。禁用控件仍接收原生事件，由控件自己决定是否响应。
可见性结合几何、视口与内容裁剪、目标计算样式，不判断像素遮挡；覆盖层仍会拦截点击。
`click_at(id, point(px(10.), px(10.)), cx)` 可以选择裁剪后可见的部分，不会绕过命中测试。

观察依赖 feature，因此被测制品与生产制品并非逐字节相同。透明包装器不增加布局盒子，
但可见性检查会额外计算一次样式；style/drag 谓词不能依赖调用次数。
GPUI 没有公开未观察祖先的继承绘制透明度，因此无法推断该情况。
实现没有使用 GPUI fork 或 Cargo patch 绕过这些限制。

失败时按具体情况检查注册路径、观察配置、完成帧、键盘焦点、裁剪与覆盖层、异步完成条件。

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

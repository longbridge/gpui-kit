---
title: UI 自动化测试
description: 为 GPUI Kit 应用编写完整的无头 UI 测试，涵盖依赖配置、真实输入事件、布局断言和 CI 接入。
order: -2.3
example: false
---

# UI 自动化测试

使用 `gpui-test`，可以通过 GPUI 的真实事件分发操作应用视图，再用普通 Rust 断言验证渲染状态和业务结果。测试会创建无头窗口，通过 `ElementId` 定位控件，点击、输入文本，并检查焦点、值和布局。

本指南介绍进程内的行为与布局自动化。测试框架不会启动打包后的应用，也不会检查像素。如果需要验证原生窗口、平台集成或视觉效果，应另外保留相应测试。

## 配置测试项目

目前从 GPUI Kit 源码检出目录引入 `gpui-test`。请使用包含测试 crate 的版本，并让 Kit 和测试 crate 来自同一提交。不需要 GPUI fork 或 Cargo 补丁。

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
gpui-test = { path = "../gpui-kit/crates/test" }
```

已有应用可以在自己的 package 中添加这两个开发依赖。普通 `gpui-kit` 依赖必须解析到相同来源和版本，测试时 feature 才能合并。将 `test-support` 放在开发依赖中，让普通应用构建不启用观察功能。直接使用组件 crate 的应用也可以启用 `gpui-component/test-support`。

## 一个完整测试

把下面代码复制到 `tests/ui.rs`。示例使用 GPUI Kit 的统一入口，初始化组件库，用 `Root` 包装视图，并像真实应用一样将输入状态保存在视图上。

测试会输入 Unicode 姓名，通过 Backspace 编辑，点击 Save，检查状态文本与布局，最后验证保存的业务值。以下代码直接引用仓库集成测试的源码，会实际编译运行。

<<< ../../../crates/test/tests/ui.rs{rust}

在自己的应用中，应从 library crate 导入生产视图及其构造函数。不要在测试中另写一份视图实现，否则测试与应用可能逐渐不一致。本例内联定义视图，是为了让整个示例可以直接复制到新项目。

在 `ui-tests/` 中运行：

```sh
cargo generate-lockfile
cargo test --test ui --locked
```

将 `Cargo.lock` 一起提交。在 GPUI Kit 源码目录中，可以直接运行同一个示例：

```sh
cargo test -p gpui-test --test ui --locked
```

## 选择稳定的测试目标

Kit Button 使用构造函数中的 ID，例如 `Button::new("save")`。Input 可以通过 `Input::new(&state).id("name")` 指定 ID；默认 ID 包含输入状态的 entity ID。

原生 GPUI div 通过 `.id(...).observe()` 显式参与观察。视图知道逻辑文本时，用 `.text(value)` 提供；包装器不会从任意子元素中提取文字。生产视图可按 feature 条件添加：

```rust
let status = div().id("status").child(message.clone());
#[cfg(feature = "test-support")]
let status = status.observe().text(message.clone());
```

这里的 `message` 是 `SharedString`，`ObserveElement` 也应在相同条件下导入。采用这种写法时，需要在应用中声明 `test-support` feature，启用对应的 Kit feature，并将 `gpui-test` 作为可选依赖供视图代码使用；测试命令增加 `--features test-support`。仅添加开发依赖，只能供集成测试使用，不能供被测试的应用 library 使用。

为需要查询的目标选择唯一 ID。列表可以使用 `("row", record_id)` 这样的复合 ID，使查询在重排后仍对应同一条记录。不同作用域出现相同 ID 时，窗口级查找存在歧义，会 panic。

## 操作与断言

| API | 行为 |
| --- | --- |
| `window.find(id)` | 返回最近完成帧的独立快照；目标未注册时返回 `None`。 |
| `window.click(id, cx)` | 在目标中心依次发送鼠标移动、按下和释放，经过 GPUI 命中测试。 |
| `window.input(text, cx)` | 通过模拟按键向当前键盘焦点输入文本，不负责聚焦目标，也不会替换整个值。 |
| `cx.simulate_keystrokes(handle.into(), "backspace")` | 复用 GPUI 现有接口发送特殊按键和快捷键。 |

快照提供 `bounds()`、`visible()`、`focused()`、`disabled()` 和 `text()`。围绕具体行为做断言，例如保存结果、输入焦点、禁用控件保持无响应，或浮层位于触发按钮下方。还要检查业务状态或输出结果，避免仅凭显示文字就让错误的流程通过测试。

文本输入按字符模拟，不覆盖操作系统输入法的完整组合输入流程。密码输入框不会报告文本；需要验证实际结果时，读取应用状态。

## 查询前完成一帧

第一次查询前先绘制。点击和输入辅助方法会在事件分发前后刷新并绘制。直接修改状态、改变焦点、调整窗口大小或执行 GPUI 键盘动作后，需要显式刷新：

```rust
cx.update_window(handle.into(), |_, window, cx| {
    window.refresh();
    window.draw(cx).clear(cx);
    assert!(window.find("name").unwrap().focused());
}).unwrap();
```

使用 `TestAppContext::update_window` 执行这些交互。带类型的 `WindowHandle::update` 已经借用了根 entity，不能在同一个回调中安全地重绘它。可以用带类型的 update 检查或修改模型，返回后再通过 `update_window` 绘制。

异步任务应在窗口 update 之外通过 `cx.run_until_parked()` 驱动测试执行器，然后刷新并查询。等待定时器、网络或外部服务的任务还需要可控测试时钟或模拟响应；执行器停驻不代表这些任务都已完成。不要用固定时长的 sleep 代替完成条件。

快照不会原地更新。下一帧后重新调用 `find` 获取状态。缓存视图保留最近绘制的事实，外部状态变化后需要 refresh。元素卸载后，在释放其 element state 的帧完成时，查询结果会消失。

## 可见性与失败排查

点击不存在或不可见的目标会 panic，并包含目标 ID。禁用控件仍然接收原生事件，由控件自身决定是否响应；测试应检查业务状态没有变化。

可见性结合布局尺寸、视口与内容裁剪、目标的计算样式判断。覆盖层可能拦截点击，即使目标报告可见。部分裁剪元素的中心也可能落在可见区域之外。这些情况下，辅助方法不会绕过命中测试直接调用回调。GPUI 没有公开未观察祖先的透明绘制信息，包装器无法推断这种祖先透明度。

遇到失败，依次检查：

1. 目标是否有稳定 ID，且通过 Kit 的测试支持或 `.observe()` 注册。
2. 是否完成了反映当前状态的刷新与绘制。
3. 预期输入框是否有焦点，是否有覆盖层或裁剪拦截点击。
4. 异步依赖是否已在测试执行器中完成。

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
      - run: cargo test -p gpui-test --locked
```

应用仓库需要安装自身的平台依赖，并改为在测试 package 中运行 `cargo test --test ui --locked`。将锁定版本的 Kit 源码放到 manifest 声明的路径，再按照普通原生构建的环境配置增加 Linux 和 Windows job。

仓库测试还覆盖只读与禁用输入、焦点变化、缓存视图、挂载与卸载、多窗口隔离、原生命中测试，以及列表从 1,000 个元素缩减后的清理。大列表用例验证正确性，不是渲染性能基准。

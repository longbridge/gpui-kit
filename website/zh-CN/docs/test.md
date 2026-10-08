---
title: Testing
description: 通过 Rust 单元测试、TestAppContext、真实 UI 交互和布局断言测试 GPUI Kit 应用与 GPUI 行为。
order: -3.39
example: false
---

# 测试

本指南统一介绍 GPUI Kit 应用和 GPUI 的测试方式。根据要验证的行为选择测试层级：

- 纯数据转换、校验和状态转换使用普通 Rust `#[test]`。
- [Entity](./entity)、[Action](./action)、[订阅](./event)和异步 [Task](./task) 使用 `#[gpui_kit::test]` 与 `TestAppContext`，按需创建窗口。
- UI 集成测试渲染真实应用视图，通过 `gpui_kit::test` 派发[事件](./event)，再检查控件状态、布局和业务结果。
- 像素检查使用独立的离屏渲染器；原生窗口和平台集成保留相应测试。

类型和 `#[gpui_kit::test]` 均由 Kit 根模块提供，应用无需再添加 GPUI 依赖。测试模块应显式导入用到的类型：`use gpui_kit::*;` 也会引入 GPUI 的 `test` 宏，可能遮蔽 Rust 原生的 `#[test]`。下方完整示例使用显式导入。

## 什么是 UI 集成测试？

**UI 集成测试**在无头窗口中渲染真实组件或应用视图，模拟点击、键盘输入和滚动，
验证组件状态、焦点、布局及业务回调。例如，给 Checkbox 增加 UI 集成测试，
可以验证点击是否修改了宿主持有的值，以及禁用时是否拒绝同样的交互。

`#[gpui_kit::test]` 负责运行测试并提供 GPUI 上下文；
`gpui_kit::test` 提供操作和检查界面的工具：

```rust
use gpui_kit::{TestAppContext, Window};
use gpui_kit::test::TestWindowExt;
```

当行为涉及组件之间的协作，例如输入内容、保存对话框、检查父视图中的结果，
就适合使用 UI 集成测试。测试通过 [`ElementId`](./element_id) 定位控件，派发真实 GPUI 事件，
再用普通 Rust 断言检查结果。

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
    tests/common/mod.rs
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

下面直接引用仓库中会参与编译的 `tests/ui.rs`。它初始化组件库，并像真实应用一样将输入状态保存在视图上。其中 `mod common;` 会加载配套的
`tests/common/mod.rs` fixture。这个辅助文件以明确的 640 × 480 窗口尺寸调用公开的
`gpui_kit::open_window`，由该函数用 `gpui_kit::base::Root` 包装视图，并返回窗口句柄与视图 entity。它只是测试准备代码，不是额外的库依赖。

测试会输入 Unicode 姓名，通过 Backspace 编辑，点击 Save，检查状态节点的 AccessKit 角色、名称与布局，最后验证保存的业务值。测试并不能验证屏幕阅读器是否实际播报。以下代码直接引用仓库集成测试的源码，会实际编译运行。

<<< ../../../crates/kit/tests/ui.rs{rust}

独立项目必须复制**两个**文件；只复制 `ui.rs` 会在 `mod common;` 处失败。在上述布局的 `ui-tests/` 目录运行：

```sh
mkdir -p tests/common
cp ../gpui-kit/crates/kit/tests/ui.rs tests/ui.rs
cp ../gpui-kit/crates/kit/tests/common/mod.rs tests/common/mod.rs
cargo generate-lockfile
cargo test --test ui --locked
```

将 `Cargo.lock` 一起提交。在自己的应用中，应从 library crate 导入生产视图及其构造函数，替换示例中的 `Profile`，继续沿用窗口设置与交互写法。另写一份测试视图容易与应用实现脱节。在 GPUI Kit 源码目录中，可以直接运行同一个测试：

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
| Command | 原生选项 selected 状态、面板焦点作用域和行边界 |
| Combobox | 原生 expanded 状态与焦点作用域；选择结果通过事件及状态验证 |
| Select | 无障碍值（包含标题前缀）、展开、焦点作用域 |
| ListItem / SidebarMenuItem | 几何；其他状态仅在原生无障碍属性提供时可读 |
| Accordion | 触发器展开状态；标题与面板边界 |
| Tree | 原生树与节点角色、名称、选中与展开状态；根节点焦点作用域 |
| Table / DataTable | 原生表格部件；DataTable 行选中状态与根焦点作用域 |
| DatePicker / Calendar | 日期选择器显示的日期值、展开与焦点作用域；日历项名称与边界 |
| Slider | 轨道与滑块边界；`ElementSnapshot::value()` 不读取数值型无障碍属性 |
| Stepper | 步骤与触发器边界；通过应用内容验证导航结果 |
| Dialog / Sheet | 宿主焦点作用域与内容表面边界；子控件保留各自属性 |
| Menu | 菜单项名称与选中状态、菜单焦点作用域、子菜单边界 |
| Notification | Alert 角色与边界；关闭按钮沿用 Button 观察 |
| Progress / ProgressCircle | 进度指示器角色与名称；`ElementSnapshot::value()` 不读取数值型无障碍属性 |
| TitleBar | `title-bar` 与 `window-controls` 边界；窗口控制按钮（`minimize`、`maximize` 或 `restore`、`close`）仅在标题栏自行绘制时可读 |
| Dock | 区域、分组与内容边界及焦点作用域；Tab 保留原生选中状态 |

优先使用构造函数 ID。Input 和 Select 支持 `.id("name")`，默认 ID 包含状态 entity ID。
TabBar 内的 Tab 使用下标 ID。Select 已有的 `"input"` 子元素是触发区域：
`window.within("language").click("input", cx)`。

原生 div 只注册观察，不再填写另一份测试状态：

```rust
use gpui_kit::TestSupportExt as _;

let target = div().id("details").test_support().child(content);
```

`TestSupportExt` 始终可用。关闭 `test-support` 时，`.test_support()` 直接返回原生元素，
保留其准确类型；启用后保留身份、布局、事件与无障碍接口，不增加布局容器。
重复调用只保留一个注册项。先调用 `.test_support()`，再调用 `.track_focus(&handle)`，
让包装器观察实际焦点绑定；Kit 控件在内部完成这件事。`focused()` 检查焦点作用域内
是否存在键盘焦点，也包括 Input 外框中的编辑器。如果 GPUI 声明元素可聚焦，但没有
观察到绑定，`focused()` 会报错并给出修复提示，不再静默返回 `None`。这能发现
`.track_focus(&handle).test_support()` 的顺序错误；隐式 `.focusable()` 句柄也无法读取，
应改用显式句柄。没有观察到绑定、也没有声明焦点 action 时返回 `None`。
这个诊断是尽力而为的，依赖原生 accessibility 的 `Action::Focus`。
自定义元素如果没有声明此 action，遗漏绑定时仍可能返回 `None`，因此 `None`
不能证明元素无法获得焦点。检测到遗漏绑定时，Debug 输出
`focused: <binding missed>`，格式化本身不会 panic。

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
这个接口无法可靠地正面断言 enabled 属性。验证按钮接受操作时，应执行操作并断言
它应该产生的结果，例如：

```rust
window.click("save", cx);
assert_eq!(window.find("status").label(), Some("Saved: Ada"));
```

这里应使用应用真实的预期结果。`assert_ne!(button.disabled(), Some(true))` 或
`button.disabled().is_none()` 都不能证明按钮可以正常响应。

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

从 `gpui_kit::test` 导入 `TestWindowExt` 使用查询和交互方法：

| API | 行为 |
| --- | --- |
| `window.find(id)` | 严格返回最近完成帧的 `ElementSnapshot`；缺失时 panic，列出注册路径与排查提示。 |
| `window.try_find(id)` | 缺失时返回 `None`，歧义仍会 panic。 |
| `window.find_all(id)` | 按当前帧边界原点的 y、x 排序，返回所有已注册匹配项，包括不可见元素；同坐标顺序不保证，缺失时返回空 `Vec`。 |
| `window.click(id, cx)` | 在中心发送原生鼠标移动、按下与释放。 |
| `window.click_at(id, offset, cx)` | 相对于目标左上角的像素偏移点击，适合部分裁剪。 |
| `window.click_with_modifiers(id, modifiers, cx)` | 携带修饰键在中心左键点击；发送修饰键变化事件，结束后恢复之前的修饰键状态。 |
| `window.click_with_options(id, options, cx)` | 用 `ClickOptions` 组合偏移、鼠标键、点击次数和修饰键；结束后恢复之前的修饰键状态。 |
| `window.right_click(id, cx)` / `double_click(id, cx)` | 原生右键或两次点击序列。 |
| `window.hover(id, cx)` | 移动指针，不按键。 |
| `window.scroll(id, delta, cx)` | 原生滚轮事件，`ScrollDelta` 保留 GPUI 的方向与单位。 |
| `window.drag_to(from_id, to_id, cx)` | 定位两个目标，在其中心之间通过真实命中测试拖拽。 |
| `window.drag(from, to, cx)` | 窗口坐标之间的左键拖拽，经过真实拖拽创建与放置命中测试。 |
| `window.press("backspace", cx)` | 使用 GPUI 按键解析器，为特殊键或快捷键发送原生 key-down/key-up 事件。 |
| `window.input(text, cx)` | 向当前焦点逐字符输入，不自动聚焦或替换整个值。 |

`ClickOptions::new()` 默认在中心单击左键，不带修饰键。例如，
`ClickOptions::new().with_offset(point(px(8.), px(8.)))
.with_button(MouseButton::Right).with_count(2).with_modifiers(Modifiers::shift())`
组合局部偏移、右键双击和 Shift。点击次数必须大于零，可配置点击保留 Caps Lock 状态。
`find_all` 的顺序只表示当前帧几何位置，不表示稳定身份、绘制顺序或业务顺序；
统计可见匹配项时，用 `visible()` 过滤。

作用域支持 `find`、`try_find`、`find_all`、嵌套 `within`、`click`、`click_at`、
`click_with_modifiers`、`click_with_options`、`right_click`、`double_click`、`hover`、`scroll`、`drag_to`、`press` 和 `input`。
`drag_to` 的两个 ID 都在当前作用域中解析。跨作用域拖拽或指定偏移时，可查询目标后
将窗口坐标传给 `window.drag`。

```rust
let mut dialog = window.within("dialog");
dialog.click("name", cx);
dialog.input("Ada", cx);
dialog.press("backspace", cx);
dialog.hover("help", cx);
```

作用域内的键盘操作不会移动焦点，必须先有一个已观察的焦点绑定位于该作用域中，
否则派发前就报错。`input` 在每个字符前检查，所以处理器把焦点移到作用域外时，
剩余文字不会输入到其他控件。需要窗口级快捷键时，使用 `window.press`。

自定义输入控件需要在实际承载焦点的元素上调用
`.id("editor").test_support().track_focus(&focus_handle)`，使用控件真正的焦点句柄。
未观察的输入控件，或没有绑定焦点句柄的外层容器，即使实际焦点位于作用域内，
也无法通过检查。窗口级 `input` 和 `press` 向当前焦点派发，但不提供作用域保证。
作用域输入与窗口输入共用同一循环：开始时刷新一次，随后每个字符刷新一次，
每次作用域检查都读取已完成的帧。

`ElementSnapshot` 是某次完成绘制的独立、不可变记录。它提供 `role()`、`path()`、`bounds()`、
`visible()`、`focused()`、`disabled()`、`label()`、`value()`、`checked()`、
`indeterminate()`、`selected()` 和 `expanded()`。焦点、禁用、勾选、半选、选中、展开状态返回 `Option<bool>`：
`None` 表示无法取得，不等于 false。名称与值也可能无法取得。交互后重新查询：

```rust
let before = window.find("agree");
window.click("agree", cx);
assert_eq!(before.checked(), Some(false)); // Previous frame.
assert_eq!(window.find("agree").checked(), Some(true)); // New frame.
```

同时断言界面状态与业务结果。验证保存的模型或发出的事件也是集成测试的一部分，
但不能取代相关控件可见状态的验证。包括 Enter 在内的命令按键不应通过 IME 回调注入换行文字。
文本输入不模拟完整的系统 IME 组合输入；
密码输入框不报告值，需要时通过应用状态验证结果。

## 通过输入处理器验证文字组合

`gpui_kit::test::TestInput` 将显式传入的 GPUI `InputHandler` 与窗口连接起来。
如果 entity 实现了 `EntityInputHandler`，先用
`gpui_kit::ElementInputHandler::new(bounds, entity.clone())` 创建适配器，再传给
`TestInput::new(handler, window)`。entity 应由生产视图持有，bounds 使用
真实输入区域。这个桥接器直接调用指定处理器，不会选择当前焦点控件，也不运行系统 IME。

`commit_text(text, cx)` 通过 `replace_text_in_range` 提交文字；
`compose_text(replacement, text, selection, cx)` 通过
`replace_and_mark_text_in_range` 更新组合文字。替换范围和选区使用 UTF-16 code unit，
不是 UTF-8 字节偏移；选区相对于组合文字。替换范围为 `None` 时，处理器替换现有的
标记范围或选区。空的组合文字按处理器协议取消预编辑。
`marked_text_range(cx)` 返回文档中的标记范围；`selected_text_range(cx)` 返回
`Option<UTF16Selection>`，包含选区方向。`unmark_text(cx)` 结束标记并保留文字。修改方法调用处理器后完成一帧，可验证组合状态变化与提交行为。系统候选窗口、
焦点路由和系统 IME 集成仍需原生测试。向当前焦点输入普通文字时使用 `window.input`。

## 窗口尺寸、显示缩放与剪贴板

在窗口 update 外调用 GPUI 已有的
`TestAppContext::simulate_window_resize(handle, size)` 和
`simulate_window_scale_factor_change(handle, scale)`，再用 `render_frame`
刷新布局。不同显示缩放下，鼠标坐标仍使用窗口内的逻辑像素。应同时检查编辑状态是否
保留、布局是否调整，并在新位置激活控件。

用生产快捷键（`secondary-c`、`secondary-x`、`secondary-v`）验证剪贴板操作，
需要时通过 `cx.read_from_clipboard()` 检查内容。测试平台在窗口之间共享剪贴板，
编辑器状态则应互相独立。这些测试验证测试平台适配器与应用快捷键，不验证系统剪贴板服务。

激活外部 Link 后，可以用 `cx.opened_url()` 检查测试平台收到的 URL，
同时验证应用的点击回调。它不代表浏览器已打开目标页面。测试平台未公开鼠标指针
样式的读取 API；不能根据 URL 或回调断言指针样式。

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

GPUI `dispatch_action` 会排队执行。继续修改 action 将读取的值之前，应先完成派发，
例如离开 `update_window` 后运行 `cx.run_until_parked()`；结果或计时器完成使用 `wait_for`。
旧的非同步 GPUI `Animation` 使用真实时钟 `Instant`，推进测试时钟不会让动画结束。
Sheet/Notification 几何测试会等待真实入场时长，再断言最终边界。
Base motion 则可以响应公开的 `cx.set_reduce_motion(true)` 偏好，用于测试展开后的最终几何。

## 按无障碍属性查询与分步事件

导入 `gpui_kit::test::{TestQueryExt, TestEventExt}`，可使用更多查询与事件操作。
`elements()` 返回当前帧中已观测的元素；`find_by_label(label)`、
`try_find_by_label(label)` 和 `find_all_by_label(label)` 精确匹配原生无障碍标签，区分大小写；
`find_all_by_role(role)` 按 AccessKit `Role` 查询。结果包括不可见的注册元素，
先按 y、再按 x 排序，同位置按路径区分。必需查询缺少目标时 panic；唯一目标查询
遇到歧义也会 panic。作用域查询只包含后代，不包含作用域节点自身。这些方法也支持 `within`
作用域，不会发现未观测的文字或尚未绘制的虚拟行。标签是无障碍名称，不是屏幕文字；
名称会随本地化变化时，优先使用稳定 ID。
锁定版本的 GPUI 不提供匿名绘制文本的枚举。因此 Kit 没有通用的绘制文本
`HasText` 断言；精确匹配无障碍标签不能证明文字已经实际绘制。

需要检查手势中间状态时，可分别发送事件：

```rust
use gpui_kit::test::TestEventExt;

let start = window.find("handle").bounds().center();
window.pointer_down(start, MouseButton::Left, cx);
window.pointer_move(start + point(px(24.), px(0.)), Some(MouseButton::Left), cx);
// 在鼠标按住期间检查状态。
window.pointer_up(start + point(px(24.), px(0.)), MouseButton::Left, cx);
```

坐标使用窗口内的像素位置。`pointer_move(position, pressed_button, cx)`
需要显式指定按住的按钮；这些方法不保存按钮状态。`pointer_down` 与 `pointer_up`
各发送一次点击事件。普通点击、悬停、滚动、拖动和分步鼠标事件使用窗口当前的修饰键；
配置式点击临时使用指定修饰键，结束后恢复之前的状态。`change_modifiers(modifiers, cx)`
修改修饰键并保留 caps lock，手势结束后应恢复之前的修饰键。
`key_down(key, is_held, cx)` 与 `key_up(key, cx)` 支持分别发送按下、松开和自动重复
事件（`is_held = true`）。键盘事件合并按键字符串中的修饰键与窗口已按住的修饰键，
不修改窗口保存的修饰键状态。每一步先刷新，再发送事件并完成一帧。这些键盘事件不插入
文字；输入文字使用 `input`。延迟结果仍需离开窗口 update 后等待。

## 覆盖范围与失败排查

仓库通过真实输入、原生属性和布局边界验证以下组件流程。这些是具体的回归契约，
不代表已穷举每个组件的全部配置和组合。

| 测试文件 | 验证行为 |
| --- | --- |
| `test_macro.rs` | 普通 `#[test]` 与同步/异步 `#[gpui_kit::test]` 共存；独立、仅依赖 Kit 的 recipes 包复用相同契约 |
| `input.rs` 与 `input/` | Input、Textarea、Editor 的编辑、剪贴板、选区、历史记录、只读切换、Unicode、多行视口、搜索替换、补全确认，以及重绘后的状态保留 |
| `input_focus.rs` | 带静态前后缀的输入框反复 Tab/Shift-Tab 遍历、前后缀按钮的焦点与激活，以及点击 Textarea/Editor 正文后的焦点和编辑 |
| `search.rs` | Command 禁用项跳过、循环导航、中文关键词、空结果、Action 与原始索引回调、两阶段 Escape；Combobox 搜索、单选/多选、清除、空结果恢复、禁用行为及关闭时仅一次 Confirm |
| `disclosure.rs` | Accordion 互斥展开、折叠与实际面板几何；Stepper 内容导航；禁用展开与步骤操作；Slider 轨道点击、滑块拖动与禁用行为 |
| `collections.rs` | Tree 点击展开、键盘展开/折叠与选择；DataTable 行选择、键盘虚拟滚动与滚轮滚动 |
| `query_helpers.rs` | 精确无障碍标签和角色、作用域歧义、不可见注册元素与不可变快照 |
| `event_helpers.rs` | 分步鼠标和按键事件、自动重复与保持修饰键 |
| `control_steps.rs` | Button 移出释放取消、Checkbox 按键重复及 Switch 在按键释放前禁用 |
| `slider_steps.rs` | 滑轨与拖动中的实时值变化、释放事件顺序及禁用时的手势 |
| `semantic_controls.rs` | RadioGroup 作用域内的无障碍名称、受控状态及单项和整组禁用 |
| `collection_steps.rs` | List 释放时的修饰键、移出取消、重复导航与确认 |
| `composition_helpers.rs` | 显式输入处理器桥接、UTF-16 预编辑、整段提交、取消及多行 Textarea/Editor 的撤销和 readonly 切换 |
| `environment.rs` | 窗口尺寸和显示缩放、跨窗口剪贴板、Link URL 请求与应用回调 |
| `date_picker.rs` | 打开、精确预设日期与日历日期选择、月份切换、清除、Escape 与禁用行为 |
| `overlays.rs` | Dialog 校验 → 作用域 Input → 保存 → Notification；悬停显示关闭按钮；自动关闭计时；Dialog/Sheet Escape 与焦点恢复；表面边界 |
| `menu.rs` | 禁用菜单项、键盘确认、Escape、焦点恢复、子菜单悬停、嵌套菜单项激活及最内层右键菜单的事件归属 |
| `dock.rs` | Tab 选择与重排、跨分组拖放、放大和恢复分割布局 |

已有表单、Select、HoverCard、虚拟列表、指针、生命周期和隔离测试继续保留。
纯展示组件通过几何或像素断言验证，不虚构交互状态。自定义部件观察已有原生元素；
不支持的属性保持不可用，不提供手填测试值的覆盖入口。

[Input 回归测试范例](https://github.com/longbridge/gpui-kit/tree/main/crates/kit/tests/input)
展示了如何把手工编辑步骤变成可重复的 UI 测试。在仓库根目录运行编辑和焦点两个目标，
或选择单个流程：

```sh
script/test-input # 完整回归入口：Base、Component 和 Kit（使用 Bash）
cargo test -p gpui-kit --features test-support --test input --test input_focus --locked
cargo test -p gpui-kit --features test-support --test input --locked -- history::paste_is_atomic_and_separate_from_surrounding_typing --exact
cargo test -p gpui-kit --features test-support --test input_focus --locked -- reverse_tab_cycles_three_inputs_with_passive_addons --exact
```

在合并运行命令后追加 `-- --list`，可列出用例而不执行。
这些命令用于复现，不代表已有通过记录。报告运行结果时，请注明代码版本、平台、命令和实际结果。

示例流程包括输入 → 粘贴 → 输入 → Undo/Redo、Textarea 的 Enter 提交与 Shift-Enter 换行，
以及 Editor 补全 → 接受 → Undo。每个流程读取新的快照，必要时结合公开状态或应用事件验证。
补全使用提供固定响应的 provider，不连接实际语言服务器。
独立的 `input_focus` 目标在 window update 结束后验证焦点回调。
测试还覆盖公开输入法接口的预编辑、UTF-16 范围、提交/取消与 Undo 边界，以及多光标、折叠、异步 provider 的取消和失败。
普通 Input 改动以操作覆盖表和各平台 CI 为验收依据，并为实际改动补充回归用例，减少重复手工检查整套编辑操作。
这些用例不证明完整的系统 IME、辅助功能动作、系统剪贴板或像素显示正确；
这些边界需要相应的平台验证。

通过 `WindowExt` 打开 Dialog、Sheet 或 Notification 的视图，需要窗口的根视图是 `Root`。
`Root` 始终将这三类浮层渲染在应用内容之上，缓存视图也一样，无需手动挂载。

重复控件使用 `within`。Sheet 的 `"sheet"` 宿主作用域包含 `"sheet-content"` 内容表面；
Dialog 的 `"dialog"` 作用域包含以层下标标识的表面。子菜单也包含 `"popup-menu"`，
打开子菜单时应保留已解析的父作用域，或在 `"submenu"` 下查询。
不要假定打开另一层之后，原先唯一的 ID 仍然唯一。


目标缺失或不可见时点击会 panic。禁用控件仍接收原生事件，由控件自己决定是否响应。
可见性结合几何、视口与内容裁剪、目标计算样式，不判断像素遮挡；覆盖层仍会拦截点击。
`click_at(id, point(px(10.), px(10.)), cx)` 可以选择裁剪后可见的部分，不会绕过命中测试。

观察依赖 feature，因此被测制品与生产制品并非逐字节相同。透明包装器不增加布局盒子，
但可见性检查会额外计算一次样式；style/drag 谓词不能依赖调用次数。
GPUI 没有公开未观察祖先的继承绘制透明度，因此无法推断该情况。
实现没有使用 GPUI fork 或 Cargo patch 绕过这些限制。

失败时按具体情况检查注册路径、观察配置、完成帧、键盘焦点、裁剪与覆盖层、异步完成条件。

| 现象 | 优先检查 |
| --- | --- |
| 找不到 `mod common` | 连同 `tests/ui.rs` 复制 `tests/common/mod.rs`，或改用应用自己的窗口设置代码。 |
| `find` 列不出匹配路径 | 检查 `test-support`、控件 ID 或 `.test_support()`，以及首次查询前的 `render_frame`。 |
| 查询结果不唯一 | 用 `within` 定位已有父级，再查询子控件 ID。 |
| `focused()` 提示遗漏绑定，或作用域 `input` 报错 | 在 `.track_focus(&handle)` 前观察元素，输入前点击正确的输入框。 |
| 断言仍读到旧状态 | 完成帧后重新查询快照；若工作已排队，离开 `update_window` 并使用 `wait_for`。 |
| 目标可见却收不到点击 | 检查裁剪和浮层顺序；指针辅助方法使用原生命中测试。 |

## 独立验证绘制结果

值或勾选标志正确，不代表控件正确绘制。GPUI 提供
`HeadlessAppContext::with_platform`、`Window::render_to_image` 和
`HeadlessAppContext::capture_screenshot`，可以生成真实离屏图片。当前锁定的
GPUI {{gpui_pre_version}} 平台 crate 在 macOS 提供 Metal 离屏渲染器，在 Linux 提供 WGPU
离屏渲染器。使用可用的 Metal 或 WGPU 适配器 执行以下命令；Linux 可以使用
Mesa lavapipe 等软件 Vulkan 适配器：

```sh
cargo test -p gpui-kit --features test-support --test rendering --locked
```

该目标设置了 `test = false`，默认 Cargo 命令不会选择它。macOS 和 Linux CI job
都通过必需的独立步骤显式执行 `--test rendering`，同时运行交互与布局测试；
Windows 运行交互与布局测试。这使用 Cargo 的
[显式目标选择](https://doc.rust-lang.org/cargo/commands/cargo-test.html#target-selection)。
目标还使用 `harness = false`，因为 AppKit 必须在主线程初始化；普通 Rust
测试即使指定 `--test-threads=1` 仍运行在工作线程。锁定版本的平台 crate 没有
Windows 离屏渲染器，因此 Windows 明确报告跳过像素验证；macOS 或 Linux 缺少
渲染能力时测试失败，不用假图片替代。

测试向真实 Kit 控件注入两种故障：`checked()` 仍为 true，但勾号资源丢失；
`value()` 仍正确，但输入文字变透明。故障图片必须与正常控件不同，重复绘制正常
Checkbox 的图片必须一致。受控 Checkbox 用例还在明暗主题下验证：鼠标按下时
应用值和画面保持不变，释放时值只更新一次，画面同步更新。
完整测试套件还检查含行内代码的 CJK 文本换行、明暗
主题下焦点线的位置与对比度、指针激活 Button 时没有焦点线、指针与键盘切换时
Menu 只有一个高亮项、List 的键盘选中状态，以及开启或关闭焦点环时 Table 只在
键盘聚焦后显示焦点样式。另一个原生事件测试断开 Checkbox 的状态更新处理器，
验证点击不会凭空产生已勾选结果。

这些测试验证能否发现特定错误，不是完整的基准图片回归测试。应用的视觉回归应在
固定字体、尺寸、主题、焦点和动画状态下，将图片与已审查的预期结果比较。状态与
图片断言能发现不同的故障；两者都不能证明打包应用或完整 IME 行为正确。
可执行示例见
[`crates/kit/tests/rendering.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/tests/rendering.rs)。

## 接入 CI

Kit 仓库在 macOS、Linux 和 Windows 矩阵中运行交互与布局测试。macOS 和 Linux
job 还分别通过 Metal 和 WGPU 运行完整绘制测试套件，失败会使 job 失败。以下是用于 Kit 检出目录的最小 macOS workflow：

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
      - run: cargo test -p gpui-kit --features test-support --test rendering --locked
```

应用仓库需要安装自身的平台依赖，并改为在测试 package 中运行 `cargo test --test ui --locked`。将锁定版本的 Kit 源码放到 manifest 声明的路径，再按照普通原生构建的环境配置增加 Linux 和 Windows job。

仓库测试还覆盖只读与禁用输入、焦点变化、缓存视图、挂载与卸载、多窗口隔离、原生命中测试，以及列表从 1,000 个元素缩减后的清理。大列表用例验证正确性，不是渲染性能基准。

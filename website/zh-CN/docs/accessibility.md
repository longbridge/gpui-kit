---
title: Accessibility
description: 使用 AccessKit 语义与操作构建、测试 GPUI Kit 无障碍界面。
order: -3.4
---

# Accessibility

GPUI 通过 [AccessKit](https://accesskit.dev/) 向平台辅助技术提供无障碍树。一个有用的控件应具有**类型**（role，说明它是什么）、**名称**（供用户辨认）、**状态**（如值、勾选、选中、展开）和**操作**（辅助技术可请求的行为）。稳定的元素 ID 用于保持跨帧身份，并不是用户听到的名称。界面还必须能用键盘操作，并清楚显示焦点；无障碍树属性本身不会实现键盘交互。

应用通常使用 `gpui_kit::component` 中的带样式组件。交互行为由无样式的 `gpui_kit::base` 层提供。两层都通过一个 `gpui-kit` 依赖使用。优先使用标准控件，再考虑自己拼装可交互的 `div`：标准控件已经协调了指针、键盘、焦点、状态与 AccessKit 语义。

需要自定义键盘目标时，先读 [Focus](./focus)：`track_focus`、Tab 顺序和无障碍 role 分别解决交互的不同部分。

把无障碍看成一条贯穿应用的路径，而不是最后才附加的一段说明：

| 步骤 | Profile 示例中的对象 | 需要验证什么 |
| --- | --- | --- |
| 保留状态 | `Entity<InputState>` 持有输入草稿；`Profile::submitted` 持有已保存的值。 | 输入和 Save 确实改变目标模型，重新渲染后仍正确。 |
| 渲染 | `Input`、`Button` 和状态 `div` 读取该状态。 | 可见标签和结果与模型一致。 |
| 语义 | 控件通过 AccessKit 提供 role、名称、相关状态及可用操作。 | 绘制完成后的 snapshot 含有预期属性。 |
| 操作 | 指针、键盘与辅助技术触达同一命令。 | 每种输入方式都能保存；焦点移动可见且顺序可预期。 |

先做好状态和渲染，再检查语义树并操作实际窗口。无头测试中的树属性断言通过，不能证明屏幕阅读器会播报什么，也不能证明焦点环可见。

## 跟着一个完整的保存流程学习

在工作区根目录运行仓库中的 [Profile UI 测试](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/tests/ui.rs)：

```sh
cargo test -p gpui-kit --features test-support --test ui saves_a_profile_through_the_ui -- --exact
```

测试打开无头窗口，输入姓名，激活 **Save**，再检查应用状态与已渲染 GPUI 元素写出的无障碍属性。这样可以沿着一条实际路径，理解模型、可见控件和对应 AccessKit 节点如何连接。在自己的测试 crate 中配置环境的方法见[测试](./test)。

### 1. 给接收输入的控件命名

视图用 `Entity<InputState>` 保留跨帧文字。渲染时同时提供 GPUI 使用的稳定 ID，以及辅助技术读取的易懂名称：

```rust
Input::new(&self.name)
    .id("name")
    .aria_label("Profile name")
    .w(px(240.))
```

ID 与名称回答不同问题：`"name"` 让 GPUI 和测试找到这个元素；`"Profile name"` 告诉用户应该输入什么。示例还在输入框正上方绘制了 **Profile name** 可见文字。仅凭相邻位置不会自动建立无障碍关联，因此输入框仍需自己的名称。输入时 placeholder 会消失，不能很好地替代持续可见的标签。

### 2. 更新同一份模型，再从模型渲染状态

**Save** 按钮的 listener 把当前输入值复制到视图的 `submitted` 字段，并调用 `cx.notify()`。下一次 `render` 根据 `submitted` 计算状态文字，同一个值既用于可见内容，也用于无障碍名称：

```rust
let status = self.submitted.as_ref().map_or_else(
    || SharedString::from("Not saved"),
    |name| SharedString::from(format!("Saved: {name}")),
);

div()
    .id("status")
    .role(Role::Status)
    .test_support()
    .aria_label(status.clone())
    .child(status)
```

这个状态 `div` 不执行命令，只描述操作结果。稳定 ID 加 `Role::Status` 使它成为有语义的节点。`.test_support()` 只在启用相应 feature 时帮助 GPUI Kit 的测试工具找到自定义节点。role 和 label 不会自行修改模型；`cx.notify()` 使新模型值进入下一帧。

### 3. 在绘制完成后断言契约

测试在首次查询前调用 `render_frame`，再与原生元素交互，并重新读取 snapshot：

```rust
window.render_frame(cx);
assert_eq!(window.find("status").role(), Some(Role::Status));
assert_eq!(window.find("status").label(), Some("Not saved"));

window.click("name", cx);
window.input("Ada José", cx);
assert_eq!(window.find("name").label(), Some("Profile name"));
assert_eq!(window.find("name").value(), Some("Ada José"));

window.press("backspace", cx);
assert_eq!(window.find("name").value(), Some("Ada Jos"));
window.click("save", cx);
assert_eq!(window.find("status").label(), Some("Saved: Ada Jos"));
```

仓库里的完整测试还检查输入焦点、状态元素的几何位置，以及 `profile.read(cx).submitted`。断言模型可以防止仅仅更新 label 的错误通过测试。这里的 `window.click` 测试的是指针激活；`backspace` 在指针聚焦输入框后删除带重音的 `é`，验证 Unicode 编辑，并未验证能否通过键盘导航到 **Save**。可以试着删除 `.aria_label("Profile name")` 后重跑测试：label 断言会显示，不能假设旁边的可见文字自动为输入框命名。

要在真实窗口检查键盘路径，把下面这个同一视图的完整版本放入已有的 `examples/hello_world/src/main.rs`，再从仓库根目录运行 `cargo run -p hello_world`。这是本地练习；结束后恢复该示例文件。上面的无头测试没有执行这条键盘路径。

```rust
use gpui_kit::{
    AppContext, Context, Entity, Role, SharedString, Window, WindowOptions,
    component::{button::Button, input::{Input, InputState}},
    div, prelude::*, px,
};

struct Profile {
    name: Entity<InputState>,
    submitted: Option<SharedString>,
}

impl Render for Profile {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.submitted.as_ref().map_or_else(
            || SharedString::from("Not saved"),
            |name| SharedString::from(format!("Saved: {name}")),
        );

        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .child(
                div().flex().flex_col().gap_1().child("Profile name").child(
                    Input::new(&self.name)
                        .id("name")
                        .aria_label("Profile name")
                        .w(px(240.)),
                ),
            )
            .child(Button::new("save").label("Save").on_click(
                cx.listener(|this, _, _, cx| {
                    this.submitted = Some(this.name.read(cx).value());
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .id("status")
                    .role(Role::Status)
                    .aria_label(status.clone())
                    .child(status),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Profile {
                name: cx.new(|cx| InputState::new(window, cx)),
                submitted: None,
            })
        })
        .expect("failed to open window");
    });
}
```

窗口初始显示 **Not saved**。从表单前方开始，用 Tab 到 **Profile name**，确认焦点清晰可见；输入 `Ada José` 并按一次 Backspace，留下 `Ada Jos`。再用 Tab 到 **Save**，确认它的焦点提示，然后按 Enter；可见结果应变为 **Saved: Ada Jos**。**Save** 仍有焦点时，再按 Space 激活一次。用 Shift+Tab 返回输入框，再按 Tab 回到 **Save**，核对反向和正向导航。如果某个激活按键无效，应检查按钮的焦点与按键处理，不能用指针测试通过来替代判断。最后，用目标平台的辅助技术激活 **Save**，检查播报的 role、名称与结果；仅有 status role 不保证动态变化一定被播报。

把练习迁移到自己的表单时，也按这个顺序做：给真正的输入控件设置稳定 ID 和名称，让操作更新保留状态，从该状态渲染结果，再同时断言模型与重新绘制后的语义。为视力正常的用户保留控件旁的可见标签；无障碍名称是额外的契约。字段无效时，以可见文字说明错误，并确认使用辅助技术的人能发现错误和恢复操作。仅改变颜色无法说明原因或纠正方法。

## 从语义控件开始

下列控件会根据自身状态提供相应语义：

| 控件 | 无障碍契约 |
| --- | --- |
| Button、Link | Button 或 Link role、名称及激活操作。应用命令用 Button；外部目标用 Link。 |
| Checkbox、Switch、Toggle、Radio | 相应 role 及勾选或切换状态。Checkbox 还能报告混合状态。 |
| Input | 根据内容类型选择的输入 role、名称、非敏感值；未禁用时提供 `SetValue` 操作。遮罩和密码值不会暴露。 |
| Select | ComboBox role、名称、已提交的值、展开状态及无障碍激活路径。 |
| Tab、tab list | Tab 与 TabList role；Tab 报告选中状态，也可报告它在集合中的位置。 |
| Slider、Progress | 数值和范围；Slider 处理无障碍 Increment、Decrement 操作。不确定进度的 Progress 不报告数值。 |
| Table | Table、row、header、cell role、索引及可选的计数。应给 Table 根节点命名。 |

带文字的按钮默认用可见文字作为无障碍名称。纯图标按钮应显式命名：

```rust
use gpui_kit::component::{IconName, button::Button};

Button::new("search-documents")
    .icon(IconName::Search)
    .accessibility_label("Search documents")
    .tooltip("Search documents")
    .on_click(|_, window, cx| {
        // Invoke the same application command used by its keyboard route.
    })
```

`accessibility_label` 是辅助技术读取的控件名称；tooltip 是另一种提示，不能代替名称。名称应明确描述操作，并随操作变化而更新。不要假设自定义容器内的任意文字会自动成为其无障碍名称。可见表单标签与相邻输入框也不会仅凭布局位置产生标签关系：用 `Input::aria_label(...)` 命名真正的输入框，或使用已经实现该关系的组件。Input 可以把 placeholder 作为名称后备，但显式 label 更清楚；自动生成的遮罩 placeholder 不会被当成名称。

## 身份与 role

元素同时拥有 [ElementId](./element_id) 和非空无障碍 role 时，GPUI 才会把它纳入无障碍树。全局身份还包含祖先的 ID。跨帧保持 ID 稳定；重复项目使用领域数据键生成 ID，避免排序后被辅助技术误认为大量节点被移除又重建。单独的 `id` 并不是 role；没有 role 的 `div` 只是布局容器，不是可播报的控件。仅有 role 也不会让控件自动获得焦点或响应操作。

语义状态消息可以同时提供两者：

```rust
use gpui_kit::*;

div()
    .id("save-status")
    .role(Role::Status)
    .test_support()
    .aria_label("Saved")
    .child("Saved")
```

显式 label 是播报名称。启用 `test-support` 时，`.test_support()` 让后面的集成测试找到这个自定义 `div`；它不增加布局容器，在普通构建中不起作用。`Role::GenericContainer` 会从无障碍树中被过滤；有意义的节点应使用实际 role。`accessibility_id(...)` 是另外一个供平台自动化使用的作者标识：Windows 可映射为 UIA `AutomationId`，macOS 可映射为 `AXIdentifier`；Linux AT-SPI 是否支持取决于部署的 adapter。它不能代替 GPUI 的 `.id(...)`，也不能代替易懂的名称。

## 名称、状态与关系

对带 ID 的 `div`，GPUI 的 `StatefulInteractiveElement` 提供 `.role(...)`、`.aria_label(...)`、`.aria_description(...)`、`.aria_selected(...)`、`.aria_expanded(...)`、`.aria_toggled(...)`、`.aria_value(...)`、`.aria_numeric_value(...)`，以及范围与集合属性。数值控件还可报告最小值、最大值、步长和方向；标题可报告层级；列表和表格项目可报告位置与总数。应从绘制界面的同一份模型更新这些值；持有模型的 View 在状态变化后通过 [Context](./context) 通知 GPUI。description 是名称的补充，不能代替名称。`.aria_keyshortcuts(...)` 只负责向辅助技术说明快捷键；实际按键仍须通过 GPUI keybinding 注册。

当前 GPUI `div` API 没有通用的 `.aria_disabled(...)` 构建方法。Button、Checkbox 等 Base 控件在禁用时会阻止焦点与激活，但这不保证每个节点都有原生 disabled 属性。应同时核对树中可读取的状态与实际禁用行为。同样，`.track_focus(...)` 建立焦点路径并声明无障碍 Focus 操作；只有 role 或 `.focusable()` 并不会实现有用的键盘命令。

元素树建立父子关系。复合控件可以把键盘焦点留在父节点，再给当前子节点设置 `.aria_active_descendant()`。GPUI 只有在祖先确实持有焦点时才应用该子节点标记。子节点还需要自己的 ID 与 role。这属于专门的复合控件行为；适用时优先使用内置 Select、menu 或 list。

不要根据 `aria_` 前缀推断存在网页中的 `aria-labelledby`、`aria-describedby` 或 `aria-controls` 通用构建方法：当前 GPUI `div` API 没有这些方法。Table 即使有可见 caption，也应在 Table 根节点设置 `.accessibility_label(...)`；caption 容器不会自动给 Table 命名。

## 无障碍操作与键盘输入

AccessKit action 与由快捷键或菜单派发的 GPUI [Action](./action) 是两套概念。GPUI 将前者暴露为 `AccessibleAction`。带 ID 的 `div` 可以用 `.on_a11y_action(action, handler)` 注册一项操作；handler 接收可选的 `ActionData`、`&mut Window` 和 `&mut App`。`.on_click(...)` 已经声明无障碍 Click 激活，并把它路由给点击 handler；第二个 Click handler 可能重复执行命令。例如自定义 Slider 除了指针和键盘操作，还应支持 Increment、Decrement，并在模型改变后更新无障碍数值。GPUI Kit 的 Slider 已实现这些行为。

交互承诺应一致：可见文字、无障碍名称、快捷键、指针、键盘和辅助技术操作都指向同一命令。带 hitbox、能点击的绘制图形仍须另行实现语义与键盘操作。Dialog 或 Sheet 关闭后应把焦点还给触发控件；焦点应可见，顺序应符合任务流程。

对上面的 Profile 流程，使用可运行的窗口练习，把键盘验证与指针测试分开。只有 `submitted` 改变且 `cx.notify()` 触发重新渲染后，可见结果才会变化。复合控件还应检查其文档约定的方向键和 Escape 行为。在发布目标平台上完成这些检查；指针测试通过或声明了快捷键，都不能证明键盘路径可用。

### 从零构建可交互的自定义控件

前面的 `EventSurface` 练习给绘制的图形增加了 Status 节点，但图形仍不能脱离指针操作。若外观能由普通元素组合，先走组合路径。把现有 `examples/hello_world/src/main.rs` 暂时替换为下面的完整示例，在仓库根目录运行 `cargo run -p hello_world`；练习结束后恢复该示例文件。

```rust
use gpui_kit::{
    *,
    component::ThemeStyled as _,
    prelude::*,
};

struct CounterControl {
    focus: FocusHandle,
    activations: usize,
}

impl CounterControl {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            activations: 0,
        }
    }
}

impl Render for CounterControl {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let status = format!("Activations: {}", self.activations);

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .child(
                div()
                    .id("activate-counter")
                    .role(Role::Button)
                    .aria_label("Activate counter")
                    .track_focus(&self.focus)
                    .p_3()
                    .border_1()
                    .when(focused, |control| control.focus_ring_style(window, cx))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.activations += 1;
                        cx.notify();
                    }))
                    .child("Activate counter"),
            )
            .child(
                div()
                    .id("activation-status")
                    .role(Role::Status)
                    .aria_label(status.clone())
                    .child(status),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(CounterControl::new)
        })
        .expect("failed to open window");
    });
}
```

保留的 `FocusHandle` 跨渲染周期存在。其 `.tab_stop(true)` 把控件加入 Tab 顺序，`.track_focus(...)` 将句柄绑定到当前渲染的节点。稳定 ID 与 `Role::Button` 暴露控件节点，`.aria_label(...)` 为操作命名。只有该句柄获得焦点时才绘制焦点环。唯一的 `.on_click(...)` listener 更新保留的计数并调用 `cx.notify()`；GPUI 把主指针点击、焦点节点上的 Enter/Space 激活，以及 AccessKit Click 都交给这个 listener。不要再给同一节点注册第二个 `AccessibleAction::Click`，否则命令可能执行两次。

在实际窗口逐条验收。鼠标点击后，**Activations: 0** 应变为 **Activations: 1**。从控件外用 Tab 进入，确认焦点环可见；按 Enter 后应为 **Activations: 2**，按 Space 后应为 **Activations: 3**。再用 Tab 离开、Shift+Tab 返回，确认它仍在导航顺序中。使用目标平台的无障碍检查器，找到名称为 **Activate counter** 的 Button，以及名称与可见计数一致的 Status。再用辅助技术执行 Button 的 Click 操作，确认计数恰好增加一次。无头 snapshot 可以检查 role、名称和更新后的模型，但不能证明焦点环可见、平台适配器暴露了节点，或屏幕阅读器播报了变化中的 Status。

若 Tab 跳过控件，同时检查保留句柄上的 `.tab_stop(true)` 和当前渲染节点上的 `.track_focus(...)`。若 role 或名称缺失，检查同一节点是否设置稳定 ID、role 与 label。若一次激活增加两次，寻找是否有第二个 Click 或按键 handler 重复调用同一命令。此练习展示一个类似按钮的控件；真实应用命令优先使用标准 Button，它还处理组件的其他状态与样式。

## 自定义 `Element`

如果无法通过组合 `div()` 完成，底层 `Element` 提供明确的 hook。接着做 [Element 章的 EventSurface 练习](./element)：它已经在 `examples/hello_world/src/main.rs` 中实现了 `IntoElement`、布局、prepaint 和 paint。对这个现成示例做三处修改：

1. 在 `Render for SurfaceDemo` 中，与 `owner`、`color` 一起把当前计数文字传入 `EventSurface`：`status: format!("Pointer presses: {}", self.presses),`。
2. 给 `EventSurface` 结构体增加 `status: String,` 字段。
3. 替换它的 `id()` 方法，并在 `impl Element for EventSurface` 中加入两个无障碍方法：

```rust
fn id(&self) -> Option<ElementId> { Some("pointer-status".into()) }

fn a11y_role(&self) -> Option<Role> { Some(Role::Status) }

fn write_a11y_info(&self, node: &mut accesskit::Node) {
    node.set_label(self.status.clone());
}
```

完整的 EventSurface 示例已使用 `use gpui_kit::*;` 导入 `ElementId`、`Role` 和 `accesskit`。再次运行 `cargo run -p hello_world`。启用辅助技术或平台无障碍检查器，在矩形位置找到名为 **Pointer presses: 0** 的 **Status** 节点。矩形内按一次鼠标左键，可见计数与重新渲染后的节点名称都应变为 **Pointer presses: 1**；稳定 ID 让它在两帧之间保持同一身份。检查器能显示树属性，只有用屏幕阅读器实际操作才能确认变化是否播报。无障碍树只在辅助技术启用后构建；没有客户端连接时看不到树，并不能单凭这一点认定方法失效。

这只是限定范围的语义练习。`EventSurface` 仍只响应指针输入，`Role::Status` 用来描述计数，并不会让矩形成为可用键盘或辅助技术激活的控件。真正的命令应优先使用标准 Button；若确需自定义，还要分别实现跟踪焦点、键盘激活、合适的控件 role 及无障碍操作。只有元素同时有 ID 和 role 时，GPUI 才调用 `write_a11y_info`。GPUI Kit 的 `window.find(...)` 测试工具观察已注册控件和 `div().test_support()`，不能指望它找到这个原始 `Element` 的 `pointer-status`。`a11y_synthetic_children(...)` 可在 prepaint 后添加 AccessKit 子节点，例如自定义编辑器的文字片段。`A11ySubtreeBuilder::synthetic_node_id(key)` 根据父节点和稳定 key 派生子节点 ID；`push_child(...)` 把节点挂在父节点下。同一父节点的合成子节点 key 必须唯一。这是高级路径：实现者还须负责命中测试、事件派发、焦点、键盘处理和无障碍操作。

## 测试无障碍契约

UI 集成测试启用 GPUI Kit 的 `test-support` feature，并导入 `gpui_kit::test::TestWindowExt`；观察自定义 `div` 时还要导入 `TestSupportExt`。在 `window.render_frame(cx)` 后查询**已绘制**的元素。`ElementSnapshot` 提供 `role()`、`label()`、`value()`、`focused()`、`checked()`、`indeterminate()`、`selected()` 和 `expanded()`。每次交互后重新查询，因为 snapshot 只描述一个已完成的帧。上面的可运行 Profile 测试就是断言示例：它检查 role、名称、值、焦点、几何位置和保存后的模型值。

状态读取结果为 `None` 表示属性不可用，并不表示 `false`。特别是 `.disabled()` 只有在节点暴露 disabled 标志时才返回 `Some(true)`；测试禁用行为时应尝试激活，并确认结果未改变。`ElementSnapshot::value()` 读取的是字符串无障碍值，不是 Slider 的数值或绘制的文字。遮罩和密码输入有意不暴露无障碍值。当前 Input 实现只要未禁用就注册 `SetValue`，包括只读模式；handler 使用程序化替换路径。如果只读数据必须受到保护，应核实所需行为，不要假定 `readonly(true)` 能阻止这条无障碍写入路径。测试配置、帧与焦点细节见[测试](./test)。

无头 snapshot 读取 GPUI 元素生成的 AccessKit 属性；它不会检查最终的平台适配器树、屏幕阅读器实际播报或像素。应在每个目标平台用辅助技术检查播报顺序、焦点移动、编辑和操作。尤其要确认变化中的 `Role::Status` 确实被播报，不能仅凭 role 就认定会实时播报。平台 adapter 行为可能不同，[WebAssembly 支持](./webassembly.md)也需要单独验证。再配合视觉检查：焦点对比度、文字可读性、目标大小、减少动态效果，以及不能只靠颜色传达的信息；参见 [Design Guides](./design-guides)。

### 一轮可执行的验收

在计划发布的每个平台上按以下顺序检查。记录操作系统、辅助技术、应用构建版本和实际观察结果；测试中的 AccessKit 属性只能证明 GPUI 树的内容，不能替代各平台桥接层的验证。

1. **只用键盘：**从表单前方开始，Tab 到输入框和 Save，Shift+Tab 返回，逐站检查可见焦点。输入、编辑，再用 Enter 或 Space 激活 Save；确认可见结果和保存的模型。复合控件还要测试其约定的方向键、Escape 和 Tab 离开路径。
2. **使用辅助技术：**找到同一输入框和按钮，确认播报的 role、名称、适合披露时的输入值，以及保存后的状态。通过辅助技术的控件操作激活按钮。在实际应用中检查状态是否被播报；仅有 `Role::Status` 不能证明会播报。
3. **状态变化：**测试空值或无效输入、禁用与启用切换，以及流程中出现的浮层。确认用户能找到错误、禁用命令不能执行、浮层关闭后焦点回到合理位置。
4. **视觉呈现：**放大文字或界面，检查焦点对比度与裁切；在支持时减少动态效果；确认成功、错误、选中状态不只靠颜色区分。

### 检查失败时从哪里入手

| 现象 | 先检查 |
| --- | --- |
| 绘制后的 snapshot 找不到自定义节点。 | 是否有稳定 `.id(...)` 和有意义的 `.role(...)`？是否运行了 `window.render_frame(cx)`？如果用 GPUI Kit 的测试工具查找自定义 `div`，是否添加 `.test_support()` 并启用 `test-support`？ |
| role 存在，但名称缺失或不正确。 | 给控件本身命名（`Input::aria_label`、`Button::accessibility_label` 或 `div().aria_label`）；相邻文字和 tooltip 不保证建立名称关系。 |
| 状态文字变化，但 snapshot 仍是旧值。 | 交互后重新查询；检查所有者是否更新保留状态并调用 `cx.notify()`。已保存的 `ElementSnapshot` 只描述一帧。 |
| 快捷键能被播报，按下却没有反应。 | `.aria_keyshortcuts(...)` 只报告快捷键。注册 GPUI 按键绑定，并把 action handler 接到预期焦点路径；参见 [Action](./action)。 |
| 指针能激活，键盘或辅助技术却无法激活。 | 检查 Tab 停靠点、跟踪焦点、key context、action handler 和无障碍操作。role 加 hitbox 本身不提供这些路径；考虑使用内置控件。 |
| 无头断言通过，屏幕阅读器表现却不同。 | 在目标平台复现并检查平台适配器的结果。无头测试工具不验证播报、焦点视觉或适配器行为。 |

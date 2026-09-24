---
title: Accessibility
description: 使用 AccessKit 语义与操作构建、测试 GPUI Kit 无障碍界面。
order: -3.4
---

# Accessibility

GPUI 通过 AccessKit 向平台辅助技术提供无障碍树。一个有用的节点应说明控件的**类型**（role）、**名称与稳定身份**、**当前状态**（值、选中、勾选、展开）以及**可执行的操作**。界面还必须能用键盘操作，并清楚显示焦点；无障碍节点本身不会自动实现键盘交互。

应用通常使用 `gpui_kit::component` 中的带样式组件。交互行为由无样式的 `gpui_kit::base` 层提供。两层都通过一个 `gpui-kit` 依赖使用。优先使用标准控件，再考虑自己拼装可交互的 `div`：标准控件已经协调了指针、键盘、焦点、状态与 AccessKit 语义。

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
        // 调用与键盘路径相同的应用命令。
    })
```

`accessibility_label` 是辅助技术读取的控件名称；tooltip 是另一种提示，不能代替名称。名称应明确描述操作，并随操作变化而更新。不要假设自定义容器内的任意文字会自动成为其无障碍名称。可见表单标签与相邻输入框也不会仅凭布局位置产生标签关系：用 `Input::aria_label(...)` 命名真正的输入框，或使用已经实现该关系的组件。Input 可以把 placeholder 作为名称后备，但显式 label 更清楚；自动生成的遮罩 placeholder 不会被当成名称。

## 身份与 role

元素同时拥有 [ElementId](./element_id) 和非空无障碍 role 时，GPUI 才会把它纳入无障碍树。全局身份还包含祖先的 ID。跨帧保持 ID 稳定；重复项目使用领域数据键生成 ID，避免排序后被辅助技术误认为大量节点被移除又重建。单独的 `id` 并不是 role；没有 role 的 `div` 只是布局容器，不是可播报的控件。

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

## 自定义 `Element`

如果无法通过组合 `div()` 完成，底层 `Element` 提供明确的 hook：

```rust
use gpui_kit::{Role, accesskit::Node};

fn a11y_role(&self) -> Option<Role> { Some(Role::Status) }

fn write_a11y_info(&self, node: &mut Node) {
    node.set_label("Download complete");
}
```

这些方法应写在 `impl Element for ...` 中；该实现还必须提供稳定的 `id()` 以及布局、prepaint、paint 方法，见 [Element](./element)。只有元素同时有 ID 和 role 时，GPUI 才调用 `write_a11y_info`。`a11y_synthetic_children(...)` 可在 prepaint 后添加 AccessKit 子节点，例如自定义编辑器的文字片段。`A11ySubtreeBuilder::synthetic_node_id(key)` 根据父节点和稳定 key 派生子节点 ID；`push_child(...)` 把节点挂在父节点下。同一父节点的合成子节点 key 必须唯一。这是高级路径：实现者还须负责命中测试、事件派发、焦点、键盘处理和无障碍操作。

## 测试无障碍契约

UI 集成测试启用 GPUI Kit 的 `test-support` feature，并导入 `gpui_kit::test::TestWindowExt`。在 `window.render_frame(cx)` 后查询**已绘制**的原生元素。`ElementSnapshot` 提供 `role()`、`label()`、`value()`、`focused()`、`checked()`、`indeterminate()`、`selected()` 和 `expanded()`。每次交互后重新查询，因为 snapshot 只描述一个已完成的帧：

```rust
window.render_frame(cx);
assert_eq!(window.find("save-status").role(), Some(Role::Status));
assert_eq!(window.find("save-status").label(), Some("Saved"));

window.click("save", cx);
assert_eq!(window.find("save-status").label(), Some("Saved: Ada"));
```

此例假设应用的 Save handler 更新了状态 label，且 status `div` 已如上使用 `.test_support()`。还应断言真实的应用结果，并测试键盘输入。状态读取结果为 `None` 表示原生属性不可用，并不表示 `false`。特别是 `.disabled()` 只有在节点暴露 disabled 标志时才返回 `Some(true)`；测试禁用行为时应尝试激活，并确认结果未改变。`ElementSnapshot::value()` 读取的是字符串无障碍值，不是 Slider 的数值或绘制的文字。遮罩和密码输入有意不暴露无障碍值。当前 Input 实现只要未禁用就注册 `SetValue`，包括只读模式；handler 使用程序化替换路径。如果只读数据必须受到保护，应核实所需行为，不要假定 `readonly(true)` 能阻止这条无障碍写入路径。完整可运行示例和焦点、帧、平台细节见 [Testing](./test)。

无头 snapshot 能验证原生树暴露的属性及真实交互，不能代替打包应用中的屏幕阅读器测试或像素检查。应在每个目标平台用辅助技术检查播报顺序、焦点移动、编辑和操作。平台 adapter 行为可能不同，web 支持也需要单独验证。再配合视觉检查：焦点对比度、文字可读性、目标大小、减少动态效果，以及不能只靠颜色传达的信息；参见 [Design Guides](./design-guides)。

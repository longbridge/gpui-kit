---
title: Focus
description: 从零学习 GPUI 的焦点句柄、键盘导航、焦点事件与焦点约束。
order: -2.4
---

# Focus

**Focus（焦点）**决定一个 [Window](./window) 中谁接收键盘输入。GPUI 沿当前获得 Focus 的元素及其祖先构成的路径路由 [Action 和按键绑定](./action)。鼠标按下可以移动 Focus，但画出控件或给它 `ElementId` 并不会自动建立焦点目标。本章以 `gpui-kit` 使用的 `gpui-pre` {{gpui_pre_version}} API 为准；`gpui-pre` 是 GPUI 快照的发布与版本同步名称，不代表另一套渲染引擎。

## 先运行现有示例

在仓库根目录运行：

```sh
cargo run -p focus_trap
```

源码是 [`examples/focus_trap/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/focus_trap/src/main.rs)。点击外侧按钮后按 Tab，Focus 按窗口中的普通顺序移动。点击任一编号区域里的按钮，再按 Tab 或 Shift+Tab，Focus 会在该区域的按钮间循环。示例在每个容器上调用 `.focus_trap(id, &handle)`；按钮是拥有各自焦点句柄的 GPUI Kit `Button`。它演示的是**进入区域之后的约束**，不是完整弹窗生命周期。

新应用应像示例一样，先调用 `gpui_kit::init(cx)`，再创建窗口。`gpui_kit::open_window` 提供 Base `Root`；普通 Tab、Shift+Tab 导航和 Kit 的焦点约束处理都依赖它。应用初始化步骤见 [Getting Started](./getting-started)。

## 区分四个操作

| 操作 | 实际作用 | 不会自动完成的事 |
| --- | --- | --- |
| `cx.focus_handle()` | 创建供 owner 长期保存的稳定焦点目标。 | 不会注册已渲染元素，也不会移动 Focus。默认 `tab_stop` 为 `false`。 |
| `.track_focus(&handle)` | 把句柄登记到已渲染的交互元素，建立 Focus 路由与聚焦样式所需的节点。 | 不会移动 Focus，也不会自动让句柄进入 Tab 顺序。 |
| `handle.focus(window, cx)` 或 `window.focus(&handle, cx)` | 将窗口的当前 Focus 移到这个句柄。 | 不会创建已渲染节点或键盘 handler。 |
| `cx.focus_handle().tab_stop(true)` | 使已登记的句柄可被 Tab 导航访问。 | 不会立即聚焦它。 |

句柄应归长期存在的 owner 所有，通常是 [`Entity<T>`](./entity)。`Element` 每帧重新构建，应在每次 [`Render::render`](./render) 时重新附上**同一个**句柄。若每次渲染都新建句柄，焦点身份和 Tab 行为会在用户操作时变化。无状态 Kit 组件可用 `window.use_keyed_state(...)` 保留句柄；Kit 的 `Button` 就采用该方式。`ElementId` 可以作为保留状态的键，但它本身不是 Focus 目标。

### 最小可聚焦 View

以下 View 可以放进 [Getting Started](./getting-started) 展示的窗口 builder。它创建一个 Tab stop，并显示自己是否拥有精确 Focus：

```rust
use gpui_kit::*;

struct FocusPanel {
    focus_handle: FocusHandle,
}

impl FocusPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle().tab_stop(true),
        }
    }
}

impl Focusable for FocusPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FocusPanel {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let label = if self.focus_handle.is_focused(window) {
            "Focused"
        } else {
            "Press Tab or click here"
        };

        div()
            .track_focus(&self.focus_handle)
            .p_4()
            .child(label)
    }
}
```

在窗口 builder 中用 `cx.new(FocusPanel::new)` 创建它。`Focusable` 让持有 Entity 的代码取得句柄，**不会**自动调用 `.track_focus(...)`。一个 View 即使实现了 `Focusable`，若渲染树没有登记对应元素，键盘路由仍无法抵达它。需要从回调主动移动 Focus 时，调用 `self.focus_handle.focus(window, cx)`；应在打开或进入区域时调用，不要每次 `render` 都无条件调用，否则可能夺走子控件或其他控件的 Focus。

## 鼠标、Tab 和键盘命令

对调用了 `.track_focus(&handle)` 的元素，GPUI 默认在鼠标按下且命中它时聚焦该句柄。若嵌套子元素也登记了句柄，子元素可先获得 Focus，并阻止祖先默认转移；自定义鼠标处理若要自行控制 Focus，可调用 `window.prevent_default()`。只修改应用状态的鼠标回调，并不能替代键盘交互。

Tab 与 Shift+Tab 访问最近一次渲染帧登记的 **Tab stop**。默认 `FocusHandle` 不在其中。希望用户通过 Tab 访问时，在构造句柄时设置 `.tab_stop(true)`。GPUI 还提供 `FocusHandle::tab_index(n)` 以指定顺序；普通表单优先采用渲染顺序。向 `.track_focus(...)` 传入句柄时，Tab 配置要设在**句柄**上；元素另有的 `.tab_index(...)` 或 `.tab_stop(...)` 不会修改该句柄的设置。

Focus 到达已登记元素后，其 dispatch path 决定哪些 `key_context`、`on_action` 和按键绑定可用。例如，Editor 或其子元素拥有 Focus，且 Save handler 位于这条路径上，Editor 的 Save 绑定才可到达；兄弟元素的 handler 不在路径上。完整命令示例见 [Action](./action) 与 [KeyBinding](./keybinding)。自定义控件还需要可见的 Focus 样式，以及符合预期的键盘激活或导航行为；`track_focus` 只提供路由目标。语义和测试限制见 [Accessibility](./accessibility)。

## 精确 Focus 与区域内 Focus

只有句柄本身聚焦才算活动状态时，使用 `handle.is_focused(window)`。句柄或其已登记子元素聚焦都应算活动状态时，使用 `handle.contains_focused(window, cx)`；例如，面板中的文本输入框获得 Focus，面板仍保持活动外观。包含关系来自**最近一次渲染的 dispatch tree**；两个句柄必须附在预期的父子元素上。仅有 Entity 的所有权关系无法建立它。

```rust
let panel_itself = self.focus_handle.is_focused(window);
let panel_or_child = self.focus_handle.contains_focused(window, cx);
```

`window.focused(cx)` 返回当前窗口焦点目标的 `Option<FocusHandle>`，适合诊断，也可在打开弹层前保存之前的目标。返回的句柄之后可能仍在，但对应元素已不再渲染；恢复之前要检查 UI 生命周期。

## 订阅焦点变化，而不是在 `render` 中反复注册

`Context<T>` 提供 `on_focus`、`on_blur`、`on_focus_in` 和 `on_focus_out`。在构造 owner 时注册，并将返回的 [`Subscription`](./event) 保存在 owner 中。丢弃订阅即取消注册。前两个只关注指定句柄本身；`in`/`out` 也考虑已登记的子元素。

```rust
struct SearchPanel {
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
    active: bool,
}

impl SearchPanel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let entered = cx.on_focus_in(&focus_handle, window, |this, _, cx| {
            this.active = true;
            cx.notify();
        });
        let left = cx.on_focus_out(&focus_handle, window, |this, _, _, cx| {
            this.active = false;
            cx.notify();
        });

        Self {
            focus_handle,
            _subscriptions: vec![entered, left],
            active: false,
        }
    }
}
```

在面板渲染根元素上调用 `.track_focus(&self.focus_handle)`。只有当其他状态需要随 Focus 变化时才需要订阅；简单的聚焦样式通常可以在渲染时读取 `is_focused`。若每次 `render` 都注册 listener，会累积重复回调。

## 在弹层中约束并恢复 Focus

GPUI Kit 为交互容器提供 `.focus_trap(id, &container_handle)`。Base `Root` 处理 Tab 或 Shift+Tab 时，先检查当前 Focus 是否处在该容器内；若普通导航会离开，就寻找容器内的其他 Tab stop。`id` 要稳定，句柄要跨渲染保留，容器内要有可用的子 Tab stop。

```rust
div()
    .child(first_button)
    .child(second_button)
    .focus_trap("settings-dialog", &self.dialog_focus_handle)
```

**单独设置 trap 不会打开模态窗口、自动进入第一个控件、阻止鼠标聚焦外部控件，也不会在关闭后恢复 Focus。**完整模态流程应包含：

1. 用户打开弹层时保存 `window.focused(cx)`，由弹层 owner 保存此前的句柄。
2. 渲染弹层，在内部可用控件出现时把 Focus 移到它。明确选择初始目标，不要期待 trap 自动完成这一步。
3. 关闭时移除弹层，只在先前目标仍是合适的已渲染目标时恢复它。若目标已消失，选择当前存在的后备目标，例如打开弹层的按钮。协调更新和恢复时机，避免关闭中的弹层再次夺回 Focus。
4. 验证 Tab 的两个方向、Escape 或显式关闭、鼠标点击、禁用控件、嵌套弹层，以及原先目标已消失的情况。

GPUI Kit 的 `Dialog` 和 `Sheet` 自带其模态焦点行为；普通模态界面优先使用它们。手动 trap 适用于你明确负责进入、关闭和恢复生命周期的自定义容器。容器上的 `on_focus_out` listener 可用于观察 Focus 离开，但不能代替完整模态流程。

## 验证与排错

先运行 `focus_trap` 示例，观察鼠标进入以及两个方向的 Tab 行为。应用测试应渲染真实 View，点击目标控件，发送 Tab 或命令按键，并断言 owner 最终状态。[Testing](./test) 介绍 Kit 的 UI 集成测试辅助方法；焦点范围需要明确的已登记句柄，检查结果才可靠。

| 症状 | 检查 |
| --- | --- |
| Tab 跳过自定义 View | 保留的句柄设置了 `.tab_stop(true)`，当前帧的元素调用了 `.track_focus(&handle)`。 |
| 点击子控件却聚焦父容器 | 子控件有自己的已登记句柄；检查调用 `prevent_default` 的鼠标处理。 |
| 快捷键只在点击后有效 | 预期句柄已聚焦、`key_context` 匹配、handler 位于聚焦 dispatch path。 |
| 子控件聚焦时面板显得不活动 | 使用 `contains_focused`，并确认子元素在面板的已登记元素内部。 |
| Tab 离开自定义 trap | Focus 已先进入 trap；容器通过 Kit 的 Base `Root` 渲染；子控件是真正的 Tab stop。 |
| 关闭弹层后 Focus 消失 | 保存先前目标，并在关闭后恢复当前仍存在的已渲染目标。 |

相关章节：[Window](./window)、[Action](./action)、[KeyBinding](./keybinding)、[Accessibility](./accessibility) 和 [Testing](./test)。

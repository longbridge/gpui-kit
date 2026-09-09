---
title: Focus Trap
description: 将键盘焦点限制在指定容器内的工具元素。
---

# Focus Trap

Focus Trap 是一个用于将键盘焦点限制在特定容器内的工具能力，可防止用户通过 Tab 键把焦点移出当前区域。它对对话框、侧边面板和自定义覆盖层的可访问性非常重要。

**注意：** [Dialog](/zh-CN/component/dialog) 和 [Sheet](/zh-CN/component/sheet) 已内置 focus trap。只有在构建自定义类模态组件时，才需要手动使用 `focus_trap()`。

## 导入

```rust
use gpui_kit::component::FocusTrapElement;
```

## 用法

### 基础 Focus Trap

```rust
let container_handle = cx.focus_handle();

v_flex()
    .child(Button::new("btn1").label("Button 1"))
    .child(Button::new("btn2").label("Button 2"))
    .child(Button::new("btn3").label("Button 3"))
    .focus_trap("trap1", &container_handle)
// Pressing Tab will cycle: btn1 -> btn2 -> btn3 -> btn1
// Focus will not escape to elements outside this container
```

### 多个 Focus Trap

你可以在同一个应用中放置多个彼此独立的 focus trap 区域：

```rust
let trap1_handle = cx.focus_handle();
let trap2_handle = cx.focus_handle();

v_flex()
    .gap_4()
    .child(
        h_flex()
            .gap_2()
            .child(Button::new("trap1-1").label("Area 1 - Button 1"))
            .child(Button::new("trap1-2").label("Area 1 - Button 2"))
            .child(Button::new("trap1-3").label("Area 1 - Button 3"))
            .focus_trap("trap1", &trap1_handle)
    )
    .child(
        h_flex()
            .gap_2()
            .child(Button::new("trap2-1").label("Area 2 - Button 1"))
            .child(Button::new("trap2-2").label("Area 2 - Button 2"))
            .focus_trap("trap2", &trap2_handle)
    )
```

### 与 Dialog 配合

[Dialog] 已自动内置 focus trap，无需手动添加：

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Settings")
        .child(
            v_flex()
                .gap_3()
                .child(Button::new("save").label("Save"))
                .child(Button::new("cancel").label("Cancel"))
                .child(Button::new("reset").label("Reset"))
        )
})
```

### 与 Sheet 配合

[Sheet] 也已自动内置 focus trap：

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Filter Options")
        .child(
            v_flex()
                .gap_2()
                .child(Checkbox::new("option1").label("Option 1"))
                .child(Checkbox::new("option2").label("Option 2"))
                .child(Button::new("apply").label("Apply Filters"))
        )
})
```

## 工作原理

Focus trap 系统主要由三部分组成：

1. **FocusTrapContainer**：包装容器并将其注册为焦点陷阱区域。
2. **FocusTrapManager**：全局状态管理器，用于跟踪当前所有活跃的 focus trap。
3. **Root Integration**：由 [Root] 视图拦截 Tab/Shift-Tab 事件，并执行焦点循环。

当用户按下 Tab 或 Shift-Tab 时：

1. [Root] 会判断当前焦点是否位于某个 focus trap 中。
2. 如果是，则只计算该 trap 内部的下一个可聚焦元素。
3. 当焦点即将离开 trap 时，会循环回到开头或末尾。
4. 这样就能阻止焦点逸出当前容器。

### 已内置 Focus Trap 的组件

以下组件已经内置 focus trap，不需要手动调用：

- **[Dialog]**
- **[Sheet]**

## API 参考

- [FocusTrapElement](https://docs.rs/gpui-component/latest/gpui_component/trait.FocusTrapElement.html)
- [FocusTrapContainer](https://docs.rs/gpui-component/latest/gpui_component/struct.FocusTrapContainer.html)

## 示例

### 自定义模态框

```rust
struct CustomModal {
    container_handle: FocusHandle,
}

impl CustomModal {
    fn new(cx: &mut App) -> Self {
        Self {
            container_handle: cx.focus_handle(),
        }
    }
}

impl Render for CustomModal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .gap_4()
                    .p_6()
                    .bg(cx.theme().background)
                    .rounded(cx.theme().radius_lg)
                    .shadow_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child("This is a modal dialog")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("ok").primary().label("OK"))
                            .child(Button::new("cancel").label("Cancel"))
                    )
                    .focus_trap("modal", &self.container_handle)
            )
    }
}
```

### 嵌套 Focus Trap

当多个 trap 嵌套时，最内层 trap 优先：

```rust
let outer_handle = cx.focus_handle();
let inner_handle = cx.focus_handle();

div()
    .child(
        v_flex()
            .gap_4()
            .p_4()
            .border_1()
            .border_color(cx.theme().border)
            .child(Button::new("outer-1").label("Outer Button 1"))
            .child(
                h_flex()
                    .gap_2()
                    .p_4()
                    .bg(cx.theme().accent.opacity(0.1))
                    .child(Button::new("inner-1").label("Inner Button 1"))
                    .child(Button::new("inner-2").label("Inner Button 2"))
                    .focus_trap("inner", &inner_handle)
            )
            .child(Button::new("outer-2").label("Outer Button 2"))
            .focus_trap("outer", &outer_handle)
    )
```

### 条件启用 Focus Trap

```rust
struct ModalView {
    is_modal: bool,
    container_handle: FocusHandle,
}

impl Render for ModalView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = v_flex()
            .gap_2()
            .child(Button::new("btn1").label("Button 1"))
            .child(Button::new("btn2").label("Button 2"))
            .child(Button::new("btn3").label("Button 3"));

        if self.is_modal {
            content.focus_trap("conditional", &self.container_handle)
                .into_any_element()
        } else {
            content.into_any_element()
        }
    }
}
```

## 可访问性说明

- Focus trap 对模态对话框和覆盖层满足 WCAG 要求非常关键。
- 始终要提供关闭方式，例如 ESC、关闭按钮或取消按钮。
- 激活 trap 后，应让首个可聚焦元素获得焦点。
- 不要滥用 focus trap，只在真正的模态交互中使用。
- 保证容器内部的键盘导航顺序合理。

## 另请参阅

- [Root View System](/zh-CN/component/root)
- [Dialog](/zh-CN/component/dialog)
- [Sheet](/zh-CN/component/sheet)
- [focus-trap-react](https://github.com/focus-trap/focus-trap-react)

[Root]: https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html
[FocusTrapElement]: https://docs.rs/gpui-component/latest/gpui_component/trait.FocusTrapElement.html
[Dialog]: /zh-CN/component/dialog
[Sheet]: /zh-CN/component/sheet

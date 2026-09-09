---
title: DropdownButton
description: DropdownButton 由一个主按钮和一个触发下拉菜单的按钮组合而成。
---

# DropdownButton

[DropdownButton] 是一个组合型按钮组件。点击左侧主按钮时可以执行独立动作，点击右侧触发按钮时则会展开下拉菜单。

共享变体和尺寸可以设置在 DropdownButton 上。文案、图标、提示、加载状态和点击回调等动作自身的选项属于内层 [Button]。

## 导入

```rust
use gpui_kit::component::button::{Button, DropdownButton};
```

## 用法

```rust
use gpui_kit::Anchor;

DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
            .menu("Option 2", Box::new(MyAction))
            .separator()
            .menu("Option 3", Box::new(MyAction))
    })
```

### 变体

与 [Button] 一样，DropdownButton 支持不同视觉变体：

```rust
DropdownButton::new("dropdown")
    .primary()
    .button(Button::new("btn").label("Primary"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

DropdownButton 上不设置变体或尺寸时，内层按钮的值会应用到两半。

### 内层按钮选项

```rust
DropdownButton::new("dropdown")
    .button(
        Button::new("btn")
            .label("Save")
            .compact()
            .loading(is_saving)
            .tooltip("Save the current view")
            .on_click(|_, _, _| println!("Saved")),
    )
    .dropdown_menu(|menu, _, _| {
        menu.menu("Save as…", Box::new(MyAction))
    })
```

### 自定义锚点

```rust
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu_with_anchor(Anchor::BottomRight, |menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[DropdownButton]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.DropdownButton.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

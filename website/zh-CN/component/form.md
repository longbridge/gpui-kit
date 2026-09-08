---
title: Form
description: 支持字段布局、校验和多列排布的灵活表单容器。
---

# Form

Form 负责字段和底部操作区的布局。应用负责字段值、校验、提交，以及根据可用宽度选择列数。

## 导入

```rust
use gpui_kit::component::form::{field, v_form, h_form, Form, Field};
```

## 组合约定

`Form::new()` 默认使用单列，标签位于控件上方。`label_layout(Axis::Horizontal)` 将标签放到控件旁边；`columns(2)` 独立控制字段排列为两列。原有的 `horizontal()`、`vertical()`、`layout(Axis)`、`h_form()` 和 `v_form()` 继续可用。

```rust
Form::new()
    .label_layout(Axis::Horizontal)
    .columns(2)
    .child(Field::new().label("Name").child(Input::new(&name_input)))
    .child(Field::new().label("Email").child(Input::new(&email_input)))
    .footer(Button::new("save").label("Save"))
```

`child` 接收 Field。操作按钮放入 `footer`；底部操作区跨越所有列，内容靠末端对齐。应用为按钮绑定提交逻辑，Form 不会自动提交。完整的状态、回调和窗口初始化用法见[应用示例](https://github.com/longbridge/gpui-kit/tree/main/examples/ai_recipes)。

## 用法

### 基础表单

```rust
v_form()
    .child(
        field()
            .label("Name")
            .child(Input::new(&name_input))
    )
    .child(
        field()
            .label("Email")
            .child(Input::new(&email_input))
            .required(true)
    )
```

### 横向布局

```rust
h_form()
    .label_width(px(120.))
    .child(
        field()
            .label("First Name")
            .child(Input::new(&first_name))
    )
    .child(
        field()
            .label("Last Name")
            .child(Input::new(&last_name))
    )
```

### 多列表单

```rust
v_form()
    .columns(2)
    .child(
        field()
            .label("First Name")
            .child(Input::new(&first_name))
    )
    .child(
        field()
            .label("Last Name")
            .child(Input::new(&last_name))
    )
    .child(
        field()
            .label("Bio")
            .col_span(2)
            .child(Input::new(&bio_input))
    )
```

## 容器与布局

### 纵向布局

```rust
v_form()
    .gap(px(12.))
    .child(field().label("Name").child(input))
    .child(field().label("Email").child(email_input))
```

### 横向布局

```rust
h_form()
    .label_width(px(100.))
    .child(field().label("Name").child(input))
    .child(field().label("Email").child(email_input))
```

### 自定义尺寸

```rust
v_form()
    .large()
    .label_text_size(rems(1.2))
    .child(field().label("Title").child(input))

v_form()
    .small()
    .child(field().label("Code").child(input))
```

## 校验与说明

### 必填字段

```rust
field()
    .label("Email")
    .required(true)
    .child(Input::new(&email_input))
```

### 字段描述

```rust
field()
    .label("Password")
    .description("Must be at least 8 characters long")
    .child(Input::new(&password_input))
```

### 动态描述

```rust
field()
    .label("Bio")
    .description_fn(|_, _| {
        div().child("Use at most 100 words to describe yourself.")
    })
    .child(Input::new(&bio_input))
```

### 字段可见性

```rust
field()
    .label("Admin Settings")
    .visible(user.is_admin())
    .child(Switch::new("admin-mode"))
```

## 提交处理

### 基础提交模式

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

impl FormView {
    fn submit(&mut self, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).value();
        let email = self.email_input.read(cx).value();

        if name.is_empty() || email.is_empty() {
            return;
        }

        self.handle_submit(name, email, cx);
    }
}

v_form()
    .child(field().label("Name").child(Input::new(&self.name_input)))
    .child(field().label("Email").child(Input::new(&self.email_input)))
    .child(
        field()
            .label_indent(false)
            .child(
                Button::new("submit")
                    .primary()
                    .child("Submit")
                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx)))
            )
    )
```

### 操作按钮组

```rust
v_form()
    .child(field().label("Title").child(Input::new(&title)))
    .child(field().label("Content").child(Input::new(&content)))
    .child(
        field()
            .label_indent(false)
            .child(
                h_flex()
                    .gap_2()
                    .child(Button::new("save").primary().child("Save"))
                    .child(Button::new("cancel").child("Cancel"))
                    .child(Button::new("preview").outline().child("Preview"))
            )
    )
```

## 字段分组

### 相关字段组合

```rust
v_form()
    .child(
        field()
            .label("Name")
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&first_name)))
                    .child(div().flex_1().child(Input::new(&last_name)))
            )
    )
    .child(
        field()
            .label("Address")
            .items_start()
            .child(
                v_flex()
                    .gap_2()
                    .child(Input::new(&street))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&city)))
                            .child(div().w(px(100.)).child(Input::new(&zip)))
                    )
            )
    )
```

### 自定义字段组件

```rust
field()
    .label("Theme Color")
    .child(ColorPicker::new(&color_state).small())

field()
    .label("Birth Date")
    .description("We'll send you a birthday gift!")
    .child(DatePicker::new(&date_state))
```

### 条件字段

```rust
v_form()
    .child(
        field()
            .label("Account Type")
            .child(Select::new(&account_type))
    )
    .child(
        field()
            .label("Company Name")
            .visible(is_business_account)
            .child(Input::new(&company_name))
    )
```

## 网格与定位

### 列跨度

```rust
v_form()
    .columns(3)
    .child(field().label("First").child(input1))
    .child(field().label("Second").child(input2))
    .child(field().label("Third").child(input3))
    .child(
        field()
            .label("Full Width")
            .col_span(3)
            .child(Input::new(&full_width))
    )
```

### 响应式布局

```rust
v_form()
    .columns(if is_mobile { 1 } else { 2 })
    .child(field().label("Name").child(name_input))
    .child(field().label("Email").child(email_input))
    .child(
        field()
            .label("Bio")
            .when(!is_mobile, |field| field.col_span(2))
            .child(bio_input)
    )
```

## 示例

### 注册表单

```rust
struct RegistrationForm {
    first_name: Entity<InputState>,
    last_name: Entity<InputState>,
    email: Entity<InputState>,
    password: Entity<InputState>,
    confirm_password: Entity<InputState>,
    terms_accepted: bool,
}
```

### 设置表单

```rust
v_form()
    .column(2)
    .child(
        field()
            .label("Profile")
            .label_indent(false)
            .col_span(2)
            .child(Separator::horizontal())
    )
    .child(
        field()
            .label("Display Name")
            .child(Input::new(&display_name))
    )
```

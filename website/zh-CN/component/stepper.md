---
title: Stepper
description: 用于引导用户按步骤完成流程的进度组件。
---

# Stepper

Stepper 用于按步骤展示流程进度，适合表单向导、订单流程和安装步骤等场景。支持横向和纵向布局、自定义图标以及不同尺寸。

## 导入

```rust
use gpui_kit::component::stepper::{Stepper, StepperItem};
```

## 用法

### 基础 Stepper

使用 `selected_index` 设置当前步骤，索引从 `0` 开始，默认值也是 `0`。

```rust
Stepper::new("my-stepper")
    .selected_index(0)
    .items([
        StepperItem::new().child("Step 1"),
        StepperItem::new().child("Step 2"),
        StepperItem::new().child("Step 3"),
    ])
    .on_click(|step, _, _| {
        println!("Clicked step: {}", step);
    })
```

### 带图标的 Stepper

```rust
use gpui_kit::component::IconName;

Stepper::new("icon-stepper")
    .selected_index(0)
    .items([
        StepperItem::new()
            .icon(IconName::Calendar)
            .child("Order Details"),
        StepperItem::new()
            .icon(IconName::Inbox)
            .child("Shipping"),
        StepperItem::new()
            .icon(IconName::Frame)
            .child("Preview"),
        StepperItem::new()
            .icon(IconName::Info)
            .child("Finish"),
    ])
```

### 纵向布局

```rust
Stepper::new("vertical-stepper")
    .vertical()
    .selected_index(2)
    .items_center()
    .items([
        StepperItem::new()
            .pb_8()
            .icon(IconName::Building2)
            .child(v_flex().child("Step 1").child("Description for step 1.")),
        StepperItem::new()
            .pb_8()
            .icon(IconName::Asterisk)
            .child(v_flex().child("Step 2").child("Description for step 2.")),
        StepperItem::new()
            .pb_8()
            .icon(IconName::Folder)
            .child(v_flex().child("Step 3").child("Description for step 3.")),
        StepperItem::new()
            .icon(IconName::CircleCheck)
            .child(v_flex().child("Step 4").child("Description for step 4.")),
    ])
```

### 文本居中

```rust
Stepper::new("center-stepper")
    .selected_index(0)
    .text_center(true)
    .items([
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 1")
                .child("Desc for step 1."),
        ),
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 2")
                .child("Desc for step 2."),
        ),
        StepperItem::new().child(
            v_flex()
                .items_center()
                .child("Step 3")
                .child("Desc for step 3."),
        ),
    ])
```

### 不同尺寸

```rust
use gpui_kit::component::{Sizable as _, Size};

Stepper::new("stepper")
    .xsmall()
    .items([...])

Stepper::new("stepper")
    .small()
    .items([...])

Stepper::new("stepper")
    .large()
    .items([...])
```

### 禁用状态

```rust
Stepper::new("disabled-stepper")
    .disabled(true)
    .items([
        StepperItem::new().child("Step 1"),
        StepperItem::new().child("Step 2"),
    ])
```

## API 参考

- [Stepper]
- [StepperItem]

### 尺寸

实现了 [Sizable] trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸

## 示例

### 多步骤表单

```rust
Stepper::new("form-stepper")
    .w_full()
    .selected_index(form_step)
    .items([
        StepperItem::new()
            .icon(IconName::User)
            .child("Personal Info"),
        StepperItem::new()
            .icon(IconName::CreditCard)
            .child("Payment"),
        StepperItem::new()
            .icon(IconName::CircleCheck)
            .child("Confirmation"),
    ])
    .on_click(cx.listener(|this, step, _, cx| {
        this.form_step = *step;
        cx.notify();
    }))
```

### 禁用单个步骤

```rust
Stepper::new("stepper")
    .selected_index(0)
    .items([
        StepperItem::new().child("Available"),
        StepperItem::new().disabled(true).child("Locked"),
        StepperItem::new().child("Available"),
    ])
```

[Stepper]: https://docs.rs/gpui-component/latest/gpui_component/stepper/struct.Stepper.html
[StepperItem]: https://docs.rs/gpui-component/latest/gpui_component/stepper/struct.StepperItem.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

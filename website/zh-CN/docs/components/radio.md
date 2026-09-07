---
title: Radio
description: 一组互斥的单选按钮，同一时间只能选中一个选项。
---

# Radio

Radio 用于在一组选项中选择唯一结果。适合“多选一”的场景，例如设置项、问卷和支付方式选择等。

使用 `on_change` 接收请求的新值，由状态所有者保存并调用 `cx.notify()`。原有的 `on_click` 保留为兼容名称；两者设置的是同一个回调，最后一次设置生效。

## 导入

```rust
use gpui_kit::component::radio::{Radio, RadioGroup};
```

## 用法

### 基础单选按钮

```rust
Radio::new("radio-option-1")
    .label("Option 1")
    .checked(false)
    .on_change(|checked, _, _| {
        println!("Radio is now: {}", checked);
    })
```

### 受控单选按钮

```rust
struct MyView {
    radio_checked: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Radio::new("radio")
            .label("Select this option")
            .checked(self.radio_checked)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.radio_checked = *checked;
                cx.notify();
            }))
    }
}
```

### RadioGroup（推荐）

```rust
struct MyView {
    selected_option: Option<usize>,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        RadioGroup::horizontal("options")
            .children(["Option 1", "Option 2", "Option 3"])
            .selected_index(self.selected_option)
            .on_change(cx.listener(|view, selected_index: &usize, _, cx| {
                view.selected_option = Some(*selected_index);
                cx.notify();
            }))
    }
}
```

### 不同尺寸

```rust
Radio::new("small").label("Small").xsmall()
Radio::new("medium").label("Medium")
Radio::new("large").label("Large").large()
```

### 禁用状态

```rust
Radio::new("disabled")
    .label("Disabled option")
    .disabled(true)
    .checked(false)

Radio::new("disabled-checked")
    .label("Disabled and checked")
    .checked(true)
    .disabled(true)
```

### 多行标签与自定义内容

```rust
Radio::new("custom")
    .label("Primary option")
    .child(
        div()
            .text_color(cx.theme().muted_foreground)
            .child("This is additional descriptive text that provides more context.")
    )
    .w(px(300.))
```

### 自定义 Tab 顺序

```rust
Radio::new("radio")
    .label("Custom tab order")
    .tab_index(2)
    .tab_stop(true)
```

## Radio Group 用法

### 横向布局

```rust
RadioGroup::horizontal("horizontal-group")
    .children(["First", "Second", "Third"])
    .selected_index(Some(0))
    .on_change(cx.listener(|view, index, _, cx| {
        println!("Selected index: {}", index);
        cx.notify();
    }))
```

### 纵向布局

```rust
RadioGroup::vertical("vertical-group")
    .child(Radio::new("option1").label("United States"))
    .child(Radio::new("option2").label("Canada"))
    .child(Radio::new("option3").label("Mexico"))
    .selected_index(Some(1))
    .disabled(false)
```

### 带样式的分组

```rust
RadioGroup::vertical("styled-group")
    .w(px(220.))
    .p_2()
    .border_1()
    .border_color(cx.theme().border)
    .rounded(cx.theme().radius)
    .child(Radio::new("option1").label("Option 1"))
    .child(Radio::new("option2").label("Option 2"))
    .child(Radio::new("option3").label("Option 3"))
    .selected_index(Some(0))
```

### 禁用整个分组

```rust
RadioGroup::vertical("disabled-group")
    .children(["Option A", "Option B", "Option C"])
    .selected_index(Some(1))
    .disabled(true)
```

## API 参考

### Radio

| 方法 | 说明 |
| --- | --- |
| `new(id)` | 使用给定 ID 创建单选按钮 |
| `label(text)` | 设置标签文本 |
| `checked(bool)` | 设置选中状态 |
| `disabled(bool)` | 设置禁用状态 |
| `on_change(fn)` | 点击回调，参数为新的 `&bool` 选中状态 |
| `tab_stop(bool)` | 是否允许通过 Tab 聚焦，默认 `true` |
| `tab_index(isize)` | 设置 Tab 顺序，默认 `0` |

### RadioGroup

| 方法 | 说明 |
| --- | --- |
| `new(id)` | 创建未选中任何项的纵向分组 |
| `horizontal(id)` | 创建横向分组 |
| `vertical(id)` | 创建纵向分组 |
| `layout(Axis)` | 设置布局方向 |
| `child(Radio)` | 添加单个 Radio |
| `children(items)` | 通过迭代器批量添加 Radio |
| `selected_index(Option<usize>)` | 设置选中项索引 |
| `disabled(bool)` | 禁用分组内所有 Radio |
| `on_change(fn)` | 选择变化回调，参数为选中的 `&usize` 索引 |

### 样式

Radio 和 RadioGroup 都实现了 `Styled` trait。

Radio 还实现了 `Sizable` trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸

## 最佳实践

1. 互斥选项优先使用 `RadioGroup`，不要手动管理一组独立的 `Radio`。
2. 标签要明确，用户应当一眼看懂每个选项的含义。
3. 对必填项可以提供合理的默认选中项。
4. 选项顺序应符合业务逻辑，例如频率、重要性或字母顺序。
5. 单选项数量应保持适中，通常建议 2 到 7 个。
6. 多组单选项应配合清晰标题和视觉分组。
7. 选项较少时可横向排列，较多时更适合纵向排列。

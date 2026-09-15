---
title: Input
description: 带校验、掩码和多种扩展能力的文本输入组件。
---

# Input

Input 是一个单行文本输入组件，支持校验、输入掩码、前后缀元素以及多种交互状态。普通多行文本请使用 [Textarea](./textarea.md)，源代码编辑请使用 [Editor](./editor.md)。

## 导入

```rust
use gpui_kit::component::input::{Input, InputState};
```

## 用法

### 基础输入框

```rust
let input = cx.new(|cx| InputState::new(window, cx));

Input::new(&input)
```

### Placeholder

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter your name...")
);

Input::new(&input)
```

### 默认值

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("John Doe")
);

Input::new(&input)
```

### 可清空

```rust
Input::new(&input)
    .cleanable(true)
```

### 前缀和后缀

```rust
use gpui_kit::component::{Icon, IconName};

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())

Input::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())
    .suffix(Button::new("btn").ghost().icon(IconName::Info).xsmall())
```

### 密码输入

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .masked(true)
        .default_value("password123")
);

Input::new(&input)
    .content_type(InputContentType::Password)
    .mask_toggle()
```

掩码状态下，输入框不会让明文进入剪贴板，也不会通过选区暴露内容：Copy 和 Cut
不执行任何操作（上下文菜单中同样置灰），按词删除会删掉光标之前的全部内容，双击
则选中整个值而不是其中一个词。Paste 和 Select All 不受影响，通过 `mask_toggle`
显示明文后，上述操作全部恢复。

### 尺寸

```rust
Input::new(&input).large()
Input::new(&input)
Input::new(&input).small()
```

### 禁用态

```rust
Input::new(&input).disabled(true)
```

### 只读态

与 `disabled` 不同，只读输入框保持正常外观，仍然可以聚焦、选中和复制，只是拒绝用户对内容的修改。

```rust
Input::new(&input).readonly(true)
```

### 按 ESC 清空

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .clean_on_escape()
);

Input::new(&input)
```

### 输入校验

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .validate(|s, _| s.parse::<f32>().is_ok())
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
);
```

### 输入掩码

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("(999)-999-9999")
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("AAA-###-AAA")
);

use gpui_kit::component::input::MaskPattern;

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(3),
        })
);
```

### 监听事件

```rust
let input = cx.new(|cx| InputState::new(window, cx));

cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            println!("Input changed: {}", text);
        }
        InputEvent::PressEnter { secondary } => {
            println!("Enter pressed, secondary: {}", secondary);
        }
        InputEvent::Focus => println!("Input focused"),
        InputEvent::Blur => println!("Input blurred"),
    }
});
```

### 自定义外观

```rust
Input::new(&input).appearance(false)

div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(Input::new(&input).appearance(false))
```

### 触摸选择

在触摸屏上，长按会选中手指下的单词，手指按住不放时选区跟随手指移动。抬起手指后，选区上方会出现编辑菜单，列出当前可用的命令——`剪切`、`复制`、`粘贴` 和 `全选`——并在选区两端各显示一个拖动 handle。拖动 handle 会移动对应的一端，另一端保持不动；多行输入框在手指到达边缘时会自动滚动。长按空白处或空输入框时会放置光标，菜单只提供 `粘贴` 和 `全选`。

handle 和菜单属于这次手势产生的选区。只要其他操作改变了选区——点击、输入、方向键、`Escape`——它们就会消失；手指滚动内容时菜单会暂时让开。点击已选中的文字可以重新呼出菜单。

剪切、复制和粘贴通过输入框自身的 action 执行，因此自定义快捷键或打开中的补全菜单都能以同样方式处理它们。只读输入框只提供 `复制` 和 `全选`；密码输入框的内容不会进入剪贴板。

## 示例

### 搜索输入框

```rust
let search = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Search...")
);

Input::new(&search)
    .prefix(Icon::new(IconName::Search).small())
```

### 金额输入

```rust
let amount = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

div()
    .child(Input::new(&amount))
    .child(format!("Value: {}", amount.read(cx).value()))
```

### 多输入表单

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

v_flex()
    .gap_3()
    .child(Input::new(&self.name_input))
    .child(Input::new(&self.email_input))
```

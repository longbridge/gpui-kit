---
title: NumberInput
description: 带增减按钮与数字格式化能力的数值输入组件。
---

# NumberInput

NumberInput 是针对数值输入场景设计的组件，内置递增/递减按钮，并支持最小值、最大值、步进值以及千分位格式化等能力。

## 导入

```rust
use gpui_kit::component::input::{InputState, NumberInput, NumberInputEvent, StepAction};
```

## 用法

### 基础数值输入

```rust
let number_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter number")
        .default_value("1")
);

NumberInput::new(&number_input)
```

### 输入限制与字符归一化

默认情况下，NumberInput 只接受合法数字：可选的开头 `+`/`-` 符号、数字和一个小数点（例如 `-1.5`），其他字符在输入和粘贴时会被拒绝。

针对中文等输入法用户，全角数字字符会自动转换为对应的半角字符：

- 全角数字：`１２３` → `123`
- 全角符号：`＋` → `+`、`－` → `-`
- 全角小数点与中文句号：`．`、`。` → `.`

以小数点开头的输入保持原样（例如 `.5`，解析为 `0.5`），与 Web 行为一致——删除 `1.2` 的整数部分会保留 `.2`，便于继续编辑。

如需关闭默认限制，可显式设置 mask：`state.set_mask_pattern(MaskPattern::None, window, cx)`。

如需进一步限制输入（例如仅允许正整数），可使用 `pattern`：

```rust
let integer_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Integer value")
        .pattern(Regex::new(r"^\d+$").unwrap())
);

NumberInput::new(&integer_input)
```

### 最小值 / 最大值 / 步进值

默认情况下，NumberInput 以 `step(1.)` 在内部更新数值：`↑`/`↓` 键和 `+`/`-` 按钮按 1 步进并发出 `InputEvent::Change` 事件。可通过 `min`/`max` 限制范围，或设置自定义步进值。

如需仅发出 `NumberInputEvent::Step` 事件（由订阅方负责更新数值），可调用 `state.set_step(None, window, cx)`。

手动输入的越界值在输入过程中会被保留，失焦时自动收敛到范围内。步进遵循 Web 行为：无法朝按键方向移动数值的步进不会生效（例如数值已等于或低于 `min` 时按 `↓` 不会变化）。

```rust
let stepper_input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("50")
        .step(5.)
        .min(0.)
        .max(100.)
);

NumberInput::new(&stepper_input)
```

### 动态步长

使用 `step_by` 可以根据当前值和步进方向实时计算步长，例如步长随取值区间变化。步长可在边界处随方向不同，因此闭包会收到 `StepAction`；下例中以 `1.0` 为界，向下步进 `0.1`，向上步进 `0.5`。闭包还会收到一个 `Context`，可用于读取或更新其他 entity：

```rust
let price_input = cx.new(|cx|
    InputState::new(window, cx)
        .step_by(|value, action, _cx| match action {
            StepAction::Increment => if value < 1.0 { 0.1 } else { 0.5 },
            StepAction::Decrement => if value <= 1.0 { 0.1 } else { 0.5 },
        })
        .min(0.)
);

NumberInput::new(&price_input)
```

步进策略也可以在运行时通过 `set_step` 更新：

```rust
use gpui_kit::component::input::NumberStep;

state.set_step(NumberStep::Fixed(0.01), window, cx);
state.set_step(NumberStep::by_value(|v, _, _cx| if v < 1. { 0.01 } else { 0.1 }), window, cx);
state.set_step(None, window, cx); // 回退到 NumberInputEvent::Step 事件模式
```

### 数字格式化

```rust
use gpui_kit::component::input::MaskPattern;

let currency_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Amount")
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

NumberInput::new(&currency_input)
```

### 不同尺寸

```rust
NumberInput::new(&input).large()
NumberInput::new(&input)
NumberInput::new(&input).small()
```

### 前缀与后缀

```rust
use gpui_kit::component::{button::{Button, ButtonVariants}, IconName};

NumberInput::new(&input)
    .prefix(div().child("$"))

NumberInput::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )
```

### 禁用状态

```rust
NumberInput::new(&input).disabled(true)
```

### 关闭默认外观

```rust
div()
    .w_full()
    .bg(cx.theme().secondary)
    .rounded(cx.theme().radius)
    .child(NumberInput::new(&input).appearance(false))
```

### 处理 NumberInput 事件

默认情况下 NumberInput 在内部更新数值。如需回退到 `NumberInputEvent::Step` 模式（由订阅方负责更新数值），可调用 `state.set_step(None, window, cx)`：

```rust
let number_input = cx.new(|cx| InputState::new(window, cx));
let mut value: i64 = 0;

cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            if let Ok(new_value) = text.parse::<i64>() {
                view.value = new_value;
            }
        }
        _ => {}
    }
});

cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        NumberInputEvent::Step(step_action) => {
            match step_action {
                StepAction::Increment => {
                    view.value += 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
                StepAction::Decrement => {
                    view.value -= 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
            }
        }
    }
});
```

### 程序化控制

```rust
NumberInput::increment(&number_input, window, cx);
NumberInput::decrement(&number_input, window, cx);
```

## API 参考

### NumberInput

| 方法 | 说明 |
| ------------------------------ | ------------------------------------------ |
| `new(state)` | 使用 `InputState` 创建数值输入组件 |
| `placeholder(str)` | 设置占位文案 |
| `size(size)` | 设置尺寸 |
| `prefix(el)` | 添加前缀元素 |
| `suffix(el)` | 添加后缀元素 |
| `appearance(bool)` | 开启或关闭默认样式 |
| `disabled(bool)` | 设置禁用状态 |
| `increment(state, window, cx)` | 以代码方式递增 |
| `decrement(state, window, cx)` | 以代码方式递减 |

### NumberInputEvent

| 事件 | 说明 |
| ------------------ | ---------------------------------- |
| `Step(StepAction)` | 点击增减按钮时触发，仅当 `step` 为 `None` 时发出（通过 `set_step(None, ...)` 选择该模式） |

### StepAction

| 动作 | 说明 |
| ----------- | ------------------------- |
| `Increment` | 数值增加 |
| `Decrement` | 数值减少 |

### InputState（数值相关方法）

| 方法 | 说明 |
| ----------------------------------- | ------------------------------------------------------- |
| `step(impl Into<NumberStep>)` | 设置内置递增/递减的步进值（默认为 1） |
| `step_by(fn(f64, StepAction, &mut Context) -> f64)` | 步进时根据当前值和方向实时计算步长 |
| `min(f64)` | 设置最小值，步进与失焦时收敛到该值 |
| `max(f64)` | 设置最大值，步进与失焦时收敛到该值 |
| `set_step(Option<NumberStep>, ...)` | 构造后更新步进策略 |
| `set_min(Option<f64>, ...)` | 构造后更新最小值 |
| `set_max(Option<f64>, ...)` | 构造后更新最大值 |
| `pattern(regex)` | 设置校验正则，例如只允许数字 |
| `mask_pattern(MaskPattern::Number)` | 设置数字格式化规则 |
| `value()` | 获取当前展示值 |
| `unmask_value()` | 获取未格式化的真实数值 |

### MaskPattern::Number

| 字段 | 类型 | 说明 |
| ----------- | --------------- | -------------------------------------- |
| `separator` | `Option<char>` | 千分位分隔符 |
| `fraction` | `Option<usize>` | 小数位数 |

## 键盘导航

| 按键 | 行为 |
| ----------- | -------------------------- |
| `↑` | 增加数值 |
| `↓` | 减少数值 |
| `Tab` | 切换到下一个字段 |
| `Shift+Tab` | 切换到上一个字段 |
| `Enter` | 提交或确认当前值 |
| `Escape` | 清空输入（若启用） |

## 示例

### 整数计数器

```rust
struct CounterView {
    counter_input: Entity<InputState>,
    counter_value: i32,
}

impl CounterView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let counter_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("Count")
                .default_value("0")
                .pattern(Regex::new(r"^-?\d+$").unwrap())
        );

        let _subscription = cx.subscribe_in(&counter_input, window, Self::on_number_event);

        Self {
            counter_input,
            counter_value: 0,
        }
    }

    fn on_number_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                self.counter_value += 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                self.counter_value -= 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
        }
    }
}

NumberInput::new(&self.counter_input)
```

### 货币输入

```rust
struct PriceInput {
    price_input: Entity<InputState>,
    price_value: f64,
}

impl PriceInput {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let price_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("0.00")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: Some(2),
                })
        );

        Self {
            price_input,
            price_value: 0.0,
        }
    }
}

h_flex()
    .gap_2()
    .child(div().child("$"))
    .child(NumberInput::new(&self.price_input))
```

### 带上下限的数量选择器

```rust
struct QuantitySelector {
    quantity_input: Entity<InputState>,
}

impl QuantitySelector {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 按 1 步进并限制在 1..=99 范围内，无需处理事件。
        let quantity_input = cx.new(|cx|
            InputState::new(window, cx)
                .default_value("1")
                .min(1.)
                .max(99.)
        );

        Self { quantity_input }
    }
}

NumberInput::new(&self.quantity_input).small()
```

### 浮点数输入

```rust
// 按 0.1 步进，步进时保留当前值的小数位数，
// 例如 0.2 -> 0.3（而不是 0.30000000000000004）。
let float_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("0.0")
        .step(0.1)
);

NumberInput::new(&float_input)
```

## 最佳实践

1. 客户端和服务端都应校验数值输入。
2. 使用 `min`/`max` 对用户可操作范围设置明确上下限。
3. 根据业务场景选择合适的 `step` 步进值。
4. 对非法输入提供清晰反馈。
5. 在整个应用中保持统一的数字格式。
6. 高频点击增减时，必要时做节流或去抖。
7. 始终为输入提供清晰的标签与描述。

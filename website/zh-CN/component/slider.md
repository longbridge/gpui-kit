---
title: Slider
description: 通过拖动滑块在区间内选择数值的控件。
---

# Slider

Slider 用于在给定范围内选择数值，支持单值和区间选择、横向和纵向布局、自定义样式以及步进控制。

## 导入

```rust
use gpui_kit::component::slider::{Slider, SliderState, SliderEvent, SliderValue};
```

## 用法

### 基础 Slider

```rust
let slider_state = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .default_value(50.0)
        .step(1.0)
});

Slider::new(&slider_state)
```

### 处理事件

```rust
struct MyView {
    slider_state: Entity<SliderState>,
    current_value: f32,
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        let slider_state = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .default_value(25.0)
                .step(5.0)
        });

        let subscription = cx.subscribe(&slider_state, |this, _, event: &SliderEvent, cx| {
            match event {
                SliderEvent::Change(value) => {
                    this.current_value = value.start();
                    cx.notify();
                }
            }
        });

        Self {
            slider_state,
            current_value: 25.0,
        }
    }
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(Slider::new(&self.slider_state))
            .child(format!("Value: {}", self.current_value))
    }
}
```

### 区间 Slider

```rust
let range_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .default_value(20.0..80.0)
        .step(1.0)
});

Slider::new(&range_slider)
```

### 纵向 Slider

```rust
Slider::new(&slider_state)
    .vertical()
    .h(px(200.))
```

### 自定义步进

```rust
let integer_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(10.0)
        .step(1.0)
        .default_value(5.0)
});

let decimal_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(1.0)
        .step(0.01)
        .default_value(0.5)
});
```

### 最小值和最大值

```rust
let temp_slider = cx.new(|_| {
    SliderState::new()
        .min(-10.0)
        .max(40.0)
        .default_value(20.0)
        .step(0.5)
});

let percent_slider = cx.new(|_| {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .default_value(75.0)
        .step(5.0)
});
```

### 禁用状态

```rust
Slider::new(&slider_state)
    .disabled(true)
```

### 自定义样式

```rust
Slider::new(&slider_state)
    .bg(cx.theme().success)
    .text_color(cx.theme().success_foreground)
    .rounded(px(4.))
```

### Scale

Slider 支持两种比例模式：

- `Linear`
- `Logarithmic`

对数比例适合跨度很大的取值范围，可以让较小数值获得更细的控制精度。

```rust
let log_slider = cx.new(|_| {
    SliderState::new()
        .min(1.0)
        .max(1000.0)
        .default_value(10.0)
        .step(1.0)
        .scale(SliderScale::Logarithmic)
});
```

在这种模式下：

:::info
$$ v = min \times (max/min)^p $$

其中 `p` 是滑块位置对应的百分比，范围为 0 到 1。
:::

- 当滑块在 25% 时，值约为 `5.62`
- 当滑块在 50% 时，值约为 `31.62`
- 当滑块在 75% 时，值约为 `177.83`
- 当滑块在 100% 时，值为 `1000.0`

#### 类型转换

```rust
let single_value: SliderValue = 42.0.into();
let range_value: SliderValue = (10.0, 90.0).into();
let range_value: SliderValue = (10.0..90.0).into();
```

### SliderEvent

| 事件 | 说明 |
| --- | --- |
| `Change(SliderValue)` | 滑块值变化过程中持续触发 |
| `Release(SliderValue)` | 拖拽结束（松开鼠标）时触发一次 |

### 样式

Slider 实现了 `Styled` trait，支持：

- 轨道和滑块背景色
- 滑块文本颜色
- 圆角
- 尺寸定制

## 示例

### 颜色选择器

```rust
struct ColorPicker {
    hue_slider: Entity<SliderState>,
    saturation_slider: Entity<SliderState>,
    lightness_slider: Entity<SliderState>,
    alpha_slider: Entity<SliderState>,
    current_color: Hsla,
}

impl ColorPicker {
    fn new(cx: &mut Context<Self>) -> Self {
        let hue_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .default_value(0.5)
        });

        let saturation_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .default_value(1.0)
        });

        let subscriptions = [&hue_slider, &saturation_slider /* ... */]
            .iter()
            .map(|slider| {
                cx.subscribe(slider, |this, _, event: &SliderEvent, cx| {
                    match event {
                        SliderEvent::Change(_) => {
                            this.update_color(cx);
                        }
                    }
                })
            })
            .collect::<Vec<_>>();

        Self {
            hue_slider,
            saturation_slider,
            // ... other fields
        }
    }

    fn update_color(&mut self, cx: &mut Context<Self>) {
        let h = self.hue_slider.read(cx).value().start();
        let s = self.saturation_slider.read(cx).value().start();
        // ... 计算颜色
        self.current_color = hsla(h, s, l, a);
        cx.notify();
    }
}
```

### 音量控制

```rust
struct VolumeControl {
    volume_slider: Entity<SliderState>,
    volume: f32,
}

impl VolumeControl {
    fn new(cx: &mut Context<Self>) -> Self {
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(50.0)
        });

        let subscription = cx.subscribe(&volume_slider, |this, _, event: &SliderEvent, cx| {
            match event {
                SliderEvent::Change(value) => {
                    this.volume = value.start();
                    this.apply_volume_change();
                    cx.notify();
                }
            }
        });

        Self {
            volume_slider,
            volume: 50.0,
        }
    }

    fn apply_volume_change(&self) {
        println!("Volume changed to: {}%", self.volume);
    }
}
```

### 价格区间筛选

```rust
struct PriceFilter {
    price_range: Entity<SliderState>,
    min_price: f32,
    max_price: f32,
}

impl PriceFilter {
    fn new(cx: &mut Context<Self>) -> Self {
        let price_range = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1000.0)
                .step(10.0)
                .default_value(100.0..500.0)
        });

        let subscription = cx.subscribe(&price_range, |this, _, event: &SliderEvent, cx| {
            match event {
                SliderEvent::Change(value) => {
                    this.min_price = value.start();
                    this.max_price = value.end();
                    this.filter_products();
                    cx.notify();
                }
            }
        });

        Self {
            price_range,
            min_price: 100.0,
            max_price: 500.0,
        }
    }

    fn filter_products(&self) {
        println!("Filtering products: ${} - ${}", self.min_price, self.max_price);
    }
}
```

## 键盘快捷键

| 按键 | 操作 |
| --- | --- |
| `←` / `↓` | 按步进减小数值 |
| `→` / `↑` | 按步进增大数值 |
| `Page Down` | 较大幅度减小数值 |
| `Page Up` | 较大幅度增大数值 |
| `Home` | 设置为最小值 |
| `End` | 设置为最大值 |
| `Tab` | 焦点移动到下一个元素 |
| `Shift + Tab` | 焦点移动到上一个元素 |

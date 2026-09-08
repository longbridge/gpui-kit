---
title: ColorPicker
description: 支持多种颜色格式、预设色和透明通道的颜色选择组件。
---

# ColorPicker

ColorPicker 是一个通用的颜色选择组件，提供直观的颜色选择界面。它支持颜色面板、十六进制输入、精选颜色，以及 RGB、HSL 和十六进制格式，并支持 alpha 透明通道。

## 导入

```rust
use gpui_kit::component::color_picker::{ColorPicker, ColorPickerState, ColorPickerEvent};
```

## 用法

### 基础 Color Picker

```rust
use gpui_kit::{Entity, Window, Context};

let color_picker = cx.new(|cx|
    ColorPickerState::new(window, cx)
        .default_value(cx.theme().primary)
);

ColorPicker::new(&color_picker)
```

### 处理事件

```rust
use gpui_kit::{Subscription, Entity};

let color_picker = cx.new(|cx| ColorPickerState::new(window, cx));

let _subscription = cx.subscribe(&color_picker, |this, _, ev, _| match ev {
    ColorPickerEvent::Change(color) => {
        if let Some(color) = color {
            println!("Selected color: {}", color.to_hex());
            // Handle color change
        }
    }
});

ColorPicker::new(&color_picker)
```

### 设置默认颜色

```rust
use gpui_kit::Hsla;

let color_picker = cx.new(|cx|
    ColorPickerState::new(window, cx)
        .default_value(cx.theme().blue)
);
```

### 不同尺寸

```rust
ColorPicker::new(&color_picker).small()
ColorPicker::new(&color_picker)
ColorPicker::new(&color_picker).large()
ColorPicker::new(&color_picker).xsmall()
```

### 自定义精选颜色

```rust
use gpui_kit::Hsla;

let featured_colors = vec![
    cx.theme().red,
    cx.theme().green,
    cx.theme().blue,
    cx.theme().yellow,
];

ColorPicker::new(&color_picker)
    .featured_colors(featured_colors)
```

### 用图标替代色块

```rust
use gpui_kit::component::IconName;

ColorPicker::new(&color_picker)
    .icon(IconName::Palette)
```

### 带标签

```rust
ColorPicker::new(&color_picker)
    .label("Background Color")
```

### 自定义锚点位置

```rust
use gpui_kit::Anchor;

ColorPicker::new(&color_picker)
    .anchor(Anchor::TopRight)
```

## 颜色选择界面

### 调色板

组件内置多组颜色家族：

- **Stone**：中性色与石灰灰阶
- **Red**：红色系
- **Orange**：橙色系
- **Yellow**：黄色系
- **Green**：绿色系
- **Cyan**：青色系
- **Blue**：蓝色系
- **Purple**：紫色系
- **Pink**：粉色系

每个颜色家族都提供多个深浅层级，方便精准选色。

### 精选颜色区域

顶部的精选颜色区域可用于放置品牌色或常用色。若未指定，则默认使用当前主题中的核心颜色：

- 当前主题的主色
- 主题颜色的浅色变体
- 常用界面色，如 red、blue、green、yellow、cyan、magenta

### Hex 输入框

组件提供十六进制输入框，可直接输入颜色值：

- 支持标准 6 位格式 `#RRGGBB`
- 实时校验并预览
- 会自动同步到组件状态
- 按 Enter 确认

## 颜色格式

### RGB

颜色内部使用 GPUI 的 `Hsla` 表示，但可以转换为 RGB 相关值：

```rust
let color = cx.theme().blue;
// Access RGB components through Hsla methods
```

### HSL

ColorPicker 原生使用 HSL/HSLA 表示：

```rust
use gpui_kit::Hsla;

let color = Hsla::hsl(240.0, 100.0, 50.0);

let hue = color.h;
let saturation = color.s;
let lightness = color.l;
```

### Hex

标准 Web 十六进制格式：

```rust
let hex_string = color.to_hex();

if let Ok(color) = Hsla::parse_hex("#3366FF") {
    // Use parsed color
}
```

## Alpha 通道

支持透明度：

```rust
use gpui_kit::hsla;

let semi_transparent = hsla(0.5, 0.8, 0.6, 0.7);

let transparent_blue = cx.theme().blue.opacity(0.5);
```

ColorPicker 在选择颜色时会保留 alpha 值，也可通过 HSLA 的 alpha 分量进一步修改。

## API 参考

- [ColorPicker]
- [ColorPickerState]
- [ColorPickerEvent]

## 示例

### 主题颜色编辑器

```rust
struct ThemeEditor {
    primary_color: Entity<ColorPickerState>,
    secondary_color: Entity<ColorPickerState>,
    accent_color: Entity<ColorPickerState>,
}

impl ThemeEditor {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let primary_color = cx.new(|cx|
            ColorPickerState::new(window, cx)
                .default_value(cx.theme().primary)
        );

        let secondary_color = cx.new(|cx|
            ColorPickerState::new(window, cx)
                .default_value(cx.theme().secondary)
        );

        let accent_color = cx.new(|cx|
            ColorPickerState::new(window, cx)
                .default_value(cx.theme().accent)
        );

        Self {
            primary_color,
            secondary_color,
            accent_color,
        }
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("Primary Color:")
                    .child(ColorPicker::new(&self.primary_color))
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("Secondary Color:")
                    .child(ColorPicker::new(&self.secondary_color))
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("Accent Color:")
                    .child(ColorPicker::new(&self.accent_color))
            )
    }
}
```

### 品牌色选择器

```rust
use gpui_kit::component::Sizable as _;

let brand_colors = vec![
    Hsla::parse_hex("#FF6B6B").unwrap(),
    Hsla::parse_hex("#4ECDC4").unwrap(),
    Hsla::parse_hex("#45B7D1").unwrap(),
    Hsla::parse_hex("#96CEB4").unwrap(),
    Hsla::parse_hex("#FFEAA7").unwrap(),
];

ColorPicker::new(&color_picker)
    .featured_colors(brand_colors)
    .label("Brand Color")
    .large()
```

### 工具栏颜色选择器

```rust
use gpui_kit::component::{Sizable as _, IconName};

ColorPicker::new(&text_color_picker)
    .icon(IconName::Type)
    .small()
    .anchor(Anchor::BottomLeft)
```

### 调色板构建器

```rust
struct ColorPalette {
    colors: Vec<Entity<ColorPickerState>>,
}

impl ColorPalette {
    fn add_color(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let color_picker = cx.new(|cx| ColorPickerState::new(window, cx));

        cx.subscribe(&color_picker, |this, _, ev, _| match ev {
            ColorPickerEvent::Change(color) => {
                if let Some(color) = color {
                    this.update_palette_preview();
                }
            }
        });

        self.colors.push(color_picker);
        cx.notify();
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .children(
                self.colors.iter().map(|color_picker| {
                    ColorPicker::new(color_picker).small()
                })
            )
            .child(
                Button::new("add-color")
                    .icon(IconName::Plus)
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_color(window, cx);
                    }))
            )
    }
}
```

### 颜色校验

```rust
let color_picker = cx.new(|cx| ColorPickerState::new(window, cx));

let _subscription = cx.subscribe(&color_picker, |this, _, ev, _| match ev {
    ColorPickerEvent::Change(color) => {
        if let Some(color) = color {
            if this.validate_contrast(color) {
                this.apply_color(color);
            } else {
                this.show_contrast_warning();
            }
        }
    }
});
```

[ColorPicker]: https://docs.rs/gpui-component/latest/gpui_component/color_picker/struct.ColorPicker.html
[ColorPickerState]: https://docs.rs/gpui-component/latest/gpui_component/color_picker/struct.ColorPickerState.html
[ColorPickerEvent]: https://docs.rs/gpui-component/latest/gpui_component/color_picker/enum.ColorPickerEvent.html

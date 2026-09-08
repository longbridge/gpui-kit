---
title: Settings
description: 用于构建设置页、设置分组和设置项的界面组件。
---

# Settings

> Since: v0.5.0

Settings 组件用于构建应用设置界面，支持页面分组、按标题/描述/自定义关键词搜索过滤以及多种字段类型，适合实现类似 macOS 或 iOS 设置页的结构。

## 导入

```rust
use gpui_kit::component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField};
```

## 用法

### 构建设置界面

可组合以下组件来组织设置页：

- [Settings]：顶层设置容器，持有多个设置页面。
- [SettingPage]：一组相关设置的页面。
- [SettingGroup]：基于 [GroupBox] 风格的设置分组。
- [SettingItem]：单个设置项，包含标题、描述和字段。
- [SettingField]：具体字段，如 Input、Dropdown、Switch 等。

整体层级如下：

```
Settings
  SettingPage
    SettingGroup
      SettingItem
        Title
        Description (optional)
        SettingField
```

### 基础示例

```rust
use gpui_kit::component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField};

Settings::new("my-settings")
    .pages(vec![
        SettingPage::new("General")
            .group(
                SettingGroup::new()
                    .title("Basic Options")
                    .item(
                        SettingItem::new(
                            "Enable Feature",
                            SettingField::switch(
                                |cx: &App| true,
                                |val: bool, cx: &mut App| {
                                    println!("Feature enabled: {}", val);
                                },
                            )
                        )
                    )
            )
    ])
```

### 多页面

:::info
如果你希望某个页面默认展开，可在 [SettingPage] 上使用 `default_open(true)`。
:::

```rust
Settings::new("app-settings")
    .pages(vec![
        SettingPage::new("General")
            .default_open(true)
            .group(SettingGroup::new().title("Appearance").items(vec![...])),
        SettingPage::new("Software Update")
            .group(SettingGroup::new().title("Updates").items(vec![...])),
        SettingPage::new("About")
            .group(SettingGroup::new().items(vec![...])),
    ])
```

### 分组样式

```rust
use gpui_kit::component::group_box::GroupBoxVariant;

Settings::new("my-settings")
    .with_group_variant(GroupBoxVariant::Outline)
    .pages(vec![...])

Settings::new("my-settings")
    .with_group_variant(GroupBoxVariant::Fill)
    .pages(vec![...])
```

## Setting Page

### 基础页面

```rust
SettingPage::new("General")
    .group(SettingGroup::new().title("Options").items(vec![...]))
```

### 多分组

```rust
SettingPage::new("General")
    .groups(vec![
        SettingGroup::new().title("Appearance").items(vec![...]),
        SettingGroup::new().title("Font").items(vec![...]),
        SettingGroup::new().title("Other").items(vec![...]),
    ])
```

### 图标

```rust
SettingPage::new("General")
    .icon(IconName::Settings)
    .groups(vec![...])
```

### 标题后缀

使用 `title_suffix` 在页头标题后渲染自定义元素，例如一个点击后打开帮助文档的 info 图标按钮：

```rust
SettingPage::new("General")
    .title_suffix(|_, _| {
        Button::new("help")
            .icon(IconName::Info)
            .ghost()
            .xsmall()
            .on_click(|_, _, cx| cx.open_url("https://example.com/help"))
    })
    .groups(vec![...])
```

### 默认展开

```rust
SettingPage::new("General")
    .default_open(true)
    .groups(vec![...])
```

### 支持重置

```rust
SettingPage::new("General")
    .resettable(true)
    .groups(vec![...])
```

## Setting Group

### 基础分组

```rust
SettingGroup::new()
    .title("Appearance")
    .items(vec![
        SettingItem::new(...),
        SettingItem::new(...),
    ])
```

### 单项分组

```rust
SettingGroup::new()
    .title("Font")
    .item(SettingItem::new(...))
```

### 无标题分组

```rust
SettingGroup::new()
    .items(vec![...])
```

## Setting Item

### 基础设置项

```rust
SettingItem::new("Title", SettingField::switch(...))
    .description("Description text")
```

### 使用 render closure 的自定义项

```rust
SettingItem::render(|options, _, _| {
    h_flex()
        .w_full()
        .justify_between()
        .child("Custom content")
        .child(
            Button::new("action")
                .label("Action")
                .with_size(options.size)
        )
        .into_any_element()
})
```

### 纵向布局

设置项默认是横向布局；可通过 `layout(Axis::Vertical)` 切换为纵向布局：

```rust
SettingItem::new(
    "CLI Path",
    SettingField::input(...)
)
.layout(Axis::Vertical)
.description("This item uses vertical layout.")
```

### Markdown 描述

```rust
use gpui_kit::component::text::markdown;

SettingItem::new(
    "Documentation",
    SettingField::element(...)
)
.description(markdown("Rust doc for the `gpui-component` crate."))
```

### 禁用状态

通过 `disabled(true)` 可以将设置项置为不可交互状态：整行会变暗，内置字段
（Switch、Checkbox、Input、Dropdown、NumberInput）也会被自动禁用。

```rust
SettingItem::new(
    "Dark Mode",
    SettingField::switch(...)
)
.description("Switch between light and dark themes.")
.disabled(true)
```

对于 [SettingItem::render] 自定义项，整行同样会自动变暗，但其中可交互的控件
需要渲染闭包通过 `options.disabled` 自行处理：

```rust
SettingItem::render(|options, _, _| {
    h_flex()
        .child("Custom content")
        .child(
            Button::new("action")
                .label("Action")
                .with_size(options.size)
                .disabled(options.disabled)
        )
        .into_any_element()
})
.disabled(true)
```

### 搜索关键词

通过 `keywords` 可以为设置项附加额外的搜索关键词。这些关键词仅用于搜索匹配，
不会被渲染出来。例如，标题为 "Enable Two-factor auth" 的设置项可以通过 "MFA"
搜索到：

```rust
SettingItem::new(
    "Enable Two-factor auth",
    SettingField::switch(...)
)
.keywords(["MFA", "2FA"])
```

这对于没有标题和描述、但仍希望能被搜索到的 [SettingItem::render] 自定义项同样
有用：

```rust
SettingItem::render(|options, _, _| {
    h_flex().child("Custom content").into_any_element()
})
.keywords(["Advanced", "Network"])
```

## Setting Fields

[SettingField] 枚举提供了多种常见字段类型。

### Switch

```rust
SettingItem::new(
    "Dark Mode",
    SettingField::switch(
        |cx: &App| cx.theme().mode.is_dark(),
        |val: bool, cx: &mut App| {
            // Handle value change
        },
    )
    .default_value(false)
)
```

### Checkbox

```rust
SettingItem::new(
    "Auto Switch Theme",
    SettingField::checkbox(
        |cx: &App| AppSettings::global(cx).auto_switch_theme,
        |val: bool, cx: &mut App| {
            AppSettings::global_mut(cx).auto_switch_theme = val;
        },
    )
    .default_value(false)
)
```

### Input

```rust
SettingItem::new(
    "CLI Path",
    SettingField::input(
        |cx: &App| AppSettings::global(cx).cli_path.clone(),
        |val: SharedString, cx: &mut App| {
            AppSettings::global_mut(cx).cli_path = val;
        },
    )
    .default_value("/usr/local/bin/bash".into())
)
.layout(Axis::Vertical)
.description("Path to the CLI executable.")
```

### Dropdown

```rust
SettingItem::new(
    "Font Family",
    SettingField::dropdown(
        vec![
            ("Arial".into(), "Arial".into()),
            ("Helvetica".into(), "Helvetica".into()),
            ("Times New Roman".into(), "Times New Roman".into()),
        ],
        |cx: &App| AppSettings::global(cx).font_family.clone(),
        |val: SharedString, cx: &mut App| {
            AppSettings::global_mut(cx).font_family = val;
        },
    )
    .default_value("Arial".into())
)
```

### NumberInput

```rust
use gpui_kit::component::setting::NumberFieldOptions;

SettingItem::new(
    "Font Size",
    SettingField::number_input(
        NumberFieldOptions {
            min: 8.0,
            max: 72.0,
            ..Default::default()
        },
        |cx: &App| AppSettings::global(cx).font_size,
        |val: f64, cx: &mut App| {
            AppSettings::global_mut(cx).font_size = val;
        },
    )
    .default_value(14.0)
)
```

### 使用 render closure 创建自定义字段

```rust
SettingItem::new(
    "GitHub Repository",
    SettingField::render(|options, _window, _cx| {
        Button::new("open-url")
            .outline()
            .label("Repository...")
            .with_size(options.size)
            .on_click(|_, _window, cx| {
                cx.open_url("https://github.com/example/repo");
            })
    })
)
```

### 自定义字段元素

如果某个字段逻辑较复杂并且需要复用，可以实现 [SettingFieldElement] trait：

```rust
use gpui_kit::component::setting::{SettingFieldElement, RenderOptions};

struct OpenURLSettingField {
    label: SharedString,
    url: SharedString,
}

impl SettingFieldElement for OpenURLSettingField {
    type Element = Button;

    fn render_field(&self, options: &RenderOptions, _: &mut Window, _: &mut App) -> Self::Element {
        let url = self.url.clone();
        Button::new("open-url")
            .outline()
            .label(self.label.clone())
            .with_size(options.size)
            .on_click(move |_, _window, cx| {
                cx.open_url(url.as_str());
            })
    }
}
```

然后在设置项中这样使用：

```rust
SettingItem::new(
    "GitHub Repository",
    SettingField::element(OpenURLSettingField {
        label: "Repository...".into(),
        url: "https://github.com/longbridge/gpui-kit".into(),
    })
)
```

## API 参考

- [Settings]
- [SettingPage]
- [SettingGroup]
- [SettingItem]
- [SettingField]
- [NumberFieldOptions]

### 尺寸

实现了 [Sizable] trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸
- `with_size(Size)`：指定具体尺寸

## 示例

### 完整设置页示例

```rust
use gpui_kit::{App, SharedString};
use gpui_kit::component::{
    Settings, SettingPage, SettingGroup, SettingItem, SettingField,
    setting::NumberFieldOptions,
    group_box::GroupBoxVariant,
    Size,
};

Settings::new("app-settings")
    .with_size(Size::Medium)
    .with_group_variant(GroupBoxVariant::Outline)
    .pages(vec![
        SettingPage::new("General")
            .resettable(true)
            .default_open(true)
            .groups(vec![
                SettingGroup::new()
                    .title("Appearance")
                    .items(vec![
                        SettingItem::new(
                            "Dark Mode",
                            SettingField::switch(
                                |cx: &App| cx.theme().mode.is_dark(),
                                |val: bool, cx: &mut App| {
                                    // Handle theme change
                                },
                            )
                        )
                        .description("Switch between light and dark themes."),
                    ]),
                SettingGroup::new()
                    .title("Font")
                    .items(vec![
                        SettingItem::new(
                            "Font Family",
                            SettingField::dropdown(
                                vec![
                                    ("Arial".into(), "Arial".into()),
                                    ("Helvetica".into(), "Helvetica".into()),
                                ],
                                |cx: &App| "Arial".into(),
                                |val: SharedString, cx: &mut App| {
                                    // Handle font change
                                },
                            )
                        ),
                        SettingItem::new(
                            "Font Size",
                            SettingField::number_input(
                                NumberFieldOptions {
                                    min: 8.0,
                                    max: 72.0,
                                    ..Default::default()
                                },
                                |cx: &App| 14.0,
                                |val: f64, cx: &mut App| {
                                    // Handle size change
                                },
                            )
                        ),
                    ]),
            ]),
        SettingPage::new("Software Update")
            .resettable(true)
            .group(
                SettingGroup::new()
                    .title("Updates")
                    .items(vec![
                        SettingItem::new(
                            "Auto Update",
                            SettingField::switch(
                                |cx: &App| true,
                                |val: bool, cx: &mut App| {
                                    // Handle auto update
                                },
                            )
                        )
                        .description("Automatically download and install updates."),
                    ])
            ),
    ])
```

[Settings]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.Settings.html
[SettingPage]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingPage.html
[SettingGroup]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingGroup.html
[SettingItem]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingItem.html
[SettingField]: https://docs.rs/gpui-component/latest/gpui_component/setting/enum.SettingField.html
[SettingFieldElement]: https://docs.rs/gpui-component/latest/gpui_component/setting/trait.SettingFieldElement.html
[NumberFieldOptions]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.NumberFieldOptions.html
[GroupBox]: ./group-box.md
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

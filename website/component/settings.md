---
title: Settings
description: A settings UI with grouped setting items and pages.
---

# Settings

> Since: v0.5.0

The Settings component provides a UI for managing application settings. It includes grouped setting items and pages.
We can search by title, description, and custom keywords to filter the settings to display only relevant settings (Like this macOS, iOS Settings).

## Import

```rust
use gpui_kit::component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField};
```

## Usage

### Build a settings

Here we have components that can be used to build a settings page.

- [Settings] - The main settings component that holds multiple setting pages.
- [SettingPage] - A page of related setting groups.
- [SettingGroup] - A group of related setting items based on [GroupBox] style.
- [SettingItem] - A single setting item with title, description, and field.
- [SettingField] - Provide different field types like Input, Dropdown, Switch, etc.

The layout of the settings is like this:

```
Settings
  SettingPage
    SettingGroup
      SettingItem
        Title
        Description (optional)
        SettingField
```

### Basic Settings

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

### With Multiple Pages

:::info
When you want default expland a page, you can use `default_open(true)` on the [SettingPage].
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

### Group Variants

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

### Basic Page

```rust
SettingPage::new("General")
    .group(SettingGroup::new().title("Options").items(vec![...]))
```

### Multiple Groups

```rust
SettingPage::new("General")
    .groups(vec![
        SettingGroup::new().title("Appearance").items(vec![...]),
        SettingGroup::new().title("Font").items(vec![...]),
        SettingGroup::new().title("Other").items(vec![...]),
    ])
```

### Icon

```rust
SettingPage::new("General")
    .icon(IconName::Settings)
    .groups(vec![...])
```

### Title Suffix

Use `title_suffix` to render a custom element after the title in the page header,
for example an info icon button that opens the help documentation:

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

### Default Open

```rust
SettingPage::new("General")
    .default_open(true)
    .groups(vec![...])
```

### resettable

Enable reset functionality for a page:

```rust
SettingPage::new("General")
    .resettable(true)
    .groups(vec![...])
```

## Setting Group

### Basic Group

```rust
SettingGroup::new()
    .title("Appearance")
    .items(vec![
        SettingItem::new(...),
        SettingItem::new(...),
    ])
```

### Single Item

```rust
SettingGroup::new()
    .title("Font")
    .item(SettingItem::new(...))
```

### Without Title

```rust
SettingGroup::new()
    .items(vec![...])
```

## Setting Item

### Basic Item

```rust
SettingItem::new("Title", SettingField::switch(...))
    .description("Description text")
```

### Custom Item with a render closure

You can create a fully custom setting item using `SettingItem::render`:

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

### Vertical Layout

By default, setting items use horizontal layout. Use `layout(Axis::Vertical)` for vertical layout:

```rust
SettingItem::new(
    "CLI Path",
    SettingField::input(...)
)
.layout(Axis::Vertical)
.description("This item uses vertical layout.")
```

### With Markdown Description

```rust
use gpui_kit::component::text::markdown;

SettingItem::new(
    "Documentation",
    SettingField::element(...)
)
.description(markdown("Rust doc for the `gpui-component` crate."))
```

### Disabled

Use `disabled(true)` to render a setting item in a non-interactive state. The
whole row is dimmed and the built-in field (Switch, Checkbox, Input, Dropdown,
NumberInput) is automatically disabled.

```rust
SettingItem::new(
    "Dark Mode",
    SettingField::switch(...)
)
.description("Switch between light and dark themes.")
.disabled(true)
```

For [SettingItem::render] custom items, the row is still dimmed automatically,
but the renderer is responsible for honoring the disabled state on any
interactive controls inside it via `options.disabled`:

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

### Search Keywords

Use `keywords` to attach additional search terms to an item. They are only used
for search matching and are never rendered. For example, an item titled "Enable
Two-factor auth" can be made searchable via "MFA":

```rust
SettingItem::new(
    "Enable Two-factor auth",
    SettingField::switch(...)
)
.keywords(["MFA", "2FA"])
```

This is also useful for [SettingItem::render] custom items that have no title or
description but should still appear in search results:

```rust
SettingItem::render(|options, _, _| {
    h_flex().child("Custom content").into_any_element()
})
.keywords(["Advanced", "Network"])
```

## Setting Fields

The [SettingField] enum provides different field types for various input needs.

### Switch

The switch field represents a `boolean` on/off state.

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

Like the switch, but uses a checkbox UI.

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

Display a single line text input.

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

A dropdown with a list of options.

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

### Custom Field by Render Closure

The `SettingField::render` method allows you to create a custom field using a closure that returns an element.

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

### Custom Field Element

You may have a complex field that you want to reuse, you may want split the element into a separate struct to do the complex logic.

In this case, the [SettingFieldElement] trait can help you to create a custom field element.

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

Then use it in the setting item:

```rust
SettingItem::new(
    "GitHub Repository",
    SettingField::element(OpenURLSettingField {
        label: "Repository...".into(),
        url: "https://github.com/longbridge/gpui-kit".into(),
    })
)
```

## API Reference

- [Settings]
- [SettingPage]
- [SettingGroup]
- [SettingItem]
- [SettingField]
- [NumberFieldOptions]

### Sizing

Implements [Sizable] trait:

- `xsmall()` - Extra small size
- `small()` - Small size
- `medium()` - Medium size (default)
- `large()` - Large size
- `with_size(Size)` - Set specific size

## Examples

### Complete Settings Example

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

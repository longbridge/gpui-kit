---
title: 主题
description: 使用 GPUI Component 主题系统定制颜色、字体、圆角以及明暗外观。
order: -4
---

# Theme

所有组件都支持内置主题系统。[ActiveTheme] trait 用于访问当前主题中的颜色值：

```rs
use gpui_kit::component::{ActiveTheme as _};

// Access theme colors in your components
cx.theme().primary
cx.theme().background
cx.theme().foreground
```

因此，如果你希望组件使用当前主题的颜色，组件或视图就需要运行在带有 [App] 上下文的环境中。

## 渐变背景

主题颜色值继续兼容既有的字符串格式：

```json
{
  "colors": {
    "button.primary.background": "#4F46E5"
  }
}
```

支持渐变渲染的背景 token 也可以使用 CSS 风格的两段线性渐变：

```json
{
  "colors": {
    "button.primary.background": "linear-gradient(135deg, #4F46E5, #06B6D4)",
    "button.primary.hover.background": "linear-gradient(to right, red-500 25%, blue-600 75%)"
  }
}
```

`cx.theme().button_primary` 等顶层字段仍然是纯色 `Hsla`，保持兼容。需要完整 resolved token 时使用 `cx.theme().tokens.button_primary`；其中 `.color` 是纯色代表色，`.background` 是实际配置的 `Background`，包含渐变。

## Theme Registry

仓库在 [themes](https://github.com/longbridge/gpui-kit/tree/main/themes) 目录下内置了 20+ 主题。

你可以通过 [ThemeRegistry] 来加载和监听这些主题文件：

从 registry 查找主题时使用 `themes` 数组中条目的 `name`，例如 `Ayu Light`。

```rs
use std::path::PathBuf;
use gpui_kit::{App, SharedString};
use gpui_kit::component::{Theme, ThemeRegistry};

pub fn init(cx: &mut App) {
    let theme_name = SharedString::from("Ayu Light");
    // Load and watch themes from ./themes directory
    if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
        if let Some(theme) = ThemeRegistry::global(cx)
            .themes()
            .get(&theme_name)
            .cloned()
        {
            Theme::global_mut(cx).apply_config(&theme);
        }
    }) {
        tracing::error!("Failed to watch themes directory: {}", err);
    }
}
```

[ActiveTheme]: https://docs.rs/gpui-component/latest/gpui_component/theme/trait.ActiveTheme.html
[ThemeRegistry]: https://docs.rs/gpui-component/latest/gpui_component/theme/struct.ThemeRegistry.html
[App]: https://docs.rs/gpui/latest/gpui/struct.App.html

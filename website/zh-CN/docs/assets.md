---
title: Icon
description: 为 GPUI Component 应用配置内置图标、自定义 SVG 与资源加载方式。
order: -4
---

# Icon

GPUI Component 中的 [IconName] 和 [Icon] 提供了一套可直接在 GPUI 应用中使用的图标接口。

但为了尽量减小应用体积，`gpui-component` 默认 **不会内置任何图标资源**。

因此仓库把图标资源拆分到了独立的 [gpui-kit-assets] crate 中。这样你可以自行决定：

- 直接使用默认内置图标资源
- 完全不引入图标资源
- 自己维护一套 SVG 资源

## 使用默认内置资源

[gpui-kit-assets] 提供了一个默认的资源实现，包含 `assets/icons` 目录下的全部图标文件。

如果要使用默认资源，需要在 `Cargo.toml` 中添加：

```toml
[dependencies]
gpui-component = { git = "https://github.com/longbridge/gpui-kit" }
gpui-kit-assets = { git = "https://github.com/longbridge/gpui-kit" }
```

然后在创建 GPUI 应用时，通过 `with_assets` 注册资源源：

```rs
use gpui_kit::*;
use gpui_kit::assets::Assets;

let app = gpui_kit::application().with_assets(Assets);
```

完成后，你就可以像平常一样使用 `IconName` 和 `Icon`。这些图标会从默认打包资源中读取。

继续阅读下面的 [使用图标](#使用图标) 小节查看实际示例。

## 自定义资源

如果你只想带上一小部分图标，或者希望使用项目自己的 SVG 资源，可以自己构建资源源。

仓库中的 [assets] 目录包含了目前支持的全部 SVG 图标文件，文件名与 [IconName] 枚举一一对应。

你可以：

- 直接从 [assets] 目录拷贝需要的 SVG
- 或按 [IconName] 的命名规则准备自己的 SVG 文件

在 GPUI 应用中，通常可以结合 [rust-embed] 将这些 SVG 嵌入可执行文件，并通过 `AssetSource` 提供加载能力。

```rs
use anyhow::anyhow;
use gpui_kit::*;
use gpui_kit::component::{v_flex, IconName, Root};
use rust_embed::RustEmbed;
use std::borrow::Cow;

/// An asset source that loads assets from the `./assets` folder.
#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}
```

同样需要在创建应用时调用 `with_assets`：

```rs
fn main() {
    // Register Assets to GPUI application.
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        // We must initialize gpui_component before using it.
        gpui_kit::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Example);
                // The first level on the window must be Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

## 使用图标

完成资源注册后，就可以在应用中直接使用图标：

```rs
pub struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .text_center()
            .child(IconName::Inbox)
            .child(IconName::Bot)
    }
}
```

## 单独嵌入 SVG 图标

自定义图标可以通过 `Icon::data` 直接传入 SVG 字节，无须维护资源路径注册表：

```rust
use gpui_kit::component::{Icon, button::Button};

Button::new("search")
    .icon(Icon::default().data(include_bytes!("search.svg")))
    .label("Search")
```

这样可以省去该图标的资源查找。内置 `IconName` 和组件中使用的其他路径图标仍需要资源源。
数据所有权、来源替换、加载图标与自定义图标类型的说明见
[SVG 字节](./components/icon.md#svg-字节)。

## 参考资源

- [Lucide Icons](https://lucide.dev/) - GPUI Component 的图标集主要基于 Lucide 开源图标库

[rust-embed]: https://docs.rs/rust-embed/latest/rust_embed/
[IconName]: https://docs.rs/gpui_component/latest/gpui_component/icon/enum.IconName.html
[Icon]: https://docs.rs/gpui_component/latest/gpui_component/icon/struct.Icon.html
[assets]: https://github.com/longbridge/gpui-kit/tree/main/crates/assets/assets/
[gpui-kit-assets]: https://crates.io/crates/gpui-kit-assets

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


:::note NOTE — 依赖图标 crate 不等于嵌入全部图标

**补全图标目录不会让现有应用自动嵌入全部图标。** `Assets` 保留原来的
101 个组件图标，应用仍通过自己的 `AssetSource` 提供额外图标，不需要重新声明
组件自带的图标。只有显式注册 `AllAssets`，原生程序才会嵌入全部 1,830 个 SVG。
仅依赖 crate 或使用共享 `IconName` 不会引用全部 SVG 内容。

| 原生资源配置 | 嵌入的 SVG 总量 | 相对默认 `Assets` 的二进制增量 |
| --- | ---: | ---: |
| 默认组件图标（101 个） | 44.28 KiB | 0 B（基线） |
| 默认 + 2 个应用图标（103 个） | 45.04 KiB | +15.19 KiB |
| 默认 + 10 个应用图标（111 个） | 48.09 KiB | +19.19 KiB |
| 显式使用 `AllAssets`（1,830 个） | 731.45 KiB | +1.02 MiB |

**本例中，额外使用 10 个应用图标增加约 19 KiB，并不会带入整个图标库。**
这 10 个 SVG 合计 3,903 字节，二进制实际增加 19,648 字节，包含额外资源源的查找、
列表合并代码、元数据和对齐开销。这不是固定的单图标成本，也不是整个应用的大小。

测量环境：Lucide 1.43.0、Linux x86_64、Rust 1.98.0、`--release` 并移除符号。
各组使用相同的 `IconName` 查找和运行时资源路径。额外资源源回退到 `Assets`，
并合并、排序、去重两个资源源的列表。10 个额外图标为 `Accessibility`、
`AlarmClock`、`Archive`、`Award`、`Backpack`、`Bike`、`Bird`、`Camera`、
`Coffee` 和 `Compass`；两图标组使用前两个。实际结果取决于 SVG 复杂度、工具链
和资源源的实现方式。

二进制大小不等于内存占用。按需资源借用静态字节，不复制或创建缓存；实际渲染仍有
解析、栅格化和渲染缓存的开销。运行时共享名称查找可能保留名称映射表，Cargo 下载包
和构建产物也仍包含完整目录。WASM 的 `Assets::new(endpoint)` 和
`AllAssets::new(endpoint)` 沿用按需下载的 CDN 加载器，不嵌入完整资源包。

:::

## 共享名称与兼容性

`gpui_kit::assets::IconName` 提供不依赖 Component 的完整共享目录。
`gpui_kit::component::IconName` 保留为原来的兼容枚举：现有导入、穷尽匹配和
`.view(cx)` 调用均无需改动，也无需新增 trait 导入。`Icon::new(...)` 同时接受
两种类型；旧名称可以通过 `.into()` 转为共享名称。

对于新的共享枚举，需要组件实体时使用 `Icon::new(name).view(cx)`，也可导入
`gpui_kit::component::IconNameExt` 后使用 `name.view(cx)`。

`IconName::ALL` 列出完整的 1,830 个名称，`IconName::Accessibility.path()` 返回
`icons/accessibility.svg`。默认资源源只包含原来的 101 个组件图标；额外图标请使用
下文的自定义资源源，或者显式注册 `AllAssets` 使用完整资源包。

## 使用默认内置资源

[gpui-kit-assets] 提供了一个默认的资源实现，包含 `crates/assets/default-icons.txt` 中列出的原有 101 个组件图标。

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
use gpui_kit::*;
use gpui_kit::assets::Assets as ComponentAssets;
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

        if let Some(file) = Self::get(path) {
            return Ok(Some(file.data));
        }
        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = ComponentAssets.list(path)?;
        paths.extend(Self::iter().filter_map(|p| p.starts_with(path).then(|| p.into())));
        paths.sort();
        paths.dedup();
        Ok(paths)
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
[SVG 字节](../component/icon.md#svg-字节)。

## 参考资源

- [Lucide Icons](https://lucide.dev/) - GPUI Component 的图标集主要基于 Lucide 开源图标库

[rust-embed]: https://docs.rs/rust-embed/latest/rust_embed/
[IconName]: https://docs.rs/gpui-kit-assets/latest/gpui_kit_assets/enum.IconName.html
[Icon]: https://docs.rs/gpui_component/latest/gpui_component/icon/struct.Icon.html
[assets]: https://github.com/longbridge/gpui-kit/tree/main/crates/assets/assets/
[gpui-kit-assets]: https://crates.io/crates/gpui-kit-assets

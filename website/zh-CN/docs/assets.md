---
title: Icons & Assets
description: 为 GPUI Kit 应用配置内置图标、自定义 SVG 与资源加载方式。
order: -7
---

# Icons & Assets

GPUI Kit 通过 `gpui_kit::assets` 和 `gpui_kit::component` 提供 [IconName] 与 [Icon]。`gpui-kit` 默认 feature 会启用这两层，但应用仍需注册 `AssetSource` 才能按路径加载图标。底层的 [gpui-kit-assets] crate 将 SVG 资源与组件代码分开，因此你可以：

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
和构建产物也仍包含完整目录。[WebAssembly](./webassembly) 的 `Assets::new(endpoint)` 和
`AllAssets::new(endpoint)` 沿用按需下载的 CDN 加载器，不嵌入完整资源包。

:::

## 共享名称与兼容性

`gpui_kit::assets::IconName` 提供不依赖 Component 的完整共享目录。
`gpui_kit::component::IconName` 保留为原来的兼容枚举：现有导入、穷尽匹配和
`.view(cx)` 调用均无需改动，也无需新增 trait 导入。`Icon::new(...)` 同时接受
两种类型；旧名称可以通过 `.into()` 转为共享名称。

对于新的共享枚举，需要组件 [Entity](./entity) 时使用 `Icon::new(name).view(cx)`，也可导入
`gpui_kit::component::IconNameExt` 后使用 `name.view(cx)`。

`IconName::ALL` 列出完整的 1,830 个名称，`IconName::Accessibility.path()` 返回
`icons/accessibility.svg`。默认资源源只包含原来的 101 个组件图标；额外图标请使用
下文的自定义资源源，或者显式注册 `AllAssets` 使用完整资源包。

## 从默认资源源开始

`gpui_kit::assets::Assets` 是默认资源源，包含 [`default-icons.txt`](https://github.com/longbridge/gpui-kit/blob/main/crates/assets/default-icons.txt) 中列出的原有 101 个组件图标。

在 `Cargo.toml` 中添加总入口 crate；它的默认 feature 包含 `component` 与 `assets`：

```toml
[dependencies]
gpui-kit = "0.6"
```

对于原生桌面应用，应在打开窗口前注册资源源。下面是可作为 `src/main.rs` 使用的完整示例：

```rust
use gpui_kit::*;
use gpui_kit::assets::Assets;
use gpui_kit::component::{Icon, IconName};

struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(Icon::new(IconName::Inbox))
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example)
        })
        .expect("Failed to open window");
    });
}
```

`with_assets` 为整个应用安装一个资源源。`gpui_kit::init(cx)` 在创建窗口前初始化组件层。`IconName::Inbox` 对应 `icons/inbox.svg`，GPUI 将这个精确路径交给资源源。仅添加 crate 依赖不会注册资源源。完整应用初始化流程见[快速开始](./getting-started.md)。

## 用 `icon_assets!` 选择更多目录图标

应用已经依赖 `gpui-kit` 时，直接使用 `gpui_kit::assets::icon_assets!`；默认启用的 `assets` feature 已经提供宏和共享目录。**只为使用该宏，无须再添加一项 `gpui-kit-assets` 依赖。** 如果某个 crate 有意独立使用资产层，而不依赖门面 crate，可以直接依赖 `gpui-kit-assets`，调用 `gpui_kit_assets::icon_assets!`。

按应用实际绘制的图标选择资源源：

| 需求 | 传给 `with_assets` 的资源源 | 原生程序嵌入的图标数据 |
| --- | --- | --- |
| 只用默认组件图标 | `Assets` | 101 个默认 SVG |
| 默认图标加少量目录图标 | 组合 `Assets` 与 `icon_assets!` | 默认图标加选中的图标 |
| 使用全部目录图标 | `AllAssets` | 全部 1,830 个 SVG |
| 不使用 Component，只用少量目录图标 | 宏生成的资源源 | 仅选中的图标 |

宏参数必须是 **`IconName` 枚举成员的标识符**，不是字符串、路径，也不是自备 SVG 的文件名。例如，`Accessibility` 选中 `icons/accessibility.svg`。宏生成的 `ExtraIcons` 是实现了 `AssetSource` 的单元结构体：`load` 对选中路径返回借用的嵌入字节，对其他路径返回 `Ok(None)`；`list(prefix)` 列出匹配前缀的已选路径。选取发生在编译期；枚举成员拼写错误或不存在会编译失败。需要在其他模块使用时，宏也接受可见性修饰符，例如 `icon_assets!(pub ExtraIcons, [Accessibility]);`。

下面是完整的 `src/main.rs`，选取两个额外的 Lucide 图标，同时保留默认组件图标。示例适用于原生桌面应用；使用上文的 `gpui-kit` 依赖即可，无须拷贝 SVG 或新增 crate。

```rust
use gpui_kit::*;
use gpui_kit::assets::{Assets as ComponentAssets, icon_assets};
use gpui_kit::component::Icon;
use std::borrow::Cow;

icon_assets!(ExtraIcons, [Accessibility, AlarmClock]);

struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIcons.load(path)? {
            return Ok(Some(bytes));
        }
        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = ComponentAssets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(Icon::new(gpui_kit::assets::IconName::Accessibility))
            .child(Icon::new(gpui_kit::assets::IconName::AlarmClock))
    }
}

fn main() {
    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example)
        })
        .expect("Failed to open window");
    });
}
```

`with_assets` 只能注册一个资源源，因此 `AppAssets` 同时负责两个查找。必须先查询 `ExtraIcons`：原生平台的 `ComponentAssets.load` 对未知路径会报错，阻止后续回退。`list` 合并、排序并去重两个来源，使列出的资源键与 `load` 能提供的键一致。只有应用不需要默认组件图标时，才适合单独注册宏生成的资源源。

从手写的 Lucide 图标资源查找迁移时，可用一条 `icon_assets!(ExtraIcons, [...]);` 声明替换复制的 SVG 字节或文件名匹配分支。其他应用文件继续留在独立资源源中，查询该来源及 `ExtraIcons` 后再回退到默认 `Assets`，并合并三个来源的 `list` 结果。确认没有其他代码按路径读取旧 SVG 文件后，再删除旧副本。`icons/brand-mark.svg` 等自备文件不属于目录，仍需下文的自定义资源源。`IconName::ALL` 只列出名称，并不表示已注册资源源可以加载每个名称。

快速检查时，调用 `AppAssets.load("icons/accessibility.svg")` 和 `AppAssets.load("icons/inbox.svg")`；原生平台上两者都应返回 `Some`。`AppAssets.list("icons/")` 也应包含这两个路径。额外图标空白时，检查绘制的 `IconName` 是否列在宏中，以及注册的是 `AppAssets` 而非 `ComponentAssets`。在 WebAssembly 上，宏生成的资源源仍可嵌入选中图标，但内置 `Assets` 需要带 endpoint 构造，并作为组合资源源的字段保存。

## 自定义资源

`icon_assets!` 只能选择 GPUI Kit 已命名目录中的图标。应用自己的标志、插画、照片或其他图片，应由应用的 `AssetSource` 提供。仓库中的 [assets] 目录提供目录 SVG；把新文件放入应用**不会**生成新的 `IconName` 枚举成员。`icons/brand-mark.svg` 这样的资源键相对于嵌入目录，不是 URL，也不依赖程序运行时的工作目录。

下面的示例嵌入一个自有 SVG 和一个图片目录。在应用的 `Cargo.toml` 旁建立这些文件：

```text
Cargo.toml
src/main.rs
assets/icons/brand-mark.svg
assets/images/cover.png
```

除 `gpui-kit` 外，添加 [rust-embed]：

```toml
[dependencies]
gpui-kit = "0.6"
rust-embed = { version = "8.7", features = ["include-exclude"] }
```

下面是完整的原生平台 `src/main.rs`：优先提供应用文件，再回退到 GPUI Kit 的默认组件图标。`images/**/*` 会嵌入该目录下的**每个文件**，所以目录中只应放要发布的资源；也可以改用更窄的 `#[include]` 规则。示例在打开窗口前注册一个组合资源源。

```rs
use gpui_kit::*;
use gpui_kit::assets::Assets as ComponentAssets;
use gpui_kit::component::Icon;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
#[include = "images/**/*"]
struct AppFiles;

struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if let Some(file) = AppFiles::get(path) {
            return Ok(Some(file.data));
        }
        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = ComponentAssets.list(path)?;
        paths.extend(AppFiles::iter().filter_map(|p| p.starts_with(path).then(|| p.into())));
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Icon::default().path("icons/brand-mark.svg"))
            .child(img("images/cover.png").w(px(240.)).h(px(160.)).object_fit(ObjectFit::Cover))
    }
}

fn main() {
    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example)
        })
        .expect("Failed to open window");
    });
}
```

`AssetSource::load(path)` 根据精确资源键提供字节，`list(prefix)` 报告匹配的键。此例中，自有文件发生键冲突时优先；其他路径交给 `ComponentAssets`。原生平台上的内置资源源遇到未知非空路径会报错。如果应用也使用 `icon_assets!`，应在这个最终回退之前查询宏生成的资源源，并按上文方式将其路径合并到 `list`。在 WASM 上，需以 endpoint 构造 `Assets::new(endpoint)`，并将其值保存在组合资源源中，而不能使用原生平台的单元结构体。

## 使用资源路径

使用注册资源源提供的路径。`Icon::path` 用于 SVG 图标；`IconName` 把已知名称映射到这些路径。

```rust
use gpui_kit::component::{Icon, IconName};

let built_in = Icon::new(IconName::Inbox);
let custom = Icon::default().path("icons/brand-mark.svg");
let extra = Icon::new(gpui_kit::assets::IconName::Accessibility);
```

`extra` 需要上面的选取图标资源源或 `AllAssets`；`custom` 需要包含 `icons/brand-mark.svg` 的资源源。需要随文字颜色变化的**单色 SVG**，使用 `Icon::path` 或 `svg().path(...)`：GPUI 将其绘制为可着色的透明度蒙版。需要时指定尺寸和 `text_color`。照片、其他位图，以及要保留原始颜色的**多色 SVG**，使用 `img()`：

```rust
use gpui_kit::{ObjectFit, StyledImage, img, px, svg};

let tinted_mark = svg().path("icons/brand-mark.svg").size(px(24.));
let cover = img("images/cover.png")
    .w(px(240.))
    .h(px(160.))
    .object_fit(ObjectFit::Cover);
let color_artwork = img("images/illustration.svg").size(px(160.));
```

`img()` 支持常见的 PNG、JPEG、WebP、GIF、SVG，以及 GPUI 的 `Img::extensions()` 列出的其他格式。它通过字节识别位图格式，SVG 则走另一条解码路径；因此文件内容也必须有效，不能只改扩展名。默认的 `ObjectFit::Contain` 会在边界内完整显示图片；`Cover` 会铺满并可能裁剪，`Fill` 可能拉伸变形。`object_fit` 作用于 `img()`，不作用于单色 `svg()` 元素。`img()` 通过 GPUI 的图片缓存异步加载并解码；嵌入资源的字节会复制进加载器，嵌入本身不会省掉解码或运行时图片内存。动画 GIF 和 WebP 可以包含多个帧。

对于 `"images/cover.png"` 这样的字符串，GPUI 会用该键调用已注册的 `AssetSource`。`img(std::path::Path::new("/absolute/file.png"))` 读取文件系统路径，URL 字符串使用 HTTP 加载器；它们的部署方式和错误处理不同。各种来源如何加载、布局和缓存，见[图片](./image.md)。

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

## 排查资源缺失

| 现象 | 检查项 |
| --- | --- |
| 默认组件图标空白 | 确认打开窗口前调用了 `.with_assets(Assets)`，或注册了包含默认图标的组合资源源；核对路径以 `icons/` 开头、以 `.svg` 结尾。 |
| 共享目录中的图标空白 | 确认名称属于已注册资源源。`Assets` 只有 101 个默认图标；其他名称需要选取图标资源源或 `AllAssets`。 |
| 自定义图标空白 | 将 `Icon::path` 中的键与 `#[folder]` 下的相对路径逐字比较，并检查 `#[include]` 和大小写。可用 `list("icons/")` 查看资源源提供的键。 |
| 图片空白 | 确认传入的是资源键、文件系统 `Path` 还是 URL；检查资源源是否包含位图文件，以及字节是否为受支持的图片格式。 |

在原生平台，内置 `Assets` 和 `AllAssets` 遇到缺失路径会返回错误；空路径返回 `Ok(None)`。组合资源源应先查询自定义资源，再回退到内置资源。`Icon::data` 可免去该 SVG 的路径查找，但无效 SVG 字节仍可能无法绘制。

## 打包与 WebAssembly

原生平台的 `Assets` 只嵌入默认 SVG，`AllAssets` 则嵌入完整目录。`icon_assets!` 嵌入选中的 SVG 字节；`rust-embed` 嵌入 include 规则匹配的文件。源文件路径在构建时解析，打包后的原生应用运行时不需要源项目的 `assets/` 目录。传给 `img()` 的文件系统 `Path` 不同：目标文件必须在应用运行的位置存在。上面的大小测量仅供参考，请测量自己应用的 release 二进制。

在 WebAssembly 上，`Assets::new(endpoint)` 和 `AllAssets::new(endpoint)` 使用同一个按需 HTTP 资源源。对于 `icons/*.svg`，它请求 `endpoint + "/assets/" + path`，缓存成功响应，并在请求期间返回临时加载错误。请托管精确路径，并使用不带尾部斜线的 endpoint。遇到图标空白时检查浏览器的 Network 和 Console 中的状态码、CORS 和 URL；下载完成后加载器本身不会主动要求重绘。此资源源的 `list()` 返回空列表。上面的原生 `rust-embed` 示例是另一种选择，也可以用于在 WASM 中显式嵌入应用文件。部署细节见 [WebAssembly](./webassembly.md)。

## 参考资源

- [Lucide Icons](https://lucide.dev/) - GPUI Kit 的图标目录主要基于 Lucide 开源图标库。

[rust-embed]: https://docs.rs/rust-embed/latest/rust_embed/
[IconName]: https://docs.rs/gpui-kit-assets/0.6.5/gpui_kit_assets/enum.IconName.html
[Icon]: https://docs.rs/gpui-component/0.6.5/gpui_component/struct.Icon.html
[assets]: https://github.com/longbridge/gpui-kit/tree/main/crates/assets/assets/icons
[gpui-kit-assets]: https://docs.rs/crate/gpui-kit-assets/0.6.5

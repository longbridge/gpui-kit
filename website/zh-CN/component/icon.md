---
title: Icon
description: 以不同尺寸、颜色和变换方式显示 SVG 图标。
---

# Icon

Icon 支持通过资源路径或内存中的字节渲染 SVG 图标，并可定制尺寸、颜色与变换。内置的 Lucide 图标使用资源包；自定义 SVG 字节可以通过 `Icon::data` 直接传入。

在开始之前，建议先阅读 [Icons & Assets](../docs/assets.md)，了解如何在 GPUI 与 GPUI Component 应用中使用 SVG。

`gpui_kit::assets::IconName` 提供不依赖 Component 的完整共享目录。
`gpui_kit::component::IconName` 保留为原来的兼容枚举：现有导入、穷尽匹配和
`.view(cx)` 调用均无需改动，也无需新增 trait 导入。`Icon::new(...)` 同时接受
两种类型；旧名称可以通过 `.into()` 转为共享名称。

对于新的共享枚举，需要组件实体时使用 `Icon::new(name).view(cx)`，也可导入
`gpui_kit::component::IconNameExt` 后使用 `name.view(cx)`。

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


## 应用额外图标

照常注册默认 `Assets`，应用额外需要的 SVG 由自己的 `AssetSource` 提供，
并在未找到时回退到默认资源源。也可以用可选的 `icon_assets!` 声明应用额外使用的
内置 SVG，然后将其资源源与默认资源源组合。详见 [Icons & Assets](../docs/assets.md)。


## 导入

```rust
use gpui_kit::component::{Icon, IconName};
```

## 用法

### 基础图标

```rust
IconName::Heart

Icon::new(IconName::Heart)
```

### 自定义尺寸

```rust
Icon::new(IconName::Search).xsmall()
Icon::new(IconName::Search).small()
Icon::new(IconName::Search).medium()
Icon::new(IconName::Search).large()

Icon::new(IconName::Search).with_size(px(20.))
```

### 自定义颜色

```rust
Icon::new(IconName::Heart)
    .text_color(cx.theme().red)

Icon::new(IconName::Star)
    .text_color(gpui_kit::red())
```

### 旋转图标

```rust
use gpui_kit::{Transformation, radians};

Icon::new(IconName::ArrowUp)
    .rotate(radians(std::f32::consts::FRAC_PI_2))

Icon::new(IconName::ChevronRight)
    .transform(Transformation::rotate(radians(std::f32::consts::PI)))
```

### 自定义 SVG 路径

```rust
Icon::new(Icon::empty())
    .path("icons/my-custom-icon.svg")
```

### SVG 字节

通过 `data(&[u8])` 传入 SVG 字节，无须为该图标注册 `AssetSource` 路径：

```rust
use gpui_kit::component::{Icon, button::Button, menu::PopupMenuItem};

let icon = Icon::default().data(include_bytes!("search.svg"));

Button::new("search").icon(icon.clone()).label("Search");
PopupMenuItem::new("Search").icon(icon);
```

`data` 会将输入复制到共享存储中，因此输入无须具有 `'static` 生命周期。
克隆 `Icon` 时会共享这些字节，并保留样式和变换。直接渲染与通过
`Icon::view(cx)` 创建实体视图都会保留数据源。GPUI 渲染器可能再次复制字节，
因此此 API 不承诺渲染过程零复制。

最后一次设置的数据源生效，即使新来源为空也会替换旧来源：

```rust
let bytes = include_bytes!("search.svg");
Icon::default().path("icons/old.svg").data(bytes); // 使用 SVG 字节
Icon::default().data(bytes).path("icons/search.svg"); // 使用资源路径
```

字节图标与路径图标使用相同的 SVG 渲染器，保留组件尺寸、前景色与按钮加载行为。
可以通过 `loading_icon` 指定自定义加载图标：

```rust
Button::new("search")
    .icon(Icon::default().data(include_bytes!("search.svg")))
    .loading_icon(Icon::default().data(include_bytes!("loader.svg")))
    .loading(true)
    .label("Searching")
```

`NativeMenu::menu_with_icon` 也支持字节图标，尺寸与着色继续遵循现有原生菜单规则。
应用或组件中使用的其他路径图标仍需要资源源。

### 使用 SVG 字节的自定义图标类型

图标 crate 可以导出独立类型，并实现 `From<T> for Icon`：

```rust
use gpui_kit::component::{Icon, button::Button};

pub struct Search;

impl From<Search> for Icon {
    fn from(_: Search) -> Self {
        Icon::default().data(include_bytes!("search.svg"))
    }
}

Button::new("search").icon(Search);
```

现有 `IconNamed` 实现继续提供资源路径。使用字节的类型实现上述转换即可，
无须同时实现 `IconNamed`。二进制体积能否缩小取决于实际引用的资源和构建配置。

## 可用图标

`IconName` 枚举内置了一组常见图标：

### 导航

- `ArrowUp`、`ArrowDown`、`ArrowLeft`、`ArrowRight`
- `ChevronUp`、`ChevronDown`、`ChevronLeft`、`ChevronRight`
- `ChevronsUpDown`

### 操作

- `Check`、`Close`、`Plus`、`Minus`
- `Copy`、`Delete`、`Search`、`Replace`
- `Maximize`、`Minimize`、`WindowRestore`

### 文件与文件夹

- `File`、`Folder`、`FolderOpen`、`FolderClosed`
- `BookOpen`、`Inbox`

### UI 元素

- `Menu`、`Settings`、`Settings2`、`Ellipsis`、`EllipsisVertical`
- `Eye`、`EyeOff`、`Bell`、`Info`

### 社交与外链

- `GitHub`、`Globe`、`ExternalLink`
- `Heart`、`HeartOff`、`Star`、`StarOff`
- `ThumbsUp`、`ThumbsDown`

### 状态与提醒

- `CircleCheck`、`CircleX`、`TriangleAlert`
- `Loader`、`LoaderCircle`

### 面板与布局

- `PanelLeft`、`PanelRight`、`PanelBottom`
- `PanelLeftOpen`、`PanelRightOpen`、`PanelBottomOpen`
- `LayoutDashboard`、`Frame`

### 用户与身份

- `User`、`CircleUser`、`Bot`

### 其它

- `Calendar`、`Map`、`Palette`、`Inspector`
- `Sun`、`Moon`、`Building2`

## 图标尺寸

| 尺寸 | 方法 | CSS Class | 像素 |
| ----------- | --------------------- | ------------ | ------ |
| 超小 | `.xsmall()` | `size_3()` | 12px |
| 小 | `.small()` | `size_3p5()` | 14px |
| 中 | `.medium()` | `size_4()` | 16px |
| 大 | `.large()` | `size_6()` | 24px |
| 自定义 | `.with_size(px(n))` | - | n px |

## 自定义 `IconName`

如果你需要更贴合业务的图标命名，可以自己定义 `IconName` 并实现 `IconNamed` trait。

```rust
use gpui_kit::component::IconNamed;

pub enum IconName {
    Encounters,
    Monsters,
    Spells,
}

impl IconNamed for IconName {
    fn path(self) -> gpui_kit::SharedString {
        match self {
            IconName::Encounters => "icons/encounters.svg",
            IconName::Monsters => "icons/monsters.svg",
            IconName::Spells => "icons/spells.svg",
        }
        .into()
    }
}

Button::new("my-button").icon(IconName::Spells);
Icon::new(IconName::Monsters);
```

如果你希望在元素树中直接 `render` 自定义 `IconName`，还需要实现 `RenderOnce` 并为 `IconName` 派生 `IntoElement`：

```rust
impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::empty().path(self.path())
    }
}

div()
    .child(IconName::Monsters)
```

## 示例

### 按钮中的图标

```rust
use gpui_kit::component::button::Button;

Button::new("like-btn")
    .icon(
        Icon::new(IconName::Heart)
            .text_color(cx.theme().red)
            .large()
    )
    .label("Like")
```

### 旋转加载图标

```rust
Icon::new(IconName::LoaderCircle)
    .text_color(cx.theme().muted_foreground)
    .medium()
```

### 状态图标

```rust
Icon::new(IconName::CircleCheck)
    .text_color(cx.theme().green)

Icon::new(IconName::CircleX)
    .text_color(cx.theme().red)

Icon::new(IconName::TriangleAlert)
    .text_color(cx.theme().yellow)
```

### 导航图标

```rust
Icon::new(IconName::ArrowLeft)
    .medium()
    .text_color(cx.theme().foreground)

Icon::new(IconName::ChevronDown)
    .small()
    .text_color(cx.theme().muted_foreground)
```

### 来自资源包的自定义图标

```rust
Icon::empty()
    .path("icons/my-brand-logo.svg")
    .large()
    .text_color(cx.theme().primary)
```

## 说明

- 图标以 SVG 形式渲染，可使用完整的样式能力。
- 如果未显式指定尺寸，默认尺寸会跟随当前文字大小。
- 图标默认带有 `flex-shrink-0`，避免在 Flex 布局中被意外压缩。
- 所有图标路径都相对于 assets bundle 根目录。
- Lucide.dev 图标在 16px 下效果最佳，并且在其它尺寸下也有良好缩放表现。

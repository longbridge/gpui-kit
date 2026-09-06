---
title: Icon
description: 以不同尺寸、颜色和变换方式显示 SVG 图标。
---

# Icon

Icon 支持通过资源路径或内存中的字节渲染 SVG 图标，并可定制尺寸、颜色与变换。内置的 Lucide 图标使用资源包；自定义 SVG 字节可以通过 `Icon::data` 直接传入。

在开始之前，建议先阅读 [Icons & Assets](../assets.md)，了解如何在 GPUI 与 GPUI Component 应用中使用 SVG。

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

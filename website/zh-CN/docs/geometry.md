---
title: Geometry
description: 在 GPUI 中使用带单位的坐标、布局长度与颜色。
order: -2.8
---

# 几何与颜色

GPUI 用类型说明数字的*含义*：坐标是 `Point<T>`，尺寸是 `Size<T>`，矩形是 `Bounds<T>`；`T` 指明各分量的单位。布局阶段可以使用尚需父容器尺寸才能确定的长度，绘制与命中通常使用已确定的 `Pixels`。颜色也分为直接表达通道的 `Rgba` 与便于调整色相的 `Hsla`。GPUI Kit 会重新导出 GPUI，应用示例可以从 `use gpui_kit::*;` 开始。

## Point、Size 与 Bounds

| 类型 | 字段 | 含义 |
| --- | --- | --- |
| `Point<T>` | `x`、`y` | 坐标系中的位置。 |
| `Size<T>` | `width`、`height` | 不包含位置的宽高。 |
| `Bounds<T>` | `origin: Point<T>`、`size: Size<T>` | 与坐标轴平行的矩形。 |

构造函数 `point(x, y)`、`size(width, height)`、`bounds(origin, size)` 会根据参数推断 `T`。它们也接受普通数值类型，但 UI 几何一般使用 `Pixels`：

```rust
use gpui_kit::*;

let frame: Bounds<Pixels> = bounds(
    point(px(20.), px(40.)),
    size(px(240.), px(80.)),
);
assert_eq!(frame.right(), px(260.));
assert_eq!(frame.bottom(), px(120.));
assert!(frame.contains(&point(px(20.), px(40.))));
assert!(!frame.contains(&point(px(260.), px(40.))));

let midpoint: Point<Pixels> = frame.center();
let local = point(px(35.), px(55.)).relative_to(&frame.origin);
assert_eq!(local, point(px(15.), px(15.)));
```

`right()`、`bottom()` 把宽高加到原点上。`contains()` 包含上边和左边，不包含下边和右边，因此相邻矩形不会同时认领边界上的点。`center()`、`intersects()`、`Bounds::from_corners(...)`、`Bounds::centered_at(...)` 也可用于对齐和定位。局部坐标和窗口坐标都可能是 `Point<Pixels>`：类型负责检查单位，代码仍须明确坐标原点。绘制局部数据时加上 bounds 的 origin；将指针位置换算到元素内部时减去 origin。

这些值通常在布局之后出现。自定义 [`Element`](./element) 在 `prepaint` 获得 `Bounds<Pixels>`，可以据此建立 hitbox，并在后续[绘制](./paint)中使用同一套坐标。`Bounds` 本身不会让区域自动具备交互能力。

## 为什么不直接用整数或浮点数？

`px(12.)` 产生 `Pixels`，内部以 `f32` 保存。文字度量、动画及最终栅格化前的位置都可能有小数，整数会丢失这部分精度。裸 `f32` 还可能表示坐标、缩放倍数、透明度或父容器比例，编译器无法检查混用。像素距离之间可以加减，距离乘以标量后仍是像素距离：

```rust
let inset: Pixels = px(8.);
let width: Pixels = px(120.) - inset * 2.;
let raw: f32 = width.as_f32(); // 只在需要 f32 的 API 边界转换。
```

`Pixels` 表示 GPUI 的逻辑 UI 像素，不一定是显示器的物理像素。`Pixels::scale(factor)` 得到 `ScaledPixels`；`DevicePixels` 表示整数设备像素数。例如，`px(12.).scale(2.)` 是 24 个缩放后像素，但其类型仍不同于 `DevicePixels(24)`。跨越显示缩放或栅格化边界时，应区分这些单位。不过，同为 `Point<Pixels>` 的两个值是否使用同一原点，以及宽度是否非负，仍须由应用代码保证。

## 布局前的 Length 与布局后的 Pixels

[样式](./style)长度可能依赖上下文。GPUI 用嵌套类型表达这种差异：

| 类型 | 取值 | 用途 |
| --- | --- | --- |
| `AbsoluteLength` | `Pixels` 或 `Rems` | 固定 UI 长度，或跟随根文字尺寸的长度。 |
| `DefiniteLength` | `AbsoluteLength` 或父容器比例 | 已指定数值、但可能仍需父尺寸的长度。 |
| `Length` | `DefiniteLength` 或 `Auto` | 也可由布局引擎自动决定的长度。 |

`px(24.)` 产生 `Pixels`，`rems(1.5)` 产生 `Rems`，`relative(0.5)` 产生 `DefiniteLength::Fraction(0.5)`，即相关父尺寸的一半。`auto()` 产生 `Length::Auto`。`Pixels`、`Rems`、`DefiniteLength` 都可转换成 `Length`；具体样式方法接受哪种类型，以其签名为准，在链式调用中可让 Rust 推断转换：

```rust
use gpui_kit::*;

let panel = div()
    .w(relative(0.5))
    .min_w(px(240.))
    .h(rems(3.));
```

父容器的尺寸确定之前，宽度仍是相对值；rem 也需要根 rem 尺寸。`auto` 是请求布局按规则决定数值，不等于零。GPUI 将这些值传给布局引擎，再得到像素 bounds。

`Percentage` 是另一个独立的 `Percentage(f32)` 包装类型，可用 `percentage(0.25)` 构造。这个辅助函数期望 `0.0` 到 `1.0` 的比例（调试构建中会断言）；GPUI 可把它转换为一整圈中相同比例的 `Radians`。它**不是** `relative(0.25)` 用来表示 25% 布局宽度的类型。布局比例使用 `relative`，圆周比例使用 `percentage`。

## RGBA 与 HSLA

[`Rgba`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Rgba.html) 保存红、绿、蓝及 alpha 通道，分量是 0 到 1 的 `f32`。`rgb(0x3366CC)` 读取六位 RGB 十六进制值，alpha 为 1；`rgba(0x3366CC80)` 读取 **RRGGBBAA** 顺序的八位值，其中 alpha 为 `128 / 255`（约 0.502）。GPUI 的 [`Hsla`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Hsla.html) 保存色相、饱和度、亮度及 alpha，分量同样归一化到 0 至 1。其 `hsla(0.6, 0.8, 0.5, 1.)` 构造函数使用色相比例，而不是角度，也会把四个输入限制在这个范围内。主题色及其交互状态默认使用 `Hsla`；需要处理 RGB 通道或读取十六进制颜色时再使用 `Rgba`。

```rust
use gpui_kit::*;

let source: Rgba = rgb(0x3366CC);
let tint: Hsla = source.into();
let translucent = tint.opacity(0.5); // 乘以当前 alpha。
let exact_alpha = tint.alpha(0.5);   // 替换 alpha。
let again: Rgba = translucent.to_rgb();
let from_hex_alpha: Rgba = rgba(0x3366CC80);
let red_channel: f32 = from_hex_alpha.r;
```

十六进制资源、通道数值和颜色合成用 RGBA 更直接；想单独调整色相、饱和度或亮度时，HSLA 更方便。GPUI 可以在两者之间转换，但浮点运算可能产生舍入差异，不能保证逐字节完全往返。HSL 的亮度也不等于人眼感知亮度：不同色相即使 `l` 相同，看起来也可能一明一暗。需要按感知方式插值时，GPUI Kit 提供 `Colorize::mix_oklab`。

GPUI 的 `Hsla::blend(other)` 通过 RGBA 转换把 `other` 叠到 `self` 上。其中 `Rgba::blend` 按上层颜色的 alpha 插值 RGB，并保留接收方的 alpha；它适合不透明背景的情形，不应当作两个半透明图层的一般 alpha 合成公式。

## 主题颜色与交互状态

GPUI Kit 用 `Hsla` 保存语义主题色，也为组件提供解析后的 token。主按钮会分别使用 `button_primary`、`button_primary_hover`、`button_primary_active` token。因此使用现成组件时，应采用主题 token，而不是就地调整亮度。主题可以显式提供每个 token，背景还可能是渐变。未提供某个 token 时，GPUI Kit 才按主题生成后备值：主色 hover 先将主色原有 alpha 乘以 0.9，再与背景混合；主色 active 在浅色模式将亮度乘以 0.9，深色模式乘以 0.8。主按钮状态 token 再回退到对应的主色状态 token。其他变体有各自的后备规则，没有统一的 hover 公式。

自定义纯色时，`Colorize` 为 `Hsla` 提供 `lighten`、`darken`、`hue`、`saturation` 和 `lightness`：

```rust
use gpui_kit::*;
use gpui_kit::component::Colorize;

let base: Hsla = hsla(0.6, 0.8, 0.5, 1.);
let darker = base.darken(0.1); // l = base.l * (1 - 0.1)
let quieter = base.opacity(0.6); // a = base.a * 0.6
```

`lighten(f)` 将亮度乘以 `1 + f`，`darken(f)` 乘以 `1 - f`，并非直接增减百分点。与 `hsla(...)` 构造函数不同，`Colorize::lighten` 不会限制计算结果，可能产生大于 1 的 `l`，用作主题色之前应检查。`Colorize::opacity` 与 GPUI 的 `Hsla::opacity` 都是乘以 alpha。控件状态优先使用语义主题 token；定义新颜色时，应检查文字对比度及明暗两种主题。

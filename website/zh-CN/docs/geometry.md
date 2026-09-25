---
title: Geometry
description: 在 GPUI 中使用带单位的坐标、布局长度与颜色。
order: -2.8
---

# 几何与颜色

GPUI 用类型说明数字的*含义*：坐标是 `Point<T>`，尺寸是 `Size<T>`，矩形是 `Bounds<T>`；`T` 指明各分量的单位。布局阶段可以使用尚需父容器尺寸才能确定的长度，绘制与命中通常使用已确定的 `Pixels`。颜色则区分便于调整色相的 `Hsla` 与直接表达通道的 `Rgba`。GPUI Kit 会重新导出 GPUI，应用示例可以从 `use gpui_kit::*;` 开始。

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

## 明确坐标原点

`Point<Pixels>` 记录单位，却不记录坐标系。窗口内容的左上角通常是窗口坐标的原点；元素自身的左上角是另一个原点。一次计算中同时出现两种坐标时，用变量名标明：

```rust
use gpui_kit::*;

let element_bounds = bounds(point(px(100.), px(60.)), size(px(80.), px(40.));
let pointer_in_window = point(px(125.), px(75.));
let pointer_in_element = pointer_in_window.relative_to(&element_bounds.origin);
assert_eq!(pointer_in_element, point(px(25.), px(15.)));

let marker_in_element = point(px(10.), px(8.));
let marker_in_window = element_bounds.origin + marker_in_element;
assert_eq!(marker_in_window, point(px(110.), px(68.)));
```

自定义元素在 `prepaint` 和 `paint` 收到的 `bounds` 已经位于窗口坐标中。指针事件的 `position` 也采用窗口坐标，因此可直接用 `element_bounds.contains(&pointer_in_window)` 判断；只有计算元素内部的字符或拖动手柄位置时才转换为局部坐标。不要给已经基于 `bounds` 的矩形再次加上 `element_bounds.origin`。反过来，虽然局部点和窗口 hitbox 都使用 `Point<Pixels>`，也不能直接比较。

## 从布局到输入与绘制

三个阶段分别处理不同问题：

| 阶段 | 已有的几何信息 | 职责 |
| --- | --- | --- |
| `request_layout` | 样式长度和布局节点；有些长度仍依赖父容器。 | 返回 `LayoutId`，交给布局引擎求解。 |
| `prepaint` | 当前帧已确定的 `Bounds<Pixels>`。 | 准备几何数据；需要交互区域时调用 `window.insert_hitbox(bounds, HitboxBehavior::Normal)`。 |
| `paint` | 已确定的 bounds 和准备阶段的状态。 | 用 `window.paint_quad(fill(bounds, color))` 等方法绘制，并注册当前帧的输入监听器。 |

布局树、事件派发树中的 hitbox、最终绘制场景是不同结构。绘制矩形不会自动使它可点击；插入 hitbox 也不会画出矩形。`insert_hitbox` 返回的 hitbox 可作为 `PrepaintState` 传到 `paint`，监听器再用 `hitbox.is_hovered_at(event.position, window)` 判断指针。hitbox 会记录插入时生效的 content mask，因此应先确定裁剪范围，再插入子元素的 hitbox。完整的事件处理示例见[自定义 Element 教程](./element)。

## 滚动、裁剪与计算练习

滚动视口和内容使用不同的原点。GPUI Kit 的[虚拟列表实现](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/virtual_list.rs)使用**负数**滚动偏移；它把偏移加到 item 的窗口位置，并在视口的 `ContentMask` 下对 item 执行 `prepaint`。`visible_range` 先在内容坐标中筛选 item，再由 mask 限制绘制和输入范围。筛选可见 item 本身并不会裁剪其像素。

### 运行坐标计算练习

从仓库根目录创建 `examples/hello_world/src/bin/geometry_walkthrough.rs`（若没有 `bin` 目录，先创建）。它是现有 `hello_world` 包内的第二个 binary，不是新 crate。粘贴以下完整程序，然后运行 `cargo run -p hello_world --bin geometry_walkthrough`。

纵向视口在窗口坐标 `(100, 60)`，大小为 `80 × 40`。Item A 从内容坐标 `y = 20` 开始，高度为 `20`，滚动偏移为 `-30`。指针位置使用窗口坐标。程序将它转换为 item 局部坐标，并计算各 item 与视口的交集：

```rust
use gpui_kit::*;

fn main() {
    let viewport = bounds(point(px(100.), px(60.)), size(px(80.), px(40.)));
    let scroll_y = px(-30.);
    let item_a = bounds(
        viewport.origin + point(px(0.), px(20.) + scroll_y),
        size(px(80.), px(20.)),
    );
    assert_eq!(item_a.origin, point(px(100.), px(50.)));

    let pointer_in_window = point(px(105.), px(65.));
    let pointer_in_item = pointer_in_window.relative_to(&item_a.origin);
    assert_eq!(pointer_in_item, point(px(5.), px(15.)));

    let visible_a = item_a.intersect(&viewport);
    assert_eq!(visible_a.origin, point(px(100.), px(60.)));
    assert_eq!(visible_a.size, size(px(80.), px(10.)));
    assert!(visible_a.contains(&pointer_in_window));

    let item_b = bounds(
        viewport.origin + point(px(0.), px(80.) + scroll_y),
        size(px(80.), px(20.)),
    );
    assert_eq!(item_b.origin.y, px(110.));
    assert!(!item_b.intersects(&viewport));

    println!(
        "Item A: local pointer = ({}, {}), visible height = {}; Item B visible = {}",
        pointer_in_item.x.as_f32(),
        pointer_in_item.y.as_f32(),
        visible_a.size.height.as_f32(),
        item_b.intersects(&viewport),
    );
}
```

预期输出为 `Item A: local pointer = (5, 15), visible height = 10; Item B visible = false`。若滚动偏移方向或坐标原点弄错，断言会立即失败。Item A 的顶部超出视口 10 像素，只有底部 10 像素与视口相交。Item B 位于窗口 `y = 110`，超过视口底边 `y = 100`。运行后可以删除练习文件。

`intersect()` 只计算矩形，**不会**自行裁剪绘制和输入。真正编写自定义元素时，应在子元素的 `prepaint` 和 `paint` 期间应用视口 `ContentMask`，让子元素的 hitbox 继承 mask，并让绘制像素受其限制，然后通过得到的 hitbox 判断指针。普通滚动容器会替你处理这些步骤。自定义滚动容器还应像虚拟列表那样把偏移限制在内容范围内，并让内容定位、裁剪和 hitbox 使用一致的坐标。

常见错误包括：在布局前把 `relative(0.5)` 当成 0.5 像素；把局部点与窗口 bounds 比较；定位内容时减去负数滚动偏移；或以为 `contains()` 会裁剪已绘制的子元素。遇到错位时，先写清每个中间值的原点与单位，再查看最终 bounds 和当前 content mask。

## Edges、Side 与 Placement

`Edges<T>` 按 `top`、`right`、`bottom`、`left` 保存四个独立值。`Edges<Pixels>` 可表示已确定的 padding、border 或窗口四周留白。`Edges::all(value)` 给四边设置相同值；各边不同时直接填写字段：

```rust
use gpui_kit::*;

let padding: Edges<Pixels> = Edges {
    top: px(8.),
    right: px(12.),
    bottom: px(8.),
    left: px(12.),
};
let uniform = Edges::all(px(4.));
```

`use gpui_kit::*` 导入的是 GPUI 的 `Edges`。GPUI Kit 另有可序列化、可生成 JSON schema 的 `gpui_kit::base::Edges<T>`，也通过 `gpui_kit::component::Edges<T>` 导出。它与 GPUI 的类型字段相同，但在 Rust 中是不同类型；应按目标 API 的签名选择。

[`Placement`](https://docs.rs/gpui-base/latest/gpui_base/enum.Placement.html) 表示 trigger 的**一侧**：`Top`、`Right`、`Bottom` 或 `Left`，不是四边留白或矩形。例如，`Positioner::side` 将 `Placement::Bottom` 视为优先方向；下方空间不足时可以翻到 `Top`，随后把结果限制在 viewport 内。`ResolvedPosition::placement` 记录最终选中的方向：

```rust
use gpui_kit::*;
use gpui_kit::base::{Align, Placement, Positioner};

let trigger_bounds = bounds(point(px(40.), px(40.)), size(px(100.), px(32.)));
let popup = Positioner::side(trigger_bounds)
    .placement(Placement::Bottom)
    .align(Align::Start)
    .offset(px(8.))
    .child(div().child("Menu"));
```

这里的 `trigger_bounds` 是窗口坐标中的 `Bounds<Pixels>`。GPUI Kit 的 `Side` 只表示 `Left` 或 `Right`，`Axis` 表示 `Horizontal` 或 `Vertical`。GPUI 的 `Anchor` 指定 `TopLeft`、`BottomCenter` 等参考点；`Corners<T>` 保存四个角的值，常用于圆角半径。它们与 `Placement` 的用途不同。窗口坐标与缩放详见 [Window](./window)。

## 为什么不直接用整数或浮点数？

`px(12.)` 产生 `Pixels`，内部以 `f32` 保存。文字度量、动画及最终栅格化前的位置都可能有小数，整数会丢失这部分精度。裸 `f32` 还可能表示坐标、缩放倍数、透明度或父容器比例，编译器无法检查混用。像素距离之间可以加减，距离乘以标量后仍是像素距离：

```rust
let inset: Pixels = px(8.);
let width: Pixels = px(120.) - inset * 2.;
let raw: f32 = width.as_f32(); // Convert only at API boundaries that require f32.
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

## HSLA 与 RGBA

GPUI 的 [`Hsla`](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.Hsla.html) 保存色相、饱和度、亮度及 alpha，分量是 0 到 1 的 `f32`。其 `hsla(0.6, 0.8, 0.5, 1.)` 构造函数使用色相比例，而不是角度，也会把四个输入限制在这个范围内。主题色及其交互状态默认使用 `Hsla`。[`Rgba`](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.Rgba.html) 保存红、绿、蓝及 alpha 通道，分量也为 0 到 1。`rgb(0x3366CC)` 读取六位 RGB 十六进制值，alpha 为 1；`rgba(0x3366CC80)` 读取 **RRGGBBAA** 顺序的八位值，其中 alpha 为 `128 / 255`（约 0.502）。需要处理 RGB 通道或读取十六进制颜色时再使用 `Rgba`。

```rust
use gpui_kit::*;

let tint: Hsla = hsla(0.6, 0.8, 0.5, 1.);
let translucent = tint.opacity(0.5); // Multiply the current alpha.
let exact_alpha = tint.alpha(0.5);   // Replace the alpha.
let again: Rgba = translucent.to_rgb();
let source: Rgba = rgb(0x3366CC);
let from_hex_alpha: Rgba = rgba(0x3366CC80);
let red_channel: f32 = from_hex_alpha.r;
```

想单独调整色相、饱和度或亮度时，HSLA 更方便；十六进制资源、通道数值和颜色合成用 RGBA 更直接。GPUI 可以在两者之间转换，但浮点运算可能产生舍入差异，不能保证逐字节完全往返。HSL 的亮度也不等于人眼感知亮度：不同色相即使 `l` 相同，看起来也可能一明一暗。需要按感知方式插值时，GPUI Kit 提供 `Colorize::mix_oklab`。

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

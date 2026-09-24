---
title: Paint
description: 理解 GPUI 的自定义图形绘制，以及布局、命中和绘制之间的边界。
order: -2.75
---

# Paint

GPUI 在布局与 `prepaint` 之后执行 `paint`。底层 [`Element`](./element) 拿到最终的 `Bounds<Pixels>`，才能确定绘制坐标。`paint` 向当前帧提交绘制命令，不负责确定布局或输入命中区域。矩形和边框使用 `window.paint_quad`，普通图文优先使用现有 Element，自由曲线和不规则形状使用 `window.paint_path`。只需绘制少量自定义图形时，可以使用 `canvas(prepaint, paint)`，无需实现完整的 `Element` trait。

## 构建 Path

`PathBuilder` 描述矢量路径，`build()` 成功后会把它三角化成 `Path<Pixels>`。封闭区域选 `fill()`，线条选 `stroke(width)`。路径点使用窗口像素坐标；若数据以 Element 左上角为原点，先加上最终 bounds 的 origin。

```rust
use gpui_kit::*;

fn paint_triangle(bounds: Bounds<Pixels>, color: Hsla, window: &mut Window) {
    let mut builder = PathBuilder::fill();
    builder.move_to(bounds.origin);
    builder.line_to(point(bounds.right(), bounds.top()));
    builder.line_to(point(bounds.left() + bounds.size.width / 2., bounds.bottom()));
    builder.close();

    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
```

`move_to`、`line_to`、`curve_to`（二次贝塞尔）、`cubic_bezier_to`、`arc_to`、`add_polygon` 和 `close` 用于描述路径段；`translate`、`scale`、`rotate` 可在三角化前变换路径。`PathBuilder::stroke(px(1.)).dash_array(&[px(4.), px(2.)])` 可以得到虚线。`build()` 返回 `Result`，任意几何输入不保证总能成功。路径几何没有改变时，可复用已构建的 `Path`，并用不同颜色绘制。

### 从 SVG Path 迁移

`PathBuilder` 最初是为 K 线图的绘制需求引入 GPUI 的。它内部使用 Lyon 的 SVG path builder，因此路径段词汇与 SVG 接近。熟悉 [SVG `d` 路径命令](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/d)的人，可以直接迁移路径段的概念：

| SVG Path | GPUI builder | 含义 |
| --- | --- | --- |
| `M x y` | `move_to(point)` | 开始子路径。 |
| `L x y` | `line_to(point)` | 直线。 |
| `Q cx cy x y` | `curve_to(end, control)` | 二次贝塞尔曲线。 |
| `C c1x c1y c2x c2y x y` | `cubic_bezier_to(end, control_a, control_b)` | 三次贝塞尔曲线。 |
| `A rx ry rotation large sweep x y` | `arc_to(radii, rotation, large_arc, sweep, end)` | 椭圆弧。 |
| `Z` | `close()` | 闭合子路径。 |

几何概念一致，但 Rust 参数顺序并非把 SVG 字符串逐字搬过来：`curve_to` 与 `cubic_bezier_to` **先传终点，再传控制点**。当前 API 的 `x_rotation` 参数类型是 `Pixels`，其数值按角度解释。坐标用 `Point<Pixels>` 表示；`build()` 先三角化，再由 `paint_path` 提交绘制。因此迁移 SVG 绘图算法的学习成本很低，同时要遵循 GPUI 的坐标类型和错误处理规则。

### 同一根 K 线，两种 Path 写法

切换 GPUI 与 SVG 源码。两者按相同顺序连接十二个角点；下方图形就是这条 SVG Path 的实际渲染。这是把影线与实体合成一条轮廓的教学示例；[GPUI Kit 的 K 线图](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/chart/candlestick_chart.rs)实际用描边 Path 绘制影线、用 quad 绘制实体。示例使用局部 100 × 128 坐标系；真实 GPUI `canvas` 绘制时，须把 `bounds.origin` 加到各点上。

<div class="doc-tabs">
  <input class="doc-tabs__input" type="radio" name="candle-source-zh" id="candle-rust-zh" checked>
  <input class="doc-tabs__input" type="radio" name="candle-source-zh" id="candle-svg-zh">
  <div class="doc-tabs__list" role="tablist" aria-label="K 线路径源码">
    <label for="candle-rust-zh" role="tab">GPUI PathBuilder</label>
    <label for="candle-svg-zh" role="tab">SVG Path</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <pre><code class="language-rust">let mut builder = PathBuilder::fill();
builder.move_to(point(px(48.), px(16.)));
builder.line_to(point(px(52.), px(16.)));
builder.line_to(point(px(52.), px(42.)));
builder.line_to(point(px(68.), px(42.)));
builder.line_to(point(px(68.), px(90.)));
builder.line_to(point(px(52.), px(90.)));
builder.line_to(point(px(52.), px(112.)));
builder.line_to(point(px(48.), px(112.)));
builder.line_to(point(px(48.), px(90.)));
builder.line_to(point(px(32.), px(90.)));
builder.line_to(point(px(32.), px(42.)));
builder.line_to(point(px(48.), px(42.)));
builder.close();
if let Ok(path) = builder.build() {
    window.paint_path(path, color);
}</code></pre>
    </section>
    <section class="doc-tabs__panel">
      <pre><code class="language-svg">&lt;svg viewBox=&quot;0 0 100 128&quot;&gt;
  &lt;path fill=&quot;currentColor&quot;
    d=&quot;M48 16 L52 16 L52 42 L68 42 L68 90
       L52 90 L52 112 L48 112 L48 90 L32 90
       L32 42 L48 42 Z&quot; /&gt;
&lt;/svg&gt;</code></pre>
    </section>
  </div>
</div>

<figure class="path-preview">
  <svg viewBox="0 0 100 128" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="candle-title-zh">
    <title id="candle-title-zh">按上方 SVG Path 绘制的 K 线</title>
    <path fill="currentColor" d="M48 16 L52 16 L52 42 L68 42 L68 90 L52 90 L52 112 L48 112 L48 90 L32 90 L32 42 L48 42 Z" />
  </svg>
  <figcaption>一条填充路径同时构成影线与实体。</figcaption>
</figure>

## 哪个阶段完成什么

| 工作 | 阶段 | 原因 |
| --- | --- | --- |
| 声明尺寸与子节点布局 | `request_layout` | Taffy 此时需要 Style，但最终 bounds 尚不存在。 |
| 根据最终 bounds 构建路径、插入 `Hitbox` | `prepaint` | 几何坐标和当前帧的输入区域已经确定。 |
| 调用 `paint_path`、`paint_quad` 或绘制子节点 | `paint` | 这时可以确定绘制顺序。 |

[GPUI Kit Plot 的折线](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/shape/line.rs) 从数据点构建描边路径；[输入框的底层 Element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 用 Path 绘制选区与文本范围装饰，而闪烁光标用 quad 绘制。两者都使用 `PathBuilder`，但输入框还必须协调文字度量和命中测试。

可以用 `window.with_content_mask` 限制绘制区域。绘制、输入 hitbox 与[无障碍语义](./accessibility)彼此独立：画出图形不会自动使它可点击，也不会自动让辅助技术读到它。交互图表还需建立命中区域或指针坐标映射；有语义的数据还需提供无障碍表示。路径三角化有成本，点和尺寸没变时不要重复构建；如果路径存的是窗口绝对坐标，窗口内位置变化时必须更新缓存。简单矩形则优先使用 `paint_quad` 或带样式的 `div()`。

## GPUI Kit 的三种状态归属

相同的绘制原语可对应不同的状态归属。缓存放在哪里，取决于数据属于谁、会保留多久，以及变化频率。

### 模型驱动的仪表：Entity 保留几何

由 [Entity](./entity) 驱动的仪表可以分别保存背景弧线、数值弧线和指针三个 `Option<Path<Pixels>>`。数值变化时只清除后两者；若 Path 使用窗口绝对坐标，origin 变化时清除全部。`canvas` 的 prepaint callback 根据 bounds 构建缺失路径，paint callback 用当前主题色绘制。这样几何失效与颜色选择彼此独立。已有模型订阅的 View 适合持有这些缓存，但不要在每次 prepaint 都无条件调用 `cx.notify()`，否则可能每帧额外安排一次 render。

### Plot：每帧重建的值使用 keyed window state

缓存依赖跨帧稳定的 [ElementId](./element_id)。[`Line`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/shape/line.rs) 是 render 时创建的值；如果缓存存在它身上，下一帧就会丢失。[`PathCaches::for_paint`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/path_cache.rs) 在当前 Plot 的 Element ID 下使用 `window.use_keyed_state`。`Line::paint_cached` 把投影后的点、描边宽度和曲线样式组成 shape key；`PathCache::get` 仅在 key 改变时重新三角化。路径以零原点构建，再克隆缓存顶点并平移到当前帧的 origin，所以滚动不会迫使曲线重新三角化，但平移本身仍有成本。数据点则用成本较低的 quad 在新 origin 绘制。这个模式要求 Element ID 稳定；同一个 series 在各帧使用同一个 slot 能提高缓存命中率。若系列按下标重排且 shape key 不同，会造成原本可避免的缓存失效。

### Input：文本编辑器统筹整个 Element 管线

GPUI Kit 的[输入框底层 Element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 不只绘制选区 Path。它测量文字、计算折行与视口几何，在 prepaint 中插入 hitbox，再根据准备好的布局绘制选区 Path、光标 quad 和文字。输入处理与 Focus 也依赖同一份几何结果。复杂编辑器需要自己实现 `Element`，因为文字布局、命中、输入路由和绘制必须共享同一份快照；选区和文本范围装饰的 Path 只是管线的一部分。

选 API 时可据此递进：已有 Entity 只需画一处装饰，用 `canvas`；每帧重建的值需要跨帧缓存，用稳定 ID 对应的 window state；文字、布局、命中和输入必须直接协同时，实现 `Element`。Trait 的三个阶段见 [Element](./element)，输入传播见 [Event](./event)。

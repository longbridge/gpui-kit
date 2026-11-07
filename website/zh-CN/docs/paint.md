---
title: Paint
description: 理解 GPUI 的自定义图形绘制，以及布局、命中和绘制之间的边界。
order: -2.75
---

# Paint

GPUI 在布局与 `prepaint` 之后执行 `paint`。底层 [`Element`](./element) 拿到最终的 `Bounds<Pixels>`，才能确定绘制坐标。`paint` 向当前帧提交绘制命令，不负责确定布局或输入命中区域。矩形和边框使用 `window.paint_quad`，普通图文优先使用现有 Element，自由曲线和不规则形状使用 `window.paint_path`。只需绘制少量自定义图形时，可以使用 `canvas(prepaint, paint)`，无需实现完整的 `Element` trait。

## 从一个三角形开始

这个练习不需要新建 crate 或添加依赖。在本地检出中，将现有 `examples/hello_world/src/main.rs` 的内容暂时替换为下面的代码，再从仓库根目录运行 `cargo run -p hello_world`。窗口左上附近应出现一个蓝色三角形。如果不想保留练习代码，完成后恢复该示例文件。

```rust
use gpui_kit::*;

struct PaintedTriangle;

impl Render for PaintedTriangle {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            canvas(
                |bounds, _, _| bounds,
                |_, bounds, window, _| {
                    let mut path = PathBuilder::fill();
                    let x = bounds.origin.x;
                    let y = bounds.origin.y;
                    path.move_to(point(x + px(24.), y + px(24.)));
                    path.line_to(point(x + px(144.), y + px(24.)));
                    path.line_to(point(x + px(84.), y + px(128.)));
                    path.close();

                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgb(0x3b82f6));
                    }
                },
            )
            .size_full(),
        )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| PaintedTriangle)
        })
        .expect("failed to open window");
    });
}
```

父 `div` 和子 `canvas` 都设置了 `.size_full()`，布局因此会给回调一个可用的矩形。第一个回调在 `prepaint` 阶段执行，并把最终 bounds 作为状态 `T` 返回；第二个回调在 `paint` 阶段收到这份值，用窗口坐标构建一条填充路径，再把 `Path<Pixels>` 交给 `paint_path`。`close()` 会把最后一个点与起点连接起来。`if let Ok` 处理三角化失败的情况，避免直接 panic。

两个 `canvas` 回调都是单次 Element 流程中的 `FnOnce` 回调。如果绘制需要准备好的数据，就从第一个回调返回拥有所有权的值；不要把 `Window` 或 `App` 的引用保存到下一帧。`canvas` 本身是当前帧的 Element；若绘图数据需要跨 render 保留，应放入 Entity 或 keyed window state。

试着把第三个点的 `px(84.)` 改成 `px(120.)` 后重新运行：变化的只有三角形的几何形状。再把 `PathBuilder::fill()` 改成 `PathBuilder::stroke(px(4.))`，可以画出轮廓。GPUI 渲染新的一帧时会重新创建元素树；这里的 Path 也会在 paint 时重新构建。小型装饰图形这样写很直接；大型或频繁重绘的路径应在测量成本后考虑缓存。

这里最重要的是坐标交接：

```text
layout: canvas bounds = origin (x, y) + size (width, height)
prepaint: pass final bounds to paint
paint: local point (24, 24) + origin (x, y) -> window point -> PathBuilder
```

bounds 建立坐标系，但不会自动把路径裁剪在 canvas 内。图形可能越过 canvas 边界；如需限制溢出，应给容器设置合适的裁剪样式，或使用 content mask。

## 让绘制响应点击

将 `examples/hello_world/src/main.rs` 替换为下面的完整示例，再运行 `cargo run -p hello_world`。每次左键点击都会在指针位置添加一个蓝色小三角形。后续点击不会抹掉已有图形，因为位置保存在 `ClickPainter` Entity 中，而不是只存在于当前帧的 `canvas` 值里。

```rust
use gpui_kit::base::ElementExt;
use gpui_kit::*;

struct ClickPainter {
    marks: Vec<Point<Pixels>>,
    canvas_bounds: Option<Bounds<Pixels>>,
}

impl ClickPainter {
    fn add_mark(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(bounds) = self.canvas_bounds {
            self.marks.push(point(
                event.position.x - bounds.origin.x,
                event.position.y - bounds.origin.y,
            ));
            cx.notify();
        }
    }
}

impl Render for ClickPainter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let marks = self.marks.clone();
        let painter = cx.entity().clone();

        div()
            .size_full()
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::add_mark))
            .on_prepaint(move |bounds, _, cx| {
                painter.update(cx, |this, _| {
                    this.canvas_bounds = Some(bounds);
                });
            })
            .child(
                canvas(
                    move |bounds, _, _| (bounds.origin, marks),
                    |_, (origin, marks), window, _| {
                        for mark in marks {
                            let center = point(origin.x + mark.x, origin.y + mark.y);
                            let mut path = PathBuilder::fill();
                            path.move_to(point(center.x, center.y - px(16.)));
                            path.line_to(point(center.x + px(16.), center.y + px(12.)));
                            path.line_to(point(center.x - px(16.), center.y + px(12.)));
                            path.close();
                            if let Ok(path) = path.build() {
                                window.paint_path(path, rgb(0x3b82f6));
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| ClickPainter {
                marks: Vec::new(),
                canvas_bounds: None,
            })
        })
        .expect("failed to open window");
    });
}
```

父 `div` 接收鼠标事件。它的 `on_prepaint` 钩子保存最终 bounds，供下次输入使用；保存 bounds 本身不请求新一帧。`add_mark` 减去该 bounds 的 origin，以 canvas 局部坐标保存位置，然后用 `cx.notify()` 请求新一帧。下一次 render 时，canvas 收到标记的快照；paint 回调加上**当前** origin，为每个标记构建路径。试着点击两次并调整窗口大小：两个标记应保持相对 canvas 的位置。如果点击没有结果，依次检查父容器尺寸是否非零、`canvas_bounds` 是否已保存，以及 handler 修改 `marks` 后是否调用 `cx.notify()`。

## 跟随真实绘图应用

在仓库根目录运行现有的 [Brush 示例](https://github.com/longbridge/gpui-kit/blob/main/examples/brush/src/main.rs)：

```sh
cargo run -p example-brush
```

在 **Drawing Canvas** 内拖动鼠标画一笔，再试试 **Size**、**Opacity**、颜色块、**Show Grid** 和 **Clear Canvas**。下面沿着示例中一笔画的输入、布局和绘制流程逐步阅读。

先记住一帧的顺序：鼠标 handler 修改 `BrushStory` 中保留的状态并调用 `cx.notify()`；`render` 创建新元素树；布局确定尺寸和位置；`prepaint` 收到 bounds；`paint` 提交本帧的路径。保存的笔画点跨帧存在，而此示例中的 `Path` 对象会在绘制时重新构建。

### 1. 为 canvas 分配空间并接收输入

`render_canvas` 把绘图 canvas 放在占满容器的 `div` 中。外层可伸缩的 **Drawing Canvas** 区域提供空间；`div` 处理鼠标事件，子 canvas 占满同一区域：

```rust
let base_div = div()
    .id("canvas")
    .size_full()
    .bg(theme.background)
    .cursor_crosshair()
    .relative()
    .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
    .on_mouse_move(cx.listener(Self::handle_mouse_move))
    .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
    .on_prepaint(move |bounds, _window, cx| {
        state_entity.update(cx, |state, _| {
            state.canvas_bounds = Some(bounds);
        })
    });
```

示例把 `canvas(...).absolute().size_full()` 作为这个 `div` 的子元素。canvas 需要自身样式或父布局分配尺寸，否则可能没有可绘制区域。父元素的 `on_prepaint` 保存最终的 `Bounds<Pixels>`，供鼠标 handler 使用。bounds 和鼠标位置都是**窗口坐标**，因此 canvas 的 origin 通常不是 `(0, 0)`。

`on_prepaint` 保存 bounds 时没有调用 `cx.notify()`：它只记录后续输入所需的几何信息，不会在每帧绘制过程中再次请求渲染。

### 2. 以 canvas 局部坐标保存点

按下鼠标时，`BrushStory::handle_mouse_down` 开始一条 `Stroke`。它先减去保存的 origin，再存储指针位置；鼠标移动时也使用同样的转换：

```rust
let local_pos = if let Some(bounds) = self.canvas_bounds {
    Point::new(
        event.position.x - bounds.origin.x,
        event.position.y - bounds.origin.y,
    )
} else {
    event.position
};
```

`BrushStory` 在 `strokes` 中保留完成的笔画，在 `current_stroke` 中保留正在绘制的笔画。handler 修改笔画时调用 `cx.notify()`，让 GPUI 渲染新的一帧。只有鼠标按下产生的一个点还不能形成可见线条：`build_stroke_path` 至少需要两个点。松开鼠标后，两个点及以上的笔画会加入已完成列表。

### 3. 把最终 bounds 传给绘制阶段

在 `render_canvas` 中，canvas 的第一个回调接收最终 bounds，连同笔画、当前笔画、网格开关和主题一起返回。第二个回调取得这些值并绘制。下面展示数据交接；[源码](https://github.com/longbridge/gpui-kit/blob/main/examples/brush/src/main.rs)还绘制了可选网格和正在画的笔画：

```rust
canvas(
    move |bounds, _window, _cx| {
        (
            strokes_for_prepaint,
            current_stroke_for_prepaint,
            show_grid_for_prepaint,
            theme_for_prepaint,
            bounds,
        )
    },
    move |_bounds,
          (strokes, current_stroke, show_grid, theme, prepaint_bounds),
          window,
          _cx| {
        for stroke in strokes.iter() {
            if let Some(path) = BrushStory::build_stroke_path(stroke, &prepaint_bounds) {
                window.paint_path(path, stroke.color);
            }
        }
        // The example also paints the grid and current_stroke here.
    },
)
.absolute()
.size_full()
```

`prepaint` 发生在布局完成之后，此时 bounds 已确定。`paint_path` 提交当前帧的绘制命令；它本身不会修改状态，也不会安排下一帧。

### 4. 用窗口坐标构建 Path

`build_stroke_path` 先把保存的 canvas 局部坐标转换回窗口坐标，再对描边路径进行三角化：

```rust
let mut builder = PathBuilder::stroke(px(stroke.size));
let first_point = Point::new(
    bounds.origin.x + stroke.points[0].x,
    bounds.origin.y + stroke.points[0].y,
);
builder.move_to(first_point);

for point in stroke.points.iter().skip(1) {
    let abs_point = Point::new(bounds.origin.x + point.x, bounds.origin.y + point.y);
    builder.line_to(abs_point);
}

builder.build().ok()
```

试着调大 **Size** 再画一条线：`stroke.size` 决定新笔画的宽度，之前的笔画保留各自保存的宽度。**Opacity** 和颜色同样在一笔开始时确定。切换 **Show Grid**，可以看到同一个 paint 回调绘制的另一组路径；点击 **Clear Canvas** 则清空保留的笔画并请求新的一帧。如果布局移动或调整 canvas 尺寸，每次绘制都会使用当前的 bounds origin 放置这些点。

如果笔画没有出现，可以按下面的顺序排查：

| 现象 | 检查项 |
| --- | --- |
| 连网格都看不到 | 确认父容器和 canvas 的布局尺寸不为零；Path 不会自己占据布局空间。 |
| 单击后没有留下笔迹 | 此示例不会把一个点画成线；拖动到采样到第二个点。 |
| 移动 canvas 后笔迹偏移 | 输入时减去 canvas origin，绘制时加上**当前** origin。 |
| 保存的点已经变化，画面却没有更新 | 确认状态持有者在有效变化后调用了 `cx.notify()`。 |
| 部分几何无声消失 | 检查 `PathBuilder::build()` 的结果；此示例把错误转成 `None`。 |

示例把构建失败视为“不绘制”；当几何来自用户数据时，应保留或报告错误。输入 handler 挂在外层 `div` 上；Path 本身不提供 hitbox 或无障碍行为。

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

`PathBuilder` 最初是为 K 线图的绘制需求引入 GPUI 的。它内部使用 Lyon 的 SVG path builder，因此路径段词汇与 SVG 接近。熟悉 [SVG 路径命令](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/d)的人，可以直接迁移路径段的概念：

| SVG Path | GPUI builder | 含义 |
| --- | --- | --- |
| `M x y` | `move_to(point)` | 开始子路径。 |
| `L x y` | `line_to(point)` | 直线。 |
| `Q cx cy x y` | `curve_to(end, control)` | 二次贝塞尔曲线。 |
| `C c1x c1y c2x c2y x y` | `cubic_bezier_to(end, control_a, control_b)` | 三次贝塞尔曲线。 |
| `A rx ry rotation large sweep x y` | `arc_to(radii, rotation, large_arc, sweep, end)` | 椭圆弧。 |
| `Z` | `close()` | 闭合子路径。 |

几何概念一致，但 Rust 参数顺序并非把 SVG 字符串逐字搬过来：`curve_to` 与 `cubic_bezier_to` **先传终点，再传控制点**。当前 API 的 `x_rotation` 参数类型是 `Pixels`，其数值按角度解释。坐标用 `Point<Pixels>` 表示；`build()` 先三角化，再由 `paint_path` 提交绘制。因此迁移 SVG 绘图算法的学习成本很低，同时要遵循 GPUI 的坐标类型和错误处理规则。

### 用两种 Path 写法绘制 GPUI Kit 标志

GPUI Kit 标志可以直观说明 SVG Path 与 `PathBuilder` 的对应关系。它由两条独立的闭合路径组成：外形使用主题前景色，内部笔画使用主题的蓝色强调色。切换 GPUI 与 SVG 源码，再与下方的渲染结果对照。示例采用局部 32 × 32 坐标系；GPUI 代码会将 bounds 的 origin 加到每个点上。`foreground` 和 `accent_color` 由调用方提供。

<div class="doc-tabs" role="group" aria-label="GPUI Kit 标志路径源码">
  <input class="doc-tabs__input" type="radio" name="logo-source-zh" id="logo-rust-zh" checked>
  <input class="doc-tabs__input" type="radio" name="logo-source-zh" id="logo-svg-zh">
  <div class="doc-tabs__list">
    <label for="logo-rust-zh">PathBuilder</label>
    <label for="logo-svg-zh">SVG</label>
  </div>
  <div class="doc-tabs__panels">
    <section class="doc-tabs__panel">
      <pre class="astro-code shiki-themes macos-classic-light macos-classic-dark" style="background-color:#f6f6f7;--shiki-dark-bg:#131313;color:#000000;--shiki-dark:#CACCCA" tabindex="0"><code><span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">let</span><span style="color:#000000;--shiki-dark:#CACCCA"> p </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> |</span><span style="color:#000000;--shiki-dark:#CACCCA">x</span><span style="color:#0433FF;--shiki-dark:#87B1F6">:</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> f32</span><span style="color:#000000;--shiki-dark:#CACCCA">, y</span><span style="color:#0433FF;--shiki-dark:#87B1F6">:</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> f32</span><span style="color:#0433FF;--shiki-dark:#87B1F6">|</span><span style="color:#0000A2;--shiki-dark:#B3C5F3"> point</span><span style="color:#000000;--shiki-dark:#CACCCA">(bounds</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">left</span><span style="color:#000000;--shiki-dark:#CACCCA">() </span><span style="color:#0433FF;--shiki-dark:#87B1F6">+</span><span style="color:#0000A2;--shiki-dark:#B3C5F3"> px</span><span style="color:#000000;--shiki-dark:#CACCCA">(x), bounds</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">top</span><span style="color:#000000;--shiki-dark:#CACCCA">() </span><span style="color:#0433FF;--shiki-dark:#87B1F6">+</span><span style="color:#0000A2;--shiki-dark:#B3C5F3"> px</span><span style="color:#000000;--shiki-dark:#CACCCA">(y));</span></span>
<span class="line"></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">let</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> mut</span><span style="color:#000000;--shiki-dark:#CACCCA"> outer </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> PathBuilder</span><span style="color:#0433FF;--shiki-dark:#87B1F6">::</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">fill</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">move_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">9</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">10</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">9</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">10</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">4</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">close</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">if</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> let</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> Ok</span><span style="color:#000000;--shiki-dark:#CACCCA">(path) </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#000000;--shiki-dark:#CACCCA"> outer</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">build</span><span style="color:#000000;--shiki-dark:#CACCCA">() {</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">    window</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">paint_path</span><span style="color:#000000;--shiki-dark:#CACCCA">(path, foreground);</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">}</span></span>
<span class="line"></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">let</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> mut</span><span style="color:#000000;--shiki-dark:#CACCCA"> accent </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> PathBuilder</span><span style="color:#0433FF;--shiki-dark:#87B1F6">::</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">fill</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">move_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">16</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">13</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">13</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">28</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">23</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">18</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">line_to</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">p</span><span style="color:#000000;--shiki-dark:#CACCCA">(</span><span style="color:#0433FF;--shiki-dark:#CC9E00">16</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">, </span><span style="color:#0433FF;--shiki-dark:#CC9E00">18</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#000000;--shiki-dark:#CACCCA">));</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">close</span><span style="color:#000000;--shiki-dark:#CACCCA">();</span></span>
<span class="line"><span style="color:#0433FF;--shiki-dark:#87B1F6">if</span><span style="color:#0433FF;--shiki-dark:#87B1F6"> let</span><span style="color:#571AB7;--shiki-dark:#CBA6F7"> Ok</span><span style="color:#000000;--shiki-dark:#CACCCA">(path) </span><span style="color:#0433FF;--shiki-dark:#87B1F6">=</span><span style="color:#000000;--shiki-dark:#CACCCA"> accent</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">build</span><span style="color:#000000;--shiki-dark:#CACCCA">() {</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">    window</span><span style="color:#0433FF;--shiki-dark:#87B1F6">.</span><span style="color:#0000A2;--shiki-dark:#B3C5F3">paint_path</span><span style="color:#000000;--shiki-dark:#CACCCA">(path, accent_color);</span></span>
<span class="line"><span style="color:#000000;--shiki-dark:#CACCCA">}</span></span></code></pre>
    </section>
    <section class="doc-tabs__panel">
      <pre><code class="language-svg">&lt;svg viewBox=&quot;0 0 32 32&quot;&gt;
  &lt;path fill=&quot;currentColor&quot; d=&quot;M4 4 L28 4 L28 9 L10 9 L10 23 L28 23 L28 28 L4 28 Z&quot; /&gt;
  &lt;path fill=&quot;#3B82F6&quot; d=&quot;M16 13 L28 13 L28 23 L23 23 L23 18 L16 18 Z&quot; /&gt;
&lt;/svg&gt;</code></pre>
    </section>
  </div>
  <figure class="path-preview">
  <svg viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="logo-title-zh">
    <title id="logo-title-zh">用两条彩色路径绘制的 GPUI Kit 标志</title>
    <path fill="currentColor" d="M4 4 L28 4 L28 9 L10 9 L10 23 L28 23 L28 28 L4 28 Z" />
    <path fill="var(--data-2, #3B82F6)" d="M16 13 L28 13 L28 23 L23 23 L23 18 L16 18 Z" />
  </svg>
  <figcaption>前景色与主题蓝色强调色分别绘制两条填充路径。</figcaption>
  </figure>
</div>

GPUI 的 `PathBuilder` 接受完整点坐标，没有 SVG 的 `H`、`V` 缩写，也不能直接解析 SVG 的 `d` 字符串。SVG 标签页把每个 `L` 坐标写全，以便逐行对照。如果已有 SVG 文件，而且不需要 `Path<Pixels>`，可以用 `svg().path("icons/logo.svg")` 渲染该资源。要把任意 SVG path 数据变成 GPUI `Path`，则需要另外的解析器，把解析出的线段逐个送入 `PathBuilder`。

## 哪个阶段完成什么

| 工作 | 阶段 | 原因 |
| --- | --- | --- |
| 声明尺寸与子节点布局 | `request_layout` | Taffy 此时需要 Style，但最终 bounds 尚不存在。 |
| 准备命中测试与绘制共用的几何、插入 `Hitbox` | `prepaint` | 几何坐标和当前帧的输入区域已经确定。 |
| 按需构建仅用于绘制的几何、调用 `paint_path`、`paint_quad` 或绘制子节点 | `paint` | 这时可以确定绘制顺序；Brush 在这里构建 Path。 |

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

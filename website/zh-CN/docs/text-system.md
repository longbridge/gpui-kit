---
title: TextSystem
description: 通过 GPUI 文本系统和 GPUI Kit 组件完成字体解析、字形塑形、测量、排版与绘制。
order: -2.76
---

# TextSystem

本文以仓库固定的 `gpui-pre` {{gpui_pre_version}} API 为准。`gpui-pre` 是 GPUI 快照的发布和版本对齐机制，并非另一套文本引擎。先使用文本元素；只有自定义元素需要 GPUI 平时替你管理的字形几何时，才直接调用 `TextSystem`。

GPUI 的 `TextSystem` 负责解析 [Font](./fonts) 并提供 font metrics。每个 [Window](./window) 都有一个 `WindowTextSystem`，在共享文本系统上增加行布局缓存。普通文本元素和 GPUI Kit 控件会替你使用这些服务。编写自定义文本几何、图表标签、编辑器，或需要直接使用字形位置的元素时，才从 `window.text_system()` 入手。

文本渲染是一条连续的流程：

1. 将 `Font` 解析为 `FontId`，请求的字体不可用时尝试回退。
2. 把 UTF-8 文本与带样式的 `TextRun` 塑形为带位置的字形 run 和行度量。
3. 根据可用宽度换行、测量。
4. 用同一布局进行命中测试，并在窗口中绘制字形。

文本塑形不可省略，因为字符串宽度通常不是各字符独立宽度的简单相加。文字系统、连字、字距调整、字体回退和 emoji 都可能改变字形数量、位置与 advance。自定义标签要测量**实际塑形后的文本**，不要用字符数乘以平均宽度。

## 跟做：从文本到像素

使用已有的 `hello_world` 示例做一个短练习。将 `examples/hello_world/src/main.rs` 替换为下面的代码，在仓库根目录运行 `cargo run -p hello_world`。界面仍用普通文本元素显示标签；同一个窗口的文本系统计算这一行塑形后的 advance，并把结果显示出来，便于修改样本观察变化。在实际应用中，如果文本固有尺寸决定布局，应把测量放在布局阶段；不要只为了打印数字就在每次 `render` 时重复昂贵测量。

```rust
use gpui_kit::*;

struct TextLab;

impl Render for TextLab {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let label: SharedString = "Office fi café 👋".into();
        let font_size = px(18.);
        let run = TextRun {
            len: label.len(), // UTF-8 bytes, not character count.
            font: window.text_style().font(),
            color: window.text_style().color,
            ..Default::default()
        };
        let line = window
            .text_system()
            .shape_line(label.clone(), font_size, &[run], None);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .text_size(font_size)
            .child(label)
            .child(format!("Shaped advance: {:.1} logical px", line.width().as_f32()))
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| TextLab))
            .expect("Failed to open window");
    });
}
```

窗口应显示单行样本文本，第二行以 `Shaped advance:` 开头。数字表示**第一行塑形后的逻辑像素宽度**，不是帧耗时，也不是外层 `div` 的宽度；具体数值取决于平台及实际解析到的字体。先把样本改为 `"iiii"`，再改为 `"WWWW"`，重新运行后应看到不同的 advance。再试 `"café 👋"`：`label.len()` 的字节数会大于屏幕上看到的字符数。不要给这个单行 API 传入 `\n`；多行接口见后文。

### 自己绘制已塑形的文字

第一个练习在 `render` 中测量，方便把结果打印出来。要观察底层绘制路径，请再次替换同一个 `examples/hello_world/src/main.rs`，在仓库根目录运行 `cargo run -p hello_world`。`canvas` 提供 prepaint 和 paint 两个回调，无须另写一套 `Element` 实现：

```rust
use gpui_kit::*;

struct CanvasTextLab;

impl Render for CanvasTextLab {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let sample: SharedString = "Office fi café 👋".into();
        let canvas_text = sample.clone();
        let font = window.text_style().font();
        let color = window.text_style().color;
        let font_size = px(18.);
        let line_height = px(24.);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .text_size(font_size)
            .child("Normal text element:")
            .child(sample)
            .child("Shaped and painted in a canvas:")
            .child(
                canvas(
                    move |_, window, _| {
                        let run = TextRun {
                            len: canvas_text.len(),
                            font,
                            color,
                            ..Default::default()
                        };
                        window
                            .text_system()
                            .shape_line(canvas_text, font_size, &[run], None)
                    },
                    move |bounds, line, window, cx| {
                        let advance = line.width();
                        line.paint(bounds.origin, line_height, TextAlign::Left, None, window, cx)
                            .expect("Failed to paint shaped text");

                        let marker = Bounds {
                            origin: point(bounds.origin.x + advance, bounds.origin.y),
                            size: size(px(1.), line_height),
                        };
                        window.paint_quad(fill(marker, color));
                    },
                )
                .w(px(320.))
                .h(line_height),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| CanvasTextLab))
            .expect("Failed to open window");
    });
}
```

canvas 在普通文本元素下方绘制相同样本。细竖线位于这一行的 **advance** 处，也就是塑形后的笔位位置，不一定是最后一个字形可见像素的边缘。`canvas` 在布局阶段申请固定的 `320 × 24` 空间；边界确定后，prepaint 回调塑形并返回 `ShapedLine`，paint 回调使用**同一结果**调用 `width()` 和 `paint(...)`。修改代码中的样本并重新运行，观察竖线移动。特别长的样本可能超出固定宽度；本练习没有实现换行或裁剪。

如果普通文字能显示而 canvas 的文字消失，请依次检查 canvas 高度、`TextRun::len` 的字节数，以及 `line.paint` 的返回值。canvas 绘制的这行文字没有文本节点的无障碍 identity、选区、hitbox 或键盘行为。界面文案应使用普通文本元素。若文字宽度决定自定义元素**申请的尺寸**，应在测量布局回调中塑形，而不是使用这个固定尺寸的 canvas；[Element](./element) 解释了这条布局边界。

实际视图可按需求递进：普通标签 → 只有固有布局需要时才计算塑形宽度 → 只有自定义绘制需要时才在元素 paint 阶段复用塑形字形。拥有自己布局或子元素的完整生命周期见 [Element](./element)。标签、按钮和普通段落不需要直接调用 `TextSystem`。

## 优先使用文本元素

普通界面文案应使用文本 child，让 GPUI 负责布局与绘制：

```rust
use gpui_kit::*;

div()
    .font_family(".SystemUIFont")
    .text_size(px(14.))
    .child(text!("Recent activity"))
```

`text!(...)` 创建 `Text` 元素，并用宏调用的源码位置生成 ID。GPUI 可借此向无障碍树提供标签，并在 ID 稳定时报告内容更新。同一个调用位置生成多个标签时，可用 `text!(id = "activity-label", value)` 显式指定 ID；同时显示的元素各自需要不同 ID。普通 `.child("Recent activity")` 也会排版和绘制文字，但没有文本节点 ID。两种写法都不会在 `render` 中直接返回测量宽度：GPUI 在元素的测量布局回调中塑形，在 prepaint 中记录边界，再在 paint 中绘制。

GPUI Kit 的 [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/text/compat.rs) 能渲染带样式 run、选择与可选滚动的 Markdown 或 HTML；解析、布局和选择行为由 Base 承担。富文档可用 `TextView::markdown("article", source)`。[Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 在 [`prepaint`](./element#prepaint) 中塑形可见行，此时已知最终宽度与输入几何；光标和指针映射使用同一份塑形布局。普通标签不需要自行实现这些工作。

## 字体解析与度量

`Font` 包含字体族，以及 weight、style、features 和可选的 fallback。`font("Family")` 构造字体；`Font::default()` 请求 `.SystemUIFont`。`cx.text_system().resolve_font(&font)` 返回 `FontId`；请求的字体族无法加载时，会尝试 GPUI 的回退栈。若所有回退都失败，则会 panic。`all_font_names()` 列出可用字体族，包括由 `add_fonts(...)` 注册的字体。应在第一帧之前注册打包字体，让第一次布局使用预期度量；之后再添加字体会使已缓存的字体解析和行布局失效，而已经开始的布局可能仍使用旧字体集合。

对已解析字体及 `Pixels` 字号，GPUI 提供 `ascent`、`descent`、`cap_height`、`x_height`、`bounding_box`、`advance` 和 `typographic_bounds`。这些 API 回答的问题不同：`advance(font_id, size, ch)` 给出该字体中单个字符对应字形的笔位移动；`typographic_bounds(font_id, size, ch)` 给出该字形的排版边界；ascent 与 descent 描述垂直度量。前两者不会对字符串塑形，也不会应用字体回退。`WindowTextSystem::layout_width(font_id, size, ch)` 只塑形一个字符，适合测量空格或特定单元格，不适合推算整句。

最终显示取决于已安装字体和平台回退。桌面上的 GPUI 会从操作系统字体集合中解析，请求的字体族可能落到另一种可用字体。Web 构建不能假定可访问桌面系统字体，必须注册所需字体。如果换行或对齐很重要，应在目标平台检查拉丁文、中日韩文字、emoji 与混合文字的代表样本。

## 塑形单行文本

`TextRun::len` 以 **UTF-8 字节**计数，所有 run 合起来应覆盖要绘制的文本；对 `shape_text` 也包括换行符字节。只在合法 UTF-8 边界拆分 run。每个 run 指定其字节范围内的字体、颜色、背景、下划线和删除线。文本参数是 [SharedString](./shared-string)。下面的例子沿用 [Plot label](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/label.rs) 的做法：

```rust
use gpui_kit::*;

fn shape_label(text: SharedString, color: Hsla, window: &mut Window) -> ShapedLine {
    let run = TextRun {
        len: text.len(),
        font: window.text_style().font(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };

    window.text_system().shape_line(text, px(14.), &[run], None)
}

// In a render, layout measurement, or paint method with a `window`:
let width = shape_label("Recent activity".into(), color, window).width();
```

这个 helper 可以复制到已有 GPUI view 或元素中使用，其中 `window` 和 `color` 已由外层提供；它不是独立的 `main`。上面的 [canvas 练习](#自己绘制已塑形的文字)给出了完整的塑形到绘制应用。生产实现可参考 [`crates/component/src/plot/label.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/label.rs)：`measure_text_width` 使用同一塑形方式，`PlotLabel::paint` 调用 `ShapedLine::paint(origin, line_height, align, align_width, window, cx)`。

`shape_line(text, font_size, runs, force_width)` 返回 `ShapedLine`：`width()` 是塑形后的 advance，结果还包含原文、带位置的字形、字体 ID、ascent、descent 和装饰 run。除非自定义布局有意指定宽度，否则 `force_width` 传 `None`。`shape_line` 只处理**一行**，不要传入包含 `\n` 的文本。只需要几何信息时，`layout_line(&str, size, runs, force_width)` 返回 `Arc<LineLayout>`；如果还要绘制，直接选用 `shape_line`。

advance 是画完一行后笔位移动的距离，不一定等于有色像素的外接矩形。字形可能伸出 advance，`font_size` 也不能单独决定一行的高度。垂直定位要结合该行的 ascent、descent 和选定的行高。需要裁剪框或命中区域时，应按实际放置的行和交互区域计算，不要把 `width()` 当作字形墨迹边界。

`LineLayout::x_for_index(byte_index)`、`index_for_x(x)` 与 `closest_index_for_x(x)` 可用于光标和命中测试。索引是原始文本中的 UTF-8 字节位置，不是 Unicode 字符数，也不是视觉列数。`x_for_index` 返回行内局部 x；`index_for_x` 在行宽处及其右侧返回 `None`，`closest_index_for_x` 则选一个最接近的插入边界。画光标时，把绘制原点加到局部 x 上；处理指针时，先减去同一个原点再查询布局。坐标使用 GPUI 的[逻辑像素](./geometry)。选择边界应保持在合法文本边界上；字形或连字不一定对应单个字符。测量、光标定位与绘制应使用同一塑形结果、字号、行高和对齐方式，才能一致。

## 多行换行

文本包含换行符或需要软换行时，使用 `shape_text(text, font_size, runs, wrap_width, line_clamp)`。它返回 `Result<SmallVec<[WrappedLine; 1]>>`。`wrap_width: Some(width)` 指定可用宽度。每个 `WrappedLine` 对应两个显式 `\n` 之间的一行源文本，内部可包含多个视觉行。`WrappedLine::size(line_height)` 包含这些视觉行的高度；`position_for_index(index, line_height)` 与 `closest_index_for_position(local_point, line_height)` 可在局部几何和**该源行内**字节偏移之间转换。要得到全文偏移，还要加上源行起始字节位置，并计算跳过的换行符字节。`line_clamp` 限制软换行边界，但 `shape_text` 仍会为每个显式换行分隔的源行返回一个 `WrappedLine`，不能保证结果总共不超过 N 行。宽度或字体变化会改变换行边界，因此应重新计算布局。

普通段落应交给 GPUI 文本元素或 GPUI Kit `TextView`。只有现有元素无法满足字形级定位、绘制或命中测试需求时，自定义元素才应直接塑形。

## 对齐 GPUI 渲染阶段

GPUI 的 [Render](./render) 流程分开布局、prepaint 和 paint；自定义文本工作应放在对应阶段：

| 阶段 | 文本工作 | 原因 |
| --- | --- | --- |
| `request_layout` | 声明样式和布局节点；固有文本尺寸或换行决定申请尺寸时，使用测量布局回调。 | 回调能拿到宽度约束，但此时还没有最终 `Bounds<Pixels>`。 |
| `prepaint` | 用已解析边界放置准备好的行、计算光标几何并建立 hitbox；若自定义元素此时才知道内容宽度，就在这里塑形。 | 输入几何必须对应当前帧。 |
| `paint` | 在准备好的原点绘制 `ShapedLine` 或 `WrappedLine`。 | 重用测量阶段或 prepaint 准备的塑形结果，使像素与命中测试一致。 |

普通 GPUI 文本元素在测量布局回调中塑形，在 prepaint 中记录边界，在 paint 中绘制准备好的 `WrappedLine`。[Input element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 则在 prepaint 中塑形可见编辑行，因为它要结合已解析的视口和滚动几何。`ShapedLine::paint` 提交一行；若 run 有背景，要单独调用 `paint_background(...)`，并处理两者的 `Result`。`WrappedLine::paint` 的可选对齐宽度参数是 `Bounds<Pixels>`，而 `ShapedLine::paint` 用 `Option<Pixels>`。绘制文字本身不会建立 hitbox、键盘焦点或无障碍名称；可交互的自定义文本元素也要实现这些契约。自定义绘制详见 [Paint](./paint)。

以图表标签为例，先问标签宽度是否会改变图表申请的尺寸。如果会，在测量布局回调中塑形并把结果带到后续阶段；prepaint 根据最终边界放置它，paint 绘制同一结果。如果图表矩形尺寸已固定，标签只是需要在其中定位，就在 prepaint 塑形并定位。两种情况都要让指针映射与绘制使用相同的行和原点。上面的练习在 `render` 中测量仅为观察数值，不能替代这些元素阶段。

## 排查文本不一致

| 症状 | 先检查 | 后续处理 |
| --- | --- | --- |
| 第一次布局解析字体时 panic | 初始化后查看 `cx.text_system().all_font_names()`。Web 上检查字体字节是否在打开窗口前注册。 | 选用可用字体族，或注册目标字体；详见[字体排错](./fonts#排查字体问题)。 |
| 字体外观意外，或不同机器的宽度不同 | 对照请求的字体族、可用字体列表和字体文件内部的 family。字体族存在不代表覆盖所有字形。 | 在目标平台核对拉丁文、中日韩文字和 emoji；需要稳定结果时打包所需字重和字体。 |
| 测量宽度与绘制标签不一致 | 对比两条路径的字体、features、字号、原文和 `force_width`。父元素的 `.text_size(...)` 不会自动修改 `shape_line` 显式传入的字号。 | 用一致的输入塑形，并复用结果完成定位和绘制。 |
| 光标或点击落在错误字符处 | 索引是否为 UTF-8 字节偏移？是否恰好一次加上或减去行的原点？ | 对同一份行布局使用 `x_for_index` 和 `closest_index_for_x`，索引保持合法边界。 |
| 最后一个字形被切掉或行高不够 | 是否把 `width()` 当作字形墨迹边界，或把 `font_size` 当成完整行高？ | 为字形伸出部分留空间；分配和裁剪时结合行度量与选定行高。 |
| 字体加载后段落换行改变 | 检查注册是否发生在首次布局之后，以及窗口是否刷新。 | 尽量在首帧前注册；之后才调用 `add_fonts` 时，调用 `cx.refresh_windows()` 并重新测量。 |

## 缓存与性能边界

`WindowTextSystem` 在当前帧和上一帧的缓存中保存行布局，并通过 `TextSystem` 共享字体解析及度量。共享系统还缓存字形光栅边界；绘制使用的字形参数包含字体、字号、缩放系数和子像素位置。缓存键包含文本、已解析的字体 run、字号与宽度约束。相同输入可复用布局；改变文本、字体、features、字号、强制宽度或换行宽度，可能需要新布局。绘制颜色和背景属于装饰 run，与字形位置分开；只改变它们未必需要重新塑形。帧缓存不保证布局会被无限期保留。若预先知道会用到哪些字体，可在后台 executor 上调用 `TextSystem::prewarm_fonts(&fonts)` 准备平台字体缓存；普通塑形仍会按需填补缺失项。应复用窗口文本系统，也不要保存跨帧的 `&mut Window` 引用。

大型虚拟化编辑器或文档只应塑形可见内容，避免每次重绘都测量所有行。GPUI Kit 的 [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) 可虚拟化可滚动的块，[Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 则塑形可见行。如果性能分析显示长行文本反复被物化，`try_layout_line_by_hash` 可在不构造连续字符串的情况下探测缓存，而 `shape_line_by_hash` 只在缓存未命中时构造文本。使用后者时，返回的 `ShapedLine.text` 始终是空占位；若还需要原文，应单独保存。调用方必须保证相同 hash 对应完全相同的文本，并将其 UTF-8 字节长度传作 `text_len`。在实际测得这项开销之前，优先使用普通 API。

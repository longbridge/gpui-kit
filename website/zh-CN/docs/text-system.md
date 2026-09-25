---
title: TextSystem
description: 通过 GPUI 文本系统和 GPUI Kit 组件完成字体解析、字形塑形、测量、排版与绘制。
order: -2.76
---

# TextSystem

GPUI 的 `TextSystem` 负责解析 [Font](./fonts) 并提供 font metrics。每个 [Window](./window) 都有一个 `WindowTextSystem`，在共享文本系统上增加行布局缓存。普通文本元素和 GPUI Kit 控件会替你使用这些服务。编写自定义文本几何、图表标签、编辑器，或需要直接使用字形位置的元素时，才从 `window.text_system()` 入手。

文本渲染是一条连续的流程：

1. 将 `Font` 解析为 `FontId`，请求的字体不可用时尝试回退。
2. 把 UTF-8 文本与带样式的 `TextRun` 塑形为带位置的字形 run 和行度量。
3. 根据可用宽度换行、测量。
4. 用同一布局进行命中测试，并在窗口中绘制字形。

文本塑形不可省略，因为字符串宽度通常不是各字符独立宽度的简单相加。文字系统、连字、字距调整、字体回退和 emoji 都可能改变字形数量、位置与 advance。自定义标签要测量**实际塑形后的文本**，不要用字符数乘以平均宽度。

## 优先使用元素

普通界面文案应使用文本 child，让 GPUI 负责布局与绘制：

```rust
use gpui_kit::*;

div()
    .font_family(".SystemUIFont")
    .text_size(px(14.))
    .child("最近活动")
```

GPUI Kit 的 [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/text/compat.rs) 能渲染带样式 run、选择与可选滚动的 Markdown 或 HTML；解析、布局和选择行为由 Base 承担。富文档可用 `TextView::markdown("article", source)`。[Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 在 [`prepaint`](./element#prepaint) 中塑形可见行，此时已知最终宽度与输入几何；光标和指针映射使用同一份塑形布局。普通标签不需要自行实现这些工作。

## 字体解析与度量

`Font` 包含字体族，以及 weight、style、features 和可选的 fallback。`font("Family")` 构造字体；`Font::default()` 请求 `.SystemUIFont`。`cx.text_system().resolve_font(&font)` 返回 `FontId`；请求的字体族无法加载时，会尝试 GPUI 的回退栈。若所有回退都失败，则会 panic。`all_font_names()` 列出可用字体族，包括由 `add_fonts(...)` 注册的字体。应在第一帧之前注册打包字体，让第一次布局使用预期度量；之后再添加字体会使已缓存的字体解析和行布局失效，而已经开始的布局可能仍使用旧字体集合。

对已解析字体及 `Pixels` 字号，GPUI 提供 `ascent`、`descent`、`cap_height`、`x_height`、`bounding_box`、`advance` 和 `typographic_bounds`。这些 API 回答的问题不同：`advance(font_id, size, ch)` 给出该字体中单个字符对应字形的笔位移动；`typographic_bounds(font_id, size, ch)` 给出该字形的排版边界；ascent 与 descent 描述垂直度量。前两者不会对字符串塑形，也不会应用字体回退。`WindowTextSystem::layout_width(font_id, size, ch)` 只塑形一个字符，适合测量空格或特定单元格，不适合推算整句。

最终显示取决于已安装字体和平台回退。桌面上的 GPUI 会从操作系统字体集合中解析，请求的字体族可能落到另一种可用字体。Web 构建不能假定可访问桌面系统字体，必须注册所需字体。如果换行或对齐很重要，应在目标平台检查拉丁文、中日韩文字、emoji 与混合文字的代表样本。

## 塑形单行文本

`TextRun::len` 以 **UTF-8 字节**计数，所有 run 合起来应覆盖要绘制的文本。每个 run 指定其字节范围内的字体、颜色、背景、下划线和删除线。文本参数是 [SharedString](./shared-string)。下面的例子沿用 [Plot label](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/label.rs) 的做法：

```rust
use gpui_kit::*;

fn shape_label(text: SharedString, color: Hsla, window: &Window) -> ShapedLine {
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

let width = shape_label(label, color, window).width();
```

`shape_line(text, font_size, runs, force_width)` 返回 `ShapedLine`：`width()` 是塑形后的 advance，结果还包含原文、带位置的字形、字体 ID、ascent、descent 和装饰 run。除非自定义布局有意指定宽度，否则 `force_width` 传 `None`。`shape_line` 只处理**一行**，不要传入包含 `\n` 的文本。只需要几何信息时，`layout_line(&str, size, runs, force_width)` 返回 `Arc<LineLayout>`；如果还要绘制，直接选用 `shape_line`。

`LineLayout::x_for_index(byte_index)`、`index_for_x(x)` 与 `closest_index_for_x(x)` 可用于光标和命中测试。索引是原始文本中的 UTF-8 字节位置，不是 Unicode 字符数，也不是视觉列数。x 位置使用 GPUI 的 [logical pixel geometry](./geometry)。选择边界应保持在合法文本边界上；测量、光标定位和绘制应使用同一份塑形布局，结果才能一致。

## 多行换行

文本包含换行符或需要软换行时，使用 `shape_text`。它返回 `WrappedLine` 集合的 `Result`。`wrap_width: Some(width)` 指定可用宽度。`line_clamp` 限制软换行边界，但 `shape_text` 仍会为每个由显式换行符分隔的源行返回一个 `WrappedLine`，不能保证结果总共不超过 N 行。行布局保存换行边界和宽度，让自定义文本元素放置每个视觉行，并将指针位置映射回源文本。宽度或字体变化会改变换行边界，因此应重新计算布局。

普通段落应交给 GPUI 文本元素或 GPUI Kit `TextView`。只有现有元素无法满足字形级定位、绘制或命中测试需求时，自定义元素才应直接塑形。

## 对齐 GPUI 渲染阶段

GPUI 的 [Render](./render) 流程分开布局、prepaint 和 paint；自定义文本工作应放在对应阶段：

| 阶段 | 文本工作 | 原因 |
| --- | --- | --- |
| `request_layout` | 声明样式和布局节点；只测量申请尺寸所必需的内容。 | 此时还没有最终 `Bounds<Pixels>`。 |
| `prepaint` | 根据已解析宽度塑形、计算行原点和光标几何、建立 hitbox。 | 布局已给出边界；输入几何必须对应当前帧。 |
| `paint` | 在准备好的原点绘制 `ShapedLine`。 | 重用 prepaint 的塑形结果，使像素与命中测试一致。 |

`ShapedLine::paint` 提交一行；若 run 有背景，另用 `paint_background(...)`。两者返回 `Result`，应处理。[Input element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 对可见文本、行号、选择区域与光标采用这种阶段划分。绘制文字本身不会建立 hitbox、键盘焦点或无障碍名称；可交互的自定义文本元素也要实现这些契约。自定义绘制详见 [Paint](./paint)。

## 缓存与性能边界

`WindowTextSystem` 跨当前帧和上一帧缓存行布局，并通过 `TextSystem` 共享字体解析及度量。共享系统还缓存字形光栅边界；绘制使用的字形参数包含字体、字号、缩放系数和子像素位置。相同文本、字体 run 和字号可复用塑形结果；改变文本、字体、features、字号或换行宽度，则可能需要新布局。若预先知道会用到哪些字体，可在后台 executor 上调用 `TextSystem::prewarm_fonts(&fonts)` 准备平台字体缓存；普通塑形仍会按需填补缺失项。这些缓存是复用窗口文本系统的理由，不能因此另起一个塑形器或保存跨帧的 `&mut Window` 引用。

大型虚拟化编辑器或文档只应塑形可见内容，避免每次重绘都测量所有行。GPUI Kit 的 [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) 可虚拟化可滚动的块，[Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 则塑形可见行。如果性能分析显示长行文本反复被物化，`try_layout_line_by_hash` 可在不构造连续字符串的情况下探测缓存，而 `shape_line_by_hash` 只在缓存未命中时构造文本。使用后者时，返回的 `ShapedLine.text` 始终是空占位；若还需要原文，应单独保存。调用方必须保证相同 hash 对应完全相同的文本，并将其 UTF-8 字节长度传作 `text_len`。在实际测得这项开销之前，优先使用普通 API。

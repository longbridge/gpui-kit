---
title: Style
description: 使用与 Tailwind CSS 相近的 utility、带类型的值和 Rust 链式构造器设置 GPUI 元素样式。
order: -2.61
---

# Style

GPUI 在构建 [Element](./element) 时设置样式。`Styled` trait 提供布局、间距、颜色、边框和文字的链式方法。许多名称有意对应 <a href="https://tailwindcss.com/docs/styling-with-utility-classes" target="_blank" rel="noopener noreferrer">Tailwind CSS utility</a>：`flex items-center gap-2 px-3` 在 Rust 中写成 `.flex().items_center().gap_2().px_3()`。可以借助这套词汇阅读和编写 GPUI 布局，但传入的是带类型的 Rust 值，而非 CSS class。

```rust
use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;

div()
    .flex()
    .items_center()
    .gap_2()
    .px_3()
    .py_2()
    .bg(cx.theme().background)
    .text_color(cx.theme().foreground)
    .child("Search results")
```

链上的每次调用都会取得元素并返回元素。render 可以根据当前状态构建新树；需要长期保存的应用状态应放在 [`Entity`](./entity) 或带 key 的元素状态中。样式链描述本帧的外观，并非样式表或长期保存的组件实例。逐帧构建组件的方式见 [RenderOnce](./render-once)。

## 从第一个布局开始

先找出负责可用空间的区域，再决定哪个子元素宽度固定、哪个可以伸展。用下面的完整程序替换 `examples/hello_world/src/main.rs`，然后在仓库根目录运行 `cargo run -p hello_world`：

```rust
use gpui_kit::*;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _};
use gpui_kit::prelude::FluentBuilder as _;

struct StyleExample;

impl Render for StyleExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = window.viewport_size().width < px(600.);
        let layout = if compact { v_flex() } else { h_flex() };
        let documents = (1..=60).map(|number| {
            div().p_2().child(format!("Document {number:02}"))
        });

        layout
            .items_stretch()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                v_flex()
                    .when(!compact, |nav| nav.w_64().flex_shrink_0())
                    .p_3()
                    .gap_2()
                    .child("Navigation")
                    .child("Overview · Documents"),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .child(h_flex().p_3().child("Documents"))
                    .child(
                        v_flex()
                            .id("document-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(v_flex().p_3().gap_2().children(documents)),
                    ),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| StyleExample)
        })
        .expect("failed to open window");
    });
}
```

窗口较宽时，导航区域以固定的 `w_64()` 宽度位于左侧，文档面板占用剩余空间。把内容区域缩小到 600 逻辑像素以下：同一个导航区域会移到文档面板上方，而文档列表仍可独立滚动。滚到 `Document 60`，再向两个方向调整窗口宽度；列表滚动时，标题栏应保持在原位。这里的阈值是在视图 render 时求值的 Rust 条件，不是 Tailwind 响应式前缀。这个小练习中的导航只是示意文本；真实应用应提供可操作的导航控件。

`h_flex()` 建立横向布局，并默认让子元素沿交叉轴居中。`.items_stretch()` 覆盖这个默认值，让两个面板占满整行高度。`v_flex()` 建立纵向布局，子元素默认沿宽度方向拉伸。宽窗口中的导航面板保持固定宽度，文档面板占用剩余宽度。滚动区域占据标题栏下方的剩余高度。窗口大小为最外层的 `.size_full()` 提供确定高度，`min_h_0()` 则允许内部弹性区域收缩成滚动视口。

### 确定尺寸该由谁负责

| 需求 | 设置位置 | 原因 |
| --- | --- | --- |
| 同级元素之间的距离 | 在父元素上设置 `.gap_3()` | gap 分隔子元素，不会在容器外边缘增加内边距。 |
| 面板内部的留白 | 在面板上设置 `.p_3()` | padding 把内容向内推，并参与面板的布局尺寸。 |
| 固定侧栏与弹性内容 | 侧栏 `.w_64().flex_shrink_0()`；内容 `.flex_1().min_w_0()` | 侧栏保持宽度，内容可收缩到小于文字自然宽度。 |
| 标题栏下方的滚动内容 | 有确定高度的纵向容器；滚动子元素 `.flex_1().min_h_0()` | 子元素可缩进剩余高度，从而形成真正的滚动视口。 |
| 父元素宽度的一半 | 在子元素上设置 `.w(relative(0.5))` | 布局时，比例按相关父尺寸解析。 |

`w_full()` 和 `h_full()` 表示填满*可用*尺寸。百分比高度仍要求上层有确定高度。最小值和最大值限制最终尺寸；它们不会让没有高度约束的滚动区域自动形成视口。横向布局中的单行长标题可在标题容器上组合 `.flex_1().min_w_0().truncate()`。`.truncate()` 只改变文字溢出方式，无法让不肯收缩的同级元素腾出宽度。

### 选择裁剪、滚动或定位

`.overflow_hidden()` 裁剪内容，但不会让内容滚动。在有状态的元素上，`.overflow_y_scroll()` 可在高度受限后启用纵向滚动。GPUI Kit 的 `.overflow_y_scrollbar()` 会加入可见滚动条，并以原元素作为滚动区域；它来自 `ScrollableElement` 扩展，而非 `Styled` 方法。每个滚动区域应有一个明确的负责元素。如果滚动条要贴着面板边缘，内容 padding 应放在滚动区域内部。滚动归属与测量方式见[编码指南](./coding-guides)。

普通 Flex 子元素占用布局空间。要让角标覆盖内容而不占一行或一列，可在容器上设置 `.relative()`，在角标上设置 `.absolute().top_0().right_0()`。偏移 setter 用于定位绝对子元素；它们不会自动将普通 Flex 子元素变成绝对定位。后面的同级元素通常绘制在前面的同级元素之上；通用 `Styled` 没有 `z_index(...)` 方法。

### 主题与尺寸尺度

应用界面的颜色和圆角应从 `cx.theme()` 读取语义值。GPUI Kit 组件已应用正常的主题外观；实例样式可用于局部布局或有意的细化。具名间距和尺寸方法使用 rem 尺度：`_1` 为 `0.25rem`、`_2` 为 `0.5rem`、`_3` 为 `0.75rem`、`_4` 为 `1rem`。在 GPUI Kit 的 `Root` 中，当前主题的基础字号决定窗口的 rem 大小，因此字号或缩放变化也会改变基于 rem 的几何尺寸。命名尺度不合适时用 `.gap(rems(0.625))` 等带类型的 setter；只有确实需要像素尺寸时才使用 `px(...)`。长度类型见[几何](./geometry)，主题与 rem 设置见[字体](./fonts)。

### 排查布局结果

| 现象 | 检查方向 |
| --- | --- |
| 窗口缩窄后导航没有移到文档上方 | 把可绘制内容区域缩到 600 逻辑像素以下。条件在 `render` 中读取 `window.viewport_size().width`；仅改变显示器缩放倍率不会跨过这个逻辑像素阈值。 |
| 无法滚到 `Document 60` | 把指针移到文档列表上并在该区域滚动。让有高度约束的列表区域拥有 `.id("document-list").overflow_y_scroll()`，并在该区域及外层纵向容器上保留 `.flex_1().min_h_0()`。 |
| 滚动时标题栏也跟着移动 | 确认标题栏与列表视口是同级元素，不要把标题栏放进滚动元素内部。 |
| 面板标题在顶部消失 | `h_flex()` 默认让子元素居中；让整行子元素拉伸，或让该面板占满高度。 |
| 标题溢出而没有截断 | 在弹性子元素上用 `.min_w_0()` 解除最小宽度约束，并限制文字宽度。 |
| 列表越过窗口而没有滚动 | 让祖先拥有确定高度，用 `.min_h_0()` 允许弹性子元素收缩，并在预期的视口上设置滚动。 |
| 滚动条缩在面板边缘以内 | 检查哪个元素负责滚动，以及 padding 是否包在滚动元素外面。 |
| 主题缩放后布局变化 | 重新检查基于 rem 的尺寸，以及仍沿用旧 rem 大小的测量缓存。 |

## 常用 `Styled` 方法

Tailwind 名称中的连字符在 GPUI 方法中写成下划线。有对应样式概念时，第一列链接到相应的 Tailwind CSS 官方参考页，并在新窗口打开。表中列的是 GPUI 方法名；可在实现 `Styled` 的值上调用，例如 `div().gap_2()`。这些表格覆盖 `Styled` 中各类独立操作，以及宏生成的通用 setter；大量数字变体按下文所述的方法族归纳，不逐个占用数千行。链接说明对应的样式概念，不表示 GPUI 与浏览器的行为完全相同。

### 显示与可见性

| 方法 | 说明 |
| --- | --- |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>block</code></a> | 使用块布局。 |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>flex</code></a> | 使用 Flexbox 布局。 |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>grid</code></a> | 使用 Grid 布局。 |
| <a href="https://tailwindcss.com/docs/display" target="_blank" rel="noopener noreferrer"><code>hidden</code></a> | 从布局和绘制中移除元素。 |
| <a href="https://tailwindcss.com/docs/visibility" target="_blank" rel="noopener noreferrer"><code>invisible</code></a> | 保留布局空间，但不绘制元素。 |
| <a href="https://tailwindcss.com/docs/visibility" target="_blank" rel="noopener noreferrer"><code>visible</code></a> | 保留布局，并恢复绘制。 |

### Flexbox 与 Grid

| 方法 | 说明 |
| --- | --- |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_row</code></a> | 沿横向排列 Flex 子项。 |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_col</code></a> | 沿纵向排列 Flex 子项。 |
| <a href="https://tailwindcss.com/docs/flex-wrap" target="_blank" rel="noopener noreferrer"><code>flex_wrap</code></a> | 允许 Flex 子项换行。 |
| <a href="https://tailwindcss.com/docs/flex-wrap" target="_blank" rel="noopener noreferrer"><code>flex_nowrap</code></a> | 让 Flex 子项保持在同一行。 |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_start</code></a> | 让子项沿交叉轴向起点对齐。 |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_center</code></a> | 让子项沿交叉轴居中。 |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_stretch</code></a> | 让子项沿交叉轴拉伸。 |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_center</code></a> | 让当前子项沿父容器的交叉轴居中。 |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_stretch</code></a> | 让当前子项沿父容器的交叉轴拉伸。 |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_center</code></a> | 让子项沿主轴居中。 |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_between</code></a> | 把主轴剩余空间分配到子项之间。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_between</code></a> | 沿交叉轴分配换行后的各行。 |
| <a href="https://tailwindcss.com/docs/flex-basis" target="_blank" rel="noopener noreferrer"><code>flex_basis</code></a> | 设置带类型的主轴初始尺寸。 |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_1</code></a> | 以零 Flex basis 伸长或收缩。 |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_auto</code></a> | 以自动 basis 为起点伸长或收缩。 |
| <a href="https://tailwindcss.com/docs/flex-grow" target="_blank" rel="noopener noreferrer"><code>flex_grow_1</code></a> | 允许 Flex 子项伸长。 |
| <a href="https://tailwindcss.com/docs/flex-shrink" target="_blank" rel="noopener noreferrer"><code>flex_shrink_0</code></a> | 阻止 Flex 子项收缩。 |
| <a href="https://tailwindcss.com/docs/grid-template-columns" target="_blank" rel="noopener noreferrer"><code>grid_cols</code></a> | 设置指定列数的 Grid 模板。 |
| <a href="https://tailwindcss.com/docs/grid-template-rows" target="_blank" rel="noopener noreferrer"><code>grid_rows</code></a> | 设置指定行数的 Grid 模板。 |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_span</code></a> | 跨越指定数量的 Grid 列。 |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_span</code></a> | 跨越指定数量的 Grid 行。 |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_row_reverse</code></a> | 反向排列横向子项。 |
| <a href="https://tailwindcss.com/docs/flex-direction" target="_blank" rel="noopener noreferrer"><code>flex_col_reverse</code></a> | 反向排列纵向子项。 |
| <a href="https://tailwindcss.com/docs/flex-wrap" target="_blank" rel="noopener noreferrer"><code>flex_wrap_reverse</code></a> | 反向排列换行后的 Flex 行。 |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_end</code></a> | 让子项沿交叉轴向终点对齐。 |
| <a href="https://tailwindcss.com/docs/align-items" target="_blank" rel="noopener noreferrer"><code>items_baseline</code></a> | 按文字基线对齐子项。 |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_start</code></a> | 让当前子项沿交叉轴向起点对齐。 |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_end</code></a> | 让当前子项沿交叉轴向终点对齐。 |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_flex_start</code></a> | 让当前子项向 Flex 起点对齐。 |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_flex_end</code></a> | 让当前子项向 Flex 终点对齐。 |
| <a href="https://tailwindcss.com/docs/align-self" target="_blank" rel="noopener noreferrer"><code>self_baseline</code></a> | 按文字基线对齐当前子项。 |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_start</code></a> | 让子项向主轴起点聚集。 |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_end</code></a> | 让子项向主轴终点聚集。 |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_around</code></a> | 在子项周围分配空间。 |
| <a href="https://tailwindcss.com/docs/justify-content" target="_blank" rel="noopener noreferrer"><code>justify_evenly</code></a> | 沿主轴均匀分配间距。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_normal</code></a> | 使用默认的交叉轴行排列。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_start</code></a> | 让换行后的各行向交叉轴起点聚集。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_center</code></a> | 让换行后的各行沿交叉轴居中。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_end</code></a> | 让换行后的各行向交叉轴终点聚集。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_around</code></a> | 在换行后的各行周围分配空间。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_evenly</code></a> | 在换行后的各行之间均匀分配空间。 |
| <a href="https://tailwindcss.com/docs/align-content" target="_blank" rel="noopener noreferrer"><code>content_stretch</code></a> | 沿交叉轴拉伸换行后的各行。 |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_initial</code></a> | 使用自动 basis，可收缩但不伸长。 |
| <a href="https://tailwindcss.com/docs/flex" target="_blank" rel="noopener noreferrer"><code>flex_none</code></a> | 同时禁止 Flex 伸长与收缩。 |
| <a href="https://tailwindcss.com/docs/flex-grow" target="_blank" rel="noopener noreferrer"><code>flex_grow</code></a> | 设置数字形式的 Flex 伸长系数。 |
| <a href="https://tailwindcss.com/docs/flex-grow" target="_blank" rel="noopener noreferrer"><code>flex_grow_0</code></a> | 禁止 Flex 伸长。 |
| <a href="https://tailwindcss.com/docs/flex-shrink" target="_blank" rel="noopener noreferrer"><code>flex_shrink</code></a> | 设置数字形式的 Flex 收缩系数。 |
| <a href="https://tailwindcss.com/docs/flex-shrink" target="_blank" rel="noopener noreferrer"><code>flex_shrink_1</code></a> | 允许 Flex 收缩。 |
| <a href="https://tailwindcss.com/docs/grid-template-columns" target="_blank" rel="noopener noreferrer"><code>grid_cols_min_content</code></a> | 以 min-content 为最小值创建列。 |
| <a href="https://tailwindcss.com/docs/grid-template-columns" target="_blank" rel="noopener noreferrer"><code>grid_cols_max_content</code></a> | 以 max-content 为约束创建列。 |
| <a href="https://tailwindcss.com/docs/grid-template-rows" target="_blank" rel="noopener noreferrer"><code>grid_rows_min_content</code></a> | 以 min-content 为最小值创建行。 |
| <a href="https://tailwindcss.com/docs/grid-template-rows" target="_blank" rel="noopener noreferrer"><code>grid_rows_max_content</code></a> | 以 max-content 为约束创建行。 |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_start</code></a> | 设置 Grid 列起始线。 |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_start_auto</code></a> | 自动选择列起始位置。 |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_end</code></a> | 设置 Grid 列结束线。 |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_end_auto</code></a> | 自动选择列结束位置。 |
| <a href="https://tailwindcss.com/docs/grid-column" target="_blank" rel="noopener noreferrer"><code>col_span_full</code></a> | 跨越完整的 Grid 列范围。 |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_start</code></a> | 设置 Grid 行起始线。 |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_start_auto</code></a> | 自动选择行起始位置。 |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_end</code></a> | 设置 Grid 行结束线。 |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_end_auto</code></a> | 自动选择行结束位置。 |
| <a href="https://tailwindcss.com/docs/grid-row" target="_blank" rel="noopener noreferrer"><code>row_span_full</code></a> | 跨越完整的 Grid 行范围。 |

### 间距与尺寸

| 方法 | 说明 |
| --- | --- |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_2</code></a> | 行间隙和列间隙均设为 `0.5rem`。 |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap</code></a> | 在双轴设置带类型的间隙。 |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_x_2</code></a> | 列间隙设为 `0.5rem`。 |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_y_2</code></a> | 行间隙设为 `0.5rem`。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>p_4</code></a> | 四边内边距设为 `1rem`。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>p</code></a> | 四边设置带类型的内边距。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>px_3</code></a> | 水平内边距设为 `0.75rem`。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>py_2</code></a> | 垂直内边距设为 `0.5rem`。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mt_4</code></a> | 上外边距设为 `1rem`。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>m</code></a> | 四边设置带类型的外边距。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>m_auto</code></a> | 四边外边距设为自动。 |
| <a href="https://tailwindcss.com/docs/width" target="_blank" rel="noopener noreferrer"><code>w</code></a> | 设置带类型的宽度。 |
| <a href="https://tailwindcss.com/docs/width" target="_blank" rel="noopener noreferrer"><code>w_full</code></a> | 填满可用宽度。 |
| <a href="https://tailwindcss.com/docs/height" target="_blank" rel="noopener noreferrer"><code>h</code></a> | 设置带类型的高度。 |
| <a href="https://tailwindcss.com/docs/height" target="_blank" rel="noopener noreferrer"><code>h_full</code></a> | 填满可用高度。 |
| <a href="https://tailwindcss.com/docs/min-width" target="_blank" rel="noopener noreferrer"><code>min_w</code></a> | 设置带类型的最小宽度。 |
| <a href="https://tailwindcss.com/docs/min-width" target="_blank" rel="noopener noreferrer"><code>min_w_0</code></a> | 允许宽度收缩到零。 |
| <a href="https://tailwindcss.com/docs/max-width" target="_blank" rel="noopener noreferrer"><code>max_w</code></a> | 设置带类型的最大宽度。 |
| <a href="https://tailwindcss.com/docs/width#setting-both-width-and-height" target="_blank" rel="noopener noreferrer"><code>size</code></a> | 同时设置带类型的宽度与高度。 |
| <a href="https://tailwindcss.com/docs/width#setting-both-width-and-height" target="_blank" rel="noopener noreferrer"><code>size_4</code></a> | 宽度和高度均设为 `1rem`。 |
| <a href="https://tailwindcss.com/docs/aspect-ratio" target="_blank" rel="noopener noreferrer"><code>aspect_square</code></a> | 保持 1:1 的宽高比。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mt</code></a> | 设置带类型的上外边距。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mb</code></a> | 设置带类型的下外边距。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mx</code></a> | 设置带类型的水平外边距。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>my</code></a> | 设置带类型的垂直外边距。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>ml</code></a> | 设置带类型的左外边距。 |
| <a href="https://tailwindcss.com/docs/margin" target="_blank" rel="noopener noreferrer"><code>mr</code></a> | 设置带类型的右外边距。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>px</code></a> | 设置带类型的水平内边距。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>py</code></a> | 设置带类型的垂直内边距。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pt</code></a> | 设置带类型的上内边距。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pb</code></a> | 设置带类型的下内边距。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pl</code></a> | 设置带类型的左内边距。 |
| <a href="https://tailwindcss.com/docs/padding" target="_blank" rel="noopener noreferrer"><code>pr</code></a> | 设置带类型的右内边距。 |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_x</code></a> | 设置带类型的列间隙。 |
| <a href="https://tailwindcss.com/docs/gap" target="_blank" rel="noopener noreferrer"><code>gap_y</code></a> | 设置带类型的行间隙。 |
| <a href="https://tailwindcss.com/docs/min-height" target="_blank" rel="noopener noreferrer"><code>min_h</code></a> | 设置带类型的最小高度。 |
| <a href="https://tailwindcss.com/docs/max-height" target="_blank" rel="noopener noreferrer"><code>max_h</code></a> | 设置带类型的最大高度。 |
| <a href="https://tailwindcss.com/docs/aspect-ratio" target="_blank" rel="noopener noreferrer"><code>aspect_ratio</code></a> | 设置数字形式的宽高比。 |

### 定位与溢出

| 方法 | 说明 |
| --- | --- |
| <a href="https://tailwindcss.com/docs/position" target="_blank" rel="noopener noreferrer"><code>relative</code></a> | 保持正常布局位置，并作为定位祖先。 |
| <a href="https://tailwindcss.com/docs/position" target="_blank" rel="noopener noreferrer"><code>absolute</code></a> | 用 inset 偏移，相对祖先定位。 |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>inset</code></a> | 四边设置带类型的偏移。 |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>inset_0</code></a> | 上、右、下、左偏移均设为零。 |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>top</code></a> | 设置带类型的上方偏移。 |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>top_0</code></a> | 上方偏移设为零。 |
| <a href="https://tailwindcss.com/docs/overflow" target="_blank" rel="noopener noreferrer"><code>overflow_hidden</code></a> | 在双轴裁剪溢出的内容。 |
| <a href="https://tailwindcss.com/docs/overflow" target="_blank" rel="noopener noreferrer"><code>overflow_x_hidden</code></a> | 只裁剪水平溢出。 |
| <a href="https://tailwindcss.com/docs/overflow" target="_blank" rel="noopener noreferrer"><code>overflow_y_hidden</code></a> | 只裁剪垂直溢出。 |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>bottom</code></a> | 设置带类型的下方偏移。 |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>left</code></a> | 设置带类型的左方偏移。 |
| <a href="https://tailwindcss.com/docs/top-right-bottom-left" target="_blank" rel="noopener noreferrer"><code>right</code></a> | 设置带类型的右方偏移。 |
| <a href="https://tailwindcss.com/docs/scrollbar-width" target="_blank" rel="noopener noreferrer"><code>scrollbar_width</code></a> | 在滚动布局中预留带类型的滚动条宽度。 |

### 颜色与边框

| 方法 | 说明 |
| --- | --- |
| <a href="https://tailwindcss.com/docs/background-color" target="_blank" rel="noopener noreferrer"><code>bg</code></a> | 设置带类型的背景填充。 |
| <a href="https://tailwindcss.com/docs/color" target="_blank" rel="noopener noreferrer"><code>text_color</code></a> | 设置带类型的文字颜色。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_1</code></a> | 四边设置一像素边框。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_t_1</code></a> | 上边设置一像素边框。 |
| <a href="https://tailwindcss.com/docs/border-color" target="_blank" rel="noopener noreferrer"><code>border_color</code></a> | 设置带类型的边框颜色。 |
| <a href="https://tailwindcss.com/docs/border-style" target="_blank" rel="noopener noreferrer"><code>border_dashed</code></a> | 绘制虚线边框。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded</code></a> | 设置带类型的圆角半径。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_lg</code></a> | 使用命名的大圆角半径。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_full</code></a> | 让圆角半径尽可能适应该元素的尺寸。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border</code></a> | 四边设置带类型的边框宽度。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_t</code></a> | 设置带类型的上边框宽度。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_b</code></a> | 设置带类型的下边框宽度。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_l</code></a> | 设置带类型的左边框宽度。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_r</code></a> | 设置带类型的右边框宽度。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_x</code></a> | 设置带类型的左右边框宽度。 |
| <a href="https://tailwindcss.com/docs/border-width" target="_blank" rel="noopener noreferrer"><code>border_y</code></a> | 设置带类型的上下边框宽度。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_t</code></a> | 设置带类型的上侧圆角。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_b</code></a> | 设置带类型的下侧圆角。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_l</code></a> | 设置带类型的左侧圆角。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_r</code></a> | 设置带类型的右侧圆角。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_tl</code></a> | 设置带类型的左上圆角。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_tr</code></a> | 设置带类型的右上圆角。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_bl</code></a> | 设置带类型的左下圆角。 |
| <a href="https://tailwindcss.com/docs/border-radius" target="_blank" rel="noopener noreferrer"><code>rounded_br</code></a> | 设置带类型的右下圆角。 |

### 排版

| 方法 | 说明 |
| --- | --- |
| <a href="https://tailwindcss.com/docs/font-family" target="_blank" rel="noopener noreferrer"><code>font_family</code></a> | 按名称设置字体族。 |
| <a href="https://tailwindcss.com/docs/font-weight" target="_blank" rel="noopener noreferrer"><code>font_weight</code></a> | 设置带类型的字重。 |
| <a href="https://tailwindcss.com/docs/font-style" target="_blank" rel="noopener noreferrer"><code>italic</code></a> | 使用斜体文字。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_size</code></a> | 设置带类型的字号。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_xs</code></a> | 使用特小字号。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_sm</code></a> | 使用小字号。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_lg</code></a> | 使用大字号。 |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_left</code></a> | 左对齐文字。 |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_center</code></a> | 让一行中的文字居中。 |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_right</code></a> | 右对齐文字。 |
| <a href="https://tailwindcss.com/docs/line-height" target="_blank" rel="noopener noreferrer"><code>line_height</code></a> | 设置带类型的行高。 |
| <a href="https://tailwindcss.com/docs/white-space" target="_blank" rel="noopener noreferrer"><code>whitespace_normal</code></a> | 允许文字正常换行。 |
| <a href="https://tailwindcss.com/docs/white-space" target="_blank" rel="noopener noreferrer"><code>whitespace_nowrap</code></a> | 阻止文字换行。 |
| <a href="https://tailwindcss.com/docs/text-overflow" target="_blank" rel="noopener noreferrer"><code>text_ellipsis</code></a> | 在溢出文字末尾添加省略号。 |
| <a href="https://tailwindcss.com/docs/text-overflow" target="_blank" rel="noopener noreferrer"><code>truncate</code></a> | 裁剪单行文字并添加省略号。 |
| <a href="https://tailwindcss.com/docs/line-clamp" target="_blank" rel="noopener noreferrer"><code>line_clamp</code></a> | 将文字限制为指定行数。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_base</code></a> | 使用基础字号。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_xl</code></a> | 使用特大字号。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_2xl</code></a> | 使用 2× 大字号。 |
| <a href="https://tailwindcss.com/docs/font-size" target="_blank" rel="noopener noreferrer"><code>text_3xl</code></a> | 使用 3× 大字号。 |
| <a href="https://tailwindcss.com/docs/text-align" target="_blank" rel="noopener noreferrer"><code>text_align</code></a> | 设置带类型的文字对齐方式。 |
| <a href="https://tailwindcss.com/docs/font-style" target="_blank" rel="noopener noreferrer"><code>not_italic</code></a> | 使用非斜体文字。 |
| <a href="https://tailwindcss.com/docs/text-decoration-line" target="_blank" rel="noopener noreferrer"><code>underline</code></a> | 添加文字下划线。 |
| <a href="https://tailwindcss.com/docs/text-decoration-line" target="_blank" rel="noopener noreferrer"><code>line_through</code></a> | 添加文字删除线。 |
| <a href="https://tailwindcss.com/docs/text-decoration-line" target="_blank" rel="noopener noreferrer"><code>text_decoration_none</code></a> | 移除文字装饰线。 |
| <a href="https://tailwindcss.com/docs/text-decoration-color" target="_blank" rel="noopener noreferrer"><code>text_decoration_color</code></a> | 设置文字装饰线颜色。 |
| <a href="https://tailwindcss.com/docs/text-decoration-style" target="_blank" rel="noopener noreferrer"><code>text_decoration_solid</code></a> | 使用实线文字装饰。 |
| <a href="https://tailwindcss.com/docs/text-decoration-style" target="_blank" rel="noopener noreferrer"><code>text_decoration_wavy</code></a> | 使用波浪线文字装饰。 |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_0</code></a> | 把文字装饰线宽度设为零。 |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_1</code></a> | 把文字装饰线宽度设为一像素。 |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_2</code></a> | 把文字装饰线宽度设为两像素。 |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_4</code></a> | 把文字装饰线宽度设为四像素。 |
| <a href="https://tailwindcss.com/docs/text-decoration-thickness" target="_blank" rel="noopener noreferrer"><code>text_decoration_8</code></a> | 把文字装饰线宽度设为八像素。 |
| <a href="https://tailwindcss.com/docs/text-overflow" target="_blank" rel="noopener noreferrer"><code>text_overflow</code></a> | 设置带类型的文字溢出行为。 |
| <a href="https://tailwindcss.com/docs/font-feature-settings" target="_blank" rel="noopener noreferrer"><code>font_features</code></a> | 设置 OpenType 字体特性。 |

### 效果与光标

| 方法 | 说明 |
| --- | --- |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_none</code></a> | 移除盒阴影。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_sm</code></a> | 使用命名的小盒阴影。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_md</code></a> | 使用命名的中等盒阴影。 |
| <a href="https://tailwindcss.com/docs/opacity" target="_blank" rel="noopener noreferrer"><code>opacity</code></a> | 用浮点值设置不透明度。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_pointer</code></a> | 悬停时使用指向手形光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_text</code></a> | 悬停时使用文本插入光标。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow</code></a> | 设置带类型的盒阴影列表。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_2xs</code></a> | 使用命名的 2× 特小盒阴影。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_xs</code></a> | 使用命名的特小盒阴影。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_lg</code></a> | 使用命名的大盒阴影。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_xl</code></a> | 使用命名的特大盒阴影。 |
| <a href="https://tailwindcss.com/docs/box-shadow" target="_blank" rel="noopener noreferrer"><code>shadow_2xl</code></a> | 使用命名的 2× 特大盒阴影。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor</code></a> | 设置带类型的鼠标光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_default</code></a> | 使用默认光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_move</code></a> | 使用移动光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_not_allowed</code></a> | 使用禁止操作光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_context_menu</code></a> | 使用上下文菜单光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_crosshair</code></a> | 使用十字光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_vertical_text</code></a> | 使用竖排文字光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_alias</code></a> | 使用别名光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_copy</code></a> | 使用复制光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_no_drop</code></a> | 使用禁止拖放光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_grab</code></a> | 使用抓取光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_grabbing</code></a> | 使用正在抓取的光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_ew_resize</code></a> | 使用水平调整尺寸光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_ns_resize</code></a> | 使用垂直调整尺寸光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_nesw_resize</code></a> | 使用东北至西南方向调整尺寸光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_nwse_resize</code></a> | 使用西北至东南方向调整尺寸光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_col_resize</code></a> | 使用列宽调整光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_row_resize</code></a> | 使用行高调整光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_n_resize</code></a> | 使用向上调整尺寸光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_e_resize</code></a> | 使用向右调整尺寸光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_s_resize</code></a> | 使用向下调整尺寸光标。 |
| <a href="https://tailwindcss.com/docs/cursor" target="_blank" rel="noopener noreferrer"><code>cursor_w_resize</code></a> | 使用向左调整尺寸光标。 |

### GPUI 特有方法

这些 API 没有直接对应的 Tailwind utility。方法名不加链接，以免暗示不存在的对应关系。

| 方法 | 说明 |
| --- | --- |
| `font` | 用带类型的 GPUI `Font` 替换文字字体。 |
| `min_size` | 同时设置带类型的最小宽度与高度。 |
| `max_size` | 同时设置带类型的最大宽度与高度。 |
| `text_bg` | 设置文字片段而非元素盒的背景颜色。 |
| `text_ellipsis_start` | 在开头截断，保留文字末尾。 |
| `text_ellipsis_middle` | 在中间截断，保留文字两端。 |
| `debug` | 在 debug 构建中绘制调试轮廓。 |
| `debug_below` | 在 debug 构建中为当前元素及符合条件的后代绘制调试轮廓。 |
| `style` | 返回元素使用的可变 `StyleRefinement`；这是 trait 的底层访问方法。 |
| `text_style` | 返回元素样式中的可变文字 refinement。 |
| `grid_location_mut` | 访问 `StyleRefinement` 中可变的 Grid 位置。 |

宏还为尺寸（`w`、`h`、`size`、`min_size`、`min_w`、`min_h`、`max_size`、`max_w`、`max_h`）、间隙（`gap`、`gap_x`、`gap_y`）、外边距（`m`、`mt`、`mb`、`mx`、`my`、`ml`、`mr`）、内边距（`p`、`pt`、`pb`、`px`、`py`、`pl`、`pr`）和偏移（`inset`、`top`、`bottom`、`left`、`right`）生成方法。例如 `w_64`、`px_3` 和 `top_0` 都是真实存在的方法。共用的数字后缀包括 `0`、`0p5`、`1`、`1p5`、`2`、`2p5`、`3`、`3p5`、`4` 到 `12` 的每个整数，以及 `16`、`20`、`24`、`32`、`40`、`48`、`56`、`64`、`72`、`80`、`96`、`112`、`128`。此外还有 `_px`、`_full` 和 `_1_2` 等分数后缀；只有接受 `auto` 的方法族才生成 `_auto`。非 `auto` 值还生成 `mt_neg_2` 这样的 `_neg_` 形式。边框各侧（`border`、`border_t`、`border_b`、`border_l`、`border_r`、`border_x`、`border_y`）有 `0` 到 `12`，以及 `16`、`20`、`24`、`32` 的像素宽度后缀。各侧和各角的圆角有 `none`、`xs`、`sm`、`md`、`lg`、`xl`、`2xl`、`3xl`、`full` 这些后缀。只有在生成的属性符合布局需要时才使用它们。

间距便捷方法使用基于 rem 的尺度：`_1` 为 `0.25rem`，`_2` 为 `0.5rem`，`_3` 为 `0.75rem`，`_4` 为 `1rem`。命名方法没有覆盖所需数值时，可使用 `.gap(rems(0.625))`、`.w(px(240.))` 或 `.w(relative(0.5))` 这类带类型的 setter。`relative(0.5)` 表示可用相对尺寸的一半；`px(...)` 表示像素。命名尺度和方法范围以 GPUI 的实现为准，不要假定每个 Tailwind class 都有对应方法。

## 样式调用改动了什么

每个实现 `Styled` 的元素都提供 `fn style(&mut self) -> &mut StyleRefinement`。例如，`.px_3()` 写入相应的可选 padding 字段，`.bg(...)` 写入 background 字段。合并样式时，未设置的字段不会覆盖已有值。解析后的 `Style` 同时包含布局数据与外观数据。

```text
Styled calls → StyleRefinement → resolved Style
                                    ├─ layout fields → Taffy → bounds
                                    └─ color, text, shadow, cursor → GPUI paint and interaction
```

在元素的 `request_layout` 阶段，GPUI 将 display、size、padding、gap、Flex 对齐、position 和 Grid 位置等布局字段连同子元素的布局 ID 交给 Taffy。Taffy 计算几何尺寸与位置。GPUI 随后在 `prepaint` 和 [Paint](./paint) 阶段使用所得 bounds 绘制并进行命中测试。Taffy 不实现 GPUI 的文字 shaping、hover listener、[Action](./action) 或绘制。

`StyleRefinement` 本身也实现了 `Styled`，因此状态样式闭包可以使用相同的 utility 方法。`.hover(|style| style.bg(...))` 这样的交互变体属于 `InteractiveElement`，需要交互型元素。条件构建方法有所不同：`.when(...)` 在构建本帧时决定是否追加链式步骤。

## Fluent 组合与 trait 边界

链式 API 由多个 trait 共同提供。`Styled` 提供样式方法，`ParentElement` 提供 `.child(...)`。`InteractiveElement` 提供 `.id(...)`，返回支持 `.overflow_y_scroll()` 等依赖身份的方法的 `Stateful<Div>`；它还提供 `.hover(...)` 等状态样式细化。每个 `IntoElement` 都实现 `FluentBuilder`；只实现 `IntoElement` 不会获得样式、子元素或交互能力。缺少某个方法时，检查接收者实现了哪个 trait，以及之前的调用是否改变了其类型。

| `FluentBuilder` 方法 | 作用 |
| --- | --- |
| `map` | 转换当前值，返回类型也可以改变。 |
| `when` | 布尔条件为真时追加构建步骤。 |
| `when_else` | 在两个返回相同 builder 类型的步骤间选择。 |
| `when_some` | 可选值存在时追加步骤，并把值传给闭包。 |
| `when_none` | 引用的可选值为空时追加步骤。 |

```rust
use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;

div()
    .id("result-row")
    .px_3()
    .py_2()
    .when(selected, |row| row.bg(cx.theme().selection))
    .when_some(subtitle, |row, text| row.child(text))
    .child(title)
```

条件闭包取得 builder 并返回 builder。条件在本次 render 中求值，并非订阅。`.map(...)` 可以返回不同类型。跨帧变化的状态应由 `Entity` 持有。

例如，`.when(selected, ...)` 在构建本帧时判断 `selected`；`.hover(|style| ...)` 则安装指针悬停时的样式细化。滚动方法需要有状态的元素，所以应先给预期的滚动负责元素设置 `.id(...)`。不能假定任意实现 `IntoElement` 的组件都接受 `.child(...)` 或 `.bg(...)`；应检查该组件自己的 builder API，或用负责这些样式的 `div()` 包裹它。

GPUI Kit 另有 `StyledExt`，提供 `h_flex`、`v_flex` 和 `refine_style` 等不依赖主题的辅助方法；`ThemeStyled` 则提供 `popover_style(cx)` 等依赖主题的外观方法。它们是在 GPUI `Styled` 之上的扩展，不属于 Tailwind utility。自行实现元素时，应从 `style()` 返回元素的 `StyleRefinement`，并在布局与绘制阶段应用解析后的样式。只实现元素确实支持的子元素与交互 trait。

Tailwind 对应关系有明确范围：这里没有 CSS 选择器、层叠、响应式前缀，也不保证每个 Tailwind utility 都存在对应的 GPUI 方法。几何布局使用 GPUI 带类型的辅助方法，产品外观使用 GPUI Kit 主题 token，行为则使用明确的元素或实体 API。

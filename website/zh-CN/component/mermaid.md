---
title: Mermaid
description: 使用 Merman 和 GPUI SVG 渲染器预览 Mermaid 图表。
---

# Mermaid

`Mermaid` 根据 Mermaid 源码显示图表，无需浏览器或 JavaScript 运行时。
Merman 负责解析和布局，生成兼容 resvg 的 SVG，再由 GPUI 转成彩色图像。
适用于流程图、时序图、类图及其他 Merman 支持的图表类型的只读预览。

## 启用

在 `gpui-kit` 依赖上启用可选的 `mermaid` feature。组件仅支持原生平台，
不在 WebAssembly 上导出。Merman 0.9.2 要求 Rust 1.95 或更新版本，
同时仍需满足 GPUI 的要求。

```toml
[dependencies]
gpui-kit = { version = "0.8", features = ["mermaid"] }
```

使用 `gpui_kit::init(cx)` 初始化，通过 `gpui_kit::open_window` 打开窗口，
详见[快速开始](../docs/getting-started.md)。

## 用法

```rust
use gpui_kit::*;
use gpui_kit::component::mermaid::{Mermaid, MermaidState};
```

创建一次状态，并在所属视图中保存为 `Entity<MermaidState>`：

```rust
let diagram = cx.new(|cx| MermaidState::new("flowchart LR\nA[Start] --> B[Done]", cx));
```

渲染时只构建展示组件：

```rust
Mermaid::new(&self.diagram)
    .accessibility_label("流程从 Start 进入 Done")
    .w_full()
    .h(rems(24.))
```

默认视口占满宽度，高度为 20 rem。图像按比例缩放以适应视口。
请提供能说明图表含义的无障碍描述；图表内部节点不支持交互，也不会作为文本控件暴露。

从编辑器或其他应用状态更新源码：

```rust
self.diagram.update(cx, |state, cx| {
    state.set_source("sequenceDiagram\nAlice->>Bob: Hello", cx);
});
```

## 渲染行为

预览区获得非零布局尺寸后立即开始渲染。源码更新采用 150 ms 防抖；首次渲染及主题、DPI
变化不会额外等待。解析、布局、SVG 处理和光栅化均在后台执行。
源码变更时取消旧任务，丢弃过期结果。空源码显示空状态；
无效或不支持的源码显示诊断信息。通过 `source()`、`is_loading()` 和 `error()`
读取当前状态。

组件通过 Merman 的 `HostTheme` 映射当前主题的语义颜色和图表配色，覆盖各图表类型的独立配置。
主题变化会重新渲染图表，预览区尺寸或屏幕缩放比例变化则复用
已解析的 SVG。光栅化同时考虑预览区的宽高，采用两倍设备像素分辨率，
既保持清晰，也避免生成整窗大小的图像。
默认采用 Classic 外观，以实线区分类图中的各个分区。标签使用原生 SVG 文本，
`resvg_safe` 在 GPUI 解析前移除仅适用于浏览器的内容。组件还会修正 Merman 0.9.2
原生类图关系标签的坐标，使文字在背景框中居中，同时对齐 Use Case 标签并补全 Info 图缺失的
SVG 视口。生成的 HTML 标签回退允许字形溢出，避免估算宽度裁掉字符；这并不替代 Merman
的文字测量与布局。
每个状态保留一份图表，替换图表或释放状态时清理 GPUI 图像缓存。

渲染采用 Merman 的 constrained 资源配置和十秒协作式截止时间；源码上限为
128 KiB，SVG 上限为 8 MiB。光栅图像每边最多 4096 像素，总计约四百万像素。
因此大型图表放大后可能不够清晰。取消与截止时间依赖协作检查，并非进程隔离或硬超时。

在自己的应用中，可参考本仓库，在工作区的 `[profile.dev.package]` 中优化 `resvg`、
`tiny-skia` 和 `tiny-skia-path`，提升开发构建的响应速度。仅优化 `resvg` 不会优化底层
像素循环；依赖库无法替应用设置编译 profile。

## 兼容性

组件使用 Merman 0.9.2，语法及布局可能与 Mermaid.js 浏览器预览不同。
编译时包含所有内置图表类型及 Cytoscape 布局，不支持 ELK 布局、数学公式标签、
外部或内嵌图像以及浏览器交互。默认使用 Dagre 布局；源码显式指定不支持的选项时
可能返回诊断信息。确定性的文本测量与 SVG 光栅化使用的系统字体可能存在差异，
尤其是较长或混合文字标签。

组件为 Merman 0.9.2 提供了范围明确的兼容修复。Timeline 保留首个命名 section 前的事件，
纵向布局也会为前一组事件留足空间。渲染器根据解析器提供的位置适配内部源码副本，
`MermaidState::source()` 保持不变。Railroad 在轨道上方显示标题，长标题自动换行；规则名与轨道垂直对齐，并适配起点标记前的预留宽度。
Flowchart 的连线标签使用不透明背景，避免连线穿过文字。
Requirement 的类型标题适配内容宽度，C4 关系标签及 State 标签背景得到对齐。
Cynefin、Event Modeling 和 Journey 根据分类填充色选择可读的文字颜色，Tree View
继承组件字体。

专门的回归样例和可选的 `.labels` 内容检查文件保存在
`crates/component/src/mermaid/fixtures`；默认忽略的 `render_audit_fixtures` 测试
使用实际的 SVG 转换和 GPUI 光栅化流程，覆盖深浅主题。设置输出目录 `GPUI_MERMAID_OUTPUT` 后，
使用 `cargo test -p gpui-component --features mermaid --lib render_audit_fixtures -- --ignored --nocapture` 运行该测试。
这些检查不代表与 Mermaid.js 完全兼容，也不保证各平台字体测量完全相同。
较长的混合文字标签仍可能留白偏紧，超大图表会缩放到受尺寸限制的预览中。

此独立组件不会替换 Markdown 代码块的渲染。原生 Story Gallery 的 **Mermaid**
示例提供可编辑源码和可搜索的选择器，覆盖当前启用的 34 类图表，部分示例包含中文标签。
Timeline 中的 `section Delivery` 表示新建名为“Delivery（交付阶段）”的分组；
后续时间段都属于该组，直到下一个 `section`。

Gallery 源码保存在 `crates/story/src/fixtures/mermaid`。运行上述手动检查时，将
`GPUI_MERMAID_FIXTURES` 设为此目录，即可验证 Gallery 实际使用的示例。

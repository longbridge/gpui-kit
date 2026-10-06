---
title: Diff
description: 用于代码审阅与变更预览的只读 unified diff 和 Git diff 展示组件。
---

# Diff

`Diff` 在只读视图中展示应用提供的 unified diff 或 Git diff，适用于代码审阅与变更预览。
需要编辑源代码时，使用 [Editor](./editor.md)。

应用从 Git、AI 响应或其他来源获取 patch。`DiffFile::parse` 为每个变更文件生成一个 `DiffFile`，
不比较修改前后的文件。一个 `DiffState` 在同一个虚拟列表中展示这些文件，每个文件以各自的文件头开始。

## 导入

```rust
use gpui_kit::*;
use gpui_kit::component::diff::{
    Diff, DiffFile, DiffLineAnnotation, DiffLinePosition, DiffLineRange,
    DiffMode, DiffSide, DiffState,
};
```

先调用 `gpui_kit::init(cx)` 初始化组件，再通过 `gpui_kit::open_window` 打开窗口。
参见 [Getting Started](../docs/getting-started.md)。

## 基础用法

在所属视图中解析 patch，并创建一次状态：

```rust
let patch = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    run();\n+    run_app();\n }\n";
let files = DiffFile::parse(patch)?;
let diff = cx.new(|cx| DiffState::new(files, cx));
```

`parse` 返回 `Result<Vec<DiffFile>, DiffParseError>`，由应用处理解析错误。
应用需要分开展示文件时，也可以只传入部分文件，或以 `[file]` 传入单个文件。

将状态作为 `Entity<DiffState>` 保存在视图中。渲染时只构建展示组件，并为它提供明确的视口高度：

```rust
Diff::new(&self.diff).h(rems(24.)).w_full()
```

不要在每次 render 中重新解析 patch 或创建状态。解析很快（release 构建下，6.5 MB、
200 个文件的 patch 约需 15 ms）；patch 特别大时，在后台 executor 中解析，
并在确认结果仍对应当前 patch 版本后再安装文档。

## Patch 输入

支持 unified diff（`---`、`+++`、`@@`）和 Git diff 文本，包括 `git format-patch` 的输出，
以及启用 `diff.noprefix` 或 `diff.mnemonicPrefix` 后生成的 patch。每个文件解析为一个文档。
`path()` 返回用于显示的路径；新增或删除的文件缺少一侧（`/dev/null`），
此时 `original_path()` 或 `modified_path()` 返回 `None`。`additions()` 和 `deletions()`
返回 patch 中的新增、删除行数。

patch 通常只包含变更行和附近上下文。行号表示源文件坐标，不代表完整源文件可用。
Diff 不计算差异，不加载仓库文件，也不应用修改或解决合并冲突。
patch 未提供的源文件内容始终不可用。

`extended_headers()` 提供 Git 的扩展头行，例如权限、相似度、重命名和二进制标记；`is_binary()` 判断二进制变更。
二进制文件、仅修改权限或纯重命名等没有源文本行的文件，在文件头下方显示摘要。
格式错误的 hunk 和不支持的 combined diff 返回 `DiffParseError`，通过 `line()` 和
`message()` 获取 patch 位置与原因。

## 显示模式与上下文

默认采用 `DiffMode::Unified` 显示模式，每处变更前后最多保留三行 patch 已提供的未修改上下文。
在创建状态时设置初始模式，之后由应用命令切换：

```rust
let diff = cx.new(|cx| {
    DiffState::new(files, cx)
        .with_mode(DiffMode::Split)
        .with_context_lines(Some(5))
});

self.diff.update(cx, |state, cx| state.set_mode(DiffMode::Unified, cx));
```

Split 将原始文本和修改后文本并排显示，没有对应行的一侧留空。
Unified 先显示删除行，再显示新增行，保留原始文件行号。

被隐藏的未修改上下文通过紧凑的展开控件表示，显示未修改行数，激活后展开整个隐藏范围。
`expand_all` 展开 patch 提供的全部上下文，保留配置的上下文行数；`collapse_all`
恢复按该行数折叠。将上下文行数设为 `None` 可关闭已有上下文的折叠：

```rust
self.diff.update(cx, |state, cx| {
    state.expand_all(cx);
    state.collapse_all(cx);
    state.set_context_lines(None, cx);
});
```

## 源文件坐标与导航

`DiffSide::Original` 和 `DiffSide::Modified` 分别表示原始文件和修改后文件。
`DiffLinePosition` 与 `DiffLineRange` 以文件的 `path()` 定位文件；patch 的新版本调整文件顺序时，
路径保持不变。它们使用**从 1 开始的源文件行号**，不是当前显示行的下标。行范围包含两端，
且只属于一个文件的一侧。切换显示模式或折叠上下文不会改变源文件行号。

```rust
self.diff.update(cx, |state, cx| {
    state.scroll_to_line(DiffLinePosition::new("src/main.rs", DiffSide::Modified, 12), cx);
    state.scroll_to_file("src/main.rs", cx);
    state.next_change(cx);
    state.previous_change(cx);
});
```

`scroll_to_line` 会按需展开 patch 已提供的隐藏上下文，不能显示 patch 中不存在的行。
`scroll_to_file` 滚动到文件头，没有源文本行的文件也可以定位，适合配合文件列表使用。
变更导航跨所有文件在各个变更组之间移动，到达两端后循环。
显示模式切换与导航命令由应用放置在工具栏或菜单中。

## 选择与复制

通过指定文件、侧和包含两端的行范围，以程序方式选择源文件行：

```rust
self.diff.update(cx, |state, cx| {
    state.set_selected_lines(Some(DiffLineRange::new("src/main.rs", DiffSide::Modified, 3, 8)), cx);
});

let source = self.diff.read(cx).selected_text(cx);

self.diff.update(cx, |state, cx| {
    state.set_selected_lines(None, cx);
});
```

`selected_lines()` 返回当前选择的行范围。行选择仅作用于 patch 已提供的源文件行；
选择不存在的一侧会清空选区。`selected_text(cx)` 返回源文本，不含行号、变更标记、
对齐空白单元格或批注内容。复制不包含 diff 前缀和未提供的间隔，不能还原完整文件，
也不能推断 patch 没有保留的源文件换行格式。

## 替换文档

```rust
self.diff.update(cx, |state, cx| {
    state.set_files(updated_files, cx);
});
```

替换文档会重置展开状态、选区与滚动位置，因为这些坐标属于原先的 patch。
状态修改方法会通知观察者，调用方不需要额外为 Diff 状态调用 `cx.notify()`。

## 外观与滚动

行号、语法高亮和文件头默认开启：

```rust
Diff::new(&self.diff)
    .line_number(true)
    .syntax_highlight(true)
    .header_visible(true)
    .h(rems(24.))
```

组件根据每个文件的路径识别语法语言。启用相应的 Cargo grammar feature，例如
`tree-sitter-rust`；`tree-sitter-languages` 包含全部内置语法。
未启用 grammar 时仍显示已有代码和整行变更颜色。

状态创建或收到新文档后，在后台线程中逐个文件准备语法高亮；代码先以无高亮形式显示，
每个文件准备完成后再显示颜色。每个 hunk 单独解析，前一个 hunk 末尾未闭合的注释或字符串
不会影响下一个 hunk。不高亮注入语言，例如 Markdown 中的代码块。超过 1,000 字节的行
不应用语法高亮。patch 片段仍可能缺少语法上下文，高亮不代表完整语法解析。
组件不计算文件差异或词级差异。

源代码使用主题中的等宽字体与语法主题。复制时保留 patch 提供的源文本空白；
`\ No newline at end of file` 标记会明确显示。所有文件的行共同参与虚拟化，
批注高度也参与测量。Diff 管理横向与纵向滚动，应避免再将它嵌入另一个滚动区域。
长行通过横向滚动查看，不支持软换行。

## 文件头

默认文件头显示路径、以 `old → new` 表示的重命名、新增与删除行数，以及文件是否为新增或删除。
`header` 替换文件头内容，保留外层布局与分隔线；`header_visible(false)` 隐藏文件头：

```rust
Diff::new(&self.diff)
    .header(|file, _, _| {
        div()
            .flex()
            .gap_2()
            .child(file.path().clone())
            .child(format!("+{}", file.additions()))
    })
    .h(rems(24.))
```

回调接收 `&DiffFile`、`&mut Window` 和 `&mut App`，返回 `IntoElement`。
回调实现 `Fn`，捕获的值必须满足 `'static`；接收的是 `App`，不是所属视图的 `Context`。
在回调中构建内容，在内容的事件处理函数中修改状态。不要在渲染回调内同步读取或更新
同一个 `DiffState` 实体。

## 源文件行批注

批注具有稳定的应用 ID 和从 1 开始的源文件位置。评论内容与草稿状态由应用持有，
在对应源文件行下方渲染：

```rust
let annotation = DiffLineAnnotation::new(
    "review-comment-42",
    DiffLinePosition::new("src/main.rs", DiffSide::Modified, 3),
);

Diff::new(&self.diff)
    .annotations([annotation])
    .annotation_content(|_, _, _| div().child("Check the fallback behavior."))
    .h(rems(24.))
```

同时提供 `annotations` 和 `annotation_content`。回调接收
`&DiffLineAnnotation`、`&mut Window` 和 `&mut App`，可通过 `id()` 与 `position()`
查找应用内容。批注 ID 在同一个 Diff 中必须稳定且唯一。批注位于隐藏上下文时，
对应源文件行展开后才会显示。无效行位置或不存在的一侧不会渲染批注。
Unified 中未修改的行只显示一次，但可以显示原始侧和修改后侧的批注。
批注回调与文件头回调具有相同的 `Fn` 和 `'static` 要求。

批注内容的高度可以随意变化。可见行每一帧都会重新测量；添加、移动、删除或替换批注时，
相关行也会重新测量。Split 模式中的配对行随较高的一侧伸展。
复制源文本时不包含批注内容；批注中的按钮和其他控件管理各自的操作。

## 鼠标与键盘交互

在代码上拖动可选择同一文件同一侧的源文本，选区可以跨越虚拟列表范围和已有上下文的折叠范围，
不能选择未提供的源文本。点击行号选择该源文件行；按住 Shift 点击同侧的另一个行号，
将选区扩展为包含两端的范围。文本选择与源文件行选择是两种不同模式。
通过 Tab 可聚焦视图，聚焦时保留普通边框。

以下默认快捷键在代码区域聚焦时生效：

| 命令 | 快捷键 |
| --- | --- |
| 纵向／横向滚动 | ↑ / ↓、← / → |
| 滚动一页 | PageUp / PageDown |
| 滚动到开头／末尾 | Home / End |
| 扩展源文件行选区 | Shift+↑ / Shift+↓ |
| 复制源文本选区 | macOS：Cmd+C；Windows/Linux：Ctrl+C |
| 全选当前侧已有的源文本 | macOS：Cmd+A；Windows/Linux：Ctrl+A |

Diff 不为变更导航绑定快捷键，是否为 `next_change` 和 `previous_change` 提供快捷键由应用决定。
全选使用当前选区所在的文件和侧；没有选区时，使用视口顶部的文件，修改后文件有文本则选择
修改后文件，否则选择原始文件。没有行选区时，Shift+↑ 和 Shift+↓ 从第一个已有源文件行开始选择。
上下文展开控件是普通按钮，可通过 Tab 到达。文件头或批注中的控件聚焦时，
使用控件自己的键盘命令；Diff 的源文本复制与滚动快捷键仅在代码区域聚焦时生效。

`DiffEvent::SelectionChanged(Option<DiffLineRange>)` 通知用户对源文件行选区的修改，
不通知每次文本选择操作或程序更新。应用可订阅状态实体，根据选中的行范围提供审阅操作。

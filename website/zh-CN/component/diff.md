---
title: Diff
description: 用于代码审阅、合并冲突和源文件预览的只读 unified diff 与 Git diff 展示组件。
---

# Diff

`Diff` 在只读视图中展示应用提供的 unified diff 或 Git diff，适用于代码审阅与变更预览。
需要编辑源代码时，使用 [Editor](./editor.md)。

应用从 Git、AI 响应或其他来源获取 patch。`DiffFile::parse` 为每个变更文件生成一个 `DiffFile`，
不比较修改前后的文件。一个 `DiffState` 在同一个虚拟列表中展示这些文件，每个文件以各自的文件头开始。
同一个列表也可以展示用于参考的完整源文件，或带有合并冲突的工作区文件。

## 导入

```rust
use gpui_kit::*;
use gpui_kit::component::diff::{
    Diff, DiffAnnotation, DiffChangeIndicator, DiffConflictResolution, DiffFile, DiffHoverHighlight,
    DiffHunkSeparator, DiffInlineUnit, DiffLinePosition, DiffLineRange, DiffMode,
    DiffSide, DiffState,
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
并在确认结果仍对应当前 patch 版本后再安装文件。

## Git history 示例

完整应用见 [tig example](https://github.com/longbridge/gpui-kit/tree/main/examples/tig)。
它在后台执行器中读取本地 Git 历史和 patch，在可调整大小的侧栏中提供虚拟化的提交、文件导航，
并用一个保留的 `DiffState` 显示当前文件。过期请求不会覆盖新选择的提交。
示例还包含键盘导航、源文本复制、运行时显示选项、加载／错误／空状态，以及可折叠的提交正文。

```sh
cargo run -p example-tig -- /path/to/repository
```

## 文件

支持 unified diff（`---`、`+++`、`@@`）和 Git diff 文本，包括 `git format-patch` 的输出，
以及启用 `diff.noprefix` 或 `diff.mnemonicPrefix` 后生成的 patch。每个文件解析为一个 `DiffFile`。
`path()` 返回用于显示的路径，同时用来标识文件；新增或删除的文件缺少一侧（`/dev/null`），
此时 `original_path()` 或 `modified_path()` 返回 `None`。`status()` 以 Git 的状态术语返回
`Added`、`Deleted`、`Modified`、`Renamed` 或 `Copied`；`additions()` 和 `deletions()`
返回 patch 中的新增、删除行数。

patch 通常只包含变更行和附近上下文。行号表示源文件坐标，不代表完整源文件可用。
Diff 不计算差异，不加载仓库文件，也不应用修改。patch 未提供的源文件内容始终不可用。

`extended_headers()` 提供 Git 的扩展头行，例如权限、相似度、重命名和二进制标记；
`is_binary()` 判断二进制变更。二进制文件、仅修改权限或纯重命名等没有源文本行的文件，
在文件头下方显示摘要。格式错误的 hunk 和不支持的 combined diff 返回 `DiffParseError`，
通过 `line()` 和 `message()` 获取 patch 位置与原因。

另有两个构造方法用于不属于 patch 的文件，二者在任何显示模式下都只显示一列：

```rust
// 用于参考的完整文件：没有变更，也不折叠。
let source = DiffFile::unchanged("src/retry.rs", &text);
// 带 Git 冲突标记的工作区文件，支持 diff3 的共同祖先部分。
let merge = DiffFile::parse_conflicts("src/retry.rs", &working_text)?;
```

## 合并冲突

有冲突的文件保留工作区文件中的行号。每个冲突在标题下分别显示当前更改、共同祖先和传入的更改；
当前更改的标题提供 **采用当前更改**、**采用传入的更改** 和 **保留双方更改**，
解决后的冲突只显示保留的行，并提供 **撤销**。Diff 只记录选择结果，写回文件由应用负责：

```rust
let state = self.diff.read(cx);
if let Some(text) = state.resolved_text("src/retry.rs") {
    // 所有冲突都已解决，保存 `text`。
}
```

只要还有未解决的冲突，`resolved_text` 就返回 `None`。`conflict_resolution(path, ix)`
读取单个冲突的解决结果，`resolve_conflict(path, ix, resolution, cx)` 设置或清除结果且不发出事件。
用户的选择会发出 `DiffEvent::ConflictResolved(path, ix)`。

`DiffFile::conflicts()` 按工作文件中的顺序返回只读的 `DiffConflict` 描述。
切片索引就是解决方法和事件中的 `ix`；`conflicts().len()` 返回冲突数量。
`lines()` 包含标记行，`current_lines()`、`base_lines()` 和 `incoming_lines()`
给出各部分源文本的位置。范围使用从 1 开始的工作文件行号，**结束行不包含在范围内**，
因此空部分用 `start == end` 表示。双向冲突的 `base_lines()` 和 `base_label()`
返回 `None`。通过 `current_label()`、`base_label()` 和 `incoming_label()` 读取标签。

例如，应用命令可以采用所有传入的更改，无需再次解析工作文件：

```rust
self.diff.update(cx, |state, cx| {
    let path = "src/retry.rs";
    let count = state.files().iter().find(|file| file.path().as_str() == path)
        .map_or(0, |file| file.conflicts().len());
    for ix in 0..count {
        state.resolve_conflict(path, ix, Some(DiffConflictResolution::Incoming), cx);
    }
});
```

未解决的冲突和不存在的文件或索引都会让 `conflict_resolution` 返回 `None`；
通过 `conflicts()` 枚举有效的冲突，可以区分这两种情况。

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

被隐藏的未修改上下文通过紧凑的展开控件表示，显示未修改行数。它的按钮可以显示紧接上方代码的几行、
紧接下方代码的几行，或展开整个范围。`with_expansion_lines(n)` 设置每次部分展开的行数（默认 20），
`with_min_collapsed_lines(n)` 让少于 `n` 行的未修改范围保持显示（默认 2）。
`expand_unchanged` 展开 patch 提供的全部上下文，保留配置的上下文行数；`collapse_unchanged`
恢复按该行数折叠；将上下文行数设为 `None` 可关闭折叠：

```rust
self.diff.update(cx, |state, cx| {
    state.expand_unchanged(cx);
    state.collapse_unchanged(cx);
    state.set_context_lines(None, cx);
});
```

## 源文件坐标与导航

`DiffSide::Original` 和 `DiffSide::Modified` 分别表示原始文件和修改后文件。
`DiffLinePosition` 与 `DiffLineRange` 以文件的 `path()` 定位文件；patch 的新版本调整文件顺序时，
路径保持不变。它们使用**从 1 开始的源文件行号**，不是当前显示行的下标。
切换显示模式或折叠上下文不会改变源文件行号。

```rust
self.diff.update(cx, |state, cx| {
    state.scroll_to_line(DiffLinePosition::new("src/main.rs", DiffSide::Modified, 12), cx);
    state.scroll_to_file("src/main.rs", cx);
    state.next_change(cx);
    state.previous_change(cx);
});
```

`scroll_to_line` 会按需展开 patch 已提供的隐藏上下文和已收起的文件，不能显示 patch 中不存在的行。
`scroll_to_file` 滚动到文件头，没有源文本行的文件也可以定位，适合配合文件列表使用。
变更导航跨所有文件在各个变更组之间移动，到达两端后循环。
显示模式切换与导航命令由应用放置在工具栏或菜单中。

每个文件头都有收起控件。应用可以用 `set_file_collapsed(path, collapsed, cx)` 和
`is_file_collapsed(path)` 控制它，例如收起审阅者标记为已查看的文件。用户切换时发出
`DiffEvent::FileCollapsed(path)` 或 `DiffEvent::FileExpanded(path)`。`set_files`
安装路径相同的新版本时，已收起的文件保持收起。

## 选择与复制

通过指定路径、侧和包含两端的行范围，以程序方式选择源文件行：

```rust
self.diff.update(cx, |state, cx| {
    state.set_selected_lines(Some(DiffLineRange::new("src/main.rs", DiffSide::Modified, 3, 8)), cx);
});

let source = self.diff.read(cx).selected_text(cx);
```

行范围通常只属于一侧。`with_end_side` 让范围结束于另一侧，此时它按 patch 顺序覆盖两行之间的
patch 内容，也就是 Unified 模式中显示的顺序：

```rust
let range = DiffLineRange::new("src/main.rs", DiffSide::Original, 10, 12)
    .with_end_side(DiffSide::Modified);
```

`selected_lines()` 返回当前范围，并裁剪到 patch 已提供的源文件行；不包含任何已提供行的范围会清空选区。
`selected_text(cx)` 返回源文本，不含行号、变更标记、对齐空白单元格或批注内容。
复制不包含 diff 前缀和未提供的间隔，不能还原完整文件，也不能推断 patch 没有保留的源文件换行格式。

## 替换文件

```rust
self.diff.update(cx, |state, cx| {
    state.set_files(updated_files, cx);
});
```

替换文件会重置展开状态、选区、冲突解决结果与滚动位置，因为这些坐标属于原先的 patch。
状态修改方法会通知观察者，调用方不需要额外为 Diff 状态调用 `cx.notify()`。

## 外观

行号、语法高亮和文件头默认开启：

```rust
Diff::new(&self.diff)
    .line_number(true)
    .syntax_highlight(true)
    .header_visible(true)
    .hunk_separator(DiffHunkSeparator::Metadata)
    .change_indicator(DiffChangeIndicator::Signs)
    .change_background(true)
    .hover_highlight(DiffHoverHighlight::None)
    .soft_wrap(false)
    .h(rems(24.))
```

- `hunk_separator` 用 `@@` 头（`Metadata`）、patch 未包含的行数（`LineInfo`）或细分隔线（`Simple`）
  标记每个 hunk。
- `change_indicator` 用 `+`、`−` 符号（`Signs`）、彩色竖条（`Bars`）或不加标记（`None`）
  标记变更行；`change_background(false)` 去掉整行底色，但保留行内强调。
- `hover_highlight` 强调指针下的行、行号或两者。
- `soft_wrap(true)` 在列宽处自动换行，行高随内容增加，不再横向滚动。

组件根据每个文件的路径识别语法语言；`DiffFile::with_language` 可以用 `rust` 等高亮语言名覆盖识别结果。
启用相应的 Cargo grammar feature，例如 `tree-sitter-rust`；`tree-sitter-languages`
包含全部内置语法。未启用 grammar 时仍显示已有代码和整行变更颜色。

两侧配对的变更行还会强调不同的单词。`with_inline_unit(Some(DiffInlineUnit::Character))`
改为按字符比较，`None` 只强调整行。几乎没有相同内容的配对行不做行内强调，因为标出几乎每个单词只会增加干扰。

状态创建或收到新文件后，在后台线程中逐个文件准备语法高亮和行内强调；代码先以无高亮形式显示，
每个文件准备完成后再显示颜色。每个 hunk 单独解析，前一个 hunk 末尾未闭合的注释或字符串
不会影响下一个 hunk。不高亮注入语言，例如 Markdown 中的代码块。超过 1,000 字节的行
不做语法高亮和行内强调；用 `with_syntax_max_line_length` 和 `with_inline_max_line_length` 调整上限。

源代码使用主题中的等宽字体与语法主题。复制时保留 patch 提供的源文本空白；
`\ No newline at end of file` 标记会明确显示。所有文件的行共同参与虚拟化，
批注高度也参与测量。Diff 管理自身的滚动，应避免再将它嵌入另一个滚动区域。

状态的 `with_*` builder 设置初始值。若要更新已有 entity，使用
`set_expansion_lines`、`set_min_collapsed_lines`、`set_inline_unit`、
`set_inline_max_line_length` 和 `set_syntax_max_line_length`。
这些方法会通知观察者，并保留选区与附近的源文本位置；强调方式改变时会丢弃旧的准备结果并重新准备。

```rust
self.diff.update(cx, |state, cx| {
    state.set_inline_unit(Some(DiffInlineUnit::Character), cx);
});
```

## 文件头

默认文件头显示收起控件、路径、以 `old → new` 表示的重命名、新增与删除行数以及文件状态。
三个插槽可向其中添加应用内容；`render_header` 替换文件头内容，保留外层布局、分隔线和收起控件：

| Builder | 位置 |
| --- | --- |
| `render_header_prefix` | 路径前方 |
| `render_header_title_suffix` | 紧接路径之后 |
| `render_header_suffix` | 行数统计和状态之后 |
| `render_header` | 替换文件头内容 |

```rust
Diff::new(&self.diff)
    .render_header_suffix(|file, _, _| div().child(format!("{} lines", file.additions())))
    .h(rems(24.))
```

每个回调接收 `&DiffFile`、`&mut Window` 和 `&mut App`，返回 `IntoElement`。
`header_visible(false)` 隐藏文件头。回调实现 `Fn`，捕获的值必须满足 `'static`；
接收的是 `App`，不是所属视图的 `Context`。在回调中构建内容，在内容的事件处理函数中修改状态。
不要在渲染回调内同步读取或更新同一个 `DiffState` 实体。

原有的 `header`、`header_prefix`、`header_title_suffix`、`header_suffix` 和
`annotation_content` 仍然可用，作为对应 `render_*` builder 的兼容别名。

## 批注

批注具有稳定的应用 ID 和目标：某个源文件行，或整个文件。评论内容与草稿状态由应用持有，
在对应行下方或文件头下方渲染：

```rust
let annotations = [
    DiffAnnotation::line(
        "review-comment-42",
        DiffLinePosition::new("src/main.rs", DiffSide::Modified, 3),
    ),
    DiffAnnotation::file("review-summary", "src/main.rs"),
];

Diff::new(&self.diff)
    .annotations(annotations)
    .render_annotation(|annotation, _, _| div().child(annotation.id().to_string()))
    .h(rems(24.))
```

同时提供 `annotations` 和 `render_annotation`。回调接收 `&DiffAnnotation`，可通过 `id()`、
`path()` 和 `position()` 查找应用内容；文件级批注的 `position()` 为 `None`。批注 ID
在同一个 Diff 中必须稳定且唯一。批注位于隐藏上下文时，对应源文件行展开后才会显示。
无效行位置或不存在的一侧不会渲染批注。Unified 中未修改的行只显示一次，但可以显示两侧的批注。

批注内容的高度可以随意变化。可见行每一帧都会重新测量；添加、移动、删除或替换批注时，
相关行也会重新测量。Split 模式中的配对行随较高的一侧伸展。复制源文本时不包含批注内容；
批注中的控件管理各自的操作。

要让审阅者发起评论，处理 `on_add_annotation`。此时悬停行旁和选中行的末尾会出现添加按钮；
如果该行在选区内，回调收到整个选区，否则只收到这一行：

```rust
Diff::new(&self.diff).on_add_annotation(cx.listener(|this, range: &DiffLineRange, _, cx| {
    this.start_comment(range.clone(), cx);
}))
```

## 鼠标与键盘交互

滚轮的两个方向独立处理：垂直输入滚动源文本行；水平输入（包括平台转换后的
Shift+滚轮手势）滚动长行，不改变垂直位置。启用软换行后不再产生水平溢出。

在代码上拖动可选择同一文件同一侧的源文本，选区可以跨越虚拟列表范围和已有上下文的折叠范围，
不能选择未提供的源文本。点击行号选择该行，在行号上拖动选择一个范围，按住 Shift 点击另一个行号
扩展为包含两端的范围。Unified 模式中，范围可以从删除行跨到新增行；Split 模式中切换到另一侧会开始新的选区。
文本选择与源文件行选择是两种不同模式。通过 Tab 可聚焦视图，聚焦时保留普通边框。

`on_line_click` 报告对某行代码的点击（文本拖选除外），并提供该行位置和点击事件。
`on_line_hover` 报告指针所在的行，指针离开时报告 `None`。

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

## 事件

订阅状态实体以提供审阅操作：

| 事件 | 时机 |
| --- | --- |
| `SelectionStarted(range)` | 按下行号开始选择 |
| `SelectionChanged(range)` | 用户操作改变了选中的行，或以 `None` 清空选区 |
| `SelectionEnded(range)` | 一次选择操作结束：松开指针、一次键盘扩展或全选 |
| `FileCollapsed(path)` / `FileExpanded(path)` | 用户在文件头切换收起状态 |
| `ConflictResolved(path, ix)` | 用户解决或撤销某个冲突 |

`set_selected_lines`、`set_file_collapsed`、`resolve_conflict` 等程序修改不发出事件。
`DiffEvent` 是 non-exhaustive 的，匹配时需要通配分支。

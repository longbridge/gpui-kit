---
title: Editor
description: 支持语法高亮、行号、折叠和文本装饰的源代码编辑器。
---

# Editor

`Editor` 用于编辑源代码。单行输入请使用 [Input](./input.md)，普通多行文本请使用 [Textarea](./textarea.md)。

## 导入

```rust
use gpui_kit::component::input::{Editor, EditorState, TabSize};
```

## 语言编辑规则

`LanguageConfig` 描述语言规则；`.auto_close(bool)` 和 `.smart_indent(bool)` 是独立的编辑器选项。
切换语言或替换规则不会重置这两个选项。自动补全、跳过结束符和成对 Backspace 使用
`auto_closing_pairs`；Enter 使用 `brackets` 和 `indentation_rules`，因此关闭自动补全后，
仍可在已有括号内换行并缩进。

```rust
use gpui_kit::component::input::{
    AutoClosingPair, BracketPair, language_config::LanguageConfig, SyntaxContext, set_language_config,
};

let rules = LanguageConfig::default()
    .brackets([BracketPair::new("{", "}"), BracketPair::new("(", ")")])
    .auto_closing_pairs([
        AutoClosingPair::new("{", "}")
            .not_in([SyntaxContext::String, SyntaxContext::Comment]),
        AutoClosingPair::new("(", ")")
            .not_in([SyntaxContext::String, SyntaxContext::Comment]),
    ]);

set_language_config("rust", rules, cx);

let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .auto_close(true)
        .smart_indent(true)
});
```

`set_language_config` 替换当前应用中指定语言的配置，已有编辑器在下一次编辑时立即使用它，
即使配置修改和编辑发生在同一个事件处理函数内。语言别名共享配置，例如 `python`、`py`、
`pyi`，不受对应 grammar feature 是否启用影响。自定义配置在 Component 初始化后仍然保留。
精确注册的自定义 grammar 名称优先于内置别名，并保留原始大小写。
未知语言使用 `LanguageConfig::default()`。

Component 安装 `LanguageProvider`，统一提供语言名称、默认规则及每个编辑器的语法提供者。
首次编辑和切换语言后的语法选择都不依赖 render。直接使用 Base 时，可通过
`set_language_provider` 安装自己的语言服务；普通 Component 使用者只需调用
`set_language_config`。高亮的 grammar 资源可使用 `highlighter::GrammarConfig`，
原有 `highlighter::LanguageConfig` 名称保持兼容。

配对使用字符串，支持多字符定界符。`auto_closing_pairs = None` 表示使用 `brackets`；
`Some(vec![])` 表示禁用全部自动配对，其 builder 设置的是 `Some`。
`auto_close_before` 指定允许自动补全的后方字符；空白和文档末尾始终允许。
`not_in` 依赖语法上下文提供者；没有提供者时 Base 按 `Code` 处理。
启用对应 Tree-sitter grammar 后，Component 会安装语法上下文提供者。

`IndentationRules::new(increase, decrease)` 接受两个已编译的 `regex::Regex`。
Enter 时分别匹配光标前、后的文本；未配置增加缩进模式时，使用结构括号判断。
这些规则不会重新格式化已有行或粘贴内容。Python 的默认规则额外识别末尾冒号，
未知语言仅使用结构括号。

这是 Monaco 风格语言配置的已支持子集，不直接加载 Monaco JSON 或 `.scm`。
包围选区、自定义 `onEnterRules` 留待后续实现。


## 基础用法

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("rust")
        .line_number(true)
        .folding(true)
        .tab_size(TabSize {
            tab_size: 4,
            hard_tabs: false,
        })
        .default_value("fn main() {\n    println!(\"Hello\");\n}")
});

Editor::new(&editor).h(px(320.))
```

使用 `.language()` 指定语法高亮语言。应用需要启用对应的 Cargo feature，例如 `tree-sitter-rust` 或 `tree-sitter-markdown`；也可以使用 `tree-sitter-languages` 包含全部内置语法。

## 编辑器选项

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("json")
        .line_number(true)
        .folding(true)
        .show_whitespaces(true)
        .default_value(source)
});
```

滚动条默认位于右侧和底部。使用 `scrollbar_placement` 可调整其位置；放在左侧的纵向滚动条位于编辑器左边缘，覆盖在行号之上；若 gutter 位于右侧，文本会与滚动条保持间距。与覆盖最后一行的底部滚动条一样，顶部滚动条覆盖在第一行之上，不为自身预留空间。运行时可用 `set_scrollbar_placement` 替换位置。

在右侧 gutter、左侧滚动条的镜像窗格中，横向轨道会避开 gutter，纵向轨道仍位于窗格外侧边缘。

```rust
EditorState::new(window, cx).scrollbar_placement(ScrollbarPlacement::BottomLeft)
```

行号和折叠图标所在的 gutter 默认位于左侧。在并排 diff 中，两侧的 gutter 可以朝向中间，使对应行的行号隔着分隔线相邻。左侧窗格用 `gutter_side` 将 gutter 放到右侧（镜像排列，折叠图标紧邻文本），并把滚动条放到外侧边缘：

```rust
EditorState::new(window, cx)
    .scrollbar_placement(ScrollbarPlacement::BottomLeft)
    .gutter_side(Side::Right)
```

无论 gutter 位于哪一侧，折叠图标默认紧邻文本，向外依次是行号和 gutter 标记。使用 `gutter_order` 可按从文本向外的顺序列出各列，未列出的列依次排在其后。在 diff 中，这样可以把变更标记放在文本与行号之间：

```rust
EditorState::new(window, cx).gutter_order([GutterColumn::FoldIcons, GutterColumn::Markers])
```

Editor Diff 示例展示了这两种布局，两个窗格同步滚动。

## 快捷键与矩形列选

以下默认快捷键在编辑器聚焦时生效。macOS 的 Option 对应 Alt 修饰键；Linux 的这些操作不使用 Super/Win。

| 操作 | macOS | Linux | Windows |
| --- | --- | --- | --- |
| 在上方／下方添加光标 | Cmd+Option+↑ / ↓ | Alt+Shift+↑ / ↓ | Ctrl+Alt+↑ / ↓ |
| 逐字符扩展所有选区 | Shift+← / → | Shift+← / → | Shift+← / → |
| 按词扩展所有选区 | Option+Shift+← / → | Ctrl+Shift+← / → | Ctrl+Shift+← / → |
| 鼠标添加光标 | Option+左键点击 | Alt+左键点击 | Alt+左键点击 |
| 矩形列选 | Option+Shift+左键拖动 | Alt+Shift+左键拖动 | Alt+Shift+左键拖动 |
| 只保留活动光标 | Escape | Escape | Escape |

Linux 额外支持与 Ghostty 一致的 Ctrl+Alt+左键拖动列选，以及 Alt+Shift+← / → 按词选择。Windows 额外支持 Alt+Shift+← / → 逐字符选择。三个平台都兼容 Alt/Option+左键拖动列选：单击添加光标，继续拖动则以鼠标按下位置为起点建立新的矩形选区。

在编辑区按住 Alt/Option 时，鼠标指针显示为 `+`。带 Alt 的选择手势优先于 Ctrl/Cmd+点击跳转定义。矩形选区按显示行生成，每行一个选区，短行会截断到已有文本边界。输入和删除同时作用于所有选区。松开鼠标结束拖动，Escape 只保留活动光标（若上下文菜单已打开，则先处理菜单的 Escape）。

使用 ↑ / ↓ 添加光标是累加操作，反向按键不会收缩矩形高度。因此这是多光标编辑与鼠标列选，并非持续的 Vim Visual Block 模式。键盘输入期间光标保持可见，空闲 300ms 后恢复闪烁。

Linux 桌面可能在编辑器收到事件之前拦截快捷键。部分桌面使用 Ctrl+Alt+↑ / ↓ 切换工作区，因此 Linux 默认不绑定这一组合。以上快捷键指键盘重映射后的逻辑修饰键。

## 搜索

编辑器内置搜索面板。编辑器聚焦时按 `Ctrl-F`（Windows/Linux）或 `Cmd-F`（macOS）打开。`Enter` 跳到下一个匹配，`Shift+Enter` 跳到上一个，`Escape` 关闭面板。

```rust
// 以代码方式打开查找面板
editor.update(cx, |state, cx| {
    state.open_search(false, cx);
});

// 关闭它
editor.update(cx, |state, cx| {
    state.close_search(cx);
});
```

`Editor` 默认启用搜索。如需禁用：

```rust
editor.update(cx, |state, cx| {
    state.set_searchable(false, cx);
});
```

只读编辑器仍可搜索——替换界面会自动隐藏。

### 自定义搜索界面

搜索引擎不依赖内置面板，应用可以在编辑器的匹配、高亮、滚动和替换之上绘制自己的搜索栏。
`set_search_query` 开始一次搜索，编辑器会一直高亮匹配项，直到调用 `close_search`。
未启用 `searchable` 的编辑器不会打开内置面板，也不拦截 `Ctrl-F` / `Cmd-F`，快捷键会冒泡到上层视图，
应用可以把它绑定到自己的搜索框。

```rust
let editor = cx.new(|cx| EditorState::new(window, cx).searchable(false));

// 从应用自己的搜索框发起搜索
editor.update(cx, |state, cx| {
    state.set_search_query("needle", true, cx);
});

// 在匹配项之间移动，每次调用都会把匹配项滚动到可见区域
editor.update(cx, |state, cx| {
    state.next_search_match(cx);
    state.previous_search_match(cx);
});

// 读取匹配情况："2/5"
let matcher = &editor.read(cx).search_session().matcher;
let label = matcher.label();
let count = matcher.len();
let current = matcher.current(); // 没有匹配时为 None

// 替换（编辑器可编辑时）
editor.update(cx, |state, cx| {
    state.replace_current_search_match("replacement", window, cx);
    state.replace_all_search_matches("replacement", window, cx);
});

// 结束搜索并清除高亮
editor.update(cx, |state, cx| {
    state.close_search(cx);
});
```

在拥有搜索框的视图上接管快捷键：

```rust
use gpui_kit::component::input::Search;

div()
    .on_action(cx.listener(|this: &mut Self, _: &Search, window, cx| {
        this.search.update(cx, |search, cx| search.focus(window, cx));
    }))
    .child(Editor::new(&this.editor))
```

## 文本装饰

```rust
let decorations = editor.update(cx, |state, cx| {
    state.create_decorations_collection(initial_decorations, cx)
});
```

保留返回的 `TextDecorationCollection`，以便更新或清空该调用方的文本样式。
范围会跟随文本编辑；丢弃句柄不会移除装饰。

### 几何范围装饰

使用独立的 `RangeDecorationCollection` 绘制连续填充或一个逻辑像素宽的边框。
它们只参与绘制，不预留行内空间、不添加 widget、不拦截鼠标事件，也不改变键盘焦点。

```rust
use gpui_kit::component::input::{RangeDecoration, RangeDecorationStyle};

let review_ranges = editor.update(cx, |state, cx| {
    state.create_range_decorations_collection(
        vec![
            RangeDecoration::new(0..8).with_style(RangeDecorationStyle::Fill),
            RangeDecoration::new(12..24), // 默认为 Frame。
        ],
        cx,
    )
});

review_ranges.set(vec![RangeDecoration::new(4..16)], cx);
review_ranges.append(vec![RangeDecoration::new(20..28)], cx);
let tracked_ranges = review_ranges.get_ranges(cx);
review_ranges.clear(cx);   // 清空，但保留集合以便复用。
review_ranges.dispose(cx); // 释放集合，使该句柄及其克隆全部失效。
```

每个集合只管理自己的条目，不同扩展不会互相覆盖。丢弃句柄后，集合仍保存在编辑器中；
调用 `dispose` 才会永久释放它。对已释放集合或已销毁编辑器的调用不会产生效果。
单个装饰不需要提供 ID。

范围使用左闭右开的 UTF-8 **字节偏移量**，不是字符下标或行号。起止偏移量向外裁剪到有效
字符边界；空范围、反向范围和完全超出文档的范围会被丢弃。文本装饰和几何装饰共享跟踪规则：

- 在两端插入文字不会扩展范围，在内部插入则会扩展。
- 替换文字时，重叠锚点裁剪到替换区域；删除整个范围会移除该装饰。
- 撤销、重做、`set_value`、`replace_all`（包括格式化）都应用相同的编辑变换。
  装饰**不是**撤销历史中的快照：撤销删除不会恢复已移除的装饰，撤销替换也不会恢复原先的
  内部锚点。如果业务需要这种语义，请依据自己的语义数据重新设置集合。
- 折叠只改变显示投影，不修改存储的范围。完全隐藏的范围不绘制；可见部分按视口裁剪，
  并跟随软换行后的实际字形位置。

填充绘制在边框下方，两者均位于选区和字形下方。同一种样式内，后创建的集合和后加入的
条目覆盖先前的条目。不调用 `with_color` 时，边框使用编辑器前景色，填充使用其 12% 透明度
版本，因此默认颜色会跟随主题变化；显式颜色由应用自行维护。

可见范围查询使用区间索引并跳过折叠的缓冲区范围，不会逐帧扫描全部装饰。设置或追加条目
会重建对应集合的索引；编辑文本时线性更新受影响的集合，不重新排序。
Editor 展示页的 **Decorations** 标签演示了两种集合的组合使用。

### 行装饰

`LineDecorationCollection` 以整行为单位绘制：横跨该行的背景色带、行号槽中的标记，或两者兼有。
条目由 `LineDecorationProvider` 提供：编辑器每绘制一帧，都会以当前可见的缓冲区行范围询问它。

```rust
use std::{ops::Range, rc::Rc};

use gpui_kit::App;
use gpui_kit::component::{
    ActiveTheme as _,
    input::{GutterMarker, LineDecoration, LineDecorationProvider},
};

/// 从零开始的缓冲区行，已排序。
struct AddedRows(Vec<usize>);

impl LineDecorationProvider for AddedRows {
    fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration> {
        let start = self.0.partition_point(|&row| row < rows.start);
        let end = self.0.partition_point(|&row| row < rows.end);
        self.0[start..end]
            .iter()
            .map(|&row| {
                LineDecoration::new(row)
                    .with_background(cx.theme().success.opacity(0.16))
                    .with_marker(GutterMarker::DiffAdded)
            })
            .collect()
    }
}

let added = editor.update(cx, |state, cx| {
    state.create_line_decorations_collection(Rc::new(AddedRows(vec![2, 3, 7])), cx)
});

added.set_provider(Rc::new(AddedRows(vec![4])), cx); // 下一帧改为询问它。
added.clear(cx);   // 停止绘制，但保留集合以便复用。
added.dispose(cx); // 释放集合，使该句柄及其克隆全部失效。
```

由于每一帧都会重新询问 provider，行由 provider 自己维护：编辑器不会随编辑移动这些行，
替换全文时也不会清空它们，并且会忽略询问范围之外的装饰。请按行为数据建立索引，让回答保持轻量；
在被询问时从 `cx` 读取颜色，使其跟随主题变化。与其他装饰集合一样，各集合相互独立；
后创建的集合绘制在先创建的集合之上。

背景横跨整行（包括行号槽），覆盖该行软换行后的全部显示行，位于当前行高亮、缩进参考线、
选区和文字下方。标记绘制在独立槽位中，只在显示行号时出现。默认顺序下，该槽位位于行号与行号槽外缘之间；
使用 `gutter_order` 可将其移到更靠近文本的位置，与 IntelliJ IDEA 放置标记的方式一致。只要有集合设置了
provider，行号槽就会保留该槽位，标记行滚入或滚出视图时宽度不变。
样式化编辑器的标记尺寸为实际字号的 90%，间距为字号的 30%。图标、预留槽位和垂直居中使用同一尺寸，
因此标记既跟随界面缩放，也跟随编辑器单独设置的 `.text_size(...)` 变化。`DiffAdded`、`DiffRemoved`、
`DiffChanged`、`Conflict`、`Bookmark` 和 `Breakpoint` 使用主题的 success、danger、warning 和
info 颜色；`GutterMarker::Custom { icon, color }` 按原样绘制给定的图标资源路径。两者都不影响
命中测试或焦点，也不提供鼠标或键盘操作。

`EditorState::row_bounds(row)` 返回某一行在窗口坐标中实际绘制的区域：多行内联补全下方的行会随
文字一起下移。该行尚未布局时返回 `None`。
Editor 展示页的 **Decorations** 标签用 provider 标记了最后三行。

`EditorState::hit_test(position)` 按最近一次绘制，返回窗口坐标中的某个点落在什么上：文本（附带在该处
单击时光标所在的偏移量），或行号栏（附带所在的 `GutterColumn`，落在边距中时为 `None`）。两种情况都会
给出该点旁边的缓冲区行，以及该点是否位于这一行的间隔行上。在行号栏中，第一行之上和最后一行之下的行为
`None`。`EditorState::gutter_bounds()` 返回行号栏实际绘制的区域。首次绘制之前两者都返回 `None`；
它们都不会改变编辑器对单击的处理。

```rust
use gpui_kit::component::input::{GutterColumn, InputHitArea};

if let Some(hit) = editor.read(cx).hit_test(position) {
    match hit.area() {
        InputHitArea::Gutter { column: Some(GutterColumn::LineNumbers), .. } => {
            // 行号：`hit.row()` 是缓冲区行。
        }
        InputHitArea::Text { offset, .. } => { /* 文本，位于 `offset` */ }
        _ => {}
    }
}
```

`EditorState::display_map()` 返回 `&DisplayMap`，供应用只读查询编辑器拥有的文本映射。
`DisplayMap`、`BufferPoint` 和 `DisplayPoint` 均可从 `gpui_kit::base::input` 或
`gpui_kit::component::input` 导入。编辑器状态负责更新映射；应用可以用它实现自定义缩略图、
同步视图，以及缓冲区坐标与显示坐标之间的转换。

```rust
use gpui_kit::base::input::BufferPoint;

let state = editor.read(cx);
let map = state.display_map();
let count = map.display_row_count();
let rows = map.buffer_line_to_display_row_range(0); // 折叠隐藏的行返回 None
let display = map.buffer_pos_to_display_pos(BufferPoint::new(0, 0));
let buffer = map.display_pos_to_buffer_pos(display);
let version = map.version();
```

映射覆盖整个已准备的文本，包括视口外的行。软换行使用最近准备的布局参数；文本尚未准备时，
映射没有显示行。编辑、重新换行、折叠或填充行变化会改变 `map.version()`。比较同一映射的版本可以判断
何时需要重新查询；滚动不会改变版本。

映射不包含内联补全的额外行。显示行序号乘以行高表示文本映射位置，不一定是实际绘制位置：
多行内联补全会把后续文本向下推，而映射和版本不变。可见行的实际几何信息应通过 `row_bounds` 获取。

缓冲区和显示坐标的序号均从零开始。被折叠隐藏的缓冲区行没有显示行范围（返回 `None`）；
缓冲区位置与显示位置之间的转换使用映射的坐标转换方法。

### 填充行

`Spacer` 是位于某个缓冲区行上方或下方的一组空白行。它不包含文本：没有行号，光标会跳过它，
点击它会落到旁边的文本行上，行装饰、选区、搜索匹配和内联颜色都不会覆盖它。它的行与其他显示行一样，
因此滚动高度、滚动到某一行以及 `row_bounds` 都会把它计算在内。并排显示的两个编辑器可以用填充行补齐
较矮的行，使两侧对应的行无论各自如何换行都从同一高度开始，就像并排 diff 那样。

```rust
use std::{ops::Range, rc::Rc};

use gpui_kit::{App, WeakEntity};
use gpui_kit::component::input::{EditorState, Spacer, SpacerProvider};

/// 为每一行补上 `other` 中同一行因换行而多出的行数。
struct Level {
    this: WeakEntity<EditorState>,
    other: WeakEntity<EditorState>,
}

impl SpacerProvider for Level {
    fn spacers(&self, lines: Range<usize>, cx: &App) -> Vec<Spacer> {
        let (Some(this), Some(other)) = (self.this.upgrade(), self.other.upgrade()) else {
            return Vec::new();
        };
        let rows = |editor: &EditorState, line| {
            editor
                .display_map()
                .buffer_line_to_display_row_range(line)
                .map_or(0, |rows| rows.len())
        };
        let (this, other) = (this.read(cx), other.read(cx));
        lines
            .filter_map(|line| {
                let (own, theirs) = (rows(this, line), rows(other, line));
                (theirs > own).then(|| Spacer::below(line, theirs - own))
            })
            .collect()
    }
}

let level = Rc::new(Level {
    this: left.downgrade(),
    other: right.downgrade(),
});
left.update(cx, |state, cx| state.set_spacer_provider(Some(level), cx));
```

与行装饰不同，填充行会推动其下方的所有行，因此编辑器一次性询问所有行的填充行并保留结果：设置 provider 时、
编辑之后、文本重新换行或折叠变化之后，以及调用 `refresh_spacers` 时。询问发生在布局阶段，此时文本已按编辑器的
宽度完成换行，并且不在编辑器的任何更新之内，因此 provider 可以读取本编辑器和其他编辑器。若结果依赖另一个
编辑器的行，请在该编辑器的 `display_map().version()` 变化时调用 `refresh_spacers`。`Spacer::above(0, rows)`
在文本顶部补行，`with_background` 为这些行（包括行号槽）填充颜色。位于折叠隐藏的行或最后一行之后的填充行不会显示。
**Editor Diff** 展示页对两侧都启用换行，并用填充行使它们对齐。

在显示映射中，填充行也是显示行：`display_row_count()` 会计入它们，`buffer_line_to_display_row_range(line)`
只包含该行文本所占的行，`display_row_to_buffer_line(row)` 把填充行视为它旁边的那一行，`is_spacer_row(row)`
用于区分填充行和文本行。填充行变化时 `version()` 也会变化，因此按版本缓存的行映射不会过期；provider 返回与当前相同的填充行时版本不变，
所以在版本变化时调用 `refresh_spacers` 最终会稳定下来。

## 行号栏右键菜单

`Editor::context_menu_with` 在构建右键菜单时知道它是在哪里打开的：
除了 capabilities，它还会收到这次单击的 `hit_test` 结果，因此菜单可以只针对行号栏
中的某一行，而文本上仍保留通常的菜单（由 `default_context_menu` 构建）。在行号栏上右键单击会像在文本
上一样把光标移到该行；`gutter_context_menu_moves_cursor(false)` 则在行号栏菜单打开期间保留当前选区。

```rust
use gpui_kit::component::input::{Editor, EditorState, default_context_menu};

let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .line_number(true)
        .gutter_context_menu_moves_cursor(false)
});

Editor::new(&editor).context_menu_with(|menu, capabilities, hit, _, _| {
    match hit {
        Some(hit) if hit.is_gutter() => match hit.row() {
            Some(row) => menu
                .menu(format!("Bookmark Line {}", row + 1), Box::new(ToggleBookmark))
                .menu(format!("Copy Line {}", row + 1), Box::new(CopyLine)),
            None => menu, // 最后一行之下：不显示菜单
        },
        _ => default_context_menu(menu, capabilities),
    }
})
```

构建函数在菜单打开时运行；可以在这里记下行号，供菜单操作读取。
`examples/editor_gutter_menu` 演示了为单击的行添加书签、选中和复制。

## 值与事件

```rust
let source = editor.read(cx).value();

editor.update(cx, |state, cx| {
    state.set_value(new_source, window, cx);
});
```

`EditorState` 会发出 `InputEvent::Change`、`Focus` 和 `Blur` 等事件。

## 字体

Editor 默认使用主题中的等宽字体 —— `mono_font_family` 和 `mono_font_size`，行高为字号的
1.5 倍。这只是默认值：在 Editor 上设置的文本样式会覆盖它，gutter 和行高都跟随字号变化。
主题加载时会核对平台默认等宽字体（`Menlo`、`Consolas`、`DejaVu Sans Mono`）是否已安装，
缺失时换成已安装的等宽字体，再不行退到 `.SystemUIFont`；你自己指定的字体族则原样使用。

```rust
Editor::new(&editor).text_sm()

Editor::new(&editor)
    .font_family("JetBrains Mono")
    .text_size(px(15.))
```

这些就是所有元素都有的 [`Styled`](https://docs.rs/gpui/latest/gpui/trait.Styled.html)
方法，`font_weight`、`line_height` 用法相同。

## 外观

```rust
Editor::new(&editor)
    .h(px(480.))
    .bordered(true)
    .disabled(false)
    .readonly(false)
    .aria_label("Rust 源代码")
```

预览文件但不允许修改时使用 `readonly`。与 `disabled` 不同，只读编辑器保持正常外观，仍然可以聚焦、选中、复制和搜索，只是拒绝用户对内容的修改。`set_value` 等程序调用不受影响。

```rust
Editor::new(&editor).readonly(true)
```

Editor 聚焦时不会应用单行 Input 的焦点边框效果。gutter、当前行背景和滚动条会作为同一个编辑器表面对齐绘制。

前后缀、密码显示切换和清除按钮只属于单行 Input。Editor 的工具栏和操作按钮应组合在组件外部。

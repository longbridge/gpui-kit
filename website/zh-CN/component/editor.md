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

高亮引擎有两种，通过 `gpui-kit` 的 Cargo feature 选择，默认都不启用：

- `tree-sitter`：解析出语法树，折叠和需要语法信息的编辑行为也依赖它。每种语言对应上面提到的一个 grammar feature，全部启用会让应用增加几十 MB。
- `syntect`：用正则表达式为约 200 种语言提供高亮，只增加几 MB。它不生成语法树，所以折叠按花括号匹配，编辑时也没有语法上下文。适合只需要显示高亮代码、不需要结构化编辑器的应用：

```toml
gpui-kit = { version = "{{gpui_kit_version}}", features = ["syntect"] }
```

两者同时启用时使用 `tree-sitter`。

## 编辑器选项

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language("json")
        .line_number(true)
        .folding(true)
        .show_whitespaces(true)
        .scroll_bounce(true)
        .default_value(source)
});
```

`scroll_bounce(true)` 让超出编辑器高度的代码在触控板或 Magic Mouse 的精细像素滚动到达顶部或底部时越过边缘并弹回，与 [Textarea](./textarea.md#滚动回弹) 相同。行号栏随代码移动，外框和滚动条保持不动；普通鼠标滚轮的按行滚动不触发回弹。

## 主动补全

`ShowCompletions` 可以在不修改文本的情况下请求补全，**没有默认快捷键**。应用应结合平台习惯选择绑定，避开 Ctrl+Space 等输入法快捷键。Editor 的绑定可以使用 `Input && mode == editor` 条件；现有 `Input` 绑定仍然有效。

```rust
use gpui_kit::component::input::ShowCompletions;

// 从应用命令分发给当前聚焦的 Editor。
window.dispatch_action(Box::new(ShowCompletions), cx);

// 也可以指定 Editor；此方法同时将焦点交给它。
editor.update(cx, |state, cx| state.show_completions(window, cx));
```

菜单使用光标前的标识符前缀，沿用编辑器按字母、数字和下划线识别单词的规则；前缀为空时仍会请求建议。手动请求使用 LSP 的 `INVOKED` 类型，不传触发字符。禁用、只读、没有 provider 或处于 IME 预编辑状态的 Editor 不会发出请求。IME 文本提交后，可以再次调用命令。

补全菜单已打开时，命令保留当前选择。否则，它会取消上一轮浮层请求和 inline suggestion，并关闭 code action 菜单，再请求补全。编辑、移动光标、关闭菜单或失去焦点后，迟到的结果不会重新打开补全。

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

### 行号槽与概览标尺装饰

需要在文本之外标记整行（例如版本控制 diff 中新增、修改或删除的行）时，使用这两个集合。
`GutterDecorationCollection` 在行号之前的窄带中绘制标记。
`OverviewRulerDecorationCollection` 在垂直滚动条轨道上绘制标记，Monaco 将这条轨道称为概览标尺。
要同时在两处显示标记，请分别创建两个集合。

```rust
use gpui_kit::component::input::{
    GutterDecoration, OverviewRulerDecoration, OverviewRulerLane,
};

let style = &cx.theme().highlight_theme.style;
let (added, deleted) = (style.status.created(cx), style.status.deleted(cx));

let gutter = editor.update(cx, |state, cx| {
    state.create_gutter_decorations_collection(
        vec![
            GutterDecoration::new(10..42).with_color(added), // 标记范围触及的每一行。
            GutterDecoration::new(60..60).with_color(deleted), // 标记从偏移 60 开始的那一行上方的边界。
        ],
        cx,
    )
});
let ruler = editor.update(cx, |state, cx| {
    state.create_overview_ruler_decorations_collection(
        vec![
            OverviewRulerDecoration::new(10..42)
                .with_lane(OverviewRulerLane::Left)
                .with_color(added.opacity(0.6)),
        ],
        cx,
    )
});
```

- 使用非空范围可以标记它触及的每一行，相当于 Monaco 的 `isWholeLine`。结束在行首的范围不标记该行。
  使用**空**范围可以标记行边界，例如被删除的行原来所在的位置。范围位于行首时，标记显示在该行上方；
  位于其他位置时，标记显示在该行下方。没有末尾换行的文档中，`text.len()` 会标记最后一行下方。
  行号槽用竖条表示非空范围，用胶囊表示边界。
- 这两种集合会保留空范围，文本装饰和几何装饰会丢弃空范围。在空范围处插入文字时，文字会插入到范围之后。
  编辑使一个范围变空时，集合会保留它并把它作为边界。它们使用与其他装饰集合相同的句柄和
  `set`/`append`/`clear`/`dispose` 方法。重新计算装饰后，请对整个集合调用 `set`；两次更新之间，
  编辑跟踪会让标记留在原来的行上。
- 只要存在行号槽集合（包括空集合），行号槽就会预留窄带，标记变化时文本不会移动。
  默认宽度为 3 px，可用 `gutter_decorations_width` 或 `set_gutter_decorations_width` 修改。
  如果范围起点位于折叠内部，行号槽会在折叠所在行绘制一条竖条，折叠之后的剩余部分照常标记；
  `with_folded_color` 可以设置折叠所在行那条竖条的颜色。
- 概览标尺有四个分区（lane）。`Left`、`Center`（默认）和 `Right` 各占轨道的三分之一；`Full` 占用整个轨道。
  按惯例，版本控制使用 `Left`，搜索结果使用 `Center`，诊断使用 `Right`。每个标记按所在行占内容高度的比例定位，
  高度至少为 6 px。同一分区中颜色相同且相互接触的标记会合并。
- 标记绘制在滚动条下方，无论滚动条是 Base 提供的还是由 `set_scrollbar_renderer` 提供的。
  滑块淡出时，标记仍然可见。
- 不调用 `with_color` 时，两个集合都使用编辑器前景色。版本控制的颜色可以读取高亮主题中
  Zed 的状态键 `created`、`modified` 和 `deleted`，对应方法为 `status.created(cx)`、
  `status.modified(cx)` 和 `status.deleted(cx)`。缺少对应键时，这些方法分别使用 `success`、
  `warning` 和 `error`。

## 值与事件

```rust
let source = editor.read(cx).value();

editor.update(cx, |state, cx| {
    state.set_value(new_source, window, cx);
});
```

`EditorState` 会发出 `InputEvent::Change`、`Focus` 和 `Blur` 等事件。

需要增量同步时，请订阅 `InputEditsEvent`。每个事件都包含自己的有序编辑记录和每次编辑后的
文档，即使多次变更一起排队也能正确对应。`set_value` 是静默操作，不会发出这两种事件。

需要在编辑器视图之间保留选区方向时，请一起保存 `selected_range()` 和
`is_selection_reversed()`，再通过 `set_selected_range_with_direction(range, reversed, cx)`
恢复。范围使用 UTF-8 字节偏移；`set_selected_range` 仍将空范围视为光标位置。

## 按内容定高

`display_map().wrap_row_count()` 返回文本软换行后占用的行数，只读编辑器可以据此按内容设定高度。布局会在第一帧计算它；如果需要在此之前得到行数，请在设置值之后，用字体、字号和文本区宽度调用 `prewrap`：

```rust
use gpui_kit::component::ActiveTheme as _;

let editor = cx.new(|cx| {
    let mut state = EditorState::new(window, cx)
        .line_number(false)
        .default_value(snippet);
    let mut font = window.text_style().font();
    font.family = cx.theme().mono_font_family.clone();
    state.prewrap(font, cx.theme().mono_font_size, width, cx);
    state
});
let rows = editor.read(cx).display_map().wrap_row_count();
```

`width` 是编辑器宽度减去内边距；显示行号时还要减去行号栏宽度。折叠的行仍计入行数。
字体和字号必须与编辑器实际使用的文本样式一致。此示例使用主题默认值；如果自定义了编辑器的字体或字号，请将相同的值传给 `prewrap`。

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

## 尺寸

Editor 与 Input、Textarea 放在同一个表单中时，可以通过 `Sizable` 设置相同的尺寸档位，
协调它们的内边距和密度，使样式表现保持一致。Editor 的代码字体仍由自身的字体样式独立配置。

```rust
Editor::new(&editor).large()
Editor::new(&editor) // medium（默认）
Editor::new(&editor).xsmall()
```

与 `Textarea` 不同，尺寸只改变代码四周的内边距，不改变字号（见[字体](#字体)）。`xsmall` 去掉代码上下的内边距。

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

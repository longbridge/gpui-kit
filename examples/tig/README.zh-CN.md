# Git history

一个受 tig 启发的桌面 Git 浏览器。浏览 HEAD 可达的最近 200 条提交，在 `Diff` 中查看
每个文件的变更，并在当前提交修改的文件之间跳转。提交和文件导航共用可调整宽度的侧栏，
Diff 占据主要工作区。提交正文默认收起，可通过标题后的展开按钮查看。默认使用 Dark 主题和 Split 布局。

```sh
cargo run -p example-tig -- .
cargo run -p example-tig -- /path/to/repository
cargo run -p example-tig --features gpui-fast -- .
```

Git 必须在 PATH 中。路径参数默认为当前目录，也可以指向工作区内的目录或裸仓库。
示例不写入仓库文件。它浏览已提交的历史，不包含暂存、切换分支、编辑源文件或显示未提交改动。

## 使用方式

先在 **Commits** 中选择提交，再在 **Changed files** 中选择文件。视口只显示选中的文件，
确保文件头与源文本始终属于同一个对象。再次选择当前文件会滚动到文件头，若文件已收起则先展开。Diff 上方显示当前提交的标题、作者、日期和短哈希。
哈希旁的复制按钮复制完整的对象 ID；标题后的展开按钮显示提交正文；正文有独立的滚动区域，避免长正文默认挤占代码空间。

在 **Diff display options → Unified / Split** 中切换布局并保留源文本选区。使用箭头按钮或 n / Shift+N 在当前文件的变更组之间跳转。
设置图标打开 **Diff display options**，集中提供换行、行号、单词或字素强调，以及上下文展开选项。
Appearance 子菜单可切换 Light 和 Dark 主题。
这些选项更新已有状态，不重新创建状态实体。长行默认换行；保留 Diff 的主题化增删行背景，
配合 gutter 中的符号和行内强调区分变更。

| 范围 | 按键 | 结果 |
| --- | --- | --- |
| Commits | ↑ / ↓ 或 k / j | 选择更新／更早的提交 |
| Commits | Enter | 聚焦 Diff |
| Changed files | ↑ / ↓ 或 k / j | 显示上一个／下一个文件 |
| Changed files | Enter | 聚焦 Diff |
| 任意区域 | [ / ] | 显示上一个／下一个文件 |
| 任意区域 | Escape | 展开侧栏并返回提交历史 |
| 任意区域 | Cmd+B / Ctrl+B | 显示／隐藏侧栏 |
| 任意区域 | n / Shift+N | 显示下一处／上一处变更 |
| 任意区域 | Cmd+R / Ctrl+R | 刷新历史，当前提交仍可用时保留选择 |
| 任意区域 | Cmd+Shift+C / Ctrl+Shift+C | 复制提交的完整哈希 |
| Diff | Cmd+C / Ctrl+C | 复制选中的源文本 |
| 任意区域 | Cmd+Q / Ctrl+Q | 退出 |

通过 Tab 在按钮、历史、文件和 Diff 之间移动。垂直滚轮滚动源文本行，水平手势滚动长行。
侧栏的两个分区和整个侧栏的宽度都可以独立调整，状态栏左侧提供侧栏折叠按钮。
点击文件后焦点保留在文件列表，Enter 才进入源文本。键盘焦点显示在当前行，
不再框住整个导航面板。快捷键 tooltip 和状态提示通过 `Kbd` 从实际 Action 绑定中读取。
状态栏显示当前提交、文件的位置及源文本选区。

## 输入与状态

`repository.rs` 在后台执行器中运行 `git log` 和 `git show`。提交记录用 NUL 分隔，
保留制表符、Unicode 和空提交标题。patch 也在 UI 线程之外解析为 `DiffFile`。
merge commit 与第一父提交比较，生成普通的两侧 patch。根提交、重命名、删除、二进制摘要
和仅元信息的变更使用同一个 Diff parser。

`Tig` 持有提交列表、选择、焦点句柄、两个虚拟化导航列表和一个 `Entity<DiffState>`。
切换提交立即清空旧源文本。每次请求携带一个版本号：早先选择的结果不会覆盖当前提交。
刷新后，若原提交的哈希仍在历史窗口内，就保留原选择。

只有 Git 提供的上下文可用。展开折叠不能恢复未提供的行。`Diff` 本身不比较完整源文件，
也不访问仓库。非 UTF-8 的 patch 输出会显示错误，不会静默修改源文本。
空仓库和没有文件变更的提交分别显示对应空状态，读取失败时提供 **Retry**。

不打开窗口检查仓库读取：

```sh
cargo run -p example-tig -- --check /path/to/repository
```

示例包含 parser 回归测试，以及使用实际视图的 UI integration tests，覆盖提交和文件导航、
启动文件定位、过期结果拒绝、焦点切换，以及提交正文的独立滚动。

## 演示

定位到一个真实的 Rust 变更，复现截图中的位置：

```sh
cargo run -p example-tig -- --commit 4890b1c2 --file crates/component/src/speech/waveform.rs --line 70 .
```

`--commit` 接受 7–64 位十六进制对象 ID，从该提交开始读取历史。`--file` 选中一个变更文件；
`--line` 定位到修改侧从 1 开始的源文本行号，必须与 `--file` 一起使用。
这些启动参数不会限制后续导航。

![Dark 主题的 Split 视图：语法高亮、换行和行内变更](screenshots/split-dark.png)

早期截图也已保留：[最初预览](screenshots/before.png) 和 [中间版本](screenshots/unified.png)。

[English](README.md)

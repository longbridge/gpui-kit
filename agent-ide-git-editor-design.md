# Agent IDE 技术方案：Git 管理 · 文件编辑 · 样式美化

> 配套 [agent-ide-plan.md](./agent-ide-plan.md)，覆盖其 P1（读懂代码）与 P2（git 管理与打磨）。
> 一切方案都基于本仓库 gpui-kit 框架的现有能力展开；外部参考为 Zed / lazygit / GitButler / Lapce / gitui 的公开实现。
> 调研日期：2026-09-07（源码核对自各仓库 main 分支）。

---

## 1. 结论速览（TL;DR）

| 问题 | 结论 |
|------|------|
| Git 后端用什么 | **维持 git2**（锁定决策 #9 结论不变），但必须藏在自己定义的 `GitRepository` trait 后面。行级 diff 用 **imara-diff 进程内直算**，不走 git2 的 patch 输出 |
| 编辑器怎么实现 | **复用 `EditorState`**（Rope + tree-sitter + 行号/折叠/搜索替换），不自研。Zed 式 Buffer/MultiBuffer 栈对本应用是 YAGNI |
| Git UI 抄谁 | 布局与信息架构抄 **Zed GitPanel**；行级交互语义抄 **lazygit**；staged/unstaged 双 diff 模型抄 **Zed "diff-of-diffs"** |
| 样式怎么做 | 主题即 JSON（Zed 兼容 schema，`themes/` 已有 21 套），新增 `version_control_*` 语义色 token；全程遵守 `skills/gpui-kit-design-guides`（token 优先、桌面优先、交互状态可见） |

> ⚠️ **对计划的一处论据修正**：锁定决策 #9 引用的「git2、Zed 先例」**已经过时**。Zed 于 2026-06-02 彻底移除 git2/libgit2（PR #53453），当前 `crates/git` 纯 spawn git CLI。git2 的真实先例是 **Lapce、gitui、GitButler（部分）**。这不推翻 git2 的选择（理由见 §3.1），但文档论据和风险预案已相应改写。

> ⚠️ **License 红线**：Zed 的 `crates/git`、`crates/text`、`crates/buffer_diff` 是 **GPL-3.0**。可以借鉴架构模式与交互设计，**不可以拷贝代码**。gpui-kit 本身 Apache-2.0，无此问题。

---

## 2. 框架能力盘点：哪些积木能用，哪些是空白

以 `gpui_kit::component::` 导入路径为准（见 `skills/gpui-kit/SKILL.md` 组件目录）：

| IDE 需求 | 框架积木 | 状态 |
|----------|---------|------|
| 多 tab / 左右下 Dock / 持久化 | `DockArea` + `Panel` trait + `DockSkin`（`examples/dock` 是标准模板，含布局 dump/load/防抖保存） | ✅ 现成 |
| 代码编辑器 | `EditorState`（`crates/base/src/input/base/state.rs`）：Rope 缓冲、tree-sitter 高亮（feature 按语言启用，约 30 种）、行号、缩进参考线、折叠、软换行、搜索替换（内置 SearchPanel）、多光标、LSP 挂点 | ✅ 现成，红线 ≤5 万行 |
| 文件树 | `Tree` / `TreeState`（`examples/editor` 有完整文件树 + `.gitignore` Ignorer 实现） | ✅ 现成 |
| 主题 | `ThemeRegistry`（`watch_dir` 热加载）+ Zed 兼容 JSON schema（`themes/` 21 套）+ light/dark + 语法配色（`highlight` 段） | ✅ 现成 |
| 表格/列表/浮层/通知 | `DataTable`、`List`、`Popover`、`PopupMenu`/`ContextMenu`、`Dialog`、`Notification`、`Command` 命令面板、`StatusBar`、`h_resizable` | ✅ 现成 |
| diff 渲染 | 无现成组件。编辑器有 `create_decorations_collection` + `TextDecoration{range, HighlightStyle}`（随编辑跟随），可支撑只读 diff 着色 | ⚠️ 需自建（本文 §3.4 给方案） |
| git 后端 | 仓库内零依赖零代码（全仓 grep 无 git2/gix） | ❌ 空白，自建 |

gpui 生态侧证：`zed-industries/awesome-gpui` 上没有任何项目发布了可复用的 git/diff 组件（GitComet、hunk、JayJay 等都是完整应用）。自建的 GitPanel 有日后 feature-gate 上游的机会，与锁定决策 #3 的思路一致。

---

## 3. Git 管理

### 3.1 后端选型：git2 vs gitoxide(gix) vs shell out

| | git2 (0.21) | gix (0.87) | spawn git CLI |
|---|---|---|---|
| status | ✅ 结构化 `statuses()` | ✅ index↔worktree 含 untracked/rename | ✅ `status --porcelain=v2 -z`，需解析 |
| 行级 diff | ✅ 但 patch 输出解析体验差 | ✅ imara-diff | ✅ `diff --numstat` / patch，需解析 |
| **stage（index 写入）** | ✅ `index.add/remove/write` | ❌ **"add and remove entries" 整条未实现** | ✅ `git add` |
| commit | ✅ | ✅ | ✅ |
| stash / push | ✅ / ✅ | ❌ / ❌ | ✅ / ✅ |
| 依赖 | C 工具链（可 vendored） | 纯 Rust | 要求系统 git（macOS 自带） |
| 与用户真实 git 的一致性 | 会漂移（reftable、SHA-256 支持滞后——Zed 弃用的直接诱因） | 会漂移 | **完全一致** |
| 先例 | Lapce、gitui、GitButler(部分) | GitButler(主引擎)、Cargo | **Zed(2026-06 起)**、lazygit |

**结论：选 git2。** 排除法很干净——gix 目前做不了 stage，直接出局；CLI 方案要自建 porcelain 解析 + 进程管理 + 凭据处理，对 status/stage/commit/diff 这个需求面属于过度工程。本应用范围（计划决策 #8：git diff 驱动 diff 面板 + status/stage/commit，仅 macOS）git2 全覆盖，vendored 构建解决分发。

**必须做的架构保险**：所有 git 调用收口到自定义 trait：

```rust
// crates/agent-ide/src/git/repository.rs
pub trait GitRepository: Send {
    fn open(workdir: PathBuf) -> Result<Self> where Self: Sized;
    fn status(&self) -> Result<RepoStatus>;                       // git2 statuses()
    fn head_blob(&self, path: &RelPath) -> Result<Option<Vec<u8>>>; // HEAD 基准字节
    fn index_blob(&self, path: &RelPath) -> Result<Option<Vec<u8>>>; // index 基准字节
    fn stage(&self, paths: &[RelPath]) -> Result<()>;
    fn unstage(&self, paths: &[RelPath]) -> Result<()>;
    fn stage_all(&self) -> Result<()>;
    fn commit(&self, message: &str) -> Result<Oid>;
    fn head_branch(&self) -> Result<BranchInfo>;
}
```

将来若撞上 reftable/SHA-256 类漂移问题，退路就是 Zed 式 CLI 实现（同一个 trait 换实现即可，UI 层零改动）。blame 若远期要做，直接抄 Zed 的 `git blame --incremental --contents -` 流式方案（stdin 喂编辑器内容，未保存修改也有正确 blame）——那是 CLI 更顺的场景，正好走 trait 换实现。

### 3.2 线程模型与刷新机制

**线程模型**：git2 的 `Repository` 是 `Send` 但 `!Sync`（libgit2 对象不可跨线程共享）。采用 **git actor 线程**（gitui/asyncgit 验证过的最简模式）：一个常驻线程持有 `Repository`，主线程经 smol channel 发命令、回包驱动 UI。agent-ide 已依赖 smol，零新增。

```
主线程 (GPUI)                        git actor 线程
┌──────────────────┐   cmd channel   ┌──────────────────┐
│ GitPanel/编辑器   │ ──────────────→ │ Repository(git2) │
│ 收到 GitEvent    │ ←────────────── │ status/diff/stage │
│ cx.notify()      │   event channel │ + imara-diff 计算  │
└──────────────────┘                 └──────────────────┘
        ↑
        └── notify crate 文件系统 watcher（事件防抖 50ms 后触发 status 刷新）
```

**刷新机制是本应用的核心差异点，必须想清楚**：Zed 里用户主要通过面板做 git 操作，动作驱动刷新即可；而在 agent-ide 里，**git 操作大多发生在终端里（agent 自己跑 `git commit`）**，我们的 GitPanel 本质上是一个「旁观仪表盘 + 便捷操作」。所以：

1. **FS watcher（notify crate）是主刷新通路**，不是可选项——agent 在终端里改了文件、跑了 git 命令，面板必须自己知道。`.git/index` 与工作区目录都在监听范围。
2. 防抖 50ms 合并事件（Zed `git_panel.rs` 的 `UPDATE_DEBOUNCE` 同款数值），防止 agent 批量写文件时刷新风暴。
3. 每次刷新产出 `RepoStatus` 快照 + 变更文件集的 diff，整体替换面板数据，UI 无状态重建。

### 3.3 数据模型与 diff 管线

状态列表（照 Zed `GitStatusEntry` 模型裁剪）：

```rust
pub struct RepoStatus {
    pub branch: BranchInfo,
    pub entries: Vec<GitStatusEntry>,   // 一个文件可同时进 staged 与 unstaged 两个分区
}

pub struct GitStatusEntry {
    pub path: RelPath,
    pub index_status: FileStatus,     // git2 Status 里的 INDEX_NEW/MODIFIED/DELETED/RENAMED
    pub worktree_status: FileStatus,  // WT_* 同上; Untracked/Conflicted 单列
    pub diff_stat: DiffStat,          // +n −m,驱动 diffstat 条渲染
}
```

**diff 数据管线（本方案最值得抄的一块：Zed "diff-of-diffs"）**：

对每个变更文件维护**两份**行级 diff，全部用 imara-diff 在进程内直算（`Algorithm::Histogram` + git 风格 postprocess，与 Zed 同款选择）：

```
diff A：HEAD blob 字节 ←imara-diff→ worktree 文件字节   （git diff）
diff B：HEAD blob 字节 ←imara-diff→ index blob 字节     （git diff --staged）
```

hunk 三态由 A、B 的几何交叠**推导**，不需要任何额外语义接口：

| 条件 | 渲染 | 语义 |
|------|------|------|
| 只在 A 中，不与 B 交叠 | **实心**背景色 | 未暂存 hunk |
| 与 B 交叠 / 只在 B 中 | **空心**（描边） | 已暂存 hunk |
| 部分交叠 | 实心 + 空心并排 | 部分暂存 |

行状态同样由几何推导（Zed `buffer_diff.rs` 规则）：worktree 侧区间为空 = `Deleted`，HEAD 侧为空 = `Added`，否则 = `Modified`。**好处：不解析 patch 文本、双份 diff 共享 HEAD 基准字节、staged 面板与 diff 视图天然同源。**

局限要写明：本方案没有 Zed 的 `Anchor`，编辑器里编辑后 hunk 区间不自动漂移。对策：文件小（红线 5 万行），**每次编辑失效即整文件重算 diff**（imara-diff 毫秒级），用行号区间+版本号失效即可，不需要锚点系统。

### 3.4 Git UI 设计（Dock 面板组合）

整体布局（全部落在现有 Dock 系统内，新增的都是 `Panel` 实现）：

```
┌ TitleBar ────────────────────────────────────────────────────────────┐
│ Sidebar        │  Center: Tab 区域                                    │
│ ├ 项目列表      │  ┌ Terminal ┐ ┌ Editor ┐ ┌ Uncommitted Changes(新) ┐ │
│ └ 文件树(新)    │  │          │ │        │ │                            │ │
│                │  └──────────┘ └────────┘ └────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────┤
│ GitPanel（bottom dock，新，默认收起为状态栏一行）                       │
├──────────────────────────────────────────────────────────────────────┤
│ StatusBar：⎇ 分支名 · ↑↓ ahead/behind · N 个变更 · 点击展开 GitPanel    │
└──────────────────────────────────────────────────────────────────────┘
```

**① GitPanel（bottom dock）**——布局抄 Zed `git_panel.rs`：

```
┌ Git ─────────────────────────────────────────────── ⌇ ─┐
│ ⎇ superpowerplan01 ▾          [全部暂存] [全部取消] [⟳]  │
│ STAGED (2)                                              │
│   ☑ M  crates/agent-ide/Cargo.toml        +12 −3 ▂▄█    │
│   ☑ A  src/git/mod.rs                    +180    ▄█     │
│ CHANGES (3)                                             │
│   ☐ M  src/terminal/mod.rs               +45 −12 ▂▄     │
│   ☐ ?  docs/design-notes.md              +96      ▄     │
│   ☐ D  examples/agent_ide/main.rs          −210         │
│ ┌────────────────────────────────────────────────────┐ │
│ │ Commit message…                              ⌘⏎    │ │
│ └────────────────────────────────────────────────────┘ │
│                                     [ ✓ Commit  ⌘⏎ ]   │
└─────────────────────────────────────────────────────────┘
```

组件映射（全部现成积木）：

| UI 元素 | gpui-kit 组件 |
|---------|--------------|
| 分支按钮 → 分支切换 | `Button` + `Popover`（列表项：分支名 + ahead/behind + 最近提交时间） |
| STAGED / CHANGES 分区列表 | `List`/`ListState`（或行数少直接手排），分区标题为 sticky label |
| 每行：状态字形 + 文件名 + diffstat 条 | `Icon`/`Label` + 自绘 4px 高 diffstat 条（added/deleted 两段，token 色） |
| ☑ 暂存勾选 | `Checkbox`，点击即 `stage/unstage`（三态：勾/取消/只对该文件） |
| Commit message 输入 | `TextareaState` **AutoHeight 风格固定 4–6 行、无行号**（Zed commit 编辑器同款约束），`⌘⏎` 提交 |
| Commit 按钮 | `Button`，staged 为空或 message 为空时 `disabled`（禁用态必须可见——design-guides 规则） |
| 非 git 项目 | 整面板替换为空态说明：「此目录不是 Git 仓库」（宾语+动词文案规范） |

**② Diff 视图（center tab「Uncommitted Changes」）**——一个普通 `Panel`，标题常驻 tab 栏，抄 Zed ProjectDiff 的信息架构但降级实现：

```
┌ Uncommitted Changes ───────────── 4 files · +333 −225 ─────┐
│ ┌ 文件列表 ─────┐ ┌ diff 主区（只读） ──────────────────────┐ │
│ │ ▸ terminal/… M│ │ 842      fn flush(&mut self) {          │ │
│ │ ▸ git/…     A │ │ 843          let f = self.render();     │ │
│ │ ▸ Cargo.toml M│ │ 844 −       self.queue.push(f);         │ │  ← 红色实心底
│ │ ▸ main.rs   D │ │     +       self.pending.push_back(f);  │ │  ← 绿色实心底
│ │               │ │     +       cx.notify();                │ │
│ │               │ │ 845 845  }                              │ │
│ │               │ │ ┄┄ ⋯ hunk 头 @@ −842,7 +842,9 @@ ┄┄     │ │  ← 弱化色小字
│ └───────────────┘ └─────────────────────────────────────────┘ │
│  [暂存 hunk] [取消暂存] [还原文件]      只读 · staged=空心描边   │
└──────────────────────────────────────────────────────────────┘
```

渲染实现**两阶段**：

- **P2 第一版（推荐，改动最小）**：只读 `EditorState`。把选中文件的 diff 合成为纯文本（hunk 头行 + `+`/`−` 行），构建文本的同时记下每个区间的颜色角色，一次性 `state.create_decorations_collection(Vec<TextDecoration>, cx)` 上色（`HighlightStyle.background_color`；已验证 API 存在且随编辑跟随，`editor/decorations.rs:15-71`）。`set_readonly(true)` + `line_number(true)`。实心/空心=unstaged/staged 用两个 token 区分（Zed 视觉语言）。
- **打磨升级版（按需）**：`uniform_list`/`v_virtual_list` 自绘 diff 行，获得左右双列行号、hunk 折叠（`⋯` 可展开）、行级 stage（lazygit 语义：空格选行、`a` 选 hunk）。第一版不做的理由：自绘要处理滚动/选区/字体度量，收益在交互细节而非信息呈现。

**③ 文件树 git 状态着色**：`Tree` 的每行 item 附加 `FileStatus`，文件名用 `version_control_{modified,added,…}` token 色 + 右侧单字母状态字形。数据直接来自 watcher 刷新的 `RepoStatus`（`examples/editor` 的文件树 + Ignorer 现成可改）。

**④ Action 化**（design-guides 桌面优先 + 命令面板可发现性）：

```rust
actions!(git, [Stage, Unstage, StageAll, UnstageAll, Commit, Refresh, OpenDiff, TogglePanel]);
// 键位: cmd-ctrl-g 开关面板; 面板内 cmd-enter 提交; diff 视图内 enter 打开文件
```

全部 action 注册进 `Command` 命令面板；勾选框之外的路径统一走 action（Zed 双轨制：checkbox 直调 + 菜单/键位走 action）。

### 3.5 Git 模块任务拆解（映射 P2）

- [ ] G1 `src/git/`：`repository.rs`（trait + git2 实现）+ actor 线程 + 事件通道
- [ ] G2 `RepoStatus` 快照 + 两份 imara-diff 管线 + DiffStat
- [ ] G3 notify watcher 接入（监听工作区 + `.git/index`，50ms 防抖 → 刷新）
- [ ] G4 GitPanel（bottom dock Panel：分区列表/勾选/全暂存/commit）
- [ ] G5 Diff 视图 Panel（只读 Editor + decorations 着色）
- [ ] G6 文件树状态着色 + StatusBar 分支/变更数
- [ ] G7 actions + 键位 + 命令面板注册
- [ ] G8 非git 项目空态 / 分支 Popover

依赖关系：G1→G2→（G3,G4,G5 可并行）→G6→G7→G8。

---

## 4. 文件编辑

### 4.1 选型：复用 `EditorState`，不自研（锁定决策 #1 P1 的落实）

Zed 的文本栈有四件我们没有的东西，逐个评估后都属于「现在不要」：

| Zed 组件 | 买到什么 | 本应用结论 |
|----------|---------|-----------|
| Rope + SumTree 摘要树 | 20 万行文件 O(log n) 编辑/定位 | YAGNI。ropey 已是 Rope；5 万行红线内性能无忧（框架注释明示定位：`state.rs:7311`） |
| Anchor（随编辑漂移的位置句柄） | diff hunk 区间免重算 | 用「编辑失效→整文件重算 diff」绕过（毫秒级），见 §3.3 |
| MultiBuffer + Excerpt | 把多文件 hunk 拼成一个虚拟 buffer（ProjectDiff 的地基） | 降级为「每文件一个只读 diff 视图 + 文件列表」（§3.4 ②），够用 |
| CRDT/Patch | 实时协作 | 永远的以后再说 |

结论：**编辑器零自研**。`EditorState` 一个 builder 链即可达到 IDE 观感（`examples/editor/src/main.rs:695-707` 验证过的真实 API）：

```rust
let editor = cx.new(|cx| {
    EditorState::new(window, cx)
        .language(lang_from_ext(path).to_string())   // → LanguageRegistry tree-sitter
        .line_number(true)
        .indent_guides(true)
        .tab_size(TabSize { tab_size: 4, hard_tabs: false })
        .soft_wrap(false)
});
editor.update(cx, |s, cx| s.set_value(text, cx));
```

### 4.2 FileEditorPanel 设计

一个实现 `Panel` 的 `FileEditorPanel`（center dock），职责闭环：

- **打开**：文件树/文件列表点击 → 按路径查 `HashMap<PathBuf, WeakEntity<FileEditorPanel>>` 复用已有 tab，否则新建并入 center tab 组（`dock_area.add_panel`）。标题 `文件名 + dirty 点`。
- **dirty 跟踪**：`cx.subscribe(&editor_state, … &InputEvent::Change)`（`examples/editor` 同款），置 dirty 并通知 tab 标题。`panel_name()` 固定 per-file，`dump()` 存路径用于会话恢复——注意计划决策 #7 已砍会话恢复，dump 返回空即可，仅保留接口。
- **保存**：`⌘S` action → `text()` 拷出 → `fs::write` → 清 dirty → 触发 git 刷新（保存即产生 worktree 变更，watcher 也会兜底）。
- **外部变更检测**（agent IDE 特有且高频——agent 在终端里改文件）：watcher 事件 + mtime 比对；若文件未 dirty → 静默重载；若 dirty → `window.open_dialog` 三选：「重新加载 / 覆盖外部修改 / 取消」（文案规范：宾语+动词）。
- **语言推断**：扩展名 → LanguageRegistry 已注册语言名；未识别语言退化为无高亮纯文本，不报错。
- **只读模式**：diff 视图复用同一面板形态，`set_readonly(true)`。

### 4.3 文件树

复用 `Tree`/`TreeState` + `examples/editor` 的 `Ignorer`（`.gitignore` 感知）改造：懒加载目录（展开时才读子目录）、git 状态着色（§3.4 ③）、点击开文件。**用领域路径 `PathBuf` 做 item id，不用索引**（skill 非协商项）。

### 4.4 红线（写进代码注释与 review checklist）

- `EditorState` 官方上限约 **5 万行**：打开前按行数/字节数 guard（如 >2MB 或 >50K 行 → 提示以只读+截断方式打开，或拒绝）。terminal scrollback 不走此面板。
- `text()` 返回借用，读取大文档时避免无谓 `to_string()`；保存路径已经是一次拷贝，属于必要成本。
- 不给 EditorState 加 LSP 之外的重量特性（折叠/搜索/多光标框架已内置，直接用）。

### 4.5 文件编辑任务拆解（映射 P1）

- [ ] E1 FileEditorPanel（打开/保存/dirty/tab 复用/语言推断/行数 guard）
- [ ] E2 文件树（Ignorer 改造 + 懒加载 + 点击打开）
- [ ] E3 外部变更检测（watcher + dialog 三选）
- [ ] E4 只读 diff 渲染模式（与 G5 共用实现）
- [ ] E5 快捷键：⌘S 保存、⌘P 文件快速打开（Combobox/Command 过滤项目文件）

---

## 5. 样式美化

### 5.1 机制：主题即数据，零硬编码

框架已内建完整链路，直接用：`ThemeRegistry::watch_dir("./themes", …)` 热加载 → `Theme::global_mut(cx).apply_config(&config)` → `Theme::change(mode, …)`。主题 JSON 是 **Zed 兼容 schema**（`themes/*.json` 21 套可直接当默认候选），`highlight` 段同时控制 UI 与 tree-sitter 语法配色——**给 IDE 换肤等于换一个 JSON 文件**。跟随系统 light/dark（`sync_system_appearance`）。

### 5.2 Git 语义色板（token 设计）

照 Zed 的 token 命名（它用 `version_control_` 前缀而非 `git_`）扩展 gpui-kit 的 `ThemeColor`（框架缺口，改动小、足够通用，符合「稳定后 feature-gate 上游」策略）：

```jsonc
// themes/catppuccin.json 的 colors 段新增（值为 Catppuccin Macchiato 官方色，仅示例）
{
  "version_control_added":    "#a6da95",   // green  — 新文件 / + 行
  "version_control_modified": "#eed49f",   // yellow — 修改文件 / ~ 标记
  "version_control_deleted":  "#ed8796",   // red    — 删除文件 / − 行
  "version_control_renamed":  "#8bd5ca",   // teal
  "version_control_conflict": "#f5a97f",   // peach
  "version_control_ignored":  "#6e738d",   // overlay0 — 弱化
  // diff hunk 六件套：实心 = 未暂存，空心(描边) = 已暂存 —— Zed 的视觉语言
  "diff_hunk_added_background":        "#a6da9526",
  "diff_hunk_added_hollow_border":     "#a6da9566",
  "diff_hunk_deleted_background":      "#ed879626",
  "diff_hunk_deleted_hollow_border":   "#ed879666",
  "diff_hunk_expander_context":        "#939ab740",
  "diff_hunk_header":                  "#6e738d"
}
```

代码里**只允许出现 token 名，禁止裸 hex**（design-guides 非协商项）：`cx.theme().version_control_modified`。浅色主题由各主题 JSON 自行给值；21 套存量主题可写个脚本批量补默认值。

### 5.3 design-guides 落地清单（评审 checklist 摘录）

- **桌面优先**：所有 Git 操作有键位 + 命令面板入口；面板可拖拽/缩放（`h_resizable`/dock 折叠）；hover 才出现的次要操作（文件行 hover 显「打开 diff」）。
- **交互状态必须可见**：hover/focus/selected/disabled/loading 全走组件默认态；commit 按钮 disabled、刷新中 `Spinner`。
- **浮层 Esc 关闭并还焦点**（Popover/Dialog 由框架覆盖，勿自绘关闭逻辑）。
- **文案：宾语+动词**：「还原 "main.rs"？」「此目录不是 Git 仓库」。
- **密度**：GitPanel 行高用紧凑档（与 Zed 一致的小行高 + 12px 字号），diff 视图用编辑器行距——数据密集界面不放大间距。
- **动效克制**：面板展开/收起用框架 motion token，不做 diff 行逐行动画。

### 5.4 默认主题推荐与视觉细节

- 默认主题推荐 **catppuccin / tokyonight**（暗色）+ macos-classic 或 aurora（亮色备选），二者的 diff 绿/红在 `themes/` 里已有成熟值可校准。
- 状态字形统一用 `IconName`（Lucide）或单字母 badge（`M`/`A`/`D`/`?`），全 app 一处定义、处处复用。
- diffstat 条：4px 高横条，绿/红两段按比例铺，超过宽度截断——比纯数字扫读快。
- 空态：工作区干净时 GitPanel 显示「工作区是干净的 ✓」；diff 视图无变更时显示引导文案。空态是好 UI 与模板 UI 的分水岭，务必做。

---

## 6. 风险与对策

| 风险 | 影响 | 对策 |
|------|------|------|
| git2/libgit2 行为漂移（reftable、SHA-256 仓库） | 打不开新格式仓库 | `GitRepository` trait 隔离，退路换 Zed 式 CLI 实现；monitoring libgit2 release notes |
| agent 批量写文件 → 刷新风暴 | UI 卡顿 | 50ms 防抖 + 快照整体替换 + diff 计算全在 actor 线程 |
| 超大文件/超大 diff | EditorState 红线 | 打开前 guard；diff 视图按文件懒加载，单文件 hunk 上限截断（「加载更多」） |
| Zed 代码 GPL-3.0 | License 传染 | 只借鉴模式与 token 命名，不拷代码；本仓库保持 Apache-2.0 干净 |
| `ThemeColor` 扩展面 | 上游合并冲突 | 新字段集中一处（`theme_color.rs` 尾部 git 段），全部有默认值，向后兼容 |
| notify 在 macOS 的 FSEvents 延迟 | 状态短暂陈旧 | 动作后主动刷新兜底 + StatusBar 提供 ⟳ 手动刷新 |

---

## 7. 参考资料（本次实际核对的源码）

- Zed git 后端：[crates/git/src/repository.rs](https://github.com/zed-industries/zed/blob/main/crates/git/src/repository.rs)（CLI spawn、`DiffType`、后台线程模型）、[git2 移除时间线](https://github.com/zed-industries/zed/commits/main/crates/git/src/repository.rs)（2026-06-02 #53453）
- Zed git UI：[git_panel.rs](https://github.com/zed-industries/zed/blob/main/crates/git_ui/src/git_panel.rs)（分区/勾选/50ms 防抖）、[project_diff.rs](https://github.com/zed-industries/zed/blob/main/crates/git_ui/src/project_diff.rs)（Uncommitted Changes tab）、[buffer_diff.rs](https://github.com/zed-industries/zed/blob/main/crates/buffer_diff/src/buffer_diff.rs)（双 diff + 几何推导状态）、[Zed git 博客](https://zed.dev/blog/git)（diff-of-diffs、删除行即编辑器坐标系）
- Zed 主题：[colors.rs](https://github.com/zed-industries/zed/blob/main/crates/theme/src/styles/colors.rs)（`version_control_*` token 命名出处）
- 库选型：[gitoxide crate-status.md](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md)（index 写入/stash/push 缺口）、[git2 crates.io](https://crates.io/crates/git2)
- 对照项目：[Lapce](https://github.com/lapce/lapce)（git2 vendored + proxy 分层）、[GitButler](https://github.com/gitbutlerapp/gitbutler)（gix 为主 + FS watcher 驱动）、[gitui/asyncgit](https://github.com/gitui-org/gitui)（git2 actor 模式）、[lazygit](https://github.com/jesseduffield/lazygit)（行级 stage 交互语义标杆）
- 生态：[awesome-gpui](https://github.com/zed-industries/awesome-gpui)（git 组件空白佐证）
- 本仓库：`skills/gpui-kit/SKILL.md`（组件目录）、`skills/gpui-kit-design-guides/references/design-guides.md`（设计规范）、`examples/dock`（Panel/DockArea 模板）、`examples/editor`（EditorState/文件树/LSP 模板）

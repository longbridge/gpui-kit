# Agent IDE 工具 — 计划（grill-me 2026-09-07 全分支已锁定）

## 背景与定位

基于 gpui-kit（本仓库）搭建自己的 agent IDE 工具：AI agent 的 GUI 前端，terminal-first。

## 锁定决策

| # | 分支 | 决策 |
|---|------|------|
| 1 | 三期划分 | **P0 主循环可用**：crate 骨架 + Terminal 组件 + 多 tab + BEL 状态机 + 项目列表（最小版）。**P1 读懂代码**：文件树 + 编辑器（复用 `InputState`+`highlighter`，不自研）+ diff 面板（git diff 驱动）。**P2 管理与打磨**：git 管理（status/stage/commit）+ 细节打磨 |
| 2 | crate 位置/命名 | `crates/agent-ide`，bin crate，进 workspace members，不发布；名字暂用 agent-ide，数据目录名 P0 结束再定 |
| 3 | 终端组件去向 | `crates/agent-ide/src/terminal/` 应用内部模块；不进 gpui-component（Zig 构建负担强加消费者）、不建独立 crate（YAGNI）；稳定后可 feature-gate 上游 |
| 4 | examples/agent_ide | 迁移完成后删除，与迁移同一 PR |
| 5 | 多 tab 承载 | Dock 系统（`LayoutTree`/`DockArea`/`TabGroup` + `Panel`），参照 `examples/dock` 与 story |
| 6 | 持久化 | 单 JSON：`dirs::data_dir()/agent-ide/state.json`，仅最近项目列表（见 #7 缩减）；不引 DB |
| 7 | 会话恢复 | **整体砍掉，第一版不涉及**；`claude --resume` 留作远期增强 |
| 8 | diff 面板数据 | git diff（staged/unstaged vs HEAD）；非 git 项目显示不可用；不做快照 |
| 9 | git 实现 | `git2` crate（结构化 diff、Zed 先例），不 shell out |
| 10 | tab 启动内容 | 只开登录 shell（`$SHELL`），不自动起 claude；无配置系统，PATH/环境变量即配置 |
| 11 | 平台 | 只保 macOS |

## 早前已锁定（上一轮讨论）

- 定位：AI agent 的 GUI 前端；真实终端路线（libghostty-vt 渲染 + portable-pty 起 PTY）；零屏幕解析、零 headless 管道。
- 状态感知：BEL(0x07) → 「需要关注」；OSC 0/2 标题 → 补充；OSC 7 → cwd（**注：默认 zsh 不发 OSC 7，第一版降级为 tab 记住项目目录**）；「等确认 vs 完成一轮」合并为「需要关注」。开 shell 后进程存活 = 终端会话存活。

## P0 任务清单（骨架）

- [x] 1. 新建 `crates/agent-ide`（Cargo.toml + main.rs：窗口/Root/主题初始化），加入 workspace members
- [x] 2. 从 `examples/agent_ide/src/terminal.rs` 迁终端到 `crates/agent-ide/src/terminal/`
- [x] 3. 迁三栏布局骨架，接 Dock 系统（terminal panel 实现 `Panel` trait 两侧）
- [x] 4. BEL → tab 状态点（需要关注）状态机
- [x] 5. 项目列表（打开本地目录 + 最近列表持久化到 state.json）
- [x] 6. 删除 `examples/agent_ide`

## 现状（examples/agent_ide）

三栏布局骨架 + 真 libghostty 终端（terminal.rs 877 行：PTY/按键编码/渲染，未抽组件）；编辑器/Browser/假数据聊天流为静态假数据；未用 Dock。

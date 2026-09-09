---
title: 设计指南
description: 面向 GPUI Kit 客户端应用的产品、视觉与交互设计规范
order: -2.1
---

# 设计指南

请在选择组件或编写布局代码之前阅读本指南。它记录 GPUI Kit 在多年桌面应用开发中形成的产品判断：界面应当原生、克制、精确，并让用户无需猜测就能完成任务。

本文是一份规范性指南：**必须**表示正确性或生态约束，**应该**表示默认选择，偏离时需要有明确理由；**可以**表示可选方法。具体方法签名仍以组件 API 文档和当前源码为准。

这些规则建立在 `gpui-base` 的行为能力、GPUI Component 的主题与组件体系，以及桌面平台共同的交互习惯之上。Shadcn 提供了开放代码、组合能力和可靠默认值等有益方法，但不决定 GPUI 应用的外观。发生冲突时，优先服从 GPUI 生命周期和用户熟悉的桌面交互。

## 设计主张

界面应该原生、安静、准确。由内容、层级与交互承担体验，装饰只用于支持它们。

1. **先清晰，后个性：** 在加入品牌表达前，先让主要任务与下一步行动清楚。
2. **先组合，后发明：** 从已有组件出发组成产品工作流；只有行为确实不同，才创建新 primitive。
3. **先 token，后数值：** 颜色、圆角、字体与间距必须构成系统，避免无法响应主题的孤立字面量。
4. **桌面约定优先于网页惯例：** 保留键盘操作、窗口框架、菜单、密集数据视图、可调整区域和持久导航。
5. **状态必须可见：** hover、焦点、选中、禁用、加载、校验与破坏性状态必须清楚且一致。

## 借鉴 Shadcn

Shadcn 最有价值的不是某一种边框颜色，而是一套建立系统的方法：

- 拥有界面最上层代码，不与封闭抽象反复对抗；
- 用小而可预测的部件组合出产品组件；
- 默认样式本身就属于同一种视觉语言；
- 代码和组合结构同时对人和 AI 可读；
- 把行为 primitive 与带视觉观点的样式层分开。

GPUI Kit 通过 Rust 库以及 `gpui-base` / `gpui-component` 分层来实现这些原则。应用通常组合或适度封装公开组件；贡献者把真正可复用的行为放入 Base，把视觉决策留在上层。

以下网页习惯不应直接复制：

| 网页习惯 | GPUI 原生默认方式 |
| --- | --- |
| 所有按钮都显示手形光标 | 按钮使用箭头，只有链接使用手形 |
| 以页面跳转为主要结构 | 使用持久窗口、pane、sidebar、tab 与 menu |
| 依赖浏览器兜底焦点和滚动 | 明确焦点所有者与各区域的滚动所有者 |
| 移动优先的单列布局 | 有最小窗口尺寸定义的可调整桌面 shell |
| 关键操作只在 hover 时出现 | 键盘和指针都可达，不依赖 hover 才存在 |
| 每行放一排 hover-only 图标按钮 | 一个可见主操作，次要命令使用 `DropdownMenu` / `ContextMenu` |
| 用 Link 样式文字执行应用 command | 使用 Button、outline 或 ghost；Link 只用于 URL、网页资源或 Email 地址 |
| 全局采用很大的触摸密度 | 默认 medium，专业数据区域才局部紧凑 |
| 通过 CSS 穿透修改后代 | 使用 typed builder、语义部件和应用组合 |

## 从用户任务开始

画界面之前先写清楚：

- 用户的主要任务；
- 正在查看或修改的对象；
- 必须立即可见的操作；
- 做决定所需的信息；
- 空、加载、错误、离线、只读和无权限状态；
- 完整流程的键盘路径。

窗口结构应由这些答案决定，不要从 dashboard 卡片网格或组件目录开始。优秀的桌面应用呈现用户的心智模型——文档、账户、项目、消息、设置——而不是内部服务架构。

先确定主要任务，再选择控件。视觉权重、位置与信息深度必须匹配功能在产品中的重要性。不能把核心结果缩成角落里的数字、图标或弱化操作，却让次要内容占据页面。当结果集本身就是产品价值，应使用摘要区域或卡片展示数量、代表性结果、当前状态和清晰的下一步。

每个操作都要有明确的对象、当前状态、作用范围和结果。界面尚未展示或暗示这些信息时，操作就出现得太早。不能因为后端已有能力就直接暴露入口；先设计用户如何理解这个对象和结果。

## 视觉语言

### 层级

使用少量、清晰的层级：

- **窗口或页面标题**标识当前对象或工作区；
- **章节标题**分隔有意义的区域；
- **正文**承载工作内容；
- **弱化文字**提供次要元数据和帮助；
- **标签**标识控件和值。

先用字号、字重、间距与分隔线建立层级，再考虑颜色和容器。避免卡片嵌套卡片；大多数桌面区域只需要背景、hairline 边界和有意图的留白。

层级必须按整个功能评审，不能只看单个组件。隐藏强调色和装饰后，主要任务、当前选择、结果摘要和下一步仍应能从结构中识别。每个控件单看都合理，如果没有形成清楚的阅读顺序与决策路径，整体设计仍然失败。

注意力是有限的。一个局部区域只需要一个清晰重点。如果所有内容都有颜色、`Badge`、粗体、容器或 `Alert`，就没有内容真正重要。先用结构和距离建立优先级；只有某个区别会改变用户的注意或行动时，才使用更强的颜色或组件。

### 颜色与主题

从 `cx.theme()` 读取颜色，并按语义角色使用：

- `background` / `foreground`：主表面和正文；
- `group_box`、`popover`、`sidebar` 及对应 foreground：具名表面；
- `muted` / `muted_foreground`：辅助信息；
- `primary`：当前决策区域的主要操作或选中强调；
- `danger`、`warning`、`success`、`info`：只表达对应语义；
- `border`、`input` 和 ring token：结构与交互。

不要把状态色当装饰，也不要只依赖颜色表达含义。自定义表面必须在明暗主题和自定义主题中验证；不能假设 foreground 一定是黑色或 background 一定是白色。

Badge 只用于有助快速扫描的短 state、count 或 classification，不能套在每个 label、metadata value、filter 或 section title 上。大多数 Badge 保持 neutral；只有真实表达 success、warning、danger 或 info 时才使用 semantic variant。满屏多色 Badge 通常说明 hierarchy 或 grouping 尚未设计完成。

Application UI 不应出现 raw hex、`rgb`/`rgba` 或 `hsla` color。颜色必须按语义角色从`cx.theme()` 解析。需要的 role 不存在时，在产品 theme/token 层定义，而不是在 call site 嵌入 palette value。Raw color 只属于 theme definition，或颜色本身就是数据的、经过审查的 data/raster content。

### 圆角、间距与密度

所有应用拥有的控件圆角都应来自主题。这样产品才能作为一个整体变得方正或圆润。圆形和胶囊使用 `radius_full()`，不要写死最大圆角。

采用紧凑、重复的间距 scale。label 与 control 的距离应小于两个 group 的距离，两个 group 的距离应小于两个 section 的距离。优先使用组件尺寸（`xsmall`、`small`、默认 `medium`、`large`），不要单独写高度。compact 用于 toolbar 和数据密集界面，不能用来把结构不清楚的布局硬塞进更小空间。

共享语义 scale 有意保持很小：间距大致为 2、4、8、12、16、24、32 px；字体大致为 12、14、16、18、20 px。这些表示关系，不是鼓励把当前数值散落在 feature 代码中。GPUI Component 的 global `Theme` 当前投射的是固定默认 `SpacingTokens`；与 color/radius 不同，`Theme::apply_semantic_tokens` 不会保存 custom spacing scale。需要不同 scale 的应用必须自行持有完整 token snapshot，并在 application component 中一致使用。

### 空间语法

间距表达关系。根据相邻对象的语义关系选择 token：

| 关系 | 常用 token | 当前 scale | 示例 |
| --- | --- | --- | --- |
| 光学校正 | `xxs` | 2 px | 图标基线、紧凑分隔 |
| 同一控件的部件 | `xs` | 4 px | 菜单图标与标签、标题与说明 |
| 紧密相关的控件 | `sm` | 8 px | 按钮图标与文字、对话框操作 |
| 同一个内容 group | `md` | 12 px | 通知各列、紧凑表单行 |
| 同一 section 的不同 group | `lg` | 16 px | panel 内边距、表单组 |
| 不同 section | `xl` | 24 px | 页面或 inspector 的主要区块 |
| 主要区域边界 | `xxl` | 32 px | 空状态留白、页面大段落 |

这些数值描述当前默认 scale；生态默认代码使用 `cx.theme().spacing_tokens()` 或对应 GPUI scale helper。产品自有 scale 应保持关系顺序，并通过 application design-system context 传递，不能假设它会持久保存在 GPUI Component global theme 中。

处理上下左右空间时遵守以下规则：

1. **先分清内部与外部。** 组件 padding 由组件拥有，组件之间的 gap 由父容器拥有。
2. **纵向节奏表达分组。** 标题到说明的距离小于说明到下一 section 的距离；相等间距意味着相等关系。
3. **横向空间服务扫描。** 重复行中的图标、label、value、badge、尾部 action 应落在稳定列上。
4. **leading/trailing 是语义。** 即使当前 API 使用 left/right，也应按阅读方向思考，为未来 RTL 留出可能。
5. **不要重复 padding。** 已有 panel inset 内的 card 不应再无条件增加一整层 panel padding。
6. **谨慎使用光学校正。** 图标或字形允许 1–2 px 修正，但必须能解释为何偏离 scale。

现有系统中的常见组合揭示了这些关系：

- 小按钮内容间距为 4 px，常规尺寸为 8 px；
- Dialog header 与 footer 内部间距为 8 px；
- 紧凑 list/menu 行通常采用 4 px 纵向、8–12 px 横向 padding；
- Sheet header 约为 leading 16 px、trailing 12 px，为关闭控件留出空间；footer 为横向 16 px、纵向 12 px；
- Notification 使用 16 px 横向 padding 和 12 px 列间距，因为图标、消息和操作属于不同 group。

这些不是所有 surface 的复制模板，而是在说明：控件内部最紧，行针对扫描优化，容器边界比内容内部花费更多空间。

### 比例与层次

先确定内容约束，再确定比例。角色不同的 pane 不要机械地各占一半。

- Sidebar 应足够容纳稳定标签，但视觉上从属于工作区；为它定义最小、首选与最大宽度，而不只是百分比。
- Master–detail 中，collection 必须仍可扫描，detail 消耗剩余空间。约 1/3 与 2/3 可作为起点，但内容约束优先。
- Inspector 与辅助 sheet 默认不能遮住主要对象；内容增长时应可调整或关闭。
- Dialog 宽度由决策复杂度决定：短确认、中等表单；复杂工作应进入独立页面或窗口。
- 最强 elevation 只给最上层决策面；同一层内使用背景和 hairline，而不是不断加重阴影。

每个主要区域要定义三件事：任务可用的最小尺寸、舒适默认尺寸、如何消费剩余空间。如果 split 表达用户工作习惯，应持久化；恢复后必须按当前窗口重新 clamp。

### 对齐细节

对齐是一套结构系统，不是最后的润色。每个 surface 应先建立少量 alignment spine：共享的 leading/trailing edge、文字 baseline、center line 与固定功能 lane。同一层级的元素即使使用不同 component，也应从上到下或从左到右落在同一条 spine 上。

<img class="alignment-light" src="/alignment-spines.svg?v=20260822-4" alt="桌面界面的对齐参考线">
<img class="alignment-dark" src="/alignment-spines-dark.svg?v=20260822-4" alt="桌面界面的对齐参考线">

纵向红色虚线贴近共享边缘或控件中心，说明内容、状态、时间与尾端操作的列对齐；横向虚线分别贴近文字基线并穿过行中心，说明文字底边和不同尺寸控件的垂直居中。右侧紧凑对照单独展示必须从结构所有者修复的一个渲染像素漂移。

- Sibling region 使用相同 content inset。同层级的 heading、toolbar、list row、empty state 与 footer 不能各自发明略有差异的起始线。
- 整个区域重复同一套 column geometry。Header、row、summary、loading state 与 inline editor 为 identity、metadata、status、number 和 action 保留相同 lane。
- 跨 row 与 section 对齐相关 control。从上到下扫描时，form label、field、description 与 validation message 应显现稳定的纵向网格。
- 水平 band 保持一致。同一 toolbar、title bar、status bar 或 row 中的项目共享 baseline 或 center line，不能逐个用 offset 微调。
- 只有真实 hierarchy、containment 或 disclosure 才引入 indentation。装饰性缩进会让 sibling 看起来像 subordinate，并破坏阅读起始线。
- Nested level 结束后必须准确回到 parent spine，不能让多层 container 的 padding 逐层漂移。
- Optional content 出现或消失时仍保持 spine。缺少 icon、badge、description 或 trailing action 不能推动其余 label；需要跨行比较时使用明确 slot 或 lane。
- 层级相同时，major region 之间也应互相对齐。Sidebar header、content title、split pane、toolbar 与 bottom bar 不必共享所有坐标，但相同层级应形成可见的连续线。

不是每一条边都必须对齐。Child 可以缩进，primary value 可以领先 supporting metadata，destructive decision 也可以获得额外间隔。但例外必须表达层级或语义，不能只是各 component padding 未协调的结果。先建立 shared spine，再明确设计例外。

精确对齐与重复 gap 是质量 invariant。两条 edge 或两段 spacing 按设计应相等时，相差一个 rendered pixel 也是 defect，不能以“肉眼大致一致”通过。必须在代表性的 window size、zoom level 与 display scale factor 下用测量工具检查 resolved bounds，直接比较 coordinate 与 distance，不能只看一张截图凭感觉验收。

这里的 rendered-pixel 容差是验证规则，不是允许用 raw pixel offset 修补代码。相等关系应来自同一个 `rem` helper、spacing token、grid definition 或 shared component inset；发生偏差时修复共同 owner。还要考虑 fractional layout 与 device rounding，确保预期 spine 在不同 zoom 下落到同一 physical pixel，而不是只在某一档看似对齐。

- 混合字号同排时按文字 baseline，而不是按 bounding box 中心对齐。
- 图标放入固定槽位，避免不同固有宽度导致 label 抖动。
- 可比较数字右对齐，文本和标识符通常左对齐（locale 另有要求除外）。
- 行尾 action 和 disclosure indicator 使用固定宽度 lane。
- 表单控件按交互 frame 对齐，而不是按下面的 help text 对齐。
- 只有两侧确实分别拥有相反边缘时才用 `justify_between`，不要用它掩盖中间结构缺失。
- Hairline 由边界所有者绘制，相邻区域不能各画一次同一条分隔线。
- 滚动条属于实际滚动的区域，并贴住面板、编辑器或窗口的尾端边缘。内容内边距可以缩进文字与行，但不能把滚动条推到界面中间；内容需要避让时，应保留明确的滚动条槽位。

### 密度层级

Medium 是生态默认值。密度应在局部上下文整体变化，而不是只改一个控件：

- **comfortable / large：** onboarding、稀疏表单、重要决策；
- **standard / medium：** 大多数应用 chrome 与工作流；
- **compact / small：** toolbar、menu、table 和重复的专业数据；
- **extra compact / xsmall：** 极少数高密度工具，不应成为全应用默认。

当前控件体现的是有限 scale：按钮 frame 大致为 20、24、32 px；input 与数据控件在 large 时可到约 44 px；table row 大致为 26、30、32、40 px。使用组件 `Size`API，让字体、图标、padding 和 hit target 一起变化。只改外框高度通常是不完整的。

### 缩放、基础字号与 `rem`

良好的 `rem` 系统能在界面 zoom 时保持设计层次。成功的 zoom 不只是每个对象都变大，而是在每一档 scale 下，title/body、control/icon、inner/outer spacing、primary/secondary region 之间仍保持相同关系。

GPUI Component 采用了 Tailwind 中有价值的相对 scale 思想。Theme 的 base `font_size`通过 `Root` 成为 window `rem`；`text_sm()`、`gap_2()`、`p_4()`、`h_8()`、`size_4()`等 GPUI scale helper 都以它解析。Typography、spacing、control 与 icon 因而共享同一条 zoom axis。

设计时关注比例：

- type step 围绕 body base size 保持相同 hierarchy；
- spacing step 围绕 typography 保持相同 grouping；
- control frame、icon、hit target 与 label 一起缩放；
- pane minimum 与 comfortable width 要容纳缩放后的内容；
- radius 与 focus treatment 相对 control frame 保持光学一致。

不能只改变文字实现 zoom。固定高度 Button 中放大 label、固定 pane minimum 中放大 document、或 virtual-list measurement 未失效时放大 row，都会破坏原有节奏并裁切内容。反过来，把包括 hairline 在内的每个 physical pixel 全部相乘，也会让界面变得沉重。

原则上 application layout 不得直接调用 `px(...)`。使用 GPUI rem-based scale helper（`p_2`、`gap_3`、`w_64`、`text_sm` 等 builder）或 semantic component Size。只有值代表 physical/raster boundary 时才使用 fixed px：one-device-pixel hairline、platform window inset、bitmap dimension、minimum hit-test tolerance，或必须匹配 external surface 的 geometry。这些必须是经过审查和记录的例外。Product spacing、typography、icon size 与普通 control geometry 保持在 relative scale 上。

不能只在默认值验证。至少使用多档 base font 检查 hierarchy、wrap、truncate、minimum window size、pane resize、focus-ring clearance、popup placement 与 virtualized row measurement。同时区分 interface zoom 与 Dock panel zoom：Dock zoom 让一个 container 保留 chrome 并填满区域，不改变 `rem` 或 application scale。

### 表面与层级

Elevation 用于解释叠放关系，而不是表达重要程度。基础窗口保持平面；区域由背景差异和 separator 划分。Popover、menu、dialog、notification 因处于内容上方，可以逐级增强阴影。不要给每张 card 都加阴影。

同类 surface 必须采用同一种处理。GPUI Component 有意让 Popover、Select、Combobox、DatePicker 和 menu 共用 popover surface，避免它们逐渐漂移。应用新增 anchored surface 时应复用该语义处理，而不是用无关的 border/shadow 字面量近似。

### 字体与图标

界面文字使用平台 UI 字体；代码、标识符、快捷键和对齐数字才使用等宽字体。正文必须易读，避免过度大写或字距，尤其不能把拉丁文字的 tracking 直接套给 CJK。

一个产品使用同一套图标。图标辅助 label，不能用猜谜替代陌生操作。仅图标按钮必须有 tooltip 与 accessible name。填充或彩色图标用于表达状态，不用于让 toolbar 显得热闹。

## 布局模式

### 选择稳定的应用框架

大多数应用适合以下一种：

- **单工作区：** toolbar/title bar 加一个主要视图；
- **Sidebar 工作区：** 持久导航与变化的 detail；
- **Master–detail：** 可调整 collection 与 detail pane；
- **文档工作区：** tab 或 DockArea 管理多个长期对象；
- **Utility window：** 单一任务与短而固定的操作路径。

全局导航在内容变化时应保持稳定。主要工作区用 `flex_1()` 消费剩余空间；可收缩的 overflow child 需要 `min_w_0()` / `min_h_0()`。滚动、虚拟列表、表格和 Dock 应使用 `Scrollable`、`VirtualList`、`Table`、`DockArea`，不要用多层 `div` 重做行为。

### 可调整的桌面窗口

桌面并不表示固定尺寸。窗口变窄时按以下优先级处理：

1. 保留主要任务；
2. 可调整区域达到有文档的最小值；
3. 折叠次要 label 或 inspector；
4. 把低频操作移入 menu；
5. 只滚动真正发生 overflow 的区域。

隐藏操作时必须提供另一条路径。不要让整个窗口滚动，而实际只有 list 或 document body 需要滚动。

GPUI flex child 即使设置 `flex_1()`，也可能因为长内容拒绝收缩。设计和代码必须约定哪些 pane 可以收缩、截断、换行或滚动。`overflow_hidden()` 也会裁掉向外绘制的 focus ring，不能为了简化 overflow 牺牲键盘焦点可见性。

### 表单与设置

每个字段使用可见 label，help 与 validation 放在所描述字段附近。相关字段应对齐，但不要强迫长 label 进入过窄固定列。独立选择用 `Checkbox`，少量可见互斥项用`RadioGroup`，较长集合用 `Select`，立即生效的设置用 `Switch`。

操作进行中禁用重复提交，保留用户输入，并在操作附近显示结果。Dialog 只用于短而聚焦的决策；需要探索或大量字段的流程使用完整页面或 sheet。

## 组件与组合

采用 Shadcn 的核心思想：组件是构建材料，不是封闭设计系统。GPUI Component 提供一致默认值，应用拥有组合和产品语义。

- 变体按语义使用。主按钮（`primary`）只留给决策区域中明确的默认提交，通常也是按 Enter 执行的操作。操作唯一、使用频繁或希望用户注意，都不会使它自动成为主按钮。管理工具栏中的“添加”通常使用默认按钮；表单中默认提交的“创建”可以使用主按钮。`danger` 表示破坏性提交，`ghost` 用于低强调的工具栏操作。
- 优先使用明确的复合部件与渲染回调，不要穿透任意后代设置样式。
- 跨产品重复且带有领域语言或规则的模式，应封装为应用组件。
- 按语义角色使用标准组件。菜单、下拉菜单、弹出层、选择器与命令面板不是可以互换的容器；它们分别拥有选择、焦点、键盘操作、关闭方式和布局契约。
- 保留同类组件的几何规则。菜单行的上下左右内边距、高度、图标槽、勾选槽、分隔线、圆角和状态样式必须统一；不能用自定义弹出层模仿一个间距仅仅接近标准组件的菜单。
- 不要仅为重命名每个方法或冻结所有能力而包装一层库组件。
- 无产品样式的复用行为放入 `gpui-base`；有主题观点的表现留在 Component 或应用。

## 交互状态

### 让结果先于点击被理解

控件应当让结果在操作前就可以预见。优先使用熟悉的桌面控件与布局，让用户不必先学习界面。文案说明动作与对象，状态说明当前是否可用，反馈则确认同一个结果。

不要把实际会打开设置流程的按钮写成“保存”，也不要用“删除”描述仅从分组移除的操作。上下文不能说明范围时，直接写出范围。按下后立即反馈；耗时操作防止重复提交，并在被改变的对象附近显示结果。只有结果本身不可见时，才补充成功提示。

每个交互控件都要设计以下状态：

| 状态 | 设计要求 |
| --- | --- |
| 静止 | 操作提示清楚但不嘈杂 |
| 悬停 | 提供轻微指针反馈，但不能成为唯一线索 |
| 按下 | 立即响应按压 |
| 打开 | 附属弹出层打开期间持续显示按下或打开状态 |
| 焦点可见 | 显示高对比键盘焦点环 |
| 选中 | 状态持久，并与悬停明确区分 |
| 禁用 | 降低强调，且不产生误导性的悬停或按下反馈 |
| 加载 | 保持上下文、防止重复操作，并解释较长等待 |
| 错误 | 说明发生了什么以及如何恢复 |

需要键盘访问的命令使用 GPUI 焦点系统和 `Action`。遵循熟悉的平台快捷键，在菜单或工具提示中展示，并在浮层打开或关闭后把焦点放到合理位置。

选中状态是信息模型的一部分，不是可选润色。标签页、分段选择、可选行、筛选项与导航入口必须持续显示选中状态。拥有下拉菜单的按钮在弹出层关闭前必须保持按下或打开外观；悬停无法说明触发按钮与弹出层之间的关系。

破坏性操作要区分可逆与不可逆。可逆变更优先 undo 或临时 notification；严重且不可撤销时才使用 `AlertDialog`，确认文案必须写出具体对象和后果。

### 指针约定

Button、Checkbox、MenuItem、Tab 等原生控件使用默认箭头；link 使用手形。文本、resize、grab、prohibited cursor 只在确实描述当前操作时使用。Cursor 只是强化 affordance，不能替代可见状态与 accessibility role。

Hover 应克制，因为键盘和 accessibility 操作没有 hover。破坏性或关键操作不能只有 hover 时才存在。行内 action 可以在 rest 时更安静，但 selection、键盘或 context menu 必须提供同一命令。

### 优先使用桌面命令入口，而不是悬停工具栏

根据命令的频率和作用范围决定入口：

- 主要或高频操作使用带文字的按钮或熟悉的工具栏控件，并保持可见；
- 当前区域的次要操作放入具有可见触发按钮的 `DropdownMenu`；
- 作用于指针下对象的命令放入 `ContextMenu`；
- 有自然键盘形式的重要命令同时提供 `Action` 与快捷键；
- 悬停时出现的图标只能是快捷入口，同一命令必须在其他位置仍可到达。

这不只是视觉偏好。GPUI Component 的 menu system 已经拥有方向键导航、confirm/cancel、disabled item、separator、submenu、shortcut 展示、focus transfer/restore 和 nested menu dismiss。自定义 hover button strip 必须重新实现这些行为，而且 keyboard-only 与许多 assistive technology workflow 根本看不到它。

用户需要明确知道“这里还有更多命令”时使用 `DropdownMenu`，例如 toolbar overflow、document action、account menu。命令只作用于当前 selection 或 pointer 下对象时使用`ContextMenu`，例如 rename、duplicate、reveal、remove。Context menu 不能是 essential command 的唯一入口；按场景同时提供 menu bar、toolbar、keyboard 或 detail view 路径。

不要为了视觉极简把所有 action 都藏入 menu。Discovery 与速度同样重要：主要 action 保持可见；danger item 有清楚 label 并与普通命令分隔；同一 command 在所有入口使用一致 verb、icon、shortcut、enabled state 与执行结果。

### 按钮表示应用操作，链接只表示外部资源

改变应用状态、确认决策、打开工具、提交数据或执行命令时使用按钮，并根据局部层级选择表现：

- 当前决策的默认提交使用主按钮（`primary`）；
- 普通可见操作使用默认按钮；
- 需要清楚边界但强调较低时使用描边按钮（`outline`）；
- 工具栏或行内的熟悉低强调操作使用幽灵按钮（`ghost`）；
- 只有符号广为人知时才使用图标按钮，并提供无障碍名称与工具提示。

不能因为按钮是界面中唯一操作、位于右上角，或团队希望获得更多点击就使用主按钮。主按钮表达默认提交及其键盘行为。添加项目、打开工具、刷新视图等普通命令，应根据局部层级使用默认按钮、描边按钮或幽灵按钮。

带下划线的 Link 只用于外部 resource target：URL、网页、在线文档或 Email 地址。它使用手形 cursor，因为语义是离开当前 application context 并访问该 resource。不能为了让功能 command 看起来安静而套 Link 样式。Link 形态的 Delete、Save、Refresh、Add、Open menu 或应用内跳转会隐藏 control affordance，并向 accessibility 暴露错误 role。

“查看”不会让 app 内 destination 变成 Link。完整报告、分析、detail panel 或 local record 仍应通过 Button、row、card、tab 或 disclosure control 打开。当 card 已说明会打开什么时，可以使用“完整分析”这类依赖上下文的短 label；下划线只留给真正交给 browser 或 mail client 的 resource。

所有应用内导航——sidebar row、tab、breadcrumb、list item、打开本地 view、切换 workspace——必须使用对应原生 component 或 Button/Action。视觉强调通过 Button variant 或 navigation component 的 selected state 决定，不能通过伪装语义实现。

## 反馈与浮层

选择足以承载当前决策的最小 surface：

- tooltip：短解释或快捷键；
- popover：不中断任务的上下文控件；
- menu：紧凑 action 列表；
- notification：无需用户决策的异步状态；
- dialog：聚焦决策或短表单；
- alert dialog：有后果操作的明确确认；
- sheet：需要更多持续空间的辅助工作。

Alert 即使不是 modal，也会打断视觉 hierarchy。它只用于当前任务中需要立即注意的重要异常信息，不能作为普通 description、tip 或空白内容的装饰 container。不需要立即注意或 action 时，使用 inline help、muted text 或普通 section。

避免 overlay 叠 overlay。Escape 关闭最上层可关闭 surface，焦点回到 trigger 或下一个逻辑目标。

Overlay action 必须指向 overlay 实际展示的 object 或 state。例如只有独立 recent-history section 可见且存在 entry 时，才显示“清除历史”。Search result、recent item 与 favorite 是不同 collection，应明确 label 和分区，不能混成一个没有解释的 list。不适用的 action 应隐藏，或 disabled 并说明原因；不能在 footer 塞一个作用不明的 trash icon。

Footer 不是无处安放 capability 的收纳区。它可以显示适用于整个 surface 的 shortcut、status 或 action，但每项都必须回答：object 是什么、为什么现在可用、影响什么 scope、执行后哪个 visible state 会改变。

## 动效

动效用于解释变化，不是环境装饰。出现、关闭、展开和空间连续性使用短 transition。如果 opacity 或 transform 已能表达关系，就不要动画大面积 layout。遵守 reduced motion，状态理解不能依赖动画，也不要给所有组件安装默认动画。

Motion policy 属于 styled/application 层。Base 可以拥有 transition 所需的生命周期机制和 geometry，但不决定所有产品都 fade 或 slide。独立动画值使用稳定 ID；被打断时从当前采样值平滑反向，而不是回到旧端点重新开始。

## 数据密集界面

Dense 不等于拥挤。在 table、tree、command palette、editor 与 dock 中：

- header 和行主要标识保持稳定；
- 可比较值对齐，必要时使用 tabular number；
- 区分 focus、hover、active row 与 multi-selection；
- sorting/filtering 可见且可逆；
- filter/reorder 后按 domain ID 保持 selection；
- virtualize 大集合，但不改变键盘语义；
- 次要 column 和 inspector 渐进披露；
- empty state 解释下一步。

一致字段的比较使用 table；异质内容扫描使用 list；真实层级使用 tree；只有用户需要安排长期工具或文档时才使用 dock。不要把复杂数据组件当作一种视觉样式。

## 界面用词

文字是界面架构的一部分。一个功能中的入口、对象、命令、状态和结果应作为整体设计，不能按照实现逐项翻译。默认使用在当前上下文中仍然准确的最短表达。

### 让上下文承担上下文

不要重复界面已经表达的信息。侧栏入口通常只需写对象或领域：使用 `Users`，而不是`User Management`；使用“快捷键”，而不是“快捷键配置管理”。如果表格内容本身都是操作，可以省略泛化的“操作”列名。对话框标题已经是“删除‘路线图’？”时，正文不必再次提问。

这是上下文经济，不是为了短而删。文字能够改变决策时必须保留：受影响范围、不可逆后果、异常前提或恢复方式。每一个额外词都应回答当前布局尚未回答的问题。

入口与对象使用名词（“用户”“外观”“订单”），命令使用动词（“保存”“复制”“导出”），状态使用形容词或短语（“离线”“已是最新版本”“待审核”）。除非能够区分真实领域概念，否则避免“管理”“模块”“页面”“功能”“操作”“系统”等包装词。

### 分别写作，而不是翻译句形

先统一意图、层级与术语，再按每种语言的自然表达分别写作。不要保留源语言的语序、词数、客套填充或词性。中文概念直译可能是 `User Management`，自然英文却是 `Users`；忠实是保留用途，不是保留字面形式。

信息架构已经表达的词应删除。在 `Settings` 中，入口通常只写 `Account`，不用重复`Account Settings`，更不能写不自然的单数 `Account Setting`。正确英文来自控件的角色和相邻文字，而不是脱离上下文的中文短语。

为重复出现的对象、命令与状态维护一份小型产品词表。工具栏、菜单、右键菜单、对话框、快捷键搜索与文档对同一概念使用同一个词，除非上下文确实改变了含义。文案必须放回真实界面评审；只看翻译文件，往往发现不了相邻文字的重复和作用范围不一致。

技术写作不追求纯中文。已经稳定的 UI framework 术语，如果翻译后不够准确，可以保留英文；API 标识符保持原名并使用代码格式。普通叙述不能仅为显得专业而夹杂英文。术语首次出现时按需说明含义，此后在界面、文档和 API 示例中保持同一种写法。

### 按钮与确认对话框

按钮默认简短，通常一至两个词，并说明结果，而不是手势或控件。使用“保存”“移动”“删除”，不用“点击进行保存”“执行移动操作”“确认删除操作”。不提交并离开的操作统一使用“取消”。`OK` 只用于确认已经读到纯信息。

简短是默认值，不是机械字数限制。当额外文字能够揭示后果，或区分容易混淆的选择时，应有意使用更长但仍可扫描的文字，例如“仅从此分组删除”与“从所有位置删除”，或“不保存并重新启动”。长度必须换来决策所需的信息，不能重复标题或正文。

能够准确概括结果时，确认按钮使用最具体的短词：

| 上下文 | 较弱 | 推荐 |
| --- | --- | --- |
| 删除对话框 | “是”“Sure”“确认删除” | “删除” |
| 未保存修改 | “确认”“是” | “放弃修改” |
| 纯信息确认 | “确认操作” | “知道了”或 `OK` |
| 无法用一个准确动词概括的复杂承诺 | “是” | “确认” |

当对话框已完整说明复杂承诺，而不存在准确的结果动词时，“确认”是合理的后备用词；它不应替代本来清楚的命令。`Sure` 是口语回应，不是稳定的英文命令，含义也不足以进入标准词表。

确认对话框应组成一个紧凑决策：

- 标题写决策或条件，例如“删除‘路线图’？”；
- 正文只补充新的作用范围、后果或恢复方式；
- 操作使用“取消”和结果词，例如“删除”；
- 破坏性样式标记破坏性结果，但不能代替准确用词。

能够说明实际情况时，不使用“提示”“警告”“错误”“确认”等泛化标题。避免“您确定要……吗”“是否需要……”“请注意……”以及状态已经表达清楚时的“成功”等套话。礼貌来自冷静、尊重的语气，不来自重复的“请”。

### 大小写、标点与符号

英文 UI 默认使用 sentence case：`Reset layout`，不用 `Reset Layout` 或 `RESET LAYOUT`。专有名词与约定俗成的缩写保持原样。只有原生命令菜单等确实受益于平台惯例时才使用 title case，并在同一类控件中保持一致。

全大写可以作为克制的排版强调，适合极短的分组标签、眉题、状态，以及既有缩写或代码。它通过紧凑的字形和适当字距形成接近加粗的层级，但不应用于按钮、长标题、完整句子或密集列表。同一区域不要同时用全大写、强色和粗体争夺注意。不要自动转换所有字符串；产品名、缩写与本地化内容需要保留正确大小写。

标签、按钮、菜单项、标签页、标题、占位文字与短状态末尾不加句号；完整的说明、警告和错误句子使用完整标点。日常成功或失败消息避免感叹号。中文句子使用全角标点，短控件文字同样按语义省略句末标点。

使用单个省略号字符（`…`），不用三个句点。凡按钮或菜单项会打开对话框、sheet、独立窗口，或命令完成前还需要用户输入或选择，文字末尾都加省略号，例如 `Settings…`、“导出…”。立即执行的命令不加。正在进行的任务使用不确定进度指示器，不用装饰性的点号表达。

错误说明发生了什么，并在有帮助时给出下一步恢复方式。成功反馈只在结果状态尚不可见时出现。使用“无法保存。请检查网络连接后重试。”，不要只显示技术代码或长篇道歉；文档已经明显进入保存状态时，不再弹出“保存成功”。

## 国际化与平台适配

文案必须承受扩展、CJK 排版和不同快捷键写法。不要按一条英文 label 固定控件宽度；不要把文字放进 raster asset；不要拼接翻译片段；只有存在 tooltip 等恢复路径时才截断 label。

尊重有意义的平台差异：Command/Control、原生窗口装饰、系统 appearance、scrollbar、menu 和 notification 能力。各平台的信息架构应稳定，但不能为了表面像素一致而消除用户熟悉的平台行为。

## AI 生成界面的规则

AI 修改 GPUI 界面前必须阅读相邻 feature、theme token 和组件文档，并先说清主要任务、state owner、component composition 和 keyboard path。不能从 React/Shadcn 示例推断 API，也不能因为方法名“听起来合理”就发明 GPUI 方法。

AI 输出只有在人能够解释 hierarchy、density、component choice 与所有例外字面量时才算完成。看起来合理的截图不是证据；键盘、焦点、动态内容、主题、resize 和失败状态都属于设计。

## 无障碍检查表

界面完成前验证：

- 所有 action 都能通过键盘到达和执行；
- focus 顺序符合视觉与任务顺序；
- focus 始终可见，overlay 关闭后正确恢复；
- control 有名称，仅图标 control 有 tooltip；
- 文字与有意义边界对比度充足；
- 状态不只依赖颜色；
- disabled 与 read-only 可区分；
- label、error、description 靠近对应 control；
- 长翻译和更大字体下仍可用；
- 即使紧凑布局，pointer target 仍舒适。

## 设计评审清单

评审不是清点部件，而是判断界面是否做出了正确取舍。依次回答：

1. **任务清楚吗？** 新用户能否直接看懂界面用途、主要操作和下一步，而不必学习、猜测或试错？
2. **操作兑现承诺吗？** 文案、控件、状态、范围、反馈与结果是否始终描述同一件事？
3. **层级明确而克制吗？** 核心功能是否获得应有的空间，同时主按钮、强色、粗体、徽标和警示保持稀缺？
4. **还能做得更少吗？** 能否删除、合并或推迟某个入口、选项或状态，同时完整保留主要任务？
5. **结构准确吗？** 同层内容是否共享对齐轴，相等间距是否精确到渲染像素，滚动条是否贴住实际滚动区域的边缘？
6. **遵守组件体系吗？** 标准控件是否保留统一的几何、状态、键盘与关闭行为，外观是否来自主题与尺度令牌？
7. **所有状态与约束下都可用吗？** 检查键盘与焦点、空白、加载、失败、无权限、长翻译、缩放、最小窗口和减少动态效果。
8. **在真实窗口中验证了吗？** 使用真实组件、文案和代表性内容亲手完成任务，而不只评审一张理想截图。

继续阅读[编码指南](./coding-guides.md)，把这些设计决策落实为 GPUI 架构和代码。

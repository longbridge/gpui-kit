---
title: Coding Guides
description: 构建可维护 GPUI Kit 应用的架构、代码风格与命名规范
order: -13
---

# 编码指南

本指南总结 GPUI Kit 中经过长期实践验证的应用架构与代码模式，面向工程师和 coding agent。请先阅读[设计指南](./design-guides.md)：代码结构的职责是保存产品意图，而不是替代产品设计。

本文是一份规范性指南：**必须**表示生命周期、正确性或生态约束；**应该**表示默认架构，偏离时需要有具体理由。精确方法签名以当前源码和 API 文档为准。

### 让常见用法自然低成本

**设计原则：框架类型与所有权安排，应让组件的常见用法默认就低成本。** 开发者应能构建、复用 UI，而不必在每个调用处手写缓存。持久 UI 文本使用 [SharedString](./shared-string)、轻量组件值使用 [RenderOnce](./render-once)，都是这种设计的例子：API 把所有权放在恰当位置，使常见路径避免重复工作。微小成本仍会随元素数和渲染次数放大；只有测到特殊热点后才添加缓存。这两种类型都不承诺零成本。具体机制见下文的[文本所有权规范](#用-sharedstring-保存持久化-ui-文本)与渲染章节。

## 架构总览

<img class="architecture-light" src="/application-layers-light.svg?v=20260822-16" alt="GPUI 应用架构层次">
<img class="architecture-dark" src="/application-layers-dark.svg?v=20260822-16" alt="GPUI 应用架构层次">

依赖只向下。上层负责领域语义与流程协调，下层负责可复用的表现和行为。通用 component 不能依赖具体应用页面；`gpui-base` 不能依赖 GPUI Component 主题。

边界定义如下：

- **app shell：** 组合窗口与 Feature Crate，不承载具体 Feature 逻辑；
- **feature crate：** 在一个公开边界内组织同一业务能力的 model、service、view、command、dialog 与 workflow；
- **app component：** 跨 Feature 复用且带有领域语义的模式；
- **gpui-component：** 带主题的通用 UI；
- **gpui-base：** 不带产品表现的可复用行为与 geometry。

### 大型应用按业务能力组织 crate

在大型 Rust 应用中，当复杂业务能力的状态、生命周期与公开边界值得独立维护时，通常将它拆成 feature crate。把它的 model [Entity](./entity)、service、[`Render`](./render) View、command、dialog 与 workflow 放在一起。编辑 Workspace 的 dialog 属于 Workspace Feature；只有可复用的 Dialog 基础组件属于 UI library。小能力可以先留在 module 中，等 crate 边界有具体收益再拆分。

```text
crates/
├── app/
│   └── src/main.rs             # 组合窗口与 Feature
├── workspace/
│   └── src/
│       ├── lib.rs              # Feature 的公开边界
│       ├── model.rs
│       ├── commands.rs
│       ├── workspace_view.rs
│       └── rename_dialog.rs
├── search/
│   └── src/
│       ├── lib.rs
│       ├── model.rs
│       ├── commands.rs
│       ├── search_view.rs
│       └── filters.rs
├── settings/
│   └── src/
│       ├── lib.rs
│       ├── model.rs
│       ├── settings_view.rs
│       └── account_dialog.rs
└── shared/
    └── src/
        ├── lib.rs
        └── recent_items.rs     # 多个 Feature 共同使用的稳定能力
```

不要反过来建立全局 `models/`、`views/`、`modals/` 与 `commands/` 目录。这种方式只是按实现角色给文件分类，却会把每个 Feature 拆散到整个应用中。

App Shell 只组合 Feature Crate，尽量不承载 Feature 逻辑。每个 Feature 拥有自己的 model Entity 和渲染它们的 View。只有确实属于整个应用的 service 或 registry 才在 Feature 内使用私有 [`Global`](./global)；仅供一个 Feature 使用的 model 不应自动成为 Global。同一 Feature 内的协作者传递 clone 后的 `Entity<T>` handle，避免反复复制大型 `Vec` 或 collection。跨 Feature 边界只暴露有意公开的 Entity 类型，或小型 handle、领域 ID、command、event、interface；model 字段和实现 module 保持私有。

例如 Workspace crate 可以导出可 clone 的 handle，把可变 model 留在公开 API 之后：

```rust
// 简化的 workspace/src/lib.rs；model 变大后可移到私有的 model.rs。
use gpui_kit::{App, Entity, SharedString};

struct WorkspaceModel {
    name: SharedString,
}

#[derive(Clone)]
pub struct WorkspaceHandle {
    model: Entity<WorkspaceModel>,
}

impl WorkspaceHandle {
    pub fn rename(&self, name: SharedString, cx: &mut App) {
        self.model.update(cx, |model, cx| {
            model.name = name;
            cx.notify();
        });
    }
}
```

应用可以保存这个 handle，并把 Workspace ID 传给 Search；Search 无须拿到 Workspace 的整个 collection 或私有 View。一个简单的无环依赖关系是：

```text
app ──▶ workspace ──▶ shared contracts
 └────▶ search ─────▶ shared contracts
workspace, search ──▶ gpui-kit
```

Feature 可以依赖稳定的共享能力和 UI 基础设施，但不能反向依赖 App Shell，也不能访问另一个 Feature 的私有 module。如果一个 Feature 必须调用另一个 Feature，应让依赖显式且单向，或把真正共享的契约放在两者下层。共享 crate 要有清晰用途和真实 owner，避免循环依赖与无所不包的 `shared` crate。

这些边界让不同工程师或多个 AI agent 可以并行处理不同能力，减少文件冲突与共享状态耦合；共享契约仍需要协调。Cargo 也可能复用已缓存的依赖，在局部变更后只重编译较小的依赖子图，但增量编译速度取决于依赖图、公开 API 改动、feature 与构建配置；增加 crate 自身也有成本，不能保证每次修改都编译更快。按所有权和协作需要拆分，不要为每个页面或 helper 建 crate。

## 初始化与 Root 所有权

在创建组件 view 前只初始化一次 GPUI Component，并让每个窗口的第一层是 `Root`：

```rust
app.run(move |cx| {
    gpui_kit::init(cx);

    gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
        cx.new(|cx| Workspace::new(window, cx))
    })
    .expect("failed to open window");
});
```

`Root` 不只是容器。它协调 dialog、sheet、notification、tooltip/menu layer、modal focus restore、focus trap 与窗口级文本选择。一个窗口只能有一个 `Root`；绕过它的 UI 可能静止时正常，却会在 overlay 嵌套或快速切换 focus 时失效。

## 理解 GPUI 阶段与上下文

GPUI 使用 retained state 与 declarative rendering。[Entity](./entity) 跨 frame 存活；`render` 返回的 [Element](./element) tree 只描述当前 frame。必须始终区分持久状态与一次 render 的输出。[Context](./context) 解释 `Context<T>`、`App` 与 `Window` 的作用范围。

- `Context<Self>`：修改当前 entity，建立 listener，emit event，并通知 observer；
- `App`：访问 global 以及读取或更新 entity，不表示当前 element 拥有这些状态；
- `Window`：拥有窗口 focus、Action dispatch、input、element keyed state、measurement 与 animation-frame request；
- layout、prepaint、paint 属于后续 phase，只有需要 resolved geometry 时才使用相应 hook。

不能把 `&mut Window`、`&mut App`、`&mut Context<_>` 保存到当前调用之外。应保存`Entity`、`WeakEntity`、`FocusHandle`、scroll handle 或领域 ID 等 typed handle。

## 选择正确的组成单元

### 值类型 UI 使用 `RenderOnce`

**默认优先用 [`RenderOnce`](./render-once) 构造轻量、可复用的组件值。** caller 传入当前 props 和 handler，GPUI 消费这个值生成元素，此后该值即可丢弃。GPUI Kit 的 `Button` 和 `Checkbox` 就遵循这一模式：显示的 label、checked 或 selected 值与 callback 来自 caller 当前持有的状态。`RenderOnce` 组件仍可使用少量带 key 的交互状态，例如 focus handle；它没有独立的 View 生命周期。不要因此认为它的 render 方法在每个显示帧都必定调用：显示一帧并不一定需要重建这个组件的元素树。

```rust
#[derive(IntoElement)]
struct EmptyState {
    title: SharedString,
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .items_center()
            .text_color(cx.theme().muted_foreground)
            .child(self.title)
    }
}
```

### 跨 frame 行为使用 `Entity<T>`

组件需要持久的复杂状态或独立生命周期，例如 subscription、异步工作、history、子 Entity 或增量更新时，使用由 Entity 支撑的 [`Render`](./render) View。Entity 存在 owning view 中，不能在每次 `render` 里重建。

```rust
struct SearchView {
    query: Entity<InputState>,
}

impl SearchView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
        Self { query }
    }
}
```

不要把每个视觉 fragment 都做成 Entity；生命周期和协调有成本，只有 retained identity 确实重要时才建立 entity boundary。

### 元素、视图与行为系统各有职责

生态中的 public unit 并非一种模板：

- Button、Checkbox、Link、Tabs 等 semantic element；
- Dialog、Popover、Select、Combobox 等 compound behavior root；
- Input、Table、Tree、Dock、notification 等 entity-backed system；
- positioning、virtualization、scroll、focus trap、motion、history、measurement 等 infrastructure。

Element 内部可以很复杂，但对 caller 仍是值。Stateful system 可以通过 render callback 让应用拥有表现，同时不重做行为。Public seam 应由行为决定，而不是 renderer 中有多少个 `div` 决定。

## 状态所有权

### 用 `SharedString` 保存持久化 UI 文本

**跨 frame 或 callback 闭包保存的 UI 文本必须优先使用 [`SharedString`](./shared-string)**，包括组件的 label、title 和 placeholder。把稳定文本存在所属 View 或 Entity 中，构建元素时 clone 这个 handle；不要在每次 render 时反复 clone `String`。可变编辑缓冲、格式化构建阶段使用 `String`，完成后再转换；单次调用内临时只读则使用 `&str`。

API 或 JSON 响应中若有供 UI 长期持有的不可变文本，应在反序列化边界就把响应字段定义为 `SharedString`。保留收到的 transport 值；派生或格式化后的展示文本放在单独的领域或 View model 中，不要改写响应字段，也不要反复把 `String` clone 进 UI。真正可变的缓冲区或仅供非 UI 短期解析的文本不受此规则约束。见 [API 响应示例](./shared-string#从-api-响应进入-view)。

这是所有权与性能规范，并不表示文本总是零分配。短值可以内联；较长的动态文本使用共享存储，clone 不复制文本字节。但首次构造仍可能分配，堆上文本的 clone 也有引用计数成本。具体存储方式与取舍见 [SharedString](./shared-string)。

状态应放在能够保持其正确性的最窄 owner 中：

- domain state 属于 model 或 feature view；
- transient view state 属于绘制它的 view；
- reusable behavior state 属于为该行为设计的 component state；
- 少量 element-local state 可以使用 GPUI keyed state；
- 共享 application service 可以使用 GPUI global。

普通 selection/toggle 优先采用 controlled value：传入当前值，接收 requested change，由 owner 更新，再 render。Callback 表达 intent，不能建立第二份隐藏真相。

```rust
Checkbox::new("show-hidden")
    .checked(self.show_hidden)
    .label("Show hidden files")
    .on_click(cx.listener(|this, checked, _, cx| {
        this.show_hidden = *checked;
        cx.notify();
    }))
```

改变渲染结果后调用 `cx.notify()`；owner 需要处理语义事件时使用 `cx.emit(...)`；生命周期跟随 Entity 时使用 `cx.subscribe(...)` / `cx.observe(...)`。返回的 Subscription 必须有明确的生命周期 owner。

### 限定所有权关系与订阅生命周期

父级用强 `Entity<T>` handle 持有子级时，子级回指父级必须使用 `WeakEntity<T>`；强回指会闭合所有权环。`WeakEntity::upgrade()` 返回 `Option`，其 `update` 返回 `Result`，所以父级已释放应作为正常生命周期结果处理。所有权和弱引用访问模式见 [Entity](./entity)。

把返回的 `Subscription` 保存在需要 callback 的 View 或 Entity 上，通常是 `_subscriptions: Vec<Subscription>`。owner 释放时 handle 随之 drop，订阅也会取消。仅存于初始化函数的局部 handle 会过早 drop；`Subscription::detach()` 则会放弃取消订阅的 handle，让 callback 持续到被订阅的 Entity 释放。默认不要 detach 随 View 存续的重复订阅；在长寿命 emitter 上反复 detach 可能使 callback 持续累积。GPUI 的 `observe` 和 `subscribe` 内部使用订阅者的弱 handle，但 callback 如果额外捕获了自己 View 的强 handle，仍可能闭合所有权环。

View 意外无法释放时，先检查强 Entity 环、捕获强 View handle 的 callback，以及生命周期超出预期的已 detach 订阅或 [Task](./task)。drop Task handle 会取消尚未完成的工作，本身不会继续保留 View。这些是优先排查点，并非所有内存泄漏原因。

读取或派生值不能触发 notify。禁止在 `render` 中无条件 notify，否则会永久重绘。形成同一 invariant 的字段应一次更新，只 notify 一次。无法获得 context 的 reusable state API 必须明确让 owner 负责 emit/notify。

### 防止状态反馈环

文本输入、selection、filter 和 controlled popup 常有两条路径：外部 owner 设置值，以及用户请求新值。同步外部值时不能再次通过 user callback 回传。使用 origin/revision 或 coherent snapshot 比较，确保一次逻辑变化只报告一次。Callback 可能同步关闭、替换或更新当前组件时，调用路径必须可重入。

## 稳定标识

[`ElementId`](./element_id) 是行为契约的一部分。它为元素提供稳定标识，并作为元素局部状态或组件状态的键。组件也可以将它用于自身的焦点、测量或动画标识；焦点与滚动通常仍由各自的 handle 管理。

- row、tab、tree node、重复 control 使用稳定 domain ID；
- 同一 control 重复出现时，以 owning object namespace child ID；
- 可插入/重排的数据不能使用翻译 label 或 mutable index 作为 ID；
- `render` 中不能生成新 random ID。

```rust
Button::new(("delete-project", project.id))
    .danger()
    .label("Delete")
```

ID 改变表示 UI identity 改变，其状态 reset 必须是有意行为。Transition channel、overlay token、scroll handle 和 persistence ID 也遵循相同规则：共享 key 会相互覆盖，每 frame 换 key 则永远无法累积状态。

## 渲染与组合

`render` 保持 declarative：读取当前 state、派生 presentation value、组合 element。Domain operation、parsing 与复杂 mutation 放入具名 method 或 service。

```rust
impl Render for ProjectView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(self.render_content(cx))
    }
}
```

当 helper 能命名有意义区域并减少读者同时记忆的状态时，提取 `render_*`。只有该区域有可复用契约或 retained lifecycle 时才提取新 component；builder chain 很长本身不是理由。

一致使用 GPUI Component fluent trait（`Sizable`、`Disableable`、`Selectable` 和 component builder）。小的 conditional refinement 使用 `.when(...)` / `.when_some(...)`；分支代表不同 interface 时使用普通 Rust control flow。

构造 custom surface 前先使用标准 semantic component。不能为了匹配一张截图就用 generic `div` 重做 menu、select、dropdown 或 command palette。复用 component 才能保留 item geometry、focus transfer、keyboard navigation、selection、disabled state、dismissal 与 accessibility contract。如果标准 component 无法表达可重复的合理 pattern，应改进其 explicit API，而不是在每个 call site styling arbitrary descendant。

应用提供的 render callback 必须无副作用。List item renderer、menu builder、dock panel renderer 可能因 measurement 或 redraw 被多次调用，不能执行业务操作、追加数据或注册无界 subscription。

## 行为与表现的边界

Base 的长期规则是：

> Base 拥有可复用行为，以及实现行为所必需的 geometry；表现层拥有产品视觉语言。

“Headless”不表示“只有一个空 div”。Popup collision、keyboard navigation、editing、virtualization、resize arithmetic、focus trap 和 dock reconciliation 需要内部结构与状态。把它们推给 caller 不是灵活，而是复制脆弱行为。

反过来，Base 不能选择品牌色、字体、密度、最终图标、variant 或应用 composition。表现能力通过 `Styled`、typed semantic-state style、explicit part、child slot 和 item renderer 暴露。不能遍历任意后代去猜 title/description/close button，必须提供语义 part。

## 主题与样式

从 active theme 读取 semantic value，使用 GPUI 的 [`Styled` method](./style) 布局：

```rust
div()
    .bg(cx.theme().background)
    .text_color(cx.theme().foreground)
    .border_1()
    .border_color(cx.theme().border)
    .rounded(cx.theme().radius)
```

规则：

- 不写死 product color、radius、spacing 或 control geometry；
- application code 不得引入 raw hex、`rgb`/`rgba` 或 `hsla`；颜色从 `cx.theme()`读取，缺少语义 role 时补入 product theme；
- application layout 使用 GPUI rem-based helper（`p_2()`、`gap_3()`、`w_64()`、`text_sm()`），而不是直接 `px(...)`；
- token 按语义而不是 palette 位置使用；
- state-independent geometry 放普通 builder chain；
- runtime hover/active/focus/focus-visible 使用 GPUI modifier；
- checked/selected/pressed/disabled 使用组件 semantic state style；
- popup ownership 使用 explicit state，使 trigger 在 dismiss 前持续渲染 open/pressed 外观；
- disabled control 不应响应时，guard hover/active refinement；
- variant 保持少而有意义，不为每个 call site 增加一个 variant。
- primary style 绑定决策区域真正的 default commit 与 Enter action，不能从 action 数量、频率或 toolbar 位置推导。
- Badge/Alert variant 保持 semantic 且稀缺。普通 metadata 使用 neutral；不能仅因为 variant 存在，就把每个 enum case 或 section 映射成不同颜色。

有效 precedence 为：instance style → active semantic state → disabled → GPUI runtime interaction refinement。后一层只替换自己设置的字段。

新的 application-owned presentation 优先使用 `Theme::semantic_tokens()`：它提供通用 color role、radius、spacing、typography、shadow scale，并有意避免 component-specific 名称。Legacy token 保留兼容性，但不应继续成为每个 application widget 的 extension point。

当前有一个必须明确的 ownership 边界：`Theme::spacing_tokens()` 投射固定默认 scale，`Theme::apply_semantic_tokens(...)` 不保存 custom spacing/elevation scale。应用如需自定义，必须自行持有 `SemanticThemeTokens`（或更窄的 design-system state）并提供给 component。不能把 custom spacing snapshot 写入 global theme 后，期待下次`cx.theme().semantic_tokens()` 仍返回它。

通过 `Theme::update(cx, |theme| ...)` 修改 GPUI Component 的 global theme。theme 里同一份颜色存了两次（`colors` 是纯色，`tokens` 是可带渐变的可绘制背景），Base 层还持有一份给 scrollbar 与 resize handle 的 projection；`update` 在闭包结束后把三份重新对齐并刷新所有窗口。通过 `Theme::global_mut(cx)` 修改只会改到你碰的那个字段——侧栏可能用新 colors 画文字、用旧 tokens 画背景——剩下的要自己做：从 `colors` 重新推导 `tokens`、调用 `Theme::sync_base(cx)`、刷新窗口。完整的 `Theme::change(...)` 会自行同步。

向外绘制的 focus ring 需要空间，ancestor `overflow_hidden()` 会裁掉它。优先让布局留出空间；产品确实需要大量 clipping 时，通过 theme focus-ring policy 保留 focused border，不能悄悄消除键盘焦点。

### 基础字号控制应用缩放

`Root::render` 会调用 `window.set_rem_size(cx.theme().font_size)`。因此 theme base font 不只是 body typography，也是 application rem-based design scale 的 reference length。这有意沿用 [Tailwind CSS 的主题模型](https://tailwindcss.com/docs/theme)中有价值的部分：具名 type、spacing、size step 共享一个 relative base，而不是变成互不相关的 pixel constant。GPUI 方法名称与布局行为见 [Style](./style)。

通过更新 base font 并 refresh window 改变 zoom：

```rust
Theme::update(cx, |theme| theme.font_size = px(18.));
```

Base font 自身是 px，因为它负责锚定 scale。Descendant application UI 通常使用`text_sm()`、`gap_2()`、`px_3()`、`h_8()`、`size_4()` 等 relative helper，让 type、whitespace、control 与 icon 一起响应。Custom component 如果把 rem-based text 与 fixed-px padding/icon geometry 混合，必须记录为什么该部分不应 zoom。

Application UI 中每个直接 `px(...)` 和 raw color constructor 都应视为 review finding。只有 documented physical/platform boundary、measured runtime geometry、raster/data color 或 theme/token definition 本身可以例外。方便或“匹配截图”都不是有效理由。

从 resolved layout 得到的 cache 必须把 `window.rem_size()` 纳入 invalidation key，或者依赖随它变化的 revision。包括 wrapped row height、text shaping/layout、virtual-list measurement、popup/dialog geometry、由 text 推导的 icon size 和 custom canvas metric。Command variable-height row 是生态中的现有案例：较大 base font 会让同一 fixed width 产生不同 wrapping，因此 rem 变化时会重新 measure。

不要把 application zoom 与 Dock panel zoom 混淆。Dock zoom 是 stateful layout operation：让一个 tab group 保留 container chrome 并填满 DockArea，同时保留退出路径；它不能修改 window rem size。

## 事件、Action 与焦点

Pointer-specific 行为使用 pointer callback；需要 key binding、menu 或多输入来源的 command 使用 GPUI [Action](./action)。Action handler 靠近拥有 command 的 view。

一个 logical desktop command 只建模一次。Toolbar Button、`DropdownMenu` item、`ContextMenu` item、menu-bar item 与 key binding 应 dispatch 同一个 Action 或调用同一 owner method，不能复制五份 mutation。条件允许时，label、icon、shortcut、enabled state 来自同一 command policy，避免不同入口互相矛盾。Menu 拥有 navigation 与 dismiss；feature owner 仍然拥有 command 是否允许以及实际执行内容。

控件选择必须符合语义。命令即使需要降低强调，也应使用 `Button` 的 `outline`、`ghost` 或图标形式，不能换成 `Link`。GPUI Kit 应用约定：`Link` 只用于交给浏览器或邮件客户端打开的 URL、网页文档和电子邮件地址；应用内目标使用相应的导航组件，命令使用 `Button` 或`Action`。这是产品设计约定，不是 `gpui_kit::base::Link` 的能力限制；后者可以通过 `open_with`把目标交给其他导航实现。

只有 nested interaction 确实必须阻止 parent 处理同一 event 时才 stop propagation。无差别阻止会破坏 menu、selection、drag 与 window command。

先绑键，再建菜单栏。`cx.set_menus` 在调用那一刻读取 keymap，把每个菜单项的快捷键固化进原生菜单；之后再注册的 binding 不会出现在菜单项旁，菜单项也不响应该按键。先 `cx.bind_keys`，再 `cx.set_menus`；keymap 之后又变了（用户 keymap 文件、切换语言重建菜单），就再调一次 `cx.set_menus`。

Focus owner 必须明确：

- 拥有 keyboard interaction 的 Entity 保存 `FocusHandle`；
- 在正确 focused region 注册 key context 与 Action；
- overlay 打开时转移 focus，关闭后恢复；
- 绘制清楚的 `focus_visible` state；
- 禁止在 `render` 中无条件 request focus。

被 track 的 handle 只有自己声明了才是 Tab stop：用 `cx.focus_handle().tab_stop(true)`（或 `.tab_index(n)`）建它，元素自身的 `tab_index`/`tab_stop` 不会作用到传给 `track_focus` 的 handle 上。Stateless component 可以在 `render` 里通过 `window.use_keyed_state(id, cx, |_, cx| cx.focus_handle().tab_stop(true))` 建这个 handle；keyed state 跨帧保留，Tab 顺序因此稳定——`Button` 就是这么做的。

`key_context` 与 `on_action` 应附着于同一个 focused region。注册了 Action 但没有正确 focus path，不算实现键盘交互。Composite widget 应完成整个 navigation model：方向键、适用时的 Home/End/Page、confirm、cancel 与 Tab，而不是几个孤立 shortcut。

Modal 必须 trap focus，并在关闭后恢复到仍有效的原目标。Nested overlay 从最上层关闭；快速 close/open 不能把 focus 恢复到正在关闭的中间 surface。

## 异步任务与副作用

异步 [Task](./task) 从 event、lifecycle hook 或具名 method 启动，不能作为 `render` 的无条件副作用。不应让 task 延长已关闭 view 生命周期时 capture weak entity。完成后通过正确 GPUI context 更新，并处理 entity/window 已不存在的情况；完成 coherent update 后只 notify 一次。

重复运行的任务应将 handle 保存在 owner 上，使 owner 释放时任务取消；默认不要 detach。确实必须独立完成的一次性任务可以调用 `Task::detach()`，但之后不再持有取消 handle。错误与弱 Entity 更新失败都需要明确处理。

用 idle/loading/loaded/failed 等明确状态表示 async operation。Refresh 时尽量保留可用旧数据；防止重复破坏性提交；可恢复错误要显示在 UI，而不是只写 log。

昂贵 parsing/computation 使用 background executor，Entity mutation 回到对应 application context。结果可能晚于 request、document、view 或 selection 的变化，必须绑定 revision/ID，拒绝 stale result。

## 布局、测量与滚动

`h_flex` 会让子元素在交叉轴上居中；`v_flex` 保持 flexbox 的默认值 `stretch`。
交叉轴居中正是一排控件想要的效果，所以图标加文字的一行什么都不用写。但一排等高的列不是这样：直接放进 `h_flex` 的列不会
填满行的高度，比行更高的列会被居中，于是它的顶部（通常是 header）被裁到窗口之外，而列附近的代码看不出原因。
子元素是列的一行，请显式写 `items_stretch()`：

```rust
h_flex()
    .items_stretch()
    .size_full()
    .child(sidebar)
    .child(content)
```

大多数 UI 使用 GPUI layout，不应自行 measurement。Measurement 是 popup、virtualization、editor、resize handle、chart 等依赖 resolved geometry 行为的深层工具。

- measurement 与 geometry 放在拥有行为的层；
- 只有普通 layout 无法表达关系时才在 prepaint 观察 bounds；
- prepaint 不能每 frame 修改无关 application state；
- measured data 带 frame/revision 范围；font、rem、width、theme、content 改变后可能 stale；
- popup flip 与 viewport clamp 等共享 geometry 必须集中实现，不能每个 overlay 各写一次。

Alignment invariant 应通过构造保证，而不是事后校正：sibling region 消费同一个 spacing token 或 shared inset，不能重复看似等价的 literal。关键重复 edge、column 与 gap 应增加 geometry assertion 或 visual regression coverage。验证不能只有默认 window；rem zoom 与 display scaling 会把 fractional coordinate 变成一个 physical pixel 的漂移，即使默认截图看起来整齐。

精度评审要测量 resolved result，但不能把测得的差值写成 raw `px(...)` 微调。应继续追踪到重复 padding、nested inset、border ownership、font metric 或 rounding，并修复结构 owner。

`h_flex()` 与 `v_flex()` 并不对称：`h_flex()` 会把 child 在 cross axis 上居中，`v_flex()` 则保持拉伸。因此 row 里的 column 拿到的是自身内容高度，而不是 row 的高度；内容比 row 高时会被居中，header 被挤出上边缘并裁掉。只要 child 拥有 header、footer 或需要按 row 高度解析的 scroll region，就给这个 column 加 `h_full()`，或给 row 加 `items_start()` / `items_stretch()`。

每个 scrollable region 只有一个 owner。Flex layout 中，可收缩 child 使用 `min_w_0()` /`min_h_0()`。Flex item 只有在自身 overflow 不是 visible 时才会放弃基于内容的 automatic minimum size，所以普通的可伸缩 child 在被明确要求之前不会围绕长内容收缩。`Scrollable` 已经为自己的 wrapper 处理了这一点，但夹在它与 flex container 之间的普通 `div` 仍需自行释放该最小值。避免意外 nested scroll；wheel input 应进入目标 axis，并在 API 不可移植时保留 platform/wasm 差异。

`Scrollable` 应附着在拥有完整面板、编辑器或窗口 viewport 的 element 上，使滚动条解析到区域边缘。内容内边距放在 scroll owner 内部，不能用带内边距的容器包住 scroll owner。滚动条悬在内容与面板边界中间，通常说明 scroll owner 错误或内边距放错了层。

## 列表、表格与大数据集

数据可能超过小型有界集合时使用 virtualization。Row identity 与 visible position 分开，不要每次 render clone 全量数据。Stateful list/table 拥有 navigation、selection、scroll coordination、visible range；item renderer 拥有 row presentation。

分离以下状态：

- source data 与 domain ID；
- filtering/sorting；
- selection；
- viewport/scroll；
- row rendering。

这样更新保持局部，也不会把 view tree 变成 data model。

Virtualization 是 behavior contract，不只是性能开关。Width、typography、rem、row content 变化时要 invalidate item measurement。即使多数 element 当前不存在，keyboard selection 与 scroll-to-item 仍必须在 model coordinate 上工作。

## 公共 API 设计

Reusable component 应遵守：

- constructor 建立合法 default；
- builder 消费并返回 `Self`，使用 domain 词汇；
- callback 描述 requested change；只有 pointer modifier/detail 有意义时才带 pointer event；
- 需要持续演进的行为接口使用私有字段、构造方法与读取方法；
- boolean reader 在同名 builder 存在时使用 `is_` / `has_`；
- reader 需要 plain field name 时，non-boolean setter 使用 `with_`；
- explicit compound part 优于检查 arbitrary descendant；
- reusable behavior 不能强制 product-level visual choice。

需要持续演进的行为状态默认使用私有字段。配置、主题令牌、几何数据和序列化结构可以有意暴露字段，但所有包含公开字段的 `pub struct` 必须标注 `#[non_exhaustive]`，并提供构造函数、`Default` 或 builder，避免调用方依赖穷举结构体字面量。这使后续增加字段无需破坏调用方。新类型及公开 API 调整必须遵守此规则；无关的已有类型另行迁移。

内部重组时保持 public module path：通过稳定 module seam 和明确 re-export，让 folder 变化不影响 downstream import。命名优先使用平台 control 术语和项目既有词汇，不使用偶然的 web-framework 词汇。

## 平台与能力边界

不能假设 native 与 web target 支持相同能力。Window decoration、accessibility bridge、system notification、clipboard、scroll gesture、font、timing 都可能不同。Platform-specific 代码放在窄 capability seam 后，并定义 fallback。

Platform branch 即使 presentation 不同，也必须保留 semantic contract。例如 system notification 的 retract 能力不同，application delivery state 仍要 coherent。尽可能分别测试 shared state machine 与 platform adapter。

## 文件、代码与命名风格

- View/entity 以产品概念命名：`ProjectList`、`ProjectEditor`、`SettingsState`；
- handler 以 intent 命名：`confirm_delete`、`open_project`、`on_query_changed`；
- 每个 module 一个主要职责；必须读无关行为才能理解 state/lifecycle 时就应拆分；
- component module、state、event 与 focused test 在共同变化时放在一起；
- comment 记录 invariant 和意外 lifecycle constraint，不复述显而易见 builder；
- 使用 `rustfmt` 并满足 workspace Clippy，不能用宽泛 `allow` 隐藏无关 warning。

### 词汇是 API 的一部分

同一概念在所有 component 中使用同一个词。命名新 method 前先搜索 GPUI、`gpui-base`与 GPUI Component；生态没有既有词时，参考 macOS/Windows control 术语。本地化文档保留准确的 API 标识符；稳定的 UI framework 术语在翻译会损失精度时也可保留英文。标识符使用代码格式，必要时在首次出现处解释。普通叙述不能为了显得专业而随意混用语言。

| 概念 | 命名模式 | 示例 |
| --- | --- | --- |
| 值类型 control | 名词 | `Button`, `Checkbox`, `Tab` |
| Retained behavior model | `<Control>State` | `InputState`, `TableState` |
| Imperative shared reference | `<Control>Handle` | `DialogHandle`, scroll handle |
| Semantic notification | `<Control>Event` | `TableEvent`, `SelectEvent` |
| Keyboard command | 动词或 intent noun | `Confirm`, `Cancel`, `SelectNext` |
| Pluggable owner | `<Role>Delegate` / `<Role>Provider` | `TableDelegate`, `CompletionProvider` |
| Caller 提供的表现 | `render_<part>` / `<part>_renderer` | `render_item` |
| Construction | `new` 或语义 constructor | `new`, `horizontal`, `vertical` |
| Fluent property | 名词或形容词 | `label`, `disabled`, `selected`, `placement` |
| 通用 non-boolean builder | `with_<field>` | `with_size`, `with_mode` |
| In-place mutation | `set_<field>` | `set_items`, `set_selected_index` |
| Boolean reader | `is_<形容词>` / `has_<名词>` | `is_open`, `is_closable`, `has_selection` |
| Plain value reader | field noun | `placement`, `selected_value` |
| Callback registration | `on_<event/intent>` | `on_click`, `on_open_change` |
| Named region renderer | `render_<region>` | `render_toolbar`, `render_content` |

新 API 中，消费并返回 `Self` 的链式构造方法不加 `set_`；通过 `&mut self` 修改状态时使用`set_`。已经公开的名称应保持兼容，现有的 `set_position` 等链式方法属于兼容例外，不作为新 API 的命名范例。

Boolean reader 只有两种：值持有某物时用 `has_<名词>`，描述状态或许可时用 `is_<形容词>`。只要动作有对应的形容词形式就用形容词：`is_closable` 而非 `can_close`，`is_zoomable` 而非 `can_zoom`，`is_copyable` 而非 `can_copy`。动作是没有形容词形式的动词短语时，改为命名它所需要的东西：用 `has_definition`，不用 `can_go_to_definition`。不再新增 `can_` reader。

Boolean builder 可叫 `disabled(bool)`，reader 叫 `is_disabled()`。含 non-boolean field 的公开接口中的非布尔字段使用 `with_item_ix(...)` 构造、`item_ix()` 读取，避免冲突。新的局部或内部零起始索引优先使用 `_ix`，现有公开名称如 `selected_index` 保持不变，不再引入 `_idx`。调用方从不构造的快照，不要为了对称而发布构造方法。

### 让外层名字承担上下文

名字总是在某个东西内部被阅读。字段在它的类型内部被读到，参数在它的方法签名内部
被读到，所以两者都不重复外层已经说过的话：`with_item_ix(ix)`，而不是
`with_item_ix(item_ix)`。

同一个类型的字段保持相同的缩写程度。其中若有一个写全了，它就成了异类，读者会去
找那个让它与众不同的区别。又因为 builder 按 `with_<field>` 命名，缩短字段会同时
缩短它的 builder，两者始终配对。

只在外层名字确实能消除歧义时才缩短。当短形式在生态的别处同时是*另一个量*的既有
术语时，在 doc comment 里说明你指的是哪一个，而不是把标识符加长 —— doc 是在调用处
被读到的，它能解释清楚一个更长的名字只能暗示的东西。

### 精确区分领域词汇

- **selected** 是持久 membership/active item；**focused** 是 keyboard target；**hovered** 是 pointer presence；**confirmed** 是 activation result，不能混用。
- **open/close** 描述 overlay/disclosure state；**show/hide** 表示 transient presentation request；**expand/collapse** 描述结构。
- **disabled** 禁止交互；**readonly** 允许导航/选择但禁止编辑；**loading** 表示操作中并应防止重复提交。这个状态一律拼作 `readonly`——一个词，与 `readonly(bool)` builder 和 `is_readonly()` reader 一致——标识符、界面标签和文档中都如此，不写 `read-only` 或 `read only`。
- **index** 是当前位置；**id** 是稳定 identity；`IndexPath` 是层级位置。重排数据不能用 index 持久化或作为 key。
- **value** 是 controlled domain data；**presentation** 是 render 用 read-only snapshot；**state** 是 retained behavior。
- **placement** 是 side/anchor policy；**position** 是 resolved geometry。
- **size** 是 semantic control tier；**width/height/bounds** 是 geometry。
- **child/children** 遵循 GPUI composition；`header`、`footer`、`trigger`、`content`等 named slot 具有额外语义。

避免 `data`、`item2`、`handle_action`、`update_ui`、`process`、`manager`、`config` 等模糊 public name。只有确实协调集合或生命周期时才使用 `Manager`，例如 `ToastManager`。

### 类型与模块命名

- Rust type/Action 使用 `UpperCamelCase`；module/function/method/field/local 使用`snake_case`；constant 使用 `SCREAMING_SNAKE_CASE`；
- component module 拥有 public seam；内部 folder 可拆 state/element/geometry/platform/test，但不能把这些实现路径泄漏到 import；
- 单一 component concept 使用 singular module；family 使用生态既有名称（`input`、`table`、`dock`）；
- 只有真正擦除 type boundary 的 wrapper 才加 `Any`，如 `AnyInputState`、`AnyElement`；
- identifier 后缀 `Id`，零基 index 后缀 `ix`，collection 使用有意义复数；同一 subsystem 不混用 `idx`、`index`、`ix`；
- predicate 尽量正向命名；正向的 `enabled`、`visible` 比多重否定更容易组合，但已建立的 control semantic（如 `disabled`）应保持一致。

### 回调与事件命名

只有真正 click-level contract 才用 `on_click`。Base controlled semantic primitive 应优先`on_change(next_value, ...)`；styled compatibility component 可以为现有 API 或 pointer detail 保留 `on_click`。Model-driven change 不能伪造 `ClickEvent`。

Lifecycle hook 名称必须准确：`on_will_change` 可 veto/prepare；`on_change` 观察请求或当前值；`on_confirm` 提交选择；`on_dismiss` 关闭 transient surface。文档必须说明 callback 发生在 internal state change 之前、之后还是代替它，以及是否可同步 re-enter。

### 文档与界面文案

Public docs 先说明 type 做什么、谁拥有 state。Example 必须使用当前可编译 API 和稳定 ID。记录 default、platform limitation、focus behavior、callback ordering，以及何时需要 notify、emit 或 theme sync。

标签、命令、确认对话框、大小写和省略号遵循[界面用词规范](./design-guides.md#界面用词)。每个领域对象、命令和状态使用一个固定术语。翻译键描述稳定意图（`dialog.delete_project.title`），不照抄源语言句子，也不包含 screen coordinate。不要用翻译片段拼句子，也不能因为两个含义碰巧有相同英文就复用同一个 key。

本地化的是意图，不是 syntax。每个 locale 可以独立决定语序、pluralization、标点与所需上下文。String 必须放进 component 并结合真实数据评审。Test 或 lint 应发现缺失 key、英文资源中的意外 CJK、三个句点组成的省略号、未经设计的 ALL CAPS 和固定术语不一致；上下文中的重复是否必要仍需人工判断。所有字符串都要放回真实组件，在代表性内容、文本扩展和应用缩放下验证。

## 测试策略

使用能够证明行为的最低层：

1. pure test：state transition、geometry、parsing、ordering；
2. GPUI context test：entity、event、subscription；
3. `VisualTestContext` interaction test：focus、keyboard、pointer、layout、rendered state；
4. example/application smoke test：完整 workflow。

Interactive component 测试 semantic contract，而非 implementation detail：pointer/keyboard activation、controlled value change、disabled、focus movement、event count/order、stable ID、关键 empty/failure state。Bug 可稳定复现时，修复前先加 regression test。

依赖真实 window system 的 UI 行为通过 accessibility tree 的 role、label、value、enabled、focus、selection 验证。每次改变 state 后重新读取 tree，因为 element index 只是 snapshot。Screenshot 只验证 semantic tree 无法表达的视觉事实；coordinate input 是最后手段。Automated 与 manual evidence 分开报告。

## 性能规则

- `render` 中不能无条件 mutate 或 notify；
- 不要每 frame 重建 Entity、Subscription、FocusHandle 和昂贵 data structure；
- coherent state change 后只 notify 最窄 owner；
- 长 collection virtualize，只 render visible range；
- 不为满足 closure 而 clone 大 string/collection，capture stable handle/shared data；
- 先 measure 再 cache，cache 必须有明确 invalidation owner；
- animation work 有界并遵守 reduced motion。

## 常见失败模式

避免以下模式：

- 一个 Entity 保存整个应用互不相关的 state；
- 长 `render` 中混入 business logic/network request；
- reorderable content 使用 random/index `ElementId`；
- literal color/radius 破坏 custom theme；
- 已有 semantic component 时仍用 clickable `div` 重做 focus/keyboard/disabled/a11y；
- duplicated local state 与 controlled model 漂移；
- `render` mutation 引发 `cx.notify()` loop；
- 没有 owner 的 nested scroll；
- 为 one-off screen 新增 component variant；
- 可逆低风险操作也弹 confirmation dialog；
- test 只调用 internal method，从不执行 pointer/keyboard 行为。

## 编码代理规则

修改前必须阅读与改动最相关的 implementation、test、re-export seam 和 component docs。必须在当前源码搜索 method signature，不能从 React、CSS 或旧 GPUI 示例类比翻译。

每项改动都应能回答：

1. behavior owner 与 presentation owner 是谁；
2. retained identity 与 state lifecycle 是什么；
3. pointer、keyboard、focus、accessibility contract 是什么；
4. layout 与 overflow owner 是谁；
5. 使用哪些 theme token，例外为何存在；
6. 哪个 test 会在行为退化时失败。

生成代码必须经过人工 review 与 test。“能编译”不是 UI quality bar；为了让生成代码看起来整洁而进行的大范围 refactor，也不能替代对仓库架构的匹配。

## 实现检查表

提交评审前，确认：

- state 与 side-effect ownership 是否明确；
- `RenderOnce` / `Entity<T>` 是否有意选择；
- repeated element 是否使用稳定 domain ID；
- theme token 与 component Size 是否替代孤立 visual literal；
- keyboard Action、focus、disabled 与 overlay 是否共同工作；
- loading、empty、error、cancellation path 是否存在；
- 长数据是否使用适当 virtualized component；
- public API 是否保持 dependency direction 与 encapsulation；
- test 是否在适当层证明 behavior；
- formatting、Clippy、targeted test 与相关 example 是否通过。

应用初始化请阅读[开始使用](./getting-started.md)，具体 API 以各组件页面为准。

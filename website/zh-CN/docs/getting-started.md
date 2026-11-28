---
title: Getting Started
description: 通过一个依赖和一个视图构建首个 GPUI Kit 桌面应用。
order: -2
---

# Getting Started

本指南从一个空目录开始，创建一个带文字和按钮的桌面窗口。开始前先安装 Rust 和对应平台的系统库，详见[安装](./installation.md)。

## 创建项目

1. 新建 Cargo 项目：

   ```sh
   cargo new gpui-hello
   cd gpui-hello
   ```

2. 在生成的 `Cargo.toml` 中加入 GPUI Kit：

   ```toml
   [dependencies]
   gpui-kit = "{{gpui_kit_version}}"
   ```

这一个依赖就包含了 GPUI、GPUI Base、带样式的 GPUI Component 和默认图标。代码通过 `use gpui_kit::*;` 使用 GPUI，通过 `gpui_kit::component` 使用组件。以后如需调整 feature 选择，见[图标与资源](./assets.md)。

## 添加视图并运行

1. 将 `src/main.rs` 替换为：

   ```rust
   use gpui_kit::component::{
       button::{Button, ButtonVariants},
       label::Label,
   };
   use gpui_kit::*;

   struct HelloWorld;

   impl Render for HelloWorld {
       fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
           div()
               .flex()
               .flex_col()
               .size_full()
               .items_center()
               .justify_center()
               .gap_2()
               .child(Label::new("Hello, World!"))
               .child(
                   Button::new("hello")
                       .primary()
                       .label("Click me")
                       .on_click(|_, _, _| println!("Clicked!")),
               )
       }
   }

   fn main() {
       application()
           .with_assets(assets::Assets)
           .run(|cx| {
               init(cx);

               open_window(WindowOptions::default(), cx, |_, cx| {
                   cx.new(|_| HelloWorld)
               })
               .expect("Failed to open window");
           });
   }
   ```

2. 在项目目录运行：

   ```sh
   cargo run
   ```

   首次构建需要编译全部依赖，会花一些时间。

窗口打开后，"Hello, World!" 下方是 **Click me** 按钮。点击按钮，终端会输出 `Clicked!`。

### 代码做了什么

1. `application()` 创建桌面应用；`.with_assets(...)` 注册默认图标资源。
2. `init(cx)` 初始化启用的 Kit 层，包括组件主题。只调用一次，并且要先于打开窗口或构造组件。
3. `open_window(...)` 从闭包创建 `Entity<HelloWorld>`，再用 [`Root`](./window) 包裹它。`Root` 承载窗口的对话框、抽屉和通知。闭包返回内容视图，不要再返回一个 `Root`。

`HelloWorld` 实现了 GPUI 的 [`Render`](./render) trait。GPUI 每次渲染这个视图时，`render` 都会返回一棵[元素树](./element)：一个包含 `Label` 和 `Button` 的 `div`。

问候语用 `Label` 而不是直接写字符串，是为了让读屏软件能读到。`.child("Hello")` 显示效果一样，但辅助技术读不到。

## 一个简短的心智模型

- [Entity<T>](./entity) 保存跨帧状态，也可以只持有不参与绘制的 model。
- 当 `T` 实现 `Render` 且被挂载时，这个 Entity 就是持久的 **View**，每次渲染都会生成新的元素树。
- 上面的 `Button` 这类元素只是为一次渲染构建的值。如果控件需要持久状态，比如输入框文字，就由所属视图保存对应的状态 [Entity](./entity)，不要在 `render` 中重新创建。
- [RenderOnce](./render-once) 组件把输入作为一个值，描述树中可复用的一部分。调用方提供状态和 handler 时适合用它；它仍可保留少量带 key 的元素状态。复杂状态、订阅和任务需要持久的 owner。

```text
Application entry → feature units (model, command, view)
                    ├─ Entity<Model>       retains state
                    └─ Entity<View>        persistent view; View implements Render
                          └─ Element tree   rebuilt on each render
                               └─ RenderOnce values compose reusable parts
```

应用增长后，拥有独立流程的功能可以把 model 和 View 放在同一个 feature crate 内。只有真正面向整个应用的状态，才使用私有 [Global](./global)。功能之间通过小型公开接口、event 或 `Entity` handle 协作，可复用部分容易接入，团队成员或 AI 代理并行修改时也有清晰边界。何时拆分以及如何确定所有权和依赖方向，详见[编码指南](./coding-guides)。

## 可选配置

上面的示例不需要以下任何配置。需要切换渲染后端，或不通过 Kit 直接依赖 GPUI 时，再回来看。

### 使用 GPUI Fast

GPUI Kit 默认使用固定版本的上游 GPUI 快照。启用 `gpui-fast` 可切换到 GPUI Fast 的保留渲染和布局引擎：

```toml
[dependencies]
gpui-kit = { version = "{{gpui_kit_version}}", features = ["gpui-fast"] }
```

- 这个 feature 会统一切换 GPUI、原生平台、WebAssembly 后端和所有已启用的 Kit 层。
- 代码无需改动：继续使用 `gpui_kit::*`、`gpui_kit::component` 和 `#[gpui_kit::test]`，不需要 `[patch]`、额外的引擎依赖或 crate 别名。
- 支持 `default-features = false`，也可与 `assets`、`test-support`、`inspector`、`profiler` 配合使用。
- GPUI Fast 依赖采用 `0.1.0` 版本要求，`cargo update` 可以更新到兼容的 `0.1.x` 版本。
- Cargo features 是累加的：启用 Fast 后，整个依赖图使用的公开类型都来自 Fast，上游包仍是构建依赖。不要将 Kit 的 Fast 类型与直接依赖上游 GPUI 得到的类型混用。
- 单独使用 Base、Component、资源、FPS、WebView 或 Shell 时，显式选择后端也要启用各自的 `gpui-fast` feature。

在此仓库中，可以分别运行 `cargo run` 和 `cargo run --features gpui-fast` 对比组件展厅。WebAssembly 构建使用 [WebAssembly](./webassembly.md) 中说明的 nightly 工具链。

### macOS 文字渲染：`font-kit`

**如果依赖的是 `gpui-kit`，可以跳过本节：它已经启用了 `font-kit`。** 本节只适用于直接依赖 GPUI 的应用。

GPUI 以固定版本的快照 crate 发布在 crates.io 上：`gpui-pre` 是 GPUI 本身，`gpui-pre-platform` 包含各原生平台后端。`gpui-kit` 已替你依赖了版本匹配的这两个 crate。在 macOS 上，文字渲染需要 `gpui-pre-platform` 的 `font-kit` feature，它默认不启用。未启用时，后端会退回 `NoopTextSystem`：窗口可以打开但不显示文字，`all_font_names()` 也返回空列表。

在直接使用 GPUI 的应用中启用它：

1. 在 `gpui_platform` 依赖上加入这个 feature。仅面向 macOS 的应用可以这样写：

   ```toml
   [dependencies]
   gpui = { package = "gpui-pre", version = "={{gpui_pre_version}}" }
   gpui_platform = { package = "gpui-pre-platform", version = "={{gpui_pre_version}}", features = ["font-kit"] }
   ```

   保留应用原有的其他平台 features，并保持两个快照版本一致。

2. 运行 `cargo run` 重新构建。
3. 如需确认，在 macOS 上运行 `cargo tree -e features -i gpui-pre-macos`，检查其中包含 `font-kit`。

这个 feature 属于 `gpui-pre-platform`，会启用 `gpui-pre-macos/font-kit`。不要把 `features = ["font-kit"]` 加到 `gpui` 或 `gpui-kit` 上，不要单独添加 `font-kit` 依赖，也不要在已使用 `gpui-kit` 的项目中加入上述配置。

## 接下来读什么

随着应用扩展，可按顺序阅读：

1. [Entity](./entity.md)、[Context](./context.md) 和 [Render](./render.md)：持有一个值，在按钮回调中修改它，并确认窗口显示新值。[Window](./window.md) 说明 `Root` 如何承载视图和浮层。
2. [Element](./element.md) 和 [RenderOnce](./render-once.md)：区分每次重建的元素树与持久状态。运行 [Paint](./paint.md) 中的 Brush 练习，确认按下指针会改变绘制结果。
3. [Focus](./focus.md)、[Action](./action.md) 和 [Event](./event.md)：用 Tab 到达交互目标，触发一个命令并观察状态变化。运行 [Task](./task.md) 中的流式示例；连续点击两次 Replay，确认旧分片不会重新出现。
4. [无障碍](./accessibility.md)和[测试](./test.md)：用键盘完成 Save 流程，检查焦点与可见结果，再运行文档中的 UI 测试，确认渲染状态和保存的模型值。在各目标平台上另行检查辅助技术的实际表现。
5. [组件目录](../component/index.md)：选择应用需要的控件，再按界面需求阅读[图标与资源](./assets.md)和[字体](./fonts.md)。

如需浏览器目标，请继续阅读 [WebAssembly](./webassembly.md)。持久输入状态与订阅的测试用例见[应用示例](https://github.com/longbridge/gpui-kit/tree/main/examples/ai_recipes)，[编码指南](./coding-guides.md)说明了这些示例遵循的约定。

## 完整且经过测试的 View

以下设置界面展示如何持有输入状态并管理订阅。它是可编译的示例，与[对应的 Rust 源文件](https://github.com/longbridge/gpui-kit/blob/main/examples/ai_recipes/src/settings.rs)保持同步。

如需验证，在仓库根目录运行[设置界面交互测试](https://github.com/longbridge/gpui-kit/blob/main/examples/ai_recipes/tests/settings.rs)，它会在 GPUI 测试窗口中挂载 `Settings`：

```sh
cargo test -p gpui-kit-recipes --test settings
```

测试通过时，输入后预览值先变为 `a`；在一次无关的重新绘制后继续输入，预览值变为 `ab`，变更次数依次为 1 和 2。这验证的是输入订阅和状态生命周期，不等同于原生界面的视觉验收。仓库中的 `gpui-kit-recipes` 默认可执行程序只打开一个简单的 bootstrap 视图，不会显示这个设置界面。

<!-- recipe:settings:start -->
```rust
use gpui_kit::component::{
    ActiveTheme, IconName, WindowExt,
    button::Button,
    checkbox::Checkbox,
    form::{Field, Form},
    input::{Input, InputEvent, InputState},
    radio::RadioGroup,
    switch::Switch,
};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, div,
};

pub struct Settings {
    name: Entity<InputState>,
    preview: SharedString,
    changes: usize,
    enabled: bool,
    remember: bool,
    delivery: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl Settings {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let subscription = cx.subscribe_in(&name, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.preview = state.read(cx).value().to_string().into();
                this.changes += 1;
                cx.notify();
            }
        });
        Self {
            name,
            preview: "".into(),
            changes: 0,
            enabled: false,
            remember: false,
            delivery: Some(0),
            _subscriptions: vec![subscription],
        }
    }

    pub fn input(&self) -> Entity<InputState> {
        self.name.clone()
    }

    pub fn preview(&self) -> &SharedString {
        &self.preview
    }

    pub fn changes(&self) -> usize {
        self.changes
    }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child("Profile")
            .child(
                Form::new()
                    .child(Field::new().label("Name").child(Input::new(&self.name)))
                    .child(Field::new().label("Preview").child(self.preview.clone()))
                    .child(
                        Field::new().label_indent(false).child(
                            Checkbox::new("remember")
                                .label("Remember name")
                                .checked(self.remember)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.remember = *value;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        Field::new().label_indent(false).child(
                            Switch::new("enabled")
                                .label("Enable notifications")
                                .checked(self.enabled)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.enabled = *value;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        Field::new().label("Delivery").child(
                            RadioGroup::new("delivery")
                                .children(["Immediately", "Daily summary"])
                                .selected_index(self.delivery)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.delivery = Some(*value);
                                    cx.notify();
                                })),
                        ),
                    )
                    .footer(
                        Button::new("about")
                            .label("About…")
                            .icon(IconName::Info)
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog.title("About").child("A complete GPUI Kit window")
                                });
                            }),
                    ),
            )
    }
}
```
<!-- recipe:settings:end -->

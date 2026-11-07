---
title: Root View
description: 使用 Root 视图为窗口启用主题、通知、对话框及其他 GPUI Component 功能。
example: false
---

# Root View

[Root] 是由 Base 提供的统一窗口根视图。应用统一通过 `gpui_kit::open_window` 创建窗口，它始终使用这个类型。Base 不提供额外的窗口创建函数；`component::Root` 重导出 Base 类型。

Base 负责内容与浮层承载、键盘焦点遍历和文本选择复制。显式调用 `gpui_component::init` 会注册窗口展示扩展，提供对话框、抽屉、通知、tooltip、菜单、触屏选择、主题与窗口边框。必须在创建窗口前初始化。仅使用 Base 的应用调用 `gpui_base::init`，无需依赖 Component 或 Kit。Cargo feature 合并不会改变窗口根类型。

下面这份完整的 **Tested consumer recipe** 在隔离的 `gpui-kit` 消费者工作区中编译。它先初始化 GPUI Kit，再打开一个以 `Root` 包裹应用视图为根的窗口。

<!-- recipe:bootstrap:start -->
```rust
use gpui_kit::{
    AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div,
};

pub fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            // The window's root view is a `Root` wrapping the view, which
            // renders dialogs, sheets and notifications above it.
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| BootstrapView)
            })
            .expect("failed to open window");
        });
}

struct BootstrapView;

impl Render for BootstrapView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("My application")
    }
}
```
<!-- recipe:bootstrap:end -->

`gpui_kit::open_window` 就是 `cx.open_window` 加上 `Root` 包裹。`Root` 必须是窗口的根视图，
以提供对话框、侧边面板、通知、焦点遍历和文本选择等窗口级能力。客户端窗口边框由窗口的
decorations 模式决定；server decorations 和 layer-shell 窗口不需要配置 Root。

`open_window` 同时返回窗口和视图，所以必须在窗口内构造的视图（比如它持有 `InputState`）也能留住句柄：

```rust
let (window, editor) = gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
    cx.new(|cx| Editor::new(window, cx))
})?;
```

## 关闭窗口与退出应用

应用自行定义退出和关闭窗口的 action 及快捷键，`gpui_kit::init` 不会安装这些绑定。应用应在关闭窗口或退出之前处理未保存的内容及确认流程。

## 浮层

`Root` 统一挂载对话框、抽屉和通知层，始终渲染在应用内容之上。应用只需调用 `window.open_dialog`、`window.open_sheet` 或 `window.push_notification`，不需要手动挂载或配置开关。子视图是否缓存不影响浮层渲染。

### 迁移到 0.7.0

`Root::render_dialog_layer`、`Root::render_sheet_layer` 和 `Root::render_notification_layer` 已删除。删除视图中对应的调用及 `.children(...)` 即可。此前自定义的层位置统一改为窗口级 Root 的浮层位置。

[Root]: https://docs.rs/gpui-base/latest/gpui_base/struct.Root.html

---
title: Input Group
description: 将输入框、文本域与文本、图标、按钮和工具栏组合使用。
---

# Input Group

使用 `InputGroup` 可以在输入框或文本域周围添加文本、图标、按钮和工具栏，
并将它们放在同一个外框内。简单的前缀或后缀可以使用 [Input](./input.md)。

下面的示例定义了可用于 GPUI Kit 应用的视图。应用初始化方式见
[快速开始](../docs/getting-started.md)。

## 带清空按钮的输入框

在视图中创建一次 `InputState`，再将它传给 `InputGroupInput`。
订阅 `InputEvent::Change`，更新依赖输入内容的界面。
将返回的 `Subscription` 保存在视图中，使回调持续有效。

下面的视图会显示字符数，并提供清空输入的按钮：

```rust
use gpui_kit::{
    AppContext as _, ClickEvent, Context, Entity, IntoElement, ParentElement as _,
    Render, Styled as _, Subscription, Window, rems,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Disableable as _, Icon,
    input::{InputEvent, InputState},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupInput, InputGroupText,
    },
};

struct SearchField {
    query: Entity<InputState>,
    _change: Subscription,
}

impl SearchField {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("搜索…"));
        let change = cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self { query, _change: change }
    }

    fn clear(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.query.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }
}

impl Render for SearchField {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.query.read(cx).value().chars().count();
        InputGroup::new("search")
            .max_w(rems(24.))
            .input(InputGroupInput::new(&self.query).aria_label("搜索"))
            .addon(InputGroupAddon::new("search-icon")
                .child(Icon::new(IconName::Search).size_4()))
            .addon(InputGroupAddon::new("search-actions")
                .align(InputGroupAddonAlignment::InlineEnd)
                .child(InputGroupText::new().child(format!("{count} 个字符")))
                .child(InputGroupButton::new("clear").label("清空")
                    .disabled(count == 0)
                    .on_click(cx.listener(Self::clear))))
    }
}
```

通过同一个状态读取或设置输入内容：

```rust
let value = self.query.read(cx).value();

self.query.update(cx, |state, cx| {
    state.set_value("gpui", window, cx);
});
cx.notify();
```

`InputEvent::Change` 用于响应用户编辑。通过 `set_value` 设置内容不会触发该事件；
程序更新输入值后，如果视图中的其他内容也需要刷新，请调用 `cx.notify()`。

## 部件与对齐

| 部件 | 用途 |
| --- | --- |
| `InputGroup` | 将一个输入控件与多个附加区域组合使用 |
| `InputGroupInput` | 使用 `InputState` 添加单行输入框 |
| `InputGroupTextarea` | 使用 `TextareaState` 添加多行文本输入 |
| `InputGroupAddon` | 放置文本、图标、按钮或自定义内容 |
| `InputGroupButton` | 添加紧凑的操作按钮 |
| `InputGroupText` | 显示辅助文字、前后缀或计数 |

用 `.input(...)` 设置输入控件，用 `.addon(...)` 添加附加区域，
在附加区域中通过 `.child(...)` 或 `.children(...)` 放置内容。
再次调用 `.input(...)` 会替换之前的输入控件；多次调用 `.addon(...)` 会保留所有附加区域。

通过 `.align(InputGroupAddonAlignment::...)` 设置位置：

| 对齐方式 | 位置 |
| --- | --- |
| `InlineStart`（默认） | 输入区域前侧 |
| `InlineEnd` | 输入区域后侧 |
| `BlockStart` | 输入区域所在行上方 |
| `BlockEnd` | 输入区域所在行下方 |

四种位置可以组合使用。同侧的附加区域及其内部内容按添加顺序排列。
为各部件设置稳定且不同的 ID。点击附加区域中的文本、图标或留白会聚焦输入框。

例如，为单行输入框添加协议前缀和域名后缀：

```rust
InputGroup::new("website")
    .input(InputGroupInput::new(&self.query).aria_label("网站"))
    .addon(InputGroupAddon::new("protocol")
        .child(InputGroupText::new().child("https://")))
    .addon(InputGroupAddon::new("domain")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupText::new().child(".com")))
```

## 按钮、图标与菜单

用 `.label(...)` 设置按钮文字，用 `.icon(...)` 设置图标。
纯图标按钮需要提供 `.aria_label(...)`，也可以通过 `.tooltip(...)` 添加提示。

```rust
use gpui_kit::component::input_group::InputGroupButtonSize;

InputGroupButton::new("clear-icon")
    .with_size(InputGroupButtonSize::IconXSmall)
    .icon(IconName::X)
    .aria_label("清空搜索")
    .tooltip("清空搜索")
    .on_click(cx.listener(Self::clear))
```

| `InputGroupButtonSize` | 用途 |
| --- | --- |
| `XSmall`（默认） | 紧凑文本按钮 |
| `Small` | 较大文本按钮 |
| `IconXSmall` | 紧凑方形图标按钮 |
| `IconSmall` | 较大方形图标按钮 |

按钮默认使用 ghost 样式。导入 `button::ButtonVariants` 后，可以使用
`.primary()`、`.secondary()` 或 `.danger()`。
用 `.outline()` 添加描边，`.disabled(true)` 禁用操作，
`.loading(true)` 显示进度并防止重复点击。点击按钮后，焦点不会被自动移回输入框。

操作菜单可以通过 `.dropdown_menu(...)` 配置，具体用法见 [Menu](./menu.md)。
上下文帮助可以将 `InputGroupButton` 传给 [Popover](./popover.md) 的 `.trigger(...)`，
再把 Popover 放入附加区域。其他 [Button](./button.md) 选项可通过 `.with_button(...)` 配置。

## 带字数统计和提交操作的 Textarea

将 `TextareaState` 传给 `InputGroupTextarea`。
`.auto_grow(min, max)` 使输入区域在指定行数范围内增高，超过最大行数后滚动显示。
固定行数使用 `.rows(n)`，固定高度使用 `InputGroupTextarea::h(...)`。

下面的完整视图会统计字符数，在内容为空或超出限制时禁用提交按钮，
并在编辑框下方显示提交的文本。提交后会清空内容，并将焦点放回文本域。

```rust
use gpui_kit::{
    AppContext as _, ClickEvent, Context, Entity, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, rems,
};
use gpui_kit::component::{
    Disableable as _, button::ButtonVariants as _, v_flex,
    input::{InputEvent, TextareaState},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupText, InputGroupTextarea,
    },
};

struct MessageComposer {
    message: Entity<TextareaState>,
    submitted: Option<SharedString>,
    _change: Subscription,
}

impl MessageComposer {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let message = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("输入消息…")
                .auto_grow(2, 6)
        });
        let change = cx.subscribe(&message, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self { message, submitted: None, _change: change }
    }

    fn submit(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.message.read(cx).value();
        if value.trim().is_empty() || value.chars().count() > 280 {
            return;
        }
        self.submitted = Some(value);
        self.message.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }
}

impl Render for MessageComposer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.message.read(cx).value();
        let count = value.chars().count();
        v_flex().max_w(rems(28.)).gap_2()
            .child(InputGroup::new("message")
                .invalid(count > 280)
                .input(InputGroupTextarea::new(&self.message).aria_label("消息"))
                .addon(InputGroupAddon::new("message-footer")
                    .align(InputGroupAddonAlignment::BlockEnd)
                    .child(InputGroupText::new().child(format!("{count}/280")))
                    .child(InputGroupButton::new("submit").ml_auto().primary().label("提交")
                        .disabled(value.trim().is_empty() || count > 280)
                        .on_click(cx.listener(Self::submit)))))
            .children(self.submitted.as_ref().map(|text| format!("已提交：{text}")))
    }
}
```

标题或上方工具栏可以使用 `BlockStart`。文本滚动时，附加区域保持原位。
更多文本设置见 [Textarea](./textarea.md)。

## 禁用、只读与校验

| 方法 | 效果 |
| --- | --- |
| `.disabled(true)` | 禁用输入区域及直接添加的 `InputGroupButton` 子部件 |
| `.readonly(true)` | 禁止编辑，同时允许聚焦、选择、复制和附加操作 |
| `.invalid(true)` | 显示错误状态，允许继续编辑 |

输入部件设置 `.disabled(true)` 时，整个组合也会禁用。
自定义交互内容和经过包装的控件需要分别传入禁用状态。

根据校验结果设置 `.invalid(...)`，并在旁边显示错误说明。
需要拒绝特定编辑内容时，使用 [`InputState::validate`](./input.md)。
即使为整个组合设置了名称，也应为输入控件单独提供 `.aria_label(...)`。

在 `InputGroupInput` 上通过 `.content_type(...)` 设置 URL、邮箱等输入提示，
通过 `InputState::masked` 配置密码遮罩。两个输入部件都支持用 `.context_menu(...)`
自定义右键菜单。

## 尺寸与样式

组合的默认尺寸为 Medium。导入 `Sizable` 后，可以使用 `.xsmall()`、`.small()`、
`.large()` 或 `.with_size(Size::Medium)`。
颜色和圆角遵循当前 [Theme](./theme.md)，宽度、间距等外观可通过 `Styled` 方法调整。

需要修改特定部件或状态时，使用以下方法：

| 组件 | 方法 | 作用位置 |
| --- | --- | --- |
| `InputGroupInput`、`InputGroupTextarea` | `editor_style` | 编辑区域的内边距、排版、背景和对齐 |
| `InputGroupButton` | `label_style` | `.label(...)` 设置的文字 |
| `InputGroupButton` | `icon_style` | 图标及其加载状态 |
| `InputGroup` | `focused_style` | 输入控件获得焦点且校验有效时的外框 |
| `InputGroup` | `invalid_style` | 校验无效时的外框，禁用时也适用 |
| `InputGroup` | `disabled_style` | 禁用时的外框 |

例如，修改 `SearchField` 中输入框和清空按钮的样式：

```rust
use gpui_kit::component::{ActiveTheme as _, Sizable as _, StyledExt as _};

InputGroup::new("styled-search")
    .small()
    .max_w(rems(24.))
    .focused_style(|style| style.border_color(cx.theme().primary))
    .invalid_style(|style| style.bg(cx.theme().danger.opacity(0.05)))
    .disabled_style(|style| style.opacity(0.7))
    .input(InputGroupInput::new(&self.query)
        .aria_label("搜索")
        .editor_style(|style| style.px_3().text_base()))
    .addon(InputGroupAddon::new("styled-actions")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupButton::new("styled-clear").label("清空").icon(IconName::X)
            .label_style(|style| style.font_semibold())
            .icon_style(|style| style.size_4())
            .on_click(cx.listener(Self::clear))))
```

多次调用样式方法会合并设置，同一属性以后一次设置为准。状态样式覆盖普通外框样式，
错误边框和外环优先于焦点与禁用样式；修改禁用样式不会启用交互。
状态样式中的 `border_color` 也会改变外环颜色。
导入 `FocusableExt` 后，可以用 `.focus_ring(false)` 隐藏默认外环。

`editor_style` 可以修改可编辑文字的样式，占位提示、光标和选区颜色遵循 Theme。
百分比内边距以编辑区域宽度为基准。附加区域和辅助文字可直接通过各自的 `Styled` 方法调整。

## JavaScript

从 `gpui-component` 导入同名部件，在 `View.init` 中创建输入状态。
使用 `.value(...)` 和 `.on_change(...)` 控制输入值：

```javascript
import { View } from "gpui-kit";
import {
  InputState, InputGroup, InputGroupInput, InputGroupAddon, InputGroupButton,
} from "gpui-component";

export default class Search extends View {
  init() {
    this.input = InputState("搜索…");
    this.query = "";
  }

  render() {
    return new InputGroup("search")
      .input(new InputGroupInput(this.input)
        .aria_label("搜索").value(this.query)
        .on_change((value, cx) => { this.query = value; cx.notify(); }))
      .addon(new InputGroupAddon("actions").align("inline-end")
        .child(new InputGroupButton("clear").label("清空")
          .disabled(this.query.length === 0)
          .on_click((_event, cx) => { this.query = ""; cx.notify(); })));
  }
}
```

程序通过 `.value(...)` 更新内容不会触发 `on_change`，设置相同的值会保留选区和撤销历史。
省略 `.value(...)` 可以让输入控件自行保存内容，需要响应编辑时使用 `on_change(value, cx)`。

`InputGroupTextarea` 接收 `TextareaState`，支持 `.rows(n)` 和 `.auto_grow(min, max)`。
两个输入部件均支持 `.placeholder(...)`。
`InputGroupInput` 还支持 `.masked(bool)` 和 `.content_type(...)`，
后者可使用 `email_address`、`url`、`new_password` 等值。

组合尺寸通过 `.size("small")` 设置，可用值为 `xsmall`、`small`、`medium` 和 `large`。
按钮尺寸为 `xsmall`、`small`、`icon-xsmall` 和 `icon-small`。
按钮图标使用资源路径，例如 `.icon("icons/search.svg")`。

上述六个样式方法也可在 JavaScript 中使用：

```javascript
new InputGroupInput(this.input)
  .editor_style(style => style.px(12).text_base());

new InputGroupButton("clear").label("清空").icon("icons/x.svg")
  .label_style(style => style.font_semibold())
  .icon_style(style => style.size_4());
```

样式回调可以调用样式方法、`when` 和 `map`，可以返回传入的样式对象，也可以不返回值。
子元素和事件回调应设置在组件上。
执行 `gpui-component-shell types <应用目录>` 可生成编辑器补全声明。

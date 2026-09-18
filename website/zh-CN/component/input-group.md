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
    input::{
        InputEvent, InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupInput, InputGroupText, InputState,
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
| `InputGroupInput` | 放入组合中的 [Input](./input.md)，使用 `InputState` |
| `InputGroupTextarea` | 放入组合中的 [Textarea](./textarea.md)，使用 `TextareaState` |
| `InputGroupAddon` | 放置文本、图标、按钮或自定义内容 |
| `InputGroupButton` | 带有紧凑组合样式的 [Button](./button.md) |
| `InputGroupText` | 显示辅助文字、前后缀或计数 |

`InputGroupInput` 和 `InputGroupTextarea` 就是普通的 `Input` 和 `Textarea`，只是以组合中的名字出现，
所以这两个控件的全部 builder——`aria_label`、`content_type`、`on_paste`、`cleanable`、`mask_toggle`
以及 `Styled` 方法——在组合里都可以使用。组合会去掉控件自身的边框、背景和焦点环，改为绘制在整个外框上。

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
纯图标按钮需要提供 `.accessibility_label(...)`，也可以通过 `.tooltip(...)` 添加提示。

```rust
InputGroupButton::new("clear-icon")
    .icon(IconName::X)
    .accessibility_label("清空搜索")
    .tooltip("清空搜索")
    .on_click(cx.listener(Self::clear))
```

按钮和其它控件一样通过 `Sizable` 设置尺寸：`.xsmall()` 是默认的紧凑尺寸，`.small()` 稍大；
只有图标的按钮在这两个尺寸下都是正方形。`.medium()` 和 `.large()` 保留标准按钮尺寸，
适合放在 block 附加区域里的主要操作。

按钮默认使用 ghost 样式。导入 `button::ButtonVariants` 后，可以使用
`.primary()`、`.secondary()` 或 `.danger()`。
用 `.outline()` 添加描边，`.disabled(true)` 禁用操作，
`.loading(true)` 显示进度并防止重复点击。点击按钮后，焦点不会被自动移回输入框。

操作菜单可以通过 `.dropdown_menu(...)` 配置，具体用法见 [Menu](./menu.md)；
`.dropdown_caret(true)` 会在文字右侧绘制下拉箭头。
上下文帮助可以将 `InputGroupButton` 传给 [Popover](./popover.md) 的 `.trigger(...)`，
再把 Popover 放入附加区域。

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
    input::{
        InputEvent, InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupText, InputGroupTextarea, TextareaState,
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

在触屏设备上，长按文本可以选择单词，拖动选择手柄可以调整范围，
编辑菜单提供剪切、复制、粘贴和全选操作。

在 Rust 中，两个输入部件还支持 `.on_paste(...)`，可在插入文本前处理剪贴板中的图片和文件。
返回 `true` 表示已处理此次粘贴，返回 `false` 则继续默认的文本插入。
输入控件处于禁用或只读状态时，不会调用该回调。
附件处理示例及 Web 端限制见 [粘贴回调](./input.md#粘贴钩子)。

## 尺寸与样式

组合的默认尺寸为 Medium。导入 `Sizable` 后，可以使用 `.xsmall()`、`.small()`、
`.large()` 或 `.with_size(Size::Medium)`；尺寸决定外框高度、文字大小，以及附加区域与控件共用的内边距。
颜色、圆角、焦点环和错误外环遵循当前 [Theme](./theme.md)，宽度、间距等外观可通过 `Styled` 方法调整。

控件本身的 `Styled` 方法作用于它编辑的文字，附加区域、按钮和文字部件也各自用同样的方式设置样式：

```rust
use gpui_kit::component::{ActiveTheme as _, Sizable as _, StyledExt as _};

InputGroup::new("styled-search")
    .small()
    .max_w(rems(24.))
    .input(InputGroupInput::new(&self.query)
        .aria_label("搜索")
        .px_3()
        .text_base())
    .addon(InputGroupAddon::new("styled-actions")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupButton::new("styled-clear").label("清空").icon(IconName::X)
            .font_semibold()
            .on_click(cx.listener(Self::clear))))
```

占位提示、光标和选区颜色遵循 Theme。导入 `FocusableExt` 后，可以用 `.focus_ring(false)` 隐藏默认外环。

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

组合和按钮的尺寸都通过 `.size("small")` 设置，可用值为 `xsmall`、`small`、`medium` 和 `large`。
按钮图标使用资源路径，例如 `.icon("icons/search.svg")`。样式方法和 Rust 一样直接作用于各个部件：

```javascript
new InputGroupInput(this.input).px(12).text_base();

new InputGroupButton("clear").label("清空").icon("icons/x.svg").font_semibold();
```

执行 `gpui-component-shell types <应用目录>` 可生成编辑器补全声明。

## 行内引用

需要将行内引用与附件、发送按钮等组合时，将包含 token 的 Input 或 Textarea 传给 `InputGroup`，并按需自定义标签：

```rust
use gpui_kit::component::{
    IconName,
    input::{InputToken, InputGroup, Textarea},
};

InputGroup::new("composer")
    .input(Textarea::new(&state)
        .token(|token, _, _| InputToken::new(token).icon(IconName::File)))
```

JavaScript 的分组输入也提供 `token` 和 `on_token_click`。重绘时保留原输入状态，需要恢复草稿时用保存的 content 调用 `set_value`。详见[原子行内 token](./input.md#原子行内-token)。

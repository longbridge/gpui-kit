---
title: Input Group
description: 将输入框、文本域、附加内容和原生操作组合在同一个外框中。
---

# Input Group

`InputGroup` 将一个输入框或文本域与文本、图标、按钮、工具栏组合在同一个外框中，
复用已有输入状态和原生编辑引擎。Group 负责外框与布局，应用负责文本状态、校验结果和操作。

颜色、圆角和焦点策略均来自当前 GPUI Component Theme。

## 导入

```rust
use gpui_kit::{AppContext as _, ClipboardItem, ParentElement as _, Styled as _};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _, Size,
    button::ButtonVariants as _,
    input::{InputContentType, InputEvent, InputState, TextareaState},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment,
        InputGroupButton, InputGroupButtonSize, InputGroupInput,
        InputGroupText, InputGroupTextarea,
    },
    menu::{DropdownMenu as _, PopupMenuItem},
    popover::Popover,
};
```

## 基本用法

在所属视图的构造函数中创建一次状态，并保留其 Entity：

```rust
let query = cx.new(|cx| InputState::new(window, cx).placeholder("搜索…"));
```

在 `render` 中用这个状态构建 Group：

```rust
InputGroup::new("search")
    .input(InputGroupInput::new(&query).aria_label("搜索组件"))
    .addon(
        InputGroupAddon::new("search-icon")
            .child(Icon::new(IconName::Search).size_4()),
    )
    .addon(
        InputGroupAddon::new("search-count")
            .align(InputGroupAddonAlignment::InlineEnd)
            .child(InputGroupText::new().child("12 个结果")),
    )
```

附加内容或其他视图内容依赖文本时，订阅 `InputEvent::Change`，从同一个状态读取值，
并通知所属视图更新。组件没有额外的 `InputGroupState`，也不会另存文本或选区。

## 组成与对齐

| 部件 | 构造与组合 | 职责 |
| --- | --- | --- |
| `InputGroup` | `new(id)`、`.input(...)`、`.addon(...)` | 共享外框与布局 |
| `InputGroupInput` | `new(&Entity<InputState>)` | 单行文本输入 |
| `InputGroupTextarea` | `new(&Entity<TextareaState>)` | 多行文本输入 |
| `InputGroupAddon` | `new(id)`、`.align(...)`、`.child(...)`、`.children(...)` | 一侧的文本、图标和操作 |
| `InputGroupButton` | `new(id)`、`.label(...)`、`.icon(...)`、`.on_click(...)` | 紧凑的原生操作按钮 |
| `InputGroupText` | `new()`、`.child(...)` | 弱化显示的辅助内容 |

所有部件都实现 `Styled`。Addon、Button 和 Text 接受普通子元素；
Group 和输入部件通过明确的类型插槽组合。

`input` 通过不透明的 `InputGroupControl` 转换类型接收 `InputGroupInput` 或
`InputGroupTextarea`，最后一次调用替换输入部件。
`addon` 追加部件，同一侧的多个 Addon 保持添加顺序。
每个 Addon 的 `.child(...)` 与 `.children(...)` 按完整的添加顺序排列文本、图标、
按钮和自定义内容。直接添加的 `InputGroupButton` 子部件继承 Group 的禁用状态。
动态增删或重排部件时，使用稳定的 ID。

| `InputGroupAddonAlignment` | 位置 |
| --- | --- |
| `InlineStart`（默认） | 输入区域前侧 |
| `InlineEnd` | 输入区域后侧 |
| `BlockStart` | 输入区域所在行上方 |
| `BlockEnd` | 输入区域所在行下方 |

行内与上下附加区域可以同时存在。Builder 的调用顺序不会改变这些区域的位置。
键盘遍历遵循原生元素顺序，按钮保留常规桌面键盘行为。

## 文本、图标与加载状态

```rust
InputGroup::new("website")
    .input(InputGroupInput::new(&query)
        .aria_label("网站").content_type(InputContentType::Url))
    .addon(InputGroupAddon::new("scheme")
        .child(InputGroupText::new().child("https://")))
    .addon(InputGroupAddon::new("domain")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupText::new().child(".com")))
```

Addon 可以放置 `Icon`、`Kbd`、`Spinner` 或应用自定义内容。
图标尺寸使用常规尺寸辅助方法明确设置。
点击普通附加内容或外框内的留白会聚焦输入区域。
Story 提供了搜索结果数量、货币文字、键盘提示和进度指示器示例。

## 按钮

通过 `.child(...)` 添加的操作会继承 Group 的禁用状态：

```rust
let copy_state = query.clone();
InputGroup::new("copyable-url")
    .readonly(true)
    .input(InputGroupInput::new(&query).aria_label("网址"))
    .addon(InputGroupAddon::new("url-actions")
        .align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupButton::new("copy-url")
            .with_size(InputGroupButtonSize::IconXSmall)
            .icon(IconName::Copy)
            .aria_label("复制网址").tooltip("复制网址")
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(
                    copy_state.read(cx).value().to_string(),
                ));
            })))
```

按钮默认使用紧凑的 ghost 样式。`ButtonVariants` 提供 `.primary()`、`.secondary()`、
`.danger()` 等语义样式。无障碍名称、提示、加载状态和描边可以直接通过
`.aria_label(...)`、`.tooltip(...)`、`.loading(...)` 和 `.outline()` 配置；
`with_button` 用于其他原生 Button 能力。按钮实现 `Selectable`、`InteractiveElement`
和 `DropdownMenu`，可以直接作为 Popover 触发器或调用 `.dropdown_menu(...)`。

| `InputGroupButtonSize` | 用途 |
| --- | --- |
| `XSmall`（默认） | 紧凑文本操作 |
| `Small` | 较大文本操作 |
| `IconXSmall` | 紧凑方形图标操作 |
| `IconSmall` | 较大方形图标操作 |

原生按钮保留自己的鼠标按下焦点策略。操作执行后，Group 不会再把焦点转回输入框。
自定义交互内容也应遵循相同的原生焦点约定。

## Textarea 与工具栏

多行内容使用 `TextareaState`。行数、换行、滚动和自动增长策略仍由该状态负责：

```rust
let message = cx.new(|cx| {
    TextareaState::new(window, cx)
        .placeholder("输入消息…").auto_grow(2, 6)
});
```

```rust
InputGroup::new("message")
    .input(InputGroupTextarea::new(&message).aria_label("消息"))
    .addon(InputGroupAddon::new("message-header")
        .align(InputGroupAddonAlignment::BlockStart)
        .border_b_1().border_color(cx.theme().border)
        .child(InputGroupText::new().child("新消息")))
    .addon(InputGroupAddon::new("message-footer")
        .align(InputGroupAddonAlignment::BlockEnd)
        .child(InputGroupText::new().child("0/280"))
        .child(InputGroupButton::new("send").ml_auto().primary().label("发送")))
```

为发送按钮连接所属视图的回调，并根据消息内容设置禁用状态。
Story 中提供了可操作的字数统计、发送、清空和示例附件。
需要固定编辑视口时，可给 `InputGroupTextarea` 设置 `.h(...)`。
滚动条属于文本视口，工具栏不会随文本滚动。

## 禁用、只读与校验

`.disabled(true)` 禁用输入区域及通过 `.child(...)` 添加的按钮，并阻止 Group 内的鼠标和键盘激活。
`.readonly(true)` 禁止编辑，同时保留聚焦、选择、复制和附加操作。
输入部件被禁用时，整个 Group 也会禁用；子部件设置为可用不能覆盖 Group 的禁用策略。

`.invalid(true)` 将应用的校验结果显示在共享外框上，不会阻止继续编辑。
`InputState::validate` 则参与判断一次文本编辑能否被接受。
在 Group 附近提供具体的错误说明，并为输入部件设置独立的无障碍名称。

输入部件保留已有的原生右键菜单，也支持通过 `.context_menu(...)` 替换。
单行输入通过原有状态和共享适配保留内容类型提示与密码遮罩。

## Theme 与尺寸

默认样式来自当前 GPUI Component Theme：

| 部分 | Theme 来源 |
| --- | --- |
| 外框边框 | `theme.input` |
| 背景 | 浅色模式透明；深色模式使用 30% 不透明度的 `theme.input` |
| 正文／辅助文字 | `theme.foreground`／`theme.muted_foreground` |
| 外框／按钮圆角 | `theme.radius`；紧凑 XS 按钮使用 `theme.radius_tokens().sm` |
| 焦点／错误颜色 | `theme.ring`／`theme.danger` |
| 外扩轮廓策略 | `theme.focus_ring` 与 `.focus_ring(...)` |

外框和紧凑按钮默认没有静态投影。焦点与错误轮廓不使用模糊，其颜色来自 Theme。
外框细线和轮廓宽度属于控件边界；文字、间距和控件尺寸遵循常规相对尺寸体系。
默认边框与背景颜色使用 150 毫秒过渡，并遵循减少动态效果的偏好。
错误边框优先于焦点边框；错误轮廓在浅色模式下使用 20% 不透明度，深色模式下使用 40%。
禁用的 Group 保留错误提示，禁用控件不会获得焦点轮廓。
Group 通过 `Sizable` 支持 `.xsmall()`、`.small()` 和 `.large()`。
Medium 是默认尺寸，也可以通过 `.with_size(Size::Medium)` 明确设置。
可使用 `Styled` 调整 Group 及各个部件。

原有 `Input::prefix` 和 `Input::suffix` 保持不变，适用于简单的独立输入框附加内容。
共享外框和工具栏组合可使用 Input Group。

## 内部样式与状态样式

各部件的 `Styled` 方法用于修改自身外框。稳定的内部部位有独立的样式构建方法：

| 组件 | 样式方法 | 作用位置 |
| --- | --- | --- |
| `InputGroupInput`、`InputGroupTextarea` | `editor_style` | 编辑区域的内边距、排版、背景和文本对齐 |
| `InputGroupButton` | `label_style` | `label` 设置的文字，与自定义子元素独立 |
| `InputGroupButton` | `icon_style` | 图标或加载图标，在默认尺寸与颜色之后应用 |
| `InputGroup` | `focused_style` | 编辑器获得焦点且校验有效时的外框 |
| `InputGroup` | `invalid_style` | 校验无效时的外框，禁用时也适用 |
| `InputGroup` | `disabled_style` | 禁用时的外框；交互仍保持禁用 |

```rust
InputGroup::new("draft")
    .focused_style(|style| style.border_color(cx.theme().primary))
    .invalid_style(|style| style.bg(cx.theme().danger.opacity(0.05)))
    .disabled_style(|style| style.opacity(0.7))
    .input(InputGroupTextarea::new(&message)
        .editor_style(|style| style.p_3().text_base()))
    .addon(InputGroupAddon::new("draft-actions")
        .align(InputGroupAddonAlignment::BlockEnd)
        .child(InputGroupButton::new("send").primary().label("发送")
            .icon(IconName::ArrowUp)
            .label_style(|style| style.font_semibold())
            .icon_style(|style| style.size_4())))
```

闭包立即执行，接收此前累积的 `StyleRefinement`。重复调用保留已有覆盖值，同一属性以后一次设置为准。组件只保存覆盖值，Theme 和尺寸的默认样式仍在渲染时计算。Addon 和 Text 已经通过 `Styled` 与普通子元素暴露内容，无需额外的内部样式方法。

Textarea 的内边距传入原生编辑引擎，光标命中、选区、IME 定位与滚动使用同一组内边距。相对内边距在布局和窗口尺寸变化后，跟随测得的编辑区域宽度更新。文本颜色和排版作用于可编辑文字；占位提示、光标和选区颜色继续使用语义 Theme token。

状态默认样式覆盖普通外框样式，各状态方法再覆盖对应状态的默认值。错误边框和外环优先于焦点与禁用样式。状态样式中的 `border_color` 也会调整默认外环的颜色；外环采用最终的边框宽度和圆角。自行设置阴影效果时，可以用 `focus_ring(false)` 关闭默认外环。

JavaScript 提供同名的六个方法。函数接收 `StyleDeclaration`，可以调用样式方法、`when` 和 `map`。函数在构建界面描述时执行，原生重绘复用已记录的样式。添加子元素、注册事件和返回其他元素会被拒绝。

```javascript
new InputGroupTextarea(this.message)
  .editor_style(style => style.p_3().text_base());

new InputGroupButton("send").label("发送").icon("icons/arrow-up.svg")
  .label_style(style => style.font_semibold())
  .icon_style(style => style.size_4());
```

## 更多组合示例

原生 Story 和 JavaScript Story 包含以下用法：

| 示例 | 可以尝试的操作 |
| --- | --- |
| 四向排列 | 前后图标，以及单行输入框上方的标题和下方的说明 |
| 图标 | 邮箱图标、带勾选标记的用户名、多个尾部图标 |
| 文本附加内容 | 货币、协议与域名后缀、工作邮箱后缀 |
| Tooltip | 密码要求和通知邮箱说明 |
| 下拉菜单 | 修改或重置文件名、选择搜索范围、选择电话区号 |
| Popover | 打开地址详情，用 Escape 关闭 |
| 标签和说明 | `@` 标签，以及放在上方附加区域中的标签 |
| 文字和图标操作 | 清空、重置、复制项目名称 |
| Spinner 位置 | 前置、后置，以及文字与 Spinner 的组合 |
| Textarea 变体 | 无附加内容、标题、剩余字数、错误和禁用状态 |
| 评论编辑 | 取消草稿，或发布后在示例中保留已提交的文本 |
| 自动增高 Textarea | 在一至八行之间增高，提交后清空 |
| 表单组合 | 将独立 Input、Input Group、Field 和 GroupBox 组合使用 |

每个输入控件都保留自己的状态，字数按 Unicode 字符计数。提交结果保留在 Story 中，
这些示例不会向外部服务发送消息或保存联系人。

### 在附加区域中使用下拉菜单

`InputGroupButton` 可以直接调用 `.dropdown_menu(...)`，再作为子元素放入附加区域。
这样可以保留菜单的打开状态、键盘导航和焦点恢复。下面的菜单会重置已保留的文件名状态：

```rust
let filename = self.filename.clone();
InputGroup::new("filename")
    .input(InputGroupInput::new(&self.filename).aria_label("File name"))
    .addon(InputGroupAddon::new("filename-menu").align(InputGroupAddonAlignment::InlineEnd)
        .child(InputGroupButton::new("filename-more").label("More")
            .dropdown_menu(move |menu, _, _| {
                let filename = filename.clone();
                menu.item(PopupMenuItem::new("Reset filename")
                    .on_click(move |_, window, cx| {
                        filename.update(cx, |state, cx| {
                            state.set_value("notes.txt", window, cx);
                        });
                    }))
            })))
```

### 在附加区域中使用 Popover

上下文详情使用现有 Popover，并直接以 `InputGroupButton` 作为触发器。
Popover 管理打开状态，输入框继续使用原来的编辑状态：

```rust
InputGroup::new("website-details")
    .input(InputGroupInput::new(&self.website).aria_label("Website"))
    .addon(InputGroupAddon::new("website-info")
        .child(Popover::new("address-details")
            .trigger(InputGroupButton::new("address-details-trigger")
                .with_size(InputGroupButtonSize::IconXSmall)
                .icon(IconName::Info).aria_label("Address details"))
            .child("The protocol prefix is separate from the editable hostname."))
        .child(InputGroupText::new().child("https://")))
```

原生 Story 使用紧凑图标触发器。JavaScript Popover 接口提供文字触发器，因此对应示例
把详情操作放在上方附加区域。所有内容均通过 `.child(...)` 添加；直接添加的按钮
部件继承禁用状态，菜单和 Popover 包装元素保留各自的组件契约。

### 使用原生自动增高

使用 `TextareaState::auto_grow` 启用原生自动增高：

```rust
let draft = cx.new(|cx| {
    TextareaState::new(window, cx)
        .placeholder("An automatically growing textarea…")
        .auto_grow(1, 8)
});
```

把状态传给 `InputGroupTextarea`，通过 `Styled` 调整字体，并把 Submit 放在下方附加
区域。组的控件插槽仍然接收原生类型明确的 Input 或 Textarea；此示例没有新增任意控件插槽。

## JavaScript shell

六个部件均已注册到 `gpui-component-shell`，复用已有的 `InputState` 和 `TextareaState` 构造函数。
shell 的通用 `.input(...)` 插槽同时接受 `InputGroupInput` 和 `InputGroupTextarea`，
重复调用会替换输入部件。在 `View.init` 中创建状态：

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

`value(...)` 仅在受控值与原生文本不同时进行同步。程序设置值不触发文本变化回调；
相同的值不会重置选区和撤销历史。省略 `value` 可直接使用非受控的持久文本状态。
`on_change(value, cx)` 报告编辑。
重新渲染会更新回调，不会重复创建订阅。

`InputGroupTextarea` 还支持 `.rows(n)` 和 `.auto_grow(min, max)`；
两个输入部件均支持 `.placeholder`。
`InputGroupInput` 支持 `.masked(bool)` 和 `.content_type(...)`，
后者使用 `InputContentType` 对应的 snake_case 名称，例如 `email_address`、`url` 和 `new_password`。
shell 按钮尺寸为 `xsmall`、`small`、`icon-xsmall`、`icon-small`。
按钮图标通过 `.icon("icons/search.svg")` 设置，无障碍名称通过 `.aria_label(...)` 设置。
`.child(...)` 仍可用于自定义按钮内容。

原生与 JavaScript Story 均提供了可操作的输入组合。
执行 `gpui-component-shell types <应用目录>` 重新生成声明，即可获得编辑器补全。

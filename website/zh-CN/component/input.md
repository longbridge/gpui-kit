---
title: Input
description: 带校验、掩码和多种扩展能力的文本输入组件。
---

# Input

多个附加元素、共享外框和文本域工具栏的组合方式，见 [Input Group](./input-group.md)。

Input 是一个单行文本输入组件，支持校验、输入掩码、前后缀元素以及多种交互状态。普通多行文本请使用 [Textarea](./textarea.md)，源代码编辑请使用 [Editor](./editor.md)。

## 导入

```rust
use gpui_kit::component::input::{Input, InputState};
```

## 用法

### 基础输入框

```rust
let input = cx.new(|cx| InputState::new(window, cx));

Input::new(&input)
```

### Placeholder

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter your name...")
);

Input::new(&input)
```

### 默认值

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("John Doe")
);

Input::new(&input)
```

### 可清空

```rust
Input::new(&input)
    .cleanable(true)
```

### 前缀和后缀

```rust
use gpui_kit::component::{Icon, IconName};

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())

Input::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )

Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())
    .suffix(Button::new("btn").ghost().icon(IconName::Info).xsmall())
```

### 密码输入

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .masked(true)
        .default_value("password123")
);

Input::new(&input)
    .content_type(InputContentType::Password)
    .mask_toggle()
```

掩码状态下，输入框不会让明文进入剪贴板，也不会通过选区暴露内容：Copy 和 Cut
不执行任何操作（上下文菜单中同样置灰），按词删除会删掉光标之前的全部内容，双击
则选中整个值而不是其中一个词。Paste 和 Select All 不受影响，通过 `mask_toggle`
显示明文后，上述操作全部恢复。

### 尺寸

```rust
Input::new(&input).large()
Input::new(&input)
Input::new(&input).small()
```

### 禁用态

```rust
Input::new(&input).disabled(true)
```

### 只读态

与 `disabled` 不同，只读输入框保持正常外观，仍然可以聚焦、选中和复制，只是拒绝用户对内容的修改。

```rust
Input::new(&input).readonly(true)
```

### 按 ESC 清空

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .clean_on_escape()
);

Input::new(&input)
```

### 输入校验

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .validate(|s, _| s.parse::<f32>().is_ok())
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
);
```

### 输入掩码

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("(999)-999-9999")
);

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("AAA-###-AAA")
);

use gpui_kit::component::input::MaskPattern;

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(3),
        })
);
```

### 监听事件

```rust
let input = cx.new(|cx| InputState::new(window, cx));

cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            println!("Input changed: {}", text);
        }
        InputEvent::PressEnter { secondary } => {
            println!("Enter pressed, secondary: {}", secondary);
        }
        InputEvent::Focus => println!("Input focused"),
        InputEvent::Blur => println!("Input blurred"),
    }
});
```

### 自定义外观

```rust
Input::new(&input).appearance(false)

div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(Input::new(&input).appearance(false))
```

### 上下文菜单

```rust
// 完全关闭右键菜单，自定义菜单也不会显示。
let input = cx.new(|cx| InputState::new(window, cx).context_menu(false));

// 或者用自己的菜单替换内置菜单。state 的上下文菜单需要保持开启（默认即开启）。
Input::new(&input).context_menu(|menu, window, cx| {
    // 可以定义自己的操作，也可以直接复用内置操作（剪切、复制、粘贴等），
    // 不必重新实现这些功能。
    menu.menu("Custom Action", Box::new(CustomAction))
        .separator()
        .menu("Cut", Box::new(input::Cut))
        .menu("Copy", Box::new(input::Copy))
        .menu("Paste", Box::new(input::Paste))
})
```

在选区外右键会把光标移到点击位置；在选区内右键会保留选区和光标。要处理指针下的文字（例如拼写建议菜单），读取 `context_menu_offset()`：

```rust
let state = input.clone();
Input::new(&input).context_menu(move |menu, _, cx| {
    // 打开此菜单的那次右键点击时，指针下的 UTF-8 偏移。
    let Some(offset) = state.read(cx).context_menu_offset() else {
        return menu;
    };
    menu.menu(format!("Look up the word at {offset}"), Box::new(LookUp))
})
```

### 触摸选择

在触摸屏上，长按会选中手指下的单词，手指按住不放时选区跟随手指移动。抬起手指后，选区上方会出现编辑菜单，列出当前可用的命令——`剪切`、`复制`、`粘贴` 和 `全选`——并在选区两端各显示一个拖动 handle。拖动 handle 会移动对应的一端，另一端保持不动；多行输入框在手指到达边缘时会自动滚动。长按空白处或空输入框时会放置光标，菜单只提供 `粘贴` 和 `全选`。

handle 和菜单属于这次手势产生的选区。只要其他操作改变了选区——点击、输入、方向键、`Escape`——它们就会消失；手指滚动内容时菜单会暂时让开。点击已选中的文字可以重新呼出菜单。

剪切、复制和粘贴通过输入框自身的 action 执行，因此自定义快捷键或打开中的补全菜单都能以同样方式处理它们。只读输入框只提供 `复制` 和 `全选`；密码输入框的内容不会进入剪贴板。

### 粘贴钩子

`on_paste` 会在默认文本插入之前拦截剪贴板内容，因此粘贴的图片和复制的文件可以保存在应用状态中，而不会被静默丢弃。`Input`、`Textarea` 和 `Editor` 均可使用。

```rust
use gpui_kit::ClipboardEntry;

let view = cx.entity().downgrade();
Textarea::new(&self.composer).on_paste(move |item, _, cx| {
    let images: Vec<_> = item.entries().iter().filter_map(|entry| match entry {
        ClipboardEntry::Image(image) => Some(image.clone()),
        _ => None,
    }).collect();
    if images.is_empty() {
        return false; // 回退到默认的文本插入
    }
    view.update(cx, |this, cx| {
        // 将图片保存在输入框之外的应用状态中，例如 `Attachment`。
        this.attachments.extend(images);
        cx.notify();
    }).ok();
    true // 已处理，输入框不再插入任何内容
})
```

当 handler 接管了粘贴时返回 `true`：`input::Paste` action 就此停止，输入框不插入任何内容。返回 `false` 则放行，engine 会像往常一样插入 `clipboard.text()`。复制的文件以 `ClipboardEntry::ExternalPaths` 的形式走同一个钩子。

已知限制：在 web 上 `read_from_clipboard()` 为 `None`（文本经由平台输入处理器到达）；那里的图片粘贴需要异步剪贴板访问和权限，不在本次范围内。

### 光标闪烁

获得焦点的输入框中，光标每 500 ms 闪烁一次。设置 `Theme::cursor_blink` 可修改应用中所有 Input、Textarea、Editor 和 OTP 输入框的光标策略，已获得焦点的输入框也会立即生效：

```rust
use std::time::Duration;
use gpui_kit::component::{Theme, input::CursorBlink};

Theme::update(cx, |theme| {
    theme.cursor_blink = CursorBlink::blinking(Duration::from_millis(530))
        .with_idle_timeout(Duration::from_secs(5));
});

// 光标保持显示，不闪烁。
Theme::update(cx, |theme| theme.cursor_blink = CursorBlink::steady());
```

`Theme::update` 会把策略同步到 Base，因此切换主题或明暗模式后策略依然保留。计时规则见 [Base Input](../base/primitives/input.md#光标闪烁)。

## 示例

### 搜索输入框

```rust
let search = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Search...")
);

Input::new(&search)
    .prefix(Icon::new(IconName::Search).small())
```

### 金额输入

```rust
let amount = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

div()
    .child(Input::new(&amount))
    .child(format!("Value: {}", amount.read(cx).value()))
```

### 多输入表单

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

v_flex()
    .gap_3()
    .child(Input::new(&self.name_input))
    .child(Input::new(&self.email_input))
```

## 列表

`list` 让输入框在用户输入时于下方显示一个 [List](list.md)，同时仍接受任意文本，相当于 HTML 的 `<input list>`。当字段接受自由文本、而已知值能帮助用户输入时（例如主机名或指标名），可以使用它。如果值必须是列表中的某一项，请改用 [Combobox](combobox.md)。若要补全光标处的词（例如 `$变量` 或 `@提及`），请像 Input story 中的 Completion 示例那样，在输入框周围组合一个菜单。

```rust
let input = cx.new(|cx| InputState::new(window, cx));
let hosts = cx.new(|cx| ListState::new(HostDelegate::new(input.clone()), window, cx));

Input::new(&input).list(&hosts)
```

`HostDelegate` 是普通的 `ListDelegate`，完整示例见 Input story。`perform_search` 返回的任务被替换时，旧任务会被丢弃，所以在任务开头等一个定时器就是防抖，旧查询的结果也不会覆盖新查询。固定列表就是在 `perform_search` 中同步过滤并返回 `Task::ready(())` 的 delegate。

和所有 `List` 一样，匹配、防抖、分组、分页和行的渲染都由 delegate 决定。delegate 有行或 `loading` 返回 true 时才显示弹层，并且只在输入框获得焦点时显示。没有行时弹层隐藏，因为自由文本不需要“无结果”提示；搜索失败可以显示为一行。只有用户输入才会触发搜索，代码修改未聚焦的输入框不会。请在 `confirm` 中写入接受的值（可以和行的标题不同），不要在 `ListEvent::Confirm` 订阅中写入，因为弹层关闭后写入的改动会被当作用户输入。禁用或只读的输入框不提供列表。

| 按键 | 操作 |
| --- | --- |
| Down | 高亮下一行，或打开弹层 |
| Up | 高亮上一行；从第一行返回输入的文本 |
| Enter | 接受高亮的行；没有高亮时，输入框照常发出 `InputEvent::PressEnter` |
| Escape | 关闭弹层 |

## 原子行内 token

使用行内 token，可以在输入中加入需要整块选中、删除的人员提及、文件引用或命令。例如，输入框显示一个名为“Alice”的标签，而 `value()` 和复制操作返回它的真实文本 `@alice`。

### 插入引用

创建并保留输入状态，在用户选中引用时插入 token：

```rust
use gpui_kit::component::input::{InlineToken, Input, InputState};

let input = cx.new(|cx| InputState::new(window, cx));

input.update(cx, |state, cx| {
    state.replace_with_token(
        InlineToken::new("person-1", "@alice").with_label("Alice"),
        window,
        cx,
    ).expect("有效的引用");
});

Input::new(&input)
```

`replace_with_token` 替换当前选区，空选区则在光标处插入，不自动添加空格。要替换 `@ali` 这样的补全查询，使用 `replace_range_with_token(range, token, window, cx)`。Rust 范围是 UTF-8 字节半开区间，可以使用 `str::find` 等方法返回的字节偏移。

ID 标识被引用的资源，同一个人被提及两次时两个 token 使用同一个 ID。`text` 用于复制和提交，`with_label` 指定显示名称；省略 `with_label` 时直接显示真实文本。

用户可以将光标移到 token 两侧，单击选中整块 token，或用 Backspace／Delete 删除它。选区覆盖部分 token 时会包含整块 token。Undo／Redo 同时恢复文本和引用。粘贴得到普通文本。

### 自定义外观与打开引用

token 默认渲染为 `InputToken`。`token` 槽位提供每个 token 的元素；需要图标时在其中返回带图标的 `InputToken`，并通过 `on_token_click` 打开引用：

```rust
use gpui_kit::component::{
    IconName,
    input::{InputToken, InlineTokenClickEvent},
};

Input::new(&input)
    .token(|token, _, _| {
        InputToken::new(token).icon(IconName::File)
    })
    .on_token_click(|event: &InlineTokenClickEvent, _, _| {
        // 根据 event.token().id() 查找并打开资源。
    });
```

也可以返回自己的单行元素。元素应保持在输入框行高内，超出可用行宽的内容会被裁切。通过 renderer 的上下文读取选中、只读和禁用状态。在事件回调中修改输入，不要在 renderer 中修改。悬停和选中样式应保持尺寸稳定。token 每次渲染时都会重新测量，因此数据到达后变宽的元素会在下一帧重新排版。

单击会先选中 token 再打开引用；拖选或 Shift 扩选不会打开引用。只读输入允许打开引用，禁用输入不允许。若要为打开选中的整块 token 提供快捷键，可以将 `ActivateToken` 绑定到自己选择的按键；辅助技术通过 token 的 click 操作触发同一个监听器。当应用能为引用给出明确名称（例如“打开文件”）时，可以通过 `context_menu` 自行添加菜单项。

### 悬停显示提示或预览

使用 `on_token_hover` 响应指针悬停，不改变文档内容。悬停不会选中或编辑，只报告进入与离开，应用据此显示 tooltip、预览或状态详情：

```rust
use gpui_kit::component::input::InlineTokenHoverEvent;

Input::new(&input)
    .on_token_hover(|event: &InlineTokenHoverEvent, _, _| {
        if event.is_hovered() {
            // 根据 event.token().id() 查找并显示预览。
        }
        // `false` 表示指针离开 token：关闭预览。
    });
```

事件携带 token、字节 `range()`、测量得到的 `bounds()` 与 `is_hovered()`。禁用 token 从不上报悬停进入，与点击规则一致；只读 token 会上报。删除、替换或禁用正在悬停的 token 时仍会发送其退出事件，提示应随之关闭。悬停与选中样式应保持尺寸一致，避免行在指针下抖动。

如果 token 内含按钮，应消费按钮的 mouse-down 和 click 事件，避免同时打开引用。所有子操作（包括无障碍操作）都应遵守 `token.is_disabled()`；会修改内容的操作还应遵守 `token.is_readonly()`。

### 保存、恢复与提交

保存草稿时，使用 `content()` 一起保留文本和引用：

```rust
let draft = input.read(cx).content();

// 稍后恢复保存的草稿：`set_value` 既接受纯文本，也接受 content。
input.update(cx, |state, cx| {
    state.set_value(draft, window, cx);
});
```

从应用自己的存储中恢复数据时，先用文本构造 `InputContent`，再把每个 token 附加到它的字节范围上。`with_token` 会随即按文本校验范围，因此 content 在设置之前就已经是自洽的：

```rust
use gpui_kit::component::input::InputContent;

let draft = InputContent::new("Ask @alice")
    .with_token(4..10, InlineToken::new("person-1", "@alice").with_label("Alice"))?;
```

提交时重新读取 `content()`：`text()` 是消息文本，`tokens()` 是其中仍然存在的引用。根据 token ID 查找资源，并在发送前处理资源已不存在的情况。

`set_value` 清空撤销历史，不触发 `InputEvent::Change`。传入纯文本会移除全部 token，即使文字未变；传入 content 则恢复其中的 token（无法显示 token 的模式除外）。需要可撤销的纯文本替换时使用 `replace_all`。token 编辑会触发 `InputEvent::Change`，包括为已有文本添加引用。程序化 setter 可以修改只读或禁用的输入，因此不应对用户开放的应用命令需要自行检查这些状态。

### 由文本派生 token

要把用户输入的部分文本显示为 token（例如光标不在附近时的 `**bold**`），将当前文本连同它的 token 传给 `set_tokens`。它保留文本、选区和撤销历史，不触发 `InputEvent::Change`：

```rust
use gpui_kit::component::input::{InlineToken, InputContent};

input.update(cx, |state, cx| {
    let content = InputContent::new(state.value())
        .with_token(2..7, InlineToken::new("bold", "**b**"))
        .expect("the text holds `**b**` at 2..7");
    // 光标位于 2..7 内时会被拒绝：文本保持原样。
    state.set_tokens(content, cx).ok();
});
```

content 中的文本不同时，`set_tokens` 返回 `TextMismatch`；token 会切开某个选区或光标时返回 `SplitsSelection`；失败时 token 保持原样。撤销/重做会按每次编辑记录的 token 重放文本，并丢弃被撤销的编辑切开的 token，因此每次变更后都应再次调用 `set_tokens`，使派生的 token 保持最新。

### 校验与适用范围

token 适用于 Input 和 Textarea，不支持 Editor、NumberInput、格式化 mask 或密码输入。ID 不能全为空白；真实文本和标签必须非空、单行且不含控制字符。范围不能重叠或切开 Unicode grapheme（例如 emoji 或带组合重音的字符）；恢复草稿时，每个 token 的真实文本必须与对应范围匹配。

token 操作返回 `Result<_, InlineTokenError>`，失败时输入保持原样。如果插入返回 `CompositionActive`，应等待用户完成当前输入法组合后再插入。

### JavaScript

在 `init()` 中创建并保留 `InputState`，渲染时将它传给 Input。JavaScript 范围使用 **UTF-16 字符串偏移**，与 `slice()`、`indexOf()` 一致：

```javascript
import { Input, InputState } from "gpui-component";

// 在 init() 中：
this.input = InputState();
this.input.set_value({
  text: "🙂 @alice",
  tokens: [{
    range: { start: 3, end: 9 },
    token: { id: "person-1", text: "@alice", label: "Alice" },
  }],
});

// 在 render() 中：
new Input(this.input)
  .on_token_click((event, cx) => {
    // 根据 event.token.id 查找并打开资源。
  })
  .on_token_hover((event, cx) => {
    // event.hovered 为 true 表示进入，为 false 表示离开；据此显示或关闭预览。
  });
```

用 `replace_with_token` 或 `replace_range_with_token` 插入引用，用 `content()` 和 `set_value(content)` 保存、恢复草稿，用 `tokens()` 读取当前引用。要删除引用，将它的当前范围传给 `set_selected_range`，再调用 `replace("")`。返回的快照是独立对象，修改快照不会更新输入。应在初始化、事件或任务回调中编辑，不要在 renderer 中编辑。

token 校验异常提供 `error.code`，例如 `InvalidBoundary` 或 `CompositionActive`；参数形状无效时也会抛出异常。Textarea 的 `TextareaState()` 提供相同方法。使用 `gpui-base` 时，改用 `InputState.new()` 或 `TextareaState.new()` 构造状态。

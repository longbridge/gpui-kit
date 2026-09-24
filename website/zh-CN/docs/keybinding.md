---
title: KeyBinding
description: 在 GPUI 中将 Action 绑定到按键、组合序列与获得 Focus 的 Key Context。
order: -2.625
---

# KeyBinding

**KeyBinding** 把一个或多个按键映射为有类型的 [Action](./action)。GPUI Kit 使用 GPUI 的 keymap：在应用上注册 binding，再把匹配的 Key Context 与 Action handler 放在当前 Focus 对应 [Element](./element) 的 Dispatch Path 上。Action 文档介绍完整的 Focus 与派发流程；本页重点说明 binding 的写法与匹配规则。

## 绑定一条命令

`KeyBinding::new(keys, action, context)` 接收按键字符串、Action 值和可选的 context predicate。在初始化时调用 `cx.bind_keys(...)`。`None` 使 binding 在整个应用中有匹配资格；`Some("Editor")` 要求当前 Focus 路径上有 `Editor` context。

```rust
use gpui_kit::*;

actions!(editor, [SaveDocument, MoveSelectionUp]);
const EDITOR_CONTEXT: &str = "Editor";

fn init_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-s", SaveDocument, Some(EDITOR_CONTEXT)),
        KeyBinding::new("up", MoveSelectionUp, Some(EDITOR_CONTEXT)),
    ]);
}
```

打开窗口之前，先调用一次 `gpui_kit::init(cx)`，再调用 `init_keys(cx)`。GPUI Kit 会在初始化时注册组件 binding，应用随后可按明确顺序追加自己的 binding。

拥有这段交互的 [Entity](./entity) 应保留 [FocusHandle](./window)，并在其 [Render](./render) 实现中一起注册 handle、context 和 handler：

```rust
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .key_context(EDITOR_CONTEXT)
            .on_action(cx.listener(Self::on_action_save_document))
            .on_action(cx.listener(Self::on_action_move_selection_up))
            .child("Editor")
    }
}
```

Handler 使用 GPUI 的常规签名，例如 `fn on_action_save_document(&mut self, _: &SaveDocument, window: &mut Window, cx: &mut Context<Self>)`。这里的 `cx` 是 owner 的 [Context](./context)。注册 binding 不会自动让元素获得 Focus；进入区域时主动设置 Focus，或让鼠标交互聚焦其 tracked handle。Focus 所属权、派发与传播详见 [Action](./action)。

## 按键字符串的写法

一次按键由 key name 和可选 modifier 组成，modifier 之间用连字符分隔。多个按键用空格分隔，构成连续输入的 **chord**：

| Binding | 含义 |
| --- | --- |
| `"secondary-s"` | 主快捷键修饰键加 S：macOS 是 Command，Linux 和 Windows 是 Control。 |
| `"ctrl-enter"`、`"alt-f4"`、`"shift-tab"` | 指定具体修饰键。 |
| `"cmd-shift-p"` | 平台修饰键加 Shift 和 P。`cmd`、`super`、`win` 都指平台修饰键；在 Windows 上它是 Windows 键，并非 Control。 |
| `"up"`、`"escape"`、`"space"`、`"backspace"` | 具名按键。 |
| `"cmd-k left"` | 连续按两次：平台修饰键加 K，然后 Left。 |

GPUI 还识别 `fn`、`ctrl`、`alt`、`shift`、`cmd`、`super`、`win` 与 `secondary`。常见的跨平台 Command/Control 快捷键应使用 `secondary`；所有平台都必须使用 Control 时才写 `ctrl`。大写 ASCII 字母如 `"A"` 表示 Shift+A，显式写 `"shift-a"` 更容易阅读。如果按键字符串或 context predicate 无法解析，`KeyBinding::new` 会 **panic**；静态定义应便于检查，用户输入则要先验证。

多个 chord 可以共用首键。只要更长的 binding 仍有可能匹配，GPUI 会暂存前缀；后续按键会完成 chord，或使前缀被重新派发。不要在可编辑区域中随意把常用文本输入键设为 chord 前缀。

## 声明 Key Context

元素的 `.key_context(...)` 声明当前节点上的事实。`KeyBinding::new` 的第三个参数是用于测试 Focus 路径上 context 的 **predicate**。二者语法相关，但用途不同：

```rust
// 挂在一个元素上的 context：标识符及 key/value 状态。
div().key_context("Editor mode=normal")

// 用作 KeyBinding::new 第三个参数的 predicate。
Some("Editor")
Some("Editor && mode == normal")
Some("Editor && !Modal")
Some("Workspace > Editor")
```

`Editor mode=normal` 在同一个节点声明两项事实；`Editor && mode == normal` 用来测试这些事实。Predicate 支持标识符、`==`、`!=`、`!`、`&&`、`||`、括号，以及表示祖先到后代 context 的 `>`。例如，`Workspace > Editor` 要求 `Editor` context 位于 `Workspace` context 下方。混用运算符时用括号明确分组。`!Modal` 可以排除包含 `Modal` 的路径，适用于对话框获得 Focus 时不应执行的 workspace 快捷键。

只有位于**当前 Focus** 对应 Dispatch Path 上的 context 才会参与匹配。Sibling 的 `Editor` context 不会激活编辑器 binding。即使 binding 匹配，如果 handler 挂在另一条分支上，它仍收不到 Action。把 context 和 handler 放在获得 Focus 的区域，或真正拥有这条命令的祖先上。

## 理解优先级

多个 binding 匹配相同按键时，GPUI 首先按 context 在 Focus 路径上的深度排序。内层 `Editor` 的 binding 优先于祖先 `Workspace` 的 binding。同一深度下，**后注册**的 binding 排在前面。`None` context 被视为匹配最深的 context，所以它不会自动成为低优先级 fallback：后注册的 `None` binding 可能排在相同按键的 `Editor` binding 前面。希望 workspace 快捷键让位于内层控件时，应显式指定 workspace context。

GPUI 可以依次尝试多个匹配的 binding。它沿 Focus 路径派发每个候选 Action，直到某个 handler 消费它；Action handler 默认停止传播。Handler 决定不处理时，可调用 `cx.propagate()` 继续派发。存在竞争 binding 时，注册顺序、context 深度和 handler 是否可达都会影响结果。

```rust
cx.bind_keys([
    KeyBinding::new("escape", ClearWorkspaceSelection, Some("Workspace")),
    KeyBinding::new("escape", CloseEditorSearch, Some("Editor")),
]);
```

Focus 位于 Workspace 内的 Editor 时，`CloseEditorSearch` 更具体；Focus 位于 Workspace 的其他区域时，只有 `ClearWorkspaceSelection` 匹配。GPUI Kit 的 Tree 与 TimeField 等组件也用自己的 context 将方向键行为限制在组件内部。

## 从 Action 反查快捷键

Action 也是**反向查找**快捷键的依据：向 window 查询当前由哪个 binding 触发该 Action。GPUI Kit 的 [Kbd 组件](../component/kbd)可显示查询结果。渲染按钮、菜单或命令面板时，应传入命令目标的 focus handle；因为此时可能是其他控件或 overlay 拥有 Focus。

```rust
use gpui_kit::component::kbd::Kbd;

let binding = window.highest_precedence_binding_for_action_in(
    &SaveDocument,
    &self.editor_focus,
);

// 同一个 Action 与目标对应的 GPUI Kit 快捷键提示。
let hint = Kbd::binding_for_action_in(
    &SaveDocument,
    &self.editor_focus,
    window,
);
div().children(hint)
```

第一个调用返回 `Option<KeyBinding>`；第二个返回可直接放在标签旁渲染的 `Option<Kbd>`。它们都会考虑目标区域的 context、较高优先级 binding 的遮蔽，以及当前 keymap（包括用户覆盖设置）。`None` 表示在可解析的目标路径上没有该 Action 可显示的 binding。这些查询基于**上一帧已经渲染的内容**，刚在当前帧首次绘制的 focus handle 可能暂时查不到结果。查到 binding 并不代表命令在当前运行状态下可以执行，也不证明目标路径上有 Action handler；owner 仍需决定是否允许执行。`window.is_action_available_in(&SaveDocument, &self.editor_focus)` 可以检查该路径上是否有元素级 Action handler。

查询 window 当前 context 时，还可以使用 `window.highest_precedence_binding_for_action(&action)` 和 `window.bindings_for_action(&action)`；后者返回全部可见 binding。已知单个 context 时，可用 `window.highest_precedence_binding_for_action_in_context(&action, KeyContext::parse("Editor")?)`。GPUI Kit 也提供 `Kbd::binding_for_action(&action, Some("Editor"), window)` 查询简单 context；传 `None` 则查询 window 当前 context。这里的 `Some(...)` 使用 **Key Context 声明语法**，例如 `"Editor mode=normal"`，而不是 `"Editor && mode == normal"` 这样的 predicate 语法。无效的 context 字符串会悄悄回退到 window 查询，因此动态输入应先验证。存在嵌套 context 或菜单改变了 Focus 时，使用具体 focus handle 更可靠。GPUI Kit 的 `Kbd::global_binding_for_action(&action, window)` 可针对空 Key Context 做最后的 fallback 查询，它不会重建嵌套 Focus 路径。

`Kbd::binding_for_action_in` 目前**只显示 chord 的第一个按键**。需要显示完整序列时，应格式化返回的 `KeyBinding` 中每个 keystroke：

```rust
use gpui_kit::AsKeystroke;
use gpui_kit::component::kbd::Kbd;

let shortcut = binding.map(|binding| {
    binding.keystrokes().iter()
        .map(|stroke| Kbd::format(stroke.as_keystroke()))
        .collect::<Vec<_>>()
        .join(" ")
});
```

`Kbd::format` 会选用对应平台的修饰键符号与按键名称：`secondary-s` 在 macOS 上显示为 `⌘S`，在 Linux 和 Windows 上显示为 `Ctrl+S`。`AsKeystroke` 让每个 binding stroke 可作为 `Keystroke` 使用。保留 `shortcut` 的可选性：没有绑定的 Action 不应显示虚构的快捷键。

## 让菜单和快捷键显示保持同步

快捷键、按钮、命令面板与菜单项应使用同一个 Action。菜单可以保存 `MenuItem::action("Save Document", SaveDocument)`；按钮可以调用 `window.dispatch_action(Box::new(SaveDocument), cx)`。当前 Focus 路径上的 owner 随后运行同一个 handler。

先注册 binding，再调用 `cx.set_menus(...)`。Native menu 在创建时读取当时的 keymap，并保留显示的快捷键。应用以后更改 binding，需要再次调用 `cx.set_menus(...)` 重建菜单，让标签和原生快捷键反映新 keymap。窗口内的菜单和 tooltip 应按上文从 Action 查询当前 binding，不要硬编码 `⌘A` 或 `Ctrl+A`：平台、目标 context 和用户 keymap 都可能改变实际显示的快捷键。GPUI Kit 的 Popup Menu 会根据命令目标或触发位置的 Focus 查询快捷键，并通过 `Kbd` 显示；简单 context 的 tooltip 可以使用 `Tooltip::new("Save Document").action(&SaveDocument, Some("Editor"))`，让它在渲染时查询。

## 用户 Keymap 与具名 Action

GPUI 的 `actions!` 宏会注册具有稳定名称的 unit Action，例如 `editor::SaveDocument`。带配置数据的 Action 应 derive `Action` 及所需的反序列化和 schema trait；`#[action(no_json)]` 表示不能从 JSON 构造的 Action。`cx.all_action_names()` 列出已注册名称，`cx.build_action(name, data)` 则用名称与可选 JSON 数据构造 Action。

GPUI 提供 Action registry 和 keymap 机制，但**用户 keymap 文件格式**、验证、加载与重载策略由应用自己负责。Loader 可以用 `cx.build_action(...)` 解析 Action 名称与数据，用 `KeyBindingContextPredicate::parse(...)` 解析 context，再通过可返回错误的 `KeyBinding::load(...)` 创建 binding。对解析错误或未知 Action 应向用户报告，而不是把不可信字符串传给解析失败会 panic 的 `KeyBinding::new(...)`。`cx.bind_keys(...)` 会追加 binding；`cx.clear_key_bindings()` 则清除整个应用的 keymap，包括组件默认 binding，因此重载时需要按所需顺序重新注册全部 binding。keymap 改变后还要重建 native menu。

## 排查快捷键问题

1. **按键：**检查当前平台实际使用的修饰键。`secondary-s` 在 macOS 上是 Command+S，在其他平台是 Control+S；Windows 上的 `cmd-s` 不是 Control+S。
2. **Focus：**检查哪个 `FocusHandle` 获得 Focus，以及已渲染元素是否用它调用 `track_focus`。只有点击后才生效，通常说明 Focus 路径有问题。
3. **Context：**检查 predicate 是否匹配路径上的 `key_context`。声明 context 用 `mode=normal`，测试它用 `mode == normal`。
4. **竞争：**检查更深层 context、同深度后注册的 binding 和 chord 前缀。`None` binding 可能排在较浅的 scoped binding 前面。
5. **Handler：**检查匹配的 Action 在当前 Focus 路径上是否有 `on_action` handler，以及更具体的 handler 是否提前消费了它。
6. **菜单与重载：**菜单仍显示旧快捷键时，在安装新 keymap 后再次调用 `cx.set_menus(...)`。组件快捷键在重载后消失时，确认 `clear_key_bindings()` 之后重新运行了所有组件初始化。

Focus 到 Action handler 的完整路由过程，继续阅读 [Action](./action)。

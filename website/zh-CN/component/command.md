---
title: Command
description: 命令面板 —— 经过过滤的命令与快捷操作列表。
---

# Command

命令面板是带有分组、由 Action 派生的快捷键提示和键盘导航的命令过滤列表。可以内嵌使用，也可以组合到现有对话框中，作为 `⌘K` 风格的菜单。失效时，Command 会创建并布局测量每一条扁平化的行；随后 `v_virtual_list` 只渲染和绘制视口行。

`Command` 拥有条目和展示策略。`CommandState` 拥有交互状态：搜索输入、焦点、选择、滚动和加载状态。

## 引入

```rust
use gpui_kit::component::command::{Command, CommandEntry, CommandGroup, CommandItem, CommandState};
```

## 组合方式

直接在 `Command` 上构建面板结构；创建一个空状态，并在面板显示期间复用它。

```text
Command
├── CommandItem                 // 未分组
├── CommandGroup
│   ├── CommandItem
│   └── CommandItem
├── separator
└── CommandGroup
    ├── CommandItem
    └── CommandItem

CommandState                    // 查询、焦点、选择、滚动
```

## 用法

### 内嵌

在应用初始化时定义 Action 和绑定。默认行会先在 Command 焦点作用域、再在应用作用域解析 Action 的当前绑定；只有找到绑定时才渲染 `Kbd` 提示。

```rust
use gpui_kit::{actions, KeyBinding};

actions!(my_app, [OpenProfile, OpenBilling]);

// During application setup:
cx.bind_keys([
    KeyBinding::new("cmd-p", OpenProfile, Some("Command")),
    KeyBinding::new("cmd-b", OpenBilling, Some("Command")),
]);

let state = cx.new(|cx| CommandState::new(window, cx));

Command::new(&state)
    .group(
        CommandGroup::new().label("Suggestions")
            .item(CommandItem::new().label("Calendar").icon(IconName::Calendar))
            .item(CommandItem::new().label("Search Emoji").icon(IconName::Search))
            .item(CommandItem::new().label("Calculator").disabled(true)),
    )
    .separator()
    .group(
        CommandGroup::new().label("Settings")
            .item(
                CommandItem::new().label("Profile")
                    .icon(IconName::User)
                    .action(Box::new(OpenProfile)),
            )
            .item(
                CommandItem::new().label("Billing")
                    .action(Box::new(OpenBilling)),
            ),
    )
    .placeholder("Type a command or search...")
    .empty(|_, _, cx| {
        v_flex()
            .items_center()
            .gap_2()
            .child(Icon::new(IconName::Search).size_8())
            .child("No results found.")
    })
    .w(px(380.))
```

不要提供手工格式化的快捷键字符串。`CommandItem::action` 同时提供可执行行为，并为默认行提供显示的绑定。自定义行拥有完整的展示内容，包括任何按键提示。

### 无搜索的快捷操作

为紧凑的操作面板关闭搜索。它没有搜索框，保留全部条目，并且 `state.focus(window, cx)` 会聚焦 Command 外框，因此仍可使用方向键、Enter 和 Escape 操作。

```rust
let actions = cx.new(|cx| CommandState::new(window, cx));

Command::new(&actions)
    .searchable(false)
    .items([
        CommandItem::new().label("New File").icon(IconName::Plus),
        CommandItem::new().label("Duplicate").icon(IconName::Copy),
        CommandItem::new().label("Move to Trash").icon(IconName::Delete),
    ])
    .w(px(380.))
```

默认的 `.searchable(true)` 下，`state.focus(window, cx)` 和 [`Focusable::focus_handle`] 会改为聚焦搜索输入框。不可搜索的面板不会调用 `on_query`。

### 在对话框中

使用现有的 [`WindowExt::open_dialog`] API 组合命令面板。`header` 渲染在可选搜索框和列表之上；`footer` 渲染在列表之下。在可搜索面板中，Escape 会清空非空查询。否则——包括具有隐藏的程序化查询的不可搜索面板——Command 会调用 `on_cancel`，然后传播 Cancel。应由宿主 Dialog 完成关闭——不要在 `on_cancel` 中再次关闭它。

```rust
use gpui_kit::component::WindowExt as _;

let state = self.command_state.clone();
window.open_dialog(cx, move |dialog, _, _| {
    let state = state.clone();
    dialog.close_button(false).p_0().content(move |content, _, _| {
        content.child(
            Command::new(&state)
                .bordered(false)
                .placeholder("Type a command or search...")
                .items([
                    CommandItem::new().label("Profile"),
                    CommandItem::new().label("Billing"),
                ])
                .on_confirm(|index, window, cx| {
                    window.push_notification(format!("Selected {index}"), cx);
                })
                // Record local cleanup only; Dialog handles the propagated Cancel.
                .on_cancel(|window, cx| {
                    window.push_notification("Command palette cancelled", cx);
                })
                .header(|state, _, cx| {
                    h_flex()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child("Commands")
                        .child(format!("{} matches", state.matched_count()))
                })
                .footer(|_, _, cx| {
                    h_flex()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child("↑↓ Navigate")
                        .child("Enter Select")
                        .child("Escape Close")
                }),
        )
    })
});
```

### 回调与 Action

回调配置在 `Command` 上，而不是从 `CommandState` 订阅。它们直接通知面板所有者：

```rust
Command::new(&state)
    .items(entries)
    .on_query(|query, window, cx| {
        // Start or update an application-owned search.
    })
    .on_select(|index, window, cx| {
        // Preview the newly highlighted IndexPath.
    })
    .on_confirm(|index, window, cx| {
        // Finish with this IndexPath, whether or not it has an Action.
    })
    .on_cancel(|window, cx| {
        // Clean up local palette state before Cancel propagates.
    })
```

`IndexPath` 始终对应最近一次 `Command` render 传入的模型，而不是内部过滤后的可见位置。
通过 `.items(...)` 传入的条目位于 section 0，`row` 等于它在该迭代器中的位置；
显式 group 使用其 group 与 item 位置；两种形式混用时，显式 group 排在隐式未分组 section 之后。
搜索过滤只改变可见内容，不改变这些坐标。

`on_query` 只在可搜索查询实际变化时运行。重新过滤可能移动高亮，因此所选 `IndexPath` 变化时会先运行 `on_select`，再运行 `on_query`。这些回调与 `on_confirm` 都会在当前 `CommandState` 更新释放其租用后交付。键盘和指针导致的高亮变化会运行 `on_select`，但从不分发 Action。只要来源窗口仍然存活，确认已启用条目时，会先分发其 Action，再调用 `on_confirm`；如果该 Action 关闭窗口，回调将无法交付。没有 Action 的条目仍会调用 `on_confirm`。在可搜索面板中，Escape 会先清空非空查询；否则调用 `on_cancel` 并继续传播 Cancel。

### 动态条目

将异步或变化的条目保存在所有者视图中，然后在该视图渲染时，根据所有者的当前数据重新构建 Command。不要通过条目构建器或 `set_entries` 修改 state。

```rust
struct StockSearch {
    state: Entity<CommandState>,
    results: Vec<CommandItem>,
}

impl StockSearch {
    fn render_palette(&self, owner: WeakEntity<Self>) -> Command {
        let results = self.results.clone();

        Command::new(&self.state)
            .items(results)
            .on_query(move |query, window, cx| {
                _ = owner.update(cx, |this, cx| this.search(query, window, cx));
            })
    }
}
```

当查询、选择和滚动改变时，已安装的模型会保留在 `CommandState` 中，因此这些交互不需要重新渲染所有者。之后的所有者渲染会安装新模型；若所选 `IndexPath` 仍存在则保留选择，并重新测量行。

## 搜索

Command 默认在条目的 label 和 keywords 中进行忽略大小写的子串匹配。空查询会匹配全部条目。分组中的条目全被过滤时，其标题会隐藏；过滤后位于首尾或相邻的分隔线不会显示。

```rust
CommandItem::new().label("Profile")
    .keywords(["account", "user"])
```

自定义或远程搜索时，在 `on_query` 中更新所有者持有的条目，并在等待时调用 `state.set_loading(true, window, cx)`，以隐藏空状态文案。响应到达后渲染新条目。

## 自定义行与虚拟滚动

`CommandItem::child` 会用惰性子元素工厂替换条目的图标和 label 内容。该工厂可能因测量、进入视口、排版或宽度失效而多次运行，因此必须无副作用。

失效时，Command 会在向 `v_virtual_list` 提供独立尺寸前创建并布局测量每一条扁平化的行。因此，自定义行可以拥有不同的固有高度；`v_virtual_list` 仍只渲染和绘制视口行。应按列表可用宽度构建行，并在所有者更新条目前保持其渲染内容稳定。

```rust
Command::new(&state)
    .item(CommandItem::new().label("compact").child(|_, _| {
        h_flex().w_full().py_1().child("Compact custom row")
    }))
    .item(CommandItem::new().label("expanded").child(|_, cx| {
        v_flex()
            .w_full()
            .py_4()
            .child("Expanded custom row")
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Extra detail"))
    }))
```

## Command

| 方法 | 签名与说明 |
| --- | --- |
| `new` | `new(&Entity<CommandState>) -> Command` 为 state 创建面板。 |
| `item` / `items` | `item(CommandItem) -> Self` 与 `items(impl IntoIterator<Item = CommandItem>) -> Self` 添加未分组条目。 |
| `group` / `separator` | `group(CommandGroup) -> Self` 添加分组；`separator() -> Self` 添加分隔线。 |
| `searchable` | `searchable(bool) -> Self` 显示或隐藏搜索框和本地过滤。默认：`true`。 |
| `on_query` | `on_query<F>(F) -> Self`，其中 `F: Fn(&str, &mut Window, &mut App) + 'static`，在可搜索查询变化后运行。 |
| `on_select` | `on_select<F>(F) -> Self`，其中 `F: Fn(IndexPath, &mut Window, &mut App) + 'static`，在高亮路径变化时运行。 |
| `on_confirm` | `on_confirm<F>(F) -> Self`，使用相同的 `IndexPath` 回调约束；在确认的 Action 分发后运行。 |
| `on_cancel` | `on_cancel<F>(F) -> Self`，其中 `F: Fn(&mut Window, &mut App) + 'static`，在 Escape 不会清空可搜索查询时，于 Cancel 传播前运行。 |
| `placeholder` | `placeholder(impl Into<SharedString>) -> Self` 设置搜索框占位文本。 |
| `empty` | `empty<F, E>(F) -> Self` 渲染无匹配时的自定义内容。 |
| `max_h` | `max_h(impl Into<DefiniteLength>) -> Self` 设置列表最大高度。默认：`18.75rem`（300px）。 |
| `bordered` | `bordered(bool) -> Self` 绘制外边框和圆角。默认：`true`。 |
| `header` | `header<F, E>(F) -> Self`，其中 `F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static`、`E: IntoElement`；渲染在搜索框和列表之上。 |
| `footer` | `footer<F, E>(F) -> Self`，使用相同的回调约束；渲染在列表之下。 |

`Command` 实现了 [`Styled`]，因此 `w`、`max_w`、`bg` 和其他样式可作用于面板外框。

## CommandItem

| 方法 | 说明 |
| --- | --- |
| `new` | 创建条目；Command 在内部生成渲染 identity。 |
| `label` | 设置可见 label 和默认搜索文本。 |
| `icon` | 为默认行设置前置图标。 |
| `action` | `action(Box<dyn Action>) -> Self` 设置点击或确认时分发的行为。默认行会显示其解析后的绑定。 |
| `checked` | 绘制尾部勾选。解析后的 Action 绑定会占用该位置。 |
| `keywords` | 添加默认匹配词。 |
| `disabled` | `Disableable::disabled(bool) -> Self` 使条目不可交互，并在键盘导航时跳过。 |
| `child` | `child<F, E>(F) -> Self`，其中 `F: Fn(&mut Window, &mut App) -> E + 'static`、`E: IntoElement`；惰性替换默认行内容。 |

## CommandGroup

| 方法 | 说明 |
| --- | --- |
| `new` | 创建无标题分组。 |
| `label` | 设置分组标题；所有条目被过滤时标题隐藏。 |
| `item` / `items` | 向分组添加一个或多个 `CommandItem`。 |
| `heading` | 返回可选标题。 |

`CommandEntry` 是 item、group 或 separator 的公共枚举。当所有者保存混合的动态条目集合时很有用；渲染时应将每个变体重新应用到新构建的 `Command`。

## CommandState

| 方法 | 签名与说明 |
| --- | --- |
| `new` | `new(&mut Window, &mut Context<Self>) -> Self` 创建空的交互状态。 |
| `query` / `set_query` | 读取查询，或通过 `set_query(query, window, cx)` 模拟输入。 |
| `selected_index` | 返回高亮条目在原始 entries 中的 `IndexPath`；section 表示顶层 entry，row 表示分组内条目。 |
| `matched_count` | 返回匹配条目数。 |
| `focus` | `focus(&self, &mut Window, &mut App)`：可搜索时聚焦输入框，否则聚焦 Command 外框。 |
| `set_loading` / `is_loading` | 显示或读取搜索加载动画；加载时隐藏空状态文案。 |

## 键盘快捷键

| 按键 | 行为 |
| --- | --- |
| `↑` / `↓` | 移动高亮，循环并跳过禁用项。 |
| `Enter` | 确认当前高亮项。 |
| `Escape` | 在可搜索面板中清空非空查询；否则调用 `on_cancel` 并传播 `Cancel`。 |

## 最佳实践

1. 在 `Command` 上构建静态条目、分组、分隔线、搜索能力和过滤器。
2. 将动态条目和异步结果保存在面板所有者中；渲染时据此重新构建 `Command`。
3. 绑定真实的 `Action`，而不是提供快捷键文本，以使提示和分发保持同步。
4. 保持 `child` 工厂无副作用；当行需要自定义展示或可变高度时使用它。
5. 在 `on_cancel` 后让宿主 Dialog 拥有取消行为；使用 header 和 footer 承载应用自有的状态和提示。
6. 每个独立渲染的面板使用各自的 [`CommandState`]。

[Command]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.Command.html
[CommandState]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandState.html
[CommandGroup]: https://docs.rs/gpui-component/latest/gpui_component/command/struct.CommandGroup.html
[WindowExt::open_dialog]: https://docs.rs/gpui-component/latest/gpui_component/trait.WindowExt.html#tymethod.open_dialog
[Focusable::focus_handle]: https://docs.rs/gpui/latest/gpui/trait.Focusable.html#tymethod.focus_handle
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html

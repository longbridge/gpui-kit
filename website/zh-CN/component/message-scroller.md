---
title: MessageScroller
description: 支持尾部跟随、未读定位和稳定 prepend 的虚拟消息列表。
---

# MessageScroller

`MessageScroller` 将 GPUI 的可变高度虚拟列表与会话常见的尾部跟随行为组合起来。它负责虚拟 row、滚动状态、append/prepend 的结构同步和可选的“跳到最新”按钮；消息数据、稳定消息 ID、未读规则和请求状态仍由应用持有。

## 适用场景与所有权

适合以下会话列表：

- 新消息到达时，用户在底部会自动跟随，向上阅读时保持当前位置。
- 历史消息从顶部插入时，当前可见内容保持稳定。
- 流式响应导致已有 row 高度变化，需要局部重新测量。
- 需要未读边界、任意 index 定位或自定义跳转按钮。

`MessageScroller` 不保存消息模型、消息 ID 到 index 的映射、网络请求、未读持久化或空/错误状态。应用负责把数据变化与状态 Entity 的变化放在同一个更新流程中。

## 导入

```rust
use std::time::Duration;

use gpui_kit::{div, IntoElement as _, ParentElement as _, StyleRefinement, Styled as _};
use gpui_kit::component::{
    ActiveTheme as _,
    button::ButtonVariants as _,
    message_scroller::{MessageScroller, MessageScrollerState},
    Sizable as _, StyledExt as _,
};
```

## 创建状态

把 `MessageScrollerState` 存在应用 view 的 Entity 中，与消息集合一起拥有：

```rust
let scroller = cx.new(|cx| MessageScrollerState::new(messages.len(), cx));

// 父 view 读取滚动状态或渲染 scroller 时，观察 Entity 以响应滚动事件。
cx.observe(&scroller, |_, _, cx| cx.notify()).detach();
```

状态构造器会把 GPUI `ListState` 设置为尾部跟随，并安装一次延迟 scroll handler。GPUI 触发 handler 时可能仍持有内部 list 借用，因此不要把 Entity 更新直接塞进 list 的同步回调之外的自定义借用逻辑。

## 渲染 row

传入按 index 渲染 row 的闭包。GPUI 会虚拟化 row，应用只需要渲染当前 index 对应的消息：

```rust
MessageScroller::new(
    "conversation",
    scroller.clone(),
    move |index, window, cx| {
        render_message(&messages[index], window, cx)
    },
)
.w_full()
.h_96()
```

`render_message(...)` 可以返回 `Message`、`MessageGroup` 或应用自己的 row。row 的稳定 element ID 应由应用根据消息 ID 生成；`MessageScroller` 只接收 index，不保存 index 到 ID 的映射。

空列表不会调用 row renderer。空状态、加载历史、网络错误和权限提示应由应用在 scroller 外层或 `item_count == 0` 的分支中组合：

```rust
if messages.is_empty() {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child("还没有消息")
        .into_any_element()
} else {
    MessageScroller::new("conversation", scroller.clone(), render_message)
        .into_any_element()
}
```

不要把空状态硬编码进 `MessageScroller`，这样应用才能区分“没有消息”“正在加载”和“加载失败”。

## Append、流式响应与重新测量

消息数据和列表结构必须同步更新：

```rust
messages.push(new_message);
scroller.update(cx, |state, cx| {
    state.append(1, cx);
});
cx.notify();
```

只有列表正在跟随尾部时，`append(...)` 才会自动滚到新的尾部；用户向上阅读时，新消息保留在列表底部并使“跳到最新”按钮出现。

流式响应通常不会增加 row 数量，而是改变现有消息的文本和高度。更新消息后重新测量对应的 index：

```rust
messages[index].content.push_str(delta);
scroller.update(cx, |state, cx| {
    state.remeasure_items(index..index + 1, cx);
});
cx.notify();
```

如果字体、窗口宽度或文本布局全局变化，重新测量所有 row：

```rust
scroller.update(cx, |state, cx| state.remeasure(cx));
```

`remeasure_items(...)` 和 `remeasure(...)` 只标记布局重新计算，不改变应用消息数据。调用方应保证 range 在当前 `item_count()` 范围内。

## 尾部跟随

状态提供两个 reader：

```rust
let following_tail = scroller.read(cx).is_following_tail();
let scrolled_up = scroller.read(cx).is_scrolled_up();
```

`is_following_tail()` 表示新增 row 是否会推动 viewport；`is_scrolled_up()` 表示用户已经离开最新内容且当前不在末尾。应用可以用后者显示自己的提示或通知，但通常直接保留内置跳转按钮即可。

用户滚动到末尾后，list 会恢复尾部跟随。调用 `scroll_to_end(...)` 会显式恢复跟随模式并滚到最新 row：

```rust
scroller.update(cx, |state, cx| state.scroll_to_end(cx));
```

“正在生成”的状态、暂停自动滚动和“新消息”提示属于应用交互；MessageScroller 只负责尾部跟随的列表行为。

## Prepend 历史消息

加载更早消息时，先在应用数据开头插入，再告诉状态增加了多少 row：

```rust
messages.splice(0..0, earlier_messages);
scroller.update(cx, |state, cx| {
    state.prepend(earlier_count, cx);
});
cx.notify();
```

`prepend(...)` 会通过 GPUI list 的 splice 保留当前 item 锚点，使用户正在阅读的内容尽量保持在原来的 viewport 位置。不要只更新 `messages` 而忘记更新 scroller；否则 renderer 的 index 与 list 的 row 数量会失去同步。

任意结构替换使用 `splice(...)`：

```rust
// 用新的 3 条记录替换 index 10..12 的两条记录。
messages.splice(10..12, replacement_messages);
scroller.update(cx, |state, cx| {
    state.splice(10..12, 3, cx);
});
```

range 必须满足 `start <= end <= item_count()`；无效 range 会返回 `false`，且不改变状态。`append(...)` 和 `prepend(...)` 同样返回是否成功。

## 未读与 index 定位

未读 ID 和消息 ID 属于应用模型。先把 ID 转成当前 index，再调用 `scroll_to_item(...)`：

```rust
if let Some(index) = messages
    .iter()
    .position(|message| message.id == first_unread_id)
{
    scroller.update(cx, |state, cx| {
        state.scroll_to_item(index, cx);
    });
}
```

`scroll_to_item(...)` 以 index 为 viewport 起始位置并暂停尾部跟随；靠近末尾时受可用滚动范围限制，index 超出当前数量时返回 `false`。它是唯一的定位原语：未读边界、搜索结果、书签消息、回复目标、深链接都先在应用侧解析成 index。组件没有 ID-native 的定位、turn anchor、peek 或可见 ID API；这些行为应由应用维护 ID/index 映射，并在需要时组合自己的 header、提示或高亮。

## Reset 与初始位置

切换会话或重新加载一组完全不同的数据时，先替换应用数据，再 `reset(...)`：

```rust
messages = load_thread(thread_id);
scroller.update(cx, |state, cx| {
    state.reset(messages.len(), cx);
});
cx.notify();
```

`reset(...)` 会重新安装 row 数量并恢复尾部跟随。若产品需要从未读位置或已保存 index 打开会话，可在 reset 后由应用调用 `scroll_to_item(...)`；已保存位置的持久化和恢复条件不属于组件状态。

## 跳到最新按钮

默认会在用户离开尾部时显示内置按钮。可以本地化标签、关闭按钮或自定义样式：

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .with_jump_button_label("跳到最新")
    .with_jump_button_style(
        StyleRefinement::default()
            .border_color(cx.theme().border),
    )
    .with_jump_button_transition(Duration::from_millis(250))
```

`Duration::ZERO` 会关闭按钮的进入/离开过渡；系统启用 reduced motion 时直接使用最终状态。

需要完全由应用提供按钮时，可以关闭内置入口，并根据 `is_scrolled_up()` 和 `scroll_to_end(...)` 组合自己的 Button：

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .jump_button(false)
```

`with_jump_button_renderer(...)` 接收已经配置好默认行为的 `Button`，因此可以调整 variant、语义尺寸、图标、可见 label 或实例样式，同时保留内置滚动操作：

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .with_jump_button_label("跳到最新")
    .with_jump_button_renderer(|button| {
        button.outline().large().label("跳到最新")
    })
```

如果只需要修改 Button 的样式，优先使用 `with_jump_button_style(...)`；需要替换 label 或 variant 时使用 renderer。

## Scrollbar、列表和 row 样式

三个 style slot 对应不同布局边界：

```rust
MessageScroller::new("conversation", scroller.clone(), render_message)
    .scrollbar(false)
    .with_content_style(
        StyleRefinement::default().bg(cx.theme().background),
    )
    .with_list_style(
        StyleRefinement::default().px_4().py_3(),
    )
    .with_row_style(
        StyleRefinement::default().pb_6(),
    )
```

| Builder | 作用范围 |
| --- | --- |
| `with_content_style(...)` | 内部 viewport 与 scrollbar 所在的容器。 |
| `with_list_style(...)` | GPUI list 本身，包括默认 list padding 的 refinement。 |
| `with_row_style(...)` | 每个 renderer row 外层的全宽包装。 |
| `Styled` | `MessageScroller` 根容器。 |

组件默认只在 row 之间保留 `pb_8()` 间距（类似 CSS gap），最后一行到消息区下方内容的间距由 list 自己的底部 padding 承担。自定义 `with_row_style(...)` 时应明确自己是否要额外增加间距，避免重复 padding。GPUI list 只在垂直方向偏移 row，因此 list padding 的水平分量（默认值与 refinement 均是）由每个 row 包装层承载。

`with_bottom_fade(color)` 让消息区底缘渐隐到指定颜色：被裁切一半的 row 融入 scroller 背后的表面，而不是在行中间生硬截断。渐隐只在读者离开末尾时显示——滚到最底时下方没有被裁内容，不再遮挡最后一行。传入 scroller 所在表面的颜色；默认关闭。

## 虚拟化、性能与可变高度

- `MessageScroller` 使用 GPUI `list(...)`，只创建 viewport 附近的 row；不需要额外 Provider、Viewport、Content 或 Item wrapper。
- renderer 应保持轻量，不要在 render 闭包中同步执行网络、文件读取或昂贵解析；先在应用状态层准备数据。
- 消息内容变化但 row 数量不变时使用 `remeasure_items(...)`，全局字体或宽度变化时使用 `remeasure(...)`。
- prepend 前后保持同一消息 ID 到数据记录的顺序，避免应用在 list 更新期间重排未涉及的消息。
- `item_count()` 是 list 记录的事实来源；应用数组长度与它不一致时应先用 `reset` 或 `splice` 修复结构。

## 可访问性

- 默认跳转按钮是可聚焦的 `Button`；`with_jump_button_label(...)` 只设置本地化 tooltip。若使用 renderer 替换为 icon-only 外观，应通过 `.label("跳到最新")` 保留可读名称，或关闭内置按钮后由应用提供自己的带 label Button。
- “跳到最新”“加载更早消息”“正在生成”“加载失败”等状态应提供文本或明确的 Button label，不依赖滚动位置和颜色。
- 键盘用户应能访问消息 row 中的 Link、Button、附件操作和应用自定义滚动入口。
- 消息区 viewport 以 log 区域（`Role::Log`）对外声明，辅助技术可以把追加的 row 当作实时新增内容播报。- 消息区上的滚轮事件是被包含的：list 还能滚动时事件不会带动外层滚动容器；到达顶部或底部边缘后才交给外层，与平台滚动容器的链式行为一致。
- 空状态和错误状态应由应用渲染可读内容；不要让一个空的虚拟列表看起来像加载失败。
- 自定义 jump transition、row 动画或流式高亮时，遵循系统 reduced motion，并提供静态最终状态。

## 组件边界

GPUI 版本保留必要的滚动行为，省略 React primitive 中重复的 Provider、Viewport、Content、Item 和 Button 导出：

- `Entity<MessageScrollerState>` 提供状态所有权与通知，不需要 React Context。
- GPUI `list(...)` 已经负责 viewport、虚拟内容、item 测量和滚动锚点。
- index renderer 已经是 row 边界，再增加 `MessageScrollerItem` 只会包装任意内容。
- 跳转操作复用现有 `Button`，应用可以关闭内置按钮并自行组合。
- 消息 ID、未读 ID、错误状态和历史加载属于业务域，由应用保留。

## API 参考

### `MessageScrollerState`

| 方法 | 说明 |
| --- | --- |
| `new(item_count, cx)` | 创建指定 row 数量并启用尾部跟随的 Entity 状态。 |
| `item_count()` | 返回当前虚拟 row 数量。 |
| `is_scrolled_up()` | 判断是否离开尾部且当前不在末尾。 |
| `is_following_tail()` | 判断是否会继续跟随新增内容。 |
| `reset(item_count, cx)` | 重置 row 数量并恢复尾部跟随。 |
| `splice(old_range, count, cx)` | 用指定数量替换已有 range；无效 range 返回 `false`。 |
| `append(count, cx)` | 在尾部增加 row。 |
| `prepend(count, cx)` | 在开头增加 row，并保留当前滚动锚点。 |
| `remeasure(cx)` | 标记所有 row 重新测量。 |
| `remeasure_items(range, cx)` | 标记指定 range 重新测量。 |
| `scroll_to_item(index, cx)` | 定位到指定 index；越界返回 `false`。 |
| `scroll_to_end(cx)` | 恢复尾部跟随并滚到最新内容。 |

### `MessageScroller`

| 方法 | 说明 |
| --- | --- |
| `new(id, state, renderer)` | 创建虚拟消息列表；renderer 接收 `(index, window, cx)`。 |
| `scrollbar(bool)` | 显示或隐藏内置 scrollbar。 |
| `jump_button(bool)` | 显示或隐藏内置“跳到最新”按钮。 |
| `with_jump_button_label(label)` | 设置按钮使用的本地化 tooltip 文本；不会替代 Button 的可读 label。 |
| `with_content_style(style)` | 调整 viewport 容器。 |
| `with_list_style(style)` | 调整 GPUI list。 |
| `with_row_style(style)` | 调整每个 row 外层包装。 |
| `with_jump_button_style(style)` | 在按钮默认样式之后应用 refinement。 |
| `with_jump_button_renderer(renderer)` | 修改已配置行为的 Button，并保留滚动操作。 |
| `with_jump_button_transition(duration)` | 设置按钮显示/隐藏过渡；零时长关闭过渡。 |
| `with_bottom_fade(color)` | 底缘渐隐到周围表面的颜色；默认关闭。 |
| `Styled` | 调整根容器。 |

### 类型链接

- [MessageScroller]
- [MessageScrollerState]

[MessageScroller]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScroller.html
[MessageScrollerState]: https://docs.rs/gpui-component/latest/gpui_component/message_scroller/struct.MessageScrollerState.html

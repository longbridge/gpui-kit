---
title: Message
description: 将发送者身份、元信息、富内容和操作组合成对齐的聊天消息。
---

# Message

`Message` 为聊天与会话界面提供消息行结构。它负责整体对齐、头像位置和具名 slot 的默认间距；应用负责消息数据、发送者、时间戳、送达状态和操作逻辑。

## 适用场景

- 需要把头像、发送者、时间、bubble、附件和 footer 组合成一条消息。
- 需要同时支持接收消息和发送消息的起始侧/结束侧布局。
- 需要把多个连续消息堆叠为一个发送者分组。
- 需要以 Ghost Bubble 展示 Markdown、代码或没有卡片表面的富内容。

只有一条状态提示时使用 `Marker`；只有一个内容 surface 时使用 `Bubble`；不要为每种消息类型创建专用的 Message wrapper。

## 导入

```rust
use gpui_kit::{ParentElement as _, StyleRefinement, Styled as _};
use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Sizable as _, StyledExt as _,
    attachment::{Attachment, AttachmentContent, AttachmentTitle},
    avatar::Avatar,
    bubble::{Bubble, BubbleVariant},
    button::{Button, ButtonVariants as _},
    message::{
        Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter,
        MessageGroup, MessageHeader,
    },
};
```

## 结构

```text
Message
├── MessageAvatar       # 可选，发送者身份
└── inner stack
    ├── MessageHeader   # 可选，发送者与时间
    ├── MessageContent  # 可选，Bubble、附件、Markdown 等
    └── MessageFooter   # 可选，送达状态、reaction、操作
```

所有具名 slot 都可以继续添加任意 GPUI element，并分别实现 `Styled`。`Message` 不持有消息模型，也不会替应用决定 header 或 footer 的文本。

## 基础用法

一条完整消息可以同时提供 avatar、header、content 和 footer：

```rust
Message::new()
    .avatar_slot(
        MessageAvatar::new()
            .child(Avatar::new().name("Alice").size_8()),
    )
    .header(
        MessageHeader::new()
            .child("Alice")
            .child("10:24"),
    )
    .content(
        MessageContent::new().bubble(
            Bubble::new().child("可以帮我检查一下吗？"),
        ),
    )
    .footer(MessageFooter::new().child("已读"))
```

只需要头像和内容时，可以使用便利的 `.avatar(...)`：

```rust
Message::new()
    .avatar(Avatar::new().name("Alice").size_8())
    .content(MessageContent::new().bubble(
        Bubble::new().child("收到的消息"),
    ))
```

## 对齐

`MessageAlignment` 会作用于消息行和 MessageContent 内的 Bubble：

```rust
Message::new()
    .alignment(MessageAlignment::Start)
    .avatar(Avatar::new().name("Alice"))
    .content(MessageContent::new().bubble(
        Bubble::new().child("对方的消息"),
    ));

Message::new()
    .alignment(MessageAlignment::End)
    .avatar(Avatar::new().name("我"))
    .content(MessageContent::new().bubble(
        Bubble::new().with_variant(BubbleVariant::Secondary).child("我发送的消息"),
    ))
```

| 值 | 用途 |
| --- | --- |
| `Start` | 接收消息或起始侧消息。 |
| `End` | 发送消息或结束侧消息。组件会反转头像与内容的行方向。 |

`MessageContent` 内的 Bubble 可以不设置自己的 alignment，让 Message 统一传播布局。独立使用 Bubble 时再显式设置 alignment。

## Avatar、Header、Content 与 Footer

### Avatar

`.avatar(...)` 会把任意 element 包装进 `MessageAvatar`；需要调整 slot 自身时使用 `.avatar_slot(...)`：

```rust
Message::new()
    .avatar_slot(
        MessageAvatar::new()
            .p_0()
            .child(Avatar::new().name("Support").size_8()),
    )
    .content(MessageContent::new().bubble(
        Bubble::new().child("我们已经处理了你的请求。"),
    ))
```

`MessageAvatar` 保留共享的 avatar 尺寸基线，并始终与消息内容的底边对齐；footer 渲染在头像行之下、按内容列缩进。身份 fallback、头像图片和名称文字由 `Avatar` 负责。

### Header

Header 适合放发送者、时间和其他低强调元信息：

```rust
Message::new()
    .header(
        MessageHeader::new()
            .child("Alice")
            .child("·")
            .child("10:24"),
    )
    .content(MessageContent::new().bubble(
        Bubble::new().child("消息内容"),
    ))
```

Header 默认有水平内容 inset。需要和 Ghost Bubble 对齐时，Message 会根据 content 自动处理；也可以显式调用 `.content_inset(false)` 或 `.content_inset(true)`。

### Content

Content 是消息主体，可以包含多个 Bubble、附件、图片、代码块或应用自己的富文本 renderer：

```rust
Message::new()
    .content(
        MessageContent::new()
            .bubble(Bubble::new().child("先看结论："))
            .bubble(
                Bubble::new()
                    .with_variant(BubbleVariant::Ghost)
                    .child("这是第二段无表面富内容。"),
            ),
    )
```

`MessageContent::bubble(...)` 会保留 Bubble 的 Ghost 元信息，用于协调 Header 和 Footer 的 inset；用普通 `.child(...)` 添加的 Bubble 不参与这个类型级传播。

### Footer

Footer 可放送达状态、reaction、Button 或其他次级信息：

```rust
Message::new()
    .content(MessageContent::new().bubble(
        Bubble::new().child("需要回复的内容"),
    ))
    .footer(
        MessageFooter::new()
            .child("未读")
            .child(Button::new("reply").ghost().small().label("回复")),
    )
```

Footer 内的 Button、Link 和其他控件由应用提供自己的事件、disabled、loading 和标签状态。

## 富内容、附件与操作

Message 通过组合现有组件表达不同内容，不增加重复的消息专用控件：

```rust
Message::new()
    .avatar(Avatar::new().name("Alice"))
    .header(MessageHeader::new().child("Alice").child("刚刚"))
    .content(
        MessageContent::new()
            .bubble(Bubble::new().child("请查看这个文件。"))
            .child(
                Attachment::new().content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new("quarterly-report.pdf")),
                ),
            ),
    )
    .footer(
        MessageFooter::new()
            .child(Button::new("download").outline().small().label("下载")),
    )
```

需要 Markdown、代码或 HTML 时，在 `MessageContent` 中放入应用选择的 text renderer；Message 只提供布局和 alignment，不改变富文本的选择、复制和交互行为。

## 消息分组

`MessageGroup` 用于堆叠同一发送者的连续消息：

```rust
MessageGroup::new()
    .child(
        Message::new()
            .avatar(Avatar::new().name("Alice"))
            .content(MessageContent::new().bubble(
                Bubble::new().child("第一条消息"),
            )),
    )
    .child(
        Message::new()
            .content(MessageContent::new().bubble(
                Bubble::new().child("同一发送者的第二条消息"),
            )),
    )
```

分组只负责垂直 stack 和共享间距；发送者变化、分组边界、时间戳和 avatar 是否重复显示由应用决定。需要不同发送者之间的间距时，在外层列表或自定义 group style 中表达。

## Ghost surface 与内容 inset

Ghost Bubble 没有背景、边框、padding，可用于 Markdown、代码或需要与消息行直接对齐的富内容：

```rust
Message::new()
    .header(
        MessageHeader::new()
            .child("系统")
            .child("刚刚"),
    )
    .content(
        MessageContent::new().bubble(
            Bubble::new()
                .with_variant(BubbleVariant::Ghost)
                .child("已完成索引更新。"),
        ),
    )
    .footer(MessageFooter::new().child("无需进一步操作"))
```

当 `MessageContent::bubble(...)` 中包含 Ghost Bubble 时，Header 和 Footer 默认移除水平 inset；这样元信息与无表面内容左边缘对齐。调用方可以覆盖该行为：

```rust
Message::new()
    .header(MessageHeader::new().content_inset(true).child("保留 inset"))
    .content(MessageContent::new().bubble(
        Bubble::new().with_variant(BubbleVariant::Ghost).child("内容"),
    ))
    .footer(MessageFooter::new().content_inset(false).child("移除 inset"))
```

`.content_inset(...)` 是 slot 的显式设置，优先于 Message 根据 Ghost Bubble 推导的默认值。`.px_0()` 等普通 `Styled` refinement 仍可用于更细的布局调整。

## 自定义样式与主题 token

`Message`、`MessageGroup`、`MessageAvatar`、`MessageHeader`、`MessageContent` 与 `MessageFooter` 都实现 `Styled`。具名 slot 之间的 stack 使用 `with_stack_style(...)`：

```rust
Message::new()
    .with_stack_style(StyleRefinement::default().gap_3())
    .p_3()
    .rounded(cx.theme().radius_lg)
    .bg(cx.theme().muted.opacity(0.35))
    .avatar_slot(
        MessageAvatar::new()
            .bg(cx.theme().secondary)
            .child(Avatar::new().name("A")),
    )
    .header(MessageHeader::new().px_0().child("Alice · 10:24"))
    .content(MessageContent::new().bubble(
        Bubble::new().child("遵循当前主题的消息 surface"),
    ))
```

推荐使用 `cx.theme()` 的语义颜色、圆角和共享 design scale。Message 的外层、inner stack、avatar、header、content 和 footer 都有独立的样式入口，调用方可以调整表面、间距、文字层级和对齐，而不需要复制 Message 的布局实现。

## 组件边界

- `Message` 不持有发送者、时间戳、送达状态、reaction 或操作状态；这些数据由应用生成对应的 child。
- `MessageContent::bubble(...)` 是专门用于 Bubble 的类型化便利入口，用于传播 Ghost surface 元信息；其他 element 使用普通 `.child(...)`。
- 应用操作使用 `Button`，URL 使用 `Link`，附件使用 `Attachment`；不创建消息专用的 Action、Link 或 Attachment wrapper。
- 需要消息列表、尾部跟随、未读定位或历史加载时，使用 `MessageScroller` 管理虚拟列表；Message 只负责单行布局。

## 可访问性

- Avatar 是身份辅助信息，不应是唯一的发送者标识；Header 应提供可读发送者或系统来源。
- 时间、送达状态、失败状态和未读信息应作为可读文本提供，不能只用颜色、位置或 icon 表达。
- Footer 中的 icon-only Button 应提供可见的 `.label(...)` 或其他可读名称，tooltip 只作为补充提示；发送者操作应使用明确的 Button/Link 语义。
- Bubble、Attachment 和富文本 child 的键盘行为由各自组件负责；Message 不会自动为普通 `div` 增加焦点。
- 应用自定义消息动画时，应在 reduced motion 下保持静态结果；Message 自身没有额外动画。
- 长消息和代码内容应保持可读的换行、选择和滚动策略，不要依赖 hover 才能访问完整内容。

## API 参考

### `Message`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建默认起始侧对齐的消息。 |
| `alignment(MessageAlignment)` | 设置起始侧或结束侧对齐。 |
| `with_stack_style(StyleRefinement)` | 调整 Header、Content、Footer 内部 stack。 |
| `avatar(element)` | 将任意 element 包装进 `MessageAvatar`。 |
| `avatar_slot(MessageAvatar)` | 设置完整的 avatar slot。 |
| `header(MessageHeader)` | 设置 header slot。 |
| `content(MessageContent)` | 设置 content slot。 |
| `footer(MessageFooter)` | 设置 footer slot。 |
| `Styled` | 调整消息行自身的布局与 surface。 |

### `MessageGroup`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建连续消息的垂直 stack。 |
| `child(element)` | 按顺序添加消息。 |
| `Styled` | 调整分组间距、宽度和布局。 |

### `MessageAvatar`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建身份 slot。 |
| `child(element)` | 添加 Avatar 或其他身份内容。 |
| `Styled` | 调整 slot 的尺寸、背景和位置。 |

### `MessageHeader` / `MessageFooter`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建对应的元信息或次级内容 slot。 |
| `content_inset(bool)` | 显式保留或移除默认水平 inset。 |
| `child(element)` | 添加文本、状态或操作。 |
| `Styled` | 调整文字、间距和布局。 |

### `MessageContent`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建消息主体 slot。 |
| `bubble(Bubble)` | 添加 Bubble，并参与 Ghost surface 的 inset 协调。 |
| `child(element)` | 添加任意富内容，不参与 Bubble 类型元信息传播。 |
| `Styled` | 调整主体的 stack、宽度和对齐。 |

### 类型链接

- [Message]
- [MessageAlignment]
- [MessageGroup]
- [MessageAvatar]
- [MessageHeader]
- [MessageContent]
- [MessageFooter]

[Message]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.Message.html
[MessageAlignment]: https://docs.rs/gpui-component/latest/gpui_component/message/enum.MessageAlignment.html
[MessageGroup]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageGroup.html
[MessageAvatar]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageAvatar.html
[MessageHeader]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageHeader.html
[MessageContent]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageContent.html
[MessageFooter]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageFooter.html

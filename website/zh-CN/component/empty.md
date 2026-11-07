---
title: Empty
description: 用媒体、文字、操作和自定义内容组合空状态。
---

# Empty

`Empty` 用于缺少内容、搜索无结果和首次使用等空状态。命名插槽负责布局与视觉层级，
应用决定何时显示空状态，并管理子组件的状态和操作。

该组件无状态，完整位于 GPUI Component 层，使用该层的主题和原生控件。

## 导入

```rust
use gpui_kit::{ParentElement as _, Styled as _, rems};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    avatar::{Avatar, AvatarGroup},
    button::{Button, ButtonVariants as _},
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia,
        EmptyMediaVariant, EmptyTitle,
    },
    input::{Input, InputState},
    link::Link,
};
```

通过 `gpui_kit::component::empty` 导入该组件。GPUI 自带的 `gpui_kit::Empty`
是另一个不渲染内容的元素。

## 基本用法

```rust
Empty::new()
    .header(
        EmptyHeader::new()
            .media(
                EmptyMedia::new()
                    .with_variant(EmptyMediaVariant::Icon)
                    .child(Icon::new(IconName::Folder)),
            )
            .title(EmptyTitle::new().child("No projects yet"))
            .description(
                EmptyDescription::new()
                    .child("Create your first project to get started."),
            ),
    )
    .content(
        EmptyContent::new()
            .flex_row()
            .flex_wrap()
            .justify_center()
            .gap_2()
            .child(Button::new("create-project").label("Create project…"))
            .child(
                Button::new("import-project")
                    .outline()
                    .label("Import project…"),
            ),
    )
    .child(
        Link::new("empty-help")
            .href("https://gpui-kit.com/docs/getting-started")
            .text_sm()
            .child("Learn more"),
    )
```

使用 Button 的常规回调连接应用操作。根部额外添加的子元素显示在 `EmptyContent`
之后，因此帮助链接可以独立于主要内容组。

## 组成

| 部分 | 组合方式 | 职责 |
| --- | --- | --- |
| `Empty` | `.header(EmptyHeader)`、`.content(EmptyContent)`、`.child(...)` | 整体对齐与间距 |
| `EmptyHeader` | `.media(EmptyMedia)`、`.title(EmptyTitle)`、`.description(EmptyDescription)` | 媒体与说明内容 |
| `EmptyMedia` | `.with_variant(...)`、`.child(...)` | 图标、图片、头像或任意媒体 |
| `EmptyTitle` | `.child(...)` | 标题文字或自定义内容 |
| `EmptyDescription` | `.child(...)` | 可换行的文字或富内容说明 |
| `EmptyContent` | `.child(...)` | 操作、输入框或其他控件 |

所有部分均提供 `new()` 和 `Default`，并实现 `Styled`。
除 `EmptyHeader` 外，其他部分都实现了 `ParentElement`。

命名插槽均为可选项，重复设置时替换原值：调用两次 `.header(...)` 会保留第二个 Header。
渲染顺序固定为 Header、Content，以及 Header 内的 Media、Title、Description，
与这些 setter 的调用顺序无关。根部直接添加的子元素按照自身插入顺序显示在两个
命名插槽之后，不会自动放进 Content。替换某个插槽不会影响其他插槽或根部额外内容。

## 边框

Empty 默认背景透明，没有可见边框。通过 `Styled` 开启边框后，默认使用虚线样式。

```rust
Empty::new()
    .border_1()
    .header(
        EmptyHeader::new()
            .title(EmptyTitle::new().child("Cloud storage is empty"))
            .description(
                EmptyDescription::new()
                    .child("Upload files to access them anywhere."),
            ),
    )
```

通过 `.border_color(...)` 调整边框的语义颜色。

## 背景

直接应用语义背景，无需新增组件变体：

```rust
Empty::new()
    .bg(cx.theme().muted.opacity(0.3))
    .header(
        EmptyHeader::new()
            .title(EmptyTitle::new().child("No notifications"))
            .description(
                EmptyDescription::new()
                    .child("New notifications will appear here."),
            ),
    )
```

## 头像

默认媒体变体不添加外框、背景或固定尺寸。现有 Avatar 保留自己的图片、回退内容、
尺寸和外观。

```rust
EmptyHeader::new()
    .media(
        EmptyMedia::new().child(
            Avatar::new()
                .name("Alex Morgan")
                .src("https://avatars.githubusercontent.com/u/5518?v=4"),
        ),
    )
    .title(EmptyTitle::new().child("Alex is offline"))
    .description(
        EmptyDescription::new()
            .child("Leave a message for Alex to read when they're back."),
    )
```

## 头像组

多个头像使用同一个媒体插槽。头像的重叠和尺寸由 AvatarGroup 管理，Empty 不检查
或修改其子元素。

```rust
EmptyHeader::new()
    .media(
        EmptyMedia::new().child(
            AvatarGroup::new()
                .child(Avatar::new().name("Alex Morgan"))
                .child(Avatar::new().name("Taylor Lee"))
                .child(Avatar::new().name("Sam Chen")),
        ),
    )
    .title(EmptyTitle::new().child("No team members"))
    .description(
        EmptyDescription::new()
            .child("Invite your team to collaborate on this project."),
    )
```

## 输入框与自定义内容

在所属视图中持有 `Entity<InputState>`，然后将现有 Input 放入 `EmptyContent`：

```rust
EmptyContent::new()
    .child(
        Input::new(&self.search)
            .prefix(Icon::new(IconName::Search).size_4())
            .cleanable(true),
    )
    .child(
        EmptyDescription::new()
            .child("Search by name or try a different keyword."),
    )
```

应用处理输入事件，并在结果列表与 Empty 之间切换。每个同时渲染的输入框各自持有
状态实体和焦点。Empty 不管理输入状态、校验、提交或加载策略。

## 受限布局

同时调整根元素与各个插槽，即可构建紧凑、起始侧对齐的空状态：

```rust
Empty::new()
    .max_w(rems(20.))
    .p_4()
    .items_start()
    .text_left()
    .header(
        EmptyHeader::new()
            .items_start()
            .title(EmptyTitle::new().child("No shared files"))
            .description(
                EmptyDescription::new()
                    .child("Add files so your team can review and edit them together."),
            ),
    )
    .content(
        EmptyContent::new()
            .items_start()
            .child(Button::new("add-files").outline().label("Add files…")),
    )
```

根元素填满可用宽度，并可在 flex 布局中增长。Header 和 Content 使用可用宽度，
上限为 24 rem。文字自然换行；Empty 不裁切子元素，也不创建滚动区域。视口和所需的
滚动由父容器提供。自定义媒体应适应容器宽度，操作行可使用 `.flex_wrap()` 处理窄空间。

## 默认样式

| 部分 | 默认值 |
| --- | --- |
| 根元素 | 居中列布局，`p_6()`、`gap_4()`、主题 `radius_tokens().xl` |
| Header | `gap_2()`，子项居中，最大宽度 24 rem |
| Media | 按内容确定尺寸的居中列布局，`mb_2()`，不收缩 |
| Icon 媒体 | `size_8()`、muted 背景、前景色、主题 `radius_tokens().lg` |
| Title | `text_sm()`，中等字重 |
| Description | `text_sm()`，1.625 行高，muted 前景色 |
| Content | 居中列布局，`gap_2p5()`、`text_sm()`，最大宽度 24 rem |

实例样式覆盖默认值和媒体变体样式。Icon 媒体提供一 rem 字号，未显式设置尺寸的
GPUI Component `Icon` 会继承该尺寸；显式设置的图标尺寸仍然有效。任意 SVG 或图片
子元素保留自己的尺寸。排版跟随应用字体和 rem 比例，使用 GPUI 的原生换行与字距；
组件不另行实现 CSS 的 `text-balance` 和 `tracking-tight`。

Empty 不创建焦点目标，也不会自动作为警告或实时状态播报。内部 Button 和 Input
保留正常的焦点与键盘行为。应用命令使用 Button，外部资源使用 Link。

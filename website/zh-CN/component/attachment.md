---
title: Attachment
description: 支持上传状态、预览和操作的可组合文件与媒体附件表面。
---

# Attachment

`Attachment` 用于在会话中展示文件或媒体条目。根组件负责状态、方向、尺寸和整体 surface；`AttachmentMedia`、`AttachmentContent`、`AttachmentActions` 分别提供预览、元数据和操作入口。应用可以在这些 slot 中继续组合 `Button`、`Progress`、`Icon` 和其他 GPUI element。

## 适用场景

- 文件上传队列、消息附件、图片预览和处理中的媒体。
- 需要让标题、描述和预览随着上传生命周期改变外观的场景。
- 需要在横向紧凑卡片与纵向预览卡片之间复用同一套语义结构的场景。

选择文件、打开预览、取消上传和重试等业务行为仍由应用通过子控件持有；`Attachment` 不保存文件模型或网络状态。

## 导入

```rust
use gpui_kit::{Axis, ParentElement as _, Styled as _};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Size, Sizable as _, StyledExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription,
        AttachmentGroup, AttachmentMedia, AttachmentStatus, AttachmentTitle,
    },
    button::{Button, ButtonVariants as _},
    progress::Progress,
    shimmer::ShimmerStyle,
};
```

## 组合结构

```text
Attachment
├── AttachmentMedia       # 图标、图片预览和 overlay
├── AttachmentContent     # 标题、描述以及任意附加内容
│   ├── AttachmentTitle
│   ├── AttachmentDescription
│   └── Progress（可选）
└── AttachmentActions     # Button 或其他操作
```

具名方法保留状态继承和方向布局；`.child(...)` 仍可添加任意 element，但会擦除具体类型。

## 基础文件附件

最小的文件附件可以只提供元数据：

```rust
Attachment::new()
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("quarterly-report.pdf"))
            .description(AttachmentDescription::new("PDF · 2.4 MB")),
    )
```

常见的文件卡片同时提供类型图标和移除操作：

```rust
Attachment::new()
    .media(AttachmentMedia::new().child(Icon::new(IconName::FileText)))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("quarterly-report.pdf"))
            .description(AttachmentDescription::new("PDF · 2.4 MB")),
    )
    .actions(
        AttachmentActions::new().child(
            Button::new("remove-report")
                .ghost()
                .xsmall()
                .icon(IconName::Close)
                .label("移除")
                .tooltip("移除附件"),
        ),
    )
```

`Attachment::new()` 默认状态为 `Complete`、方向为 `Axis::Horizontal`、尺寸为 `Size::Medium`。

## 媒体和图片预览

将 `ImageSource` 传给 `.src(...)` 即可显示图片；媒体 slot 也可以继续保留图标或其他 child：

```rust
Attachment::new()
    .media(
        AttachmentMedia::new()
            .src(preview_source)
            .child(Icon::new(IconName::Image)),
    )
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("preview.png"))
            .description(AttachmentDescription::new("PNG · 1280 × 720")),
    )
```

有图片源时，图片使用 `ObjectFit::Cover` 填满媒体区域。没有图片源时，媒体 slot 仍然可以展示图标；如果附件状态为 `Failed`，没有图片源的媒体区域使用 destructive 语义颜色。

### 预览上的 overlay

`.overlay(...)` 会把内容居中覆盖在整个媒体区域上。overlay 不会随图片的 loading opacity 一起变暗，适合 Spinner、播放按钮或错误提示：

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .media(
        AttachmentMedia::new()
            .src(preview_source)
            .overlay(Progress::new("preview-progress").value(68.)),
    )
```

也可以将 overlay 与自定义 `div()`、`Button` 组合。加载、处理和失败状态会降低图片本身的透明度；`Pending` 与 `Complete` 保持完整不透明。

## 上传生命周期

`AttachmentStatus` 只表达通用生命周期，具体的文件数据和请求状态由应用持有：

| 状态 | 用途 | 默认视觉提示 |
| --- | --- | --- |
| `Pending` | 已选择，等待上传。 | 边框使用 dashed 样式。 |
| `Uploading` | 正在传输文件。 | 标题显示 shimmer；图片预览变暗。 |
| `Processing` | 上传完成，服务端正在处理。 | 标题显示 shimmer；图片预览变暗。 |
| `Failed` | 上传或处理失败。 | 边框与描述使用 destructive 语义色；无图片媒体也使用 destructive 色。 |
| `Complete` | 已准备好供用户使用。 | 普通完成状态。 |

状态文字应写入描述，不能只依赖边框颜色：

```rust
Attachment::new()
    .status(AttachmentStatus::Uploading)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("design-assets.zip"))
            .description(AttachmentDescription::new("上传中 · 68%"))
            .child(Progress::new("attachment-progress").value(68.)),
    )
```

`Progress::loading(true)` 可用于不确定进度；确定百分比时使用 `.value(...)`。Progress 是独立组件，因此应用可以按自己的请求模型更新数值或状态。

`AttachmentStatus` 还提供 `is_pending()`、`is_uploading()`、`is_processing()`、`is_failed()`、`is_complete()` 和 `is_in_progress()`，适合在应用状态层生成描述或决定操作按钮。

## 尺寸与方向

Attachment 实现 `Sizable`，支持 `xsmall()`、`small()`、默认 medium、`large()`，也可以使用 `with_size(Size::...)`：

```rust
Attachment::new()
    .xsmall()
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("compact.txt"))
            .description(AttachmentDescription::new("TXT · 4 KB")),
    );

Attachment::new()
    .large()
    .media(AttachmentMedia::new().src(preview_source))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("presentation.pptx"))
            .description(AttachmentDescription::new("演示文稿")),
    )
```

尺寸使用共享的 design scale；应根据界面密度选择语义尺寸，避免在应用里为每个附件写独立高度。

`Axis::Horizontal` 适合消息中的紧凑附件，`Axis::Vertical` 适合图片预览：

```rust
Attachment::new()
    .axis(Axis::Vertical)
    .media(AttachmentMedia::new().src(preview_source))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("preview.png"))
            .description(AttachmentDescription::new("PNG · 1280 × 720")),
    )
```

纵向模式下有 content 时根组件使用主题 scale 中的预览宽度，没有 content 时使用更紧凑的方形媒体尺寸；媒体区域保持 `aspect_ratio(1.)`。这些是可覆盖的默认布局，不应被当作组件外部的固定像素契约。

`AttachmentMedia` 也实现 `Sizable`，因此可以独立覆盖预览尺寸：

```rust
Attachment::new()
    .media(
        AttachmentMedia::new()
            .with_size(Size::Large)
            .src(preview_source),
    )
```

显式 media 尺寸优先于根 Attachment 的尺寸；其他 `Styled` refinement 仍会在默认值之后应用。

## 内容、长文件名与操作

`AttachmentTitle` 和 `AttachmentDescription` 默认单行并截断。长文件名应提供可见的描述或 Tooltip，避免把卡片横向撑开：

```rust
Attachment::new()
    .content(
        AttachmentContent::new()
            .title(
                AttachmentTitle::new("2026-08-25-very-long-export-name.json"),
            )
            .description(AttachmentDescription::new("JSON · 18.4 MB")),
    )
```

标题会自动单行截断。需要在 hover 或辅助说明中显示完整文件名时，应由应用在外层交互容器上组合 Tooltip，或把完整名称放入可见描述；只要内容需要自动继承状态，就应继续使用 `.title(...)` 和 `.description(...)`。

操作 slot 可以包含多个 Button：

```rust
Attachment::new()
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("failed-upload.zip"))
            .description(AttachmentDescription::new("上传失败，请重试")),
    )
    .actions(
        AttachmentActions::new()
            .child(Button::new("retry").small().label("重试"))
            .child(
                Button::new("remove")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .label("移除")
                    .tooltip("移除附件"),
            ),
    )
```

`AttachmentActions` 不定义专用的 `AttachmentAction`，因此可以直接复用 Button 的 variant、尺寸、disabled、loading、tooltip 和事件 API。

## 整卡点击

设置 `.id(...)` 和 `.on_click(...)` 可以让整张卡片响应点击（例如打开预览）。点击层绘制在 `AttachmentActions` 之下，操作按钮仍然独立可点：

```rust
Attachment::new()
    .id("design-attachment")
    .on_click(|_, window, cx| {
        // 打开预览。
    })
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("design-mockups.png"))
            .description(AttachmentDescription::new("PNG · 1.8 MB")),
    )
    .actions(
        AttachmentActions::new()
            .child(Button::new("remove").ghost().xsmall().icon(IconName::Close)),
    )
```

点击状态需要稳定标识，因此 handler 只在配合 `.id(...)` 时生效。可点击的卡片 hover 时会显示 muted 底色，让它读起来是可交互的。点击意味着什么——对话框、浏览器、文件预览还是选择——由应用决定。删除、重试等次要操作应留在 `AttachmentActions` 中，不要依赖整卡点击；同时把卡片的主操作以 `Button` 或 `Link` 的形式提供在键盘可达的位置——点击层只是指针便利，不参与焦点。

## 状态继承与局部覆盖

通过具名 `.title(...)` 和 `.description(...)` 添加的 child 会继承父级状态：

```rust
Attachment::new()
    .status(AttachmentStatus::Failed)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("archive.zip"))
            .description(AttachmentDescription::new("上传失败")),
    )
```

如果某个 child 的状态与父级不同，可以显式覆盖：

```rust
Attachment::new()
    .status(AttachmentStatus::Failed)
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("archive.zip"))
            .description(
                AttachmentDescription::new("文件已恢复")
                    .status(AttachmentStatus::Complete),
            ),
    )
```

显式 child 状态优先于继承状态。普通 `.child(AttachmentTitle::new(...))` 会擦除类型，因此不会自动继承父级状态；需要状态感知表现时使用具名 builder。

标题在 `Uploading` 和 `Processing` 时使用 `ShimmerText`。可以复用 `ShimmerStyle` 调整动画：

```rust
AttachmentTitle::new("design-assets.zip")
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(std::time::Duration::from_secs(3))
            .spread(0.45)
            .reverse(true)
            .once(true),
    )
```

## 分组

`AttachmentGroup` 是可横向滚动的附件行，需要稳定的 element id 保存滚动状态：

```rust
AttachmentGroup::new("message-attachments")
    .child(first_attachment)
    .child(second_attachment)
    .child(third_attachment)
```

当附件数量可能超出消息宽度时使用该组件。选择、拖拽排序、snap 或自定义滚动按钮属于应用容器，不由 `AttachmentGroup` 保存。

## 自定义样式与主题 token

根组件和所有公开 slot 都实现 `Styled`，调用方 refinement 会在默认布局后应用：

```rust
Attachment::new()
    .rounded(cx.theme().radius_lg)
    .bg(cx.theme().group_box)
    .border_color(cx.theme().border)
    .content(
        AttachmentContent::new()
            .gap_1()
            .title(
                AttachmentTitle::new("custom-theme.txt")
                    .text_color(cx.theme().foreground),
            )
            .description(
                AttachmentDescription::new("使用主题 token")
                    .text_color(cx.theme().muted_foreground),
            ),
    )
```

优先使用 `cx.theme()` 的语义颜色、圆角和共享尺寸；不要在调用点写固定 hex 颜色或按单个组件复制 spacing scale。可分别调整：

- `Attachment`：整体宽度、背景、边框、圆角、padding 和 gap。
- `AttachmentMedia`：预览尺寸、圆角、背景和 overlay。
- `AttachmentContent`：内容宽度、文字层级和元数据间距。
- `AttachmentTitle` / `AttachmentDescription`：截断、字体和语义颜色。
- `AttachmentActions`：操作间距、位置和按钮布局。
- `AttachmentGroup`：横向 gap、padding 和滚动容器样式。

## 组件边界

此 API 有意将以下职责留给组合层：

- 直接使用 `Button`，不增加 `AttachmentAction`，保留 Button 的完整 variant、尺寸、事件和可访问性选项。
- 整卡点击通过 `.id(...)` 加 `.on_click(...)` 提供；卡片只上报点击，打开对话框、浏览器、预览还是切换选择由应用决定。
- 直接使用 `Progress`，不增加附件专属进度包装。
- `AttachmentGroup` 只负责横向间距和 overflow，不保存选择、拖拽、snap 或业务数据。

这些边界让附件仍然保持可组合，应用可以根据产品行为选择普通 Button、Link、Popover 或自己的容器。

## 可访问性

- 文件名和状态应以文本表达；失败状态不能只靠 destructive 边框或文字颜色。
- icon-only action 应提供可见的 `.label(...)` 或其他可读名称；tooltip 只作为“移除附件”“重试上传”等补充提示。
- `Progress` 的百分比和不确定状态应通过可读文本或其他状态说明补充，不能只展示一条进度条。
- 纵向图片预览上的 overlay Button 仍应可聚焦，不要让装饰层遮蔽操作目标。
- `AttachmentGroup` 的横向滚动应能够通过键盘和系统滚动输入访问；不要把唯一入口做成 hover 才出现的按钮。
- 上传和处理中的 shimmer 会遵循系统 reduced motion；应用自定义 overlay 动画时也应提供静态结果。

## API 参考

### `Attachment`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建 `Complete`、横向、medium 尺寸的附件。 |
| `id(ElementId)` | 设置整卡点击层的稳定标识。 |
| `on_click(handler)` | 整卡点击；需配合 `id(...)`，绘制在 actions 之下。 |
| `status(AttachmentStatus)` | 设置根生命周期状态。 |
| `axis(Axis)` | 设置 `Horizontal` 或 `Vertical` 布局。 |
| `media(AttachmentMedia)` | 设置预览 slot。 |
| `content(AttachmentContent)` | 设置元数据 slot。 |
| `actions(AttachmentActions)` | 设置操作 slot。 |
| `xsmall()` / `small()` / `large()` | 通过 `Sizable` 选择语义尺寸。 |
| `Styled` | 调整根 surface 和布局。 |

### `AttachmentMedia`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建空媒体 slot。 |
| `src(ImageSource)` | 设置图片预览源。 |
| `overlay(element)` | 在媒体区域上方居中添加 overlay。 |
| `with_size(Size)` | 覆盖从根组件继承的媒体尺寸。 |
| `child(element)` | 添加图标或其他媒体 child。 |
| `Styled` | 调整媒体背景、圆角和尺寸。 |

### `AttachmentContent`

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建空元数据 slot。 |
| `title(AttachmentTitle)` | 添加会继承状态的标题。 |
| `description(AttachmentDescription)` | 添加会继承状态的描述。 |
| `child(element)` | 添加任意自定义内容，不参与状态继承。 |
| `Styled` | 调整内容布局和文字 refinement。 |

### `AttachmentTitle` / `AttachmentDescription`

| 方法 | 说明 |
| --- | --- |
| `new(text)` | 创建单行标题或描述。 |
| `status(AttachmentStatus)` | 显式覆盖从父级继承的状态。 |
| `with_shimmer_style(ShimmerStyle)` | 自定义标题 loading shimmer；仅 `AttachmentTitle` 提供。 |
| `Styled` | 调整文字样式、颜色、截断等。 |

### `AttachmentActions` / `AttachmentGroup`

| 类型 | 方法 | 说明 |
| --- | --- | --- |
| `AttachmentActions` | `new()` / `child(element)` | 创建操作 slot 并组合 Button 或其他控件。 |
| `AttachmentGroup` | `new(id)` / `child(element)` | 创建带稳定 id 的横向滚动附件组。 |
| 两者 | `Styled` | 调整间距、位置和容器布局。 |

### 类型链接

- [Attachment]
- [AttachmentStatus]
- [AttachmentMedia]
- [AttachmentContent]
- [AttachmentTitle]
- [AttachmentDescription]
- [AttachmentActions]
- [AttachmentGroup]

[Attachment]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.Attachment.html
[AttachmentStatus]: https://docs.rs/gpui-component/latest/gpui_component/attachment/enum.AttachmentStatus.html
[AttachmentMedia]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentMedia.html
[AttachmentContent]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentContent.html
[AttachmentTitle]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentTitle.html
[AttachmentDescription]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentDescription.html
[AttachmentActions]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentActions.html
[AttachmentGroup]: https://docs.rs/gpui-component/latest/gpui_component/attachment/struct.AttachmentGroup.html

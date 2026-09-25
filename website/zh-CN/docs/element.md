---
title: Element
description: 了解 GPUI 的 Element 树与底层渲染生命周期。
order: -2.7
---

# Element

**Element** 是 GPUI 为当前一帧构建的 Element 树节点。Element 负责布局、准备命中测试，并把像素[绘制](./paint)到 Window。下一帧开始前，GPUI 会释放整棵 Element 树及其中注册的帧级 callback，再根据应用的最新状态重新构建。

大多数应用代码只需要组合 GPUI 和 GPUI Kit 提供的 Element：

```rust
div()
    .flex()
    .items_center()
    .gap_2()
    .child(Icon::new(IconName::Search))
    .child("Search")
```

这段代码构建了一棵 Element 树，并没有实现 GPUI 底层的 `Element` trait。

## `Element`、`IntoElement` 与 `AnyElement`

它们承担不同的职责：

| 类型 | 作用 |
| --- | --- |
| `Element` | 实现底层的布局与绘制生命周期。 |
| `IntoElement` | 把值转换为具体的 `Element`，因此字符串、组件、Entity 等值都可以传给 `.child(...)`。 |
| `AnyElement` | 擦除具体 Element 类型，适合异构集合、不同类型的条件分支、slot 和需要存储的子元素。 |

如果所有分支的类型相同，保留具体类型即可。只有边界需要容纳不同 Element 类型时才做类型擦除：

```rust
fn status_icon(online: bool) -> AnyElement {
    if online {
        Icon::new(IconName::CircleCheck).into_any_element()
    } else {
        div().child("Offline").into_any_element()
    }
}
```

GPUI Kit 会在可选 slot、表格单元格以及返回不同 UI 类型的函数中这样使用 `AnyElement`。如果调用方不需要类型擦除，通常用 `IntoElement` 作为 API 边界。

## 三个阶段

GPUI 按顺序调用 `Element` 的三个 trait 方法。第一与第二次调用之间，布局引擎还会求出最终位置：

<figure class="element-lifecycle" aria-label="GPUI Element 每帧生命周期">
  <div class="element-lifecycle__intro"><code>Render::render</code> 构建当前帧的 Element 树</div>
  <ol class="element-lifecycle__steps">
    <li><span class="element-lifecycle__number">01</span><strong>request_layout</strong><p>注册 Style 和子布局；返回 <code>LayoutId</code> 与 <code>RequestLayoutState</code>。</p></li>
    <li><span class="element-lifecycle__number">02</span><strong>prepaint</strong><p>使用最终 bounds 准备几何和 hitbox；返回 <code>PrepaintState</code>。</p></li>
    <li><span class="element-lifecycle__number">03</span><strong>paint</strong><p>绘制已准备好的内容，并注册当前帧的输入 listener。</p></li>
  </ol>
  <figcaption>Taffy 在 <code>request_layout</code> 与 <code>prepaint</code> 之间计算 <code>Bounds&lt;Pixels&gt;</code>。</figcaption>
</figure>

下一帧前，Element 树和帧级 listener 会被释放。`RequestLayoutState` 与 `PrepaintState` 只在这一轮调用中向后传递，并非长期缓存。

### 如何判断代码放在哪个阶段

先看这段代码需要什么信息，以及它要产生什么结果：

| 工作 | 所属阶段 | 原因 |
| --- | --- | --- |
| 决定宽高、padding、子元素布局；当内容尺寸影响布局时测量内容 | `request_layout` | 布局引擎必须先拿到这些输入，才能求出 bounds。测量闭包可以使用提供的可用宽度，但此时仍不知道 Element 的最终位置。 |
| 根据最终 bounds 计算文字行、Path、裁剪区域或 hitbox；决定虚拟列表中哪些子元素可见 | `prepaint` | 这是首次拿到最终像素矩形的阶段。把绘制或输入处理还要用到的结果存入 `PrepaintState`。 |
| 提交 quad、字形、Path 或图片；注册当前帧的指针、滚轮或键盘 listener | `paint` | 此时 Scene 和派发树已经可以接收绘制命令与 listener，应复用准备好的几何结果。 |

例如，一个可点击的图表先请求图表及子元素的布局。尺寸确定后，再把数据点映射到像素坐标，并为可交互区域插入 hitbox。最后根据这些坐标画出 Path，并注册指针 listener。如果标签宽度会**影响图表自身的尺寸**，就在 `request_layout` 测量；如果只需确定标签在**已定尺寸的图表内部**放在哪里，就在 `prepaint` 准备。

可以用一个问题判断边界：**这项计算会改变 Element 请求的尺寸吗？**会，就放在 layout 或其测量闭包；只依赖最终 bounds，就放在 `prepaint`；只改变像素或本帧 handler、不改变几何，就放在 `paint`。长期模型数据和订阅不属于这三个阶段，应由 [Entity](./entity) 持有。

### `request_layout`

通过 `window.request_layout` 注册 Element 的 [Style](./style) 和子布局节点。GPUI 使用 Taffy 布局引擎，在所有布局请求完成后计算尺寸和位置。

这个阶段返回 `LayoutId`，以及后续阶段需要的 `RequestLayoutState`。此时不要假设最终 `Bounds` 已经确定。

### `prepaint`

此时 GPUI 会传入布局完成后的 `Bounds`。在这里进行文字 shaping、几何计算、插入 hitbox、对子元素执行 prepaint，并准备 `paint` 所需的数据。

这些数据通过 `PrepaintState` 返回。命中测试应在这里准备，因为 GPUI 需要在绘制前建立当前帧的空间信息和派发信息。

### `paint`

使用前面准备好的状态绘制 quad、文字、路径或图片；自定义 Element 需要输入处理时，也在这个阶段注册帧级 handler。这里应复用前面算好的几何信息，不要再次进行布局计算。

`RequestLayoutState` 会传给 `prepaint` 和 `paint`；`PrepaintState` 会传给 `paint`。长期存在的应用状态应放在 [Entity](./entity) 中；少量需要跨帧保存的 Element 状态可以通过 [ElementId](./element_id) 建立关联。

### 从一个完整的底层 Element 入手

以下不可见的输入区域占满父节点。它刻意只插入 hitbox，不注册事件：hitbox 是输入命中的几何信息，本身并不是 click callback。

```rust
use gpui_kit::*;

struct EventSurface;

impl IntoElement for EventSurface {
    type Element = Self;
    fn into_element(self) -> Self::Element { self }
}

impl Element for EventSurface {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    fn request_layout(
        &mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>,
        window: &mut Window, cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>, _: &mut Self::RequestLayoutState,
        window: &mut Window, _: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>,
        _: Bounds<Pixels>, _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState, _: &mut Window, _: &mut App,
    ) {}
}
```

`request_layout` 先将 `Style` 提交给 Taffy；有子元素时，先取得子元素的 `LayoutId`，再传给 `window.request_layout`。需要按内容测量大小时，可使用 `window.request_measured_layout`。`prepaint` 拿到最终 bounds，插入 hitbox，并把返回的 `Hitbox` 交给 `paint` 使用。`paint` 再注册当前帧的输入 listener 或提交绘制命令。[GPUI Kit 的 CarouselScrollMask](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/carousel/scroll_mask.rs) 正是从这种结构出发，利用 `Hitbox::should_handle_scroll` 判断滚动手势是否被前面的区域挡住。

`HitboxBehavior::Normal` 参与命中但不遮挡后方 hitbox；`BlockMouse` 遮挡后方鼠标及滚动处理，`BlockMouseExceptScroll` 保留后方滚动。绘制、裁剪和命中是三件事：画出形状不会自动建立 hitbox，插入 hitbox 也不会自动画出形状。

### 让输入区域可见，并响应按下事件

第一个例子只是 trait 骨架：把它作为子元素挂载后，仍然看不到任何像素。要运行接下来的完整示例，请用下面的代码替换现有 [`examples/hello_world/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/hello_world/src/main.rs) 的内容。父元素给输入区域明确的 `240 × 96` 像素空间；区域请求的相对宽高才有了可参照的尺寸。整个练习沿用仓库现有示例，无须增加依赖。

```rust
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

struct SurfaceDemo {
    presses: usize,
}

impl Render for SurfaceDemo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .w(px(240.))
                    .h(px(96.))
                    .child(EventSurface {
                        owner: cx.entity().clone(),
                        color: cx.theme().primary,
                    }),
            )
            .child(format!("Pointer presses: {}", self.presses))
    }
}

struct EventSurface {
    owner: Entity<SurfaceDemo>,
    color: Hsla,
}

impl IntoElement for EventSurface {
    type Element = Self;

    fn into_element(self) -> Self::Element { self }
}

impl Element for EventSurface {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        window.paint_quad(fill(bounds, self.color));

        let hitbox = hitbox.clone();
        let owner = self.owner.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase.bubble()
                && event.button == MouseButton::Left
                && hitbox.is_hovered_at(event.position, window)
            {
                owner.update(cx, |view, cx| {
                    view.presses += 1;
                    cx.notify();
                });
            }
        });
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| SurfaceDemo { presses: 0 })
        })
        .expect("failed to open window");
    });
}
```

在仓库根目录运行 `cargo run -p hello_world`。窗口会显示 `240 × 96` 的主题色矩形和 `Pointer presses: 0`；每次在矩形内按下鼠标左键，计数依次变成 `1`、`2`，按在矩形外则不会变化。这只是展示指针输入的教学示例；它没有键盘激活、Focus、光标样式或无障碍角色。应用中的常规操作控件应优先使用现成的 [Button](../component/button)，只有确实需要控制底层生命周期时才采用这种写法。

可以按四步理解这个例子：

1. **挂载并确定尺寸。** 每次 View 渲染，`Render::render` 都创建一个新的 `EventSurface`。父元素给出确定的宽高，子元素在两个方向都请求 `relative(1.)`。如果父元素没有可用尺寸，这个百分比可能解析成零尺寸或意外的区域。
2. **准备输入几何。** `prepaint` 收到布局计算后的 `Bounds<Pixels>`，插入与矩形一致的 hitbox。`PrepaintState = Hitbox` 将它传给 `paint`；它属于当前帧的命中测试数据。
3. **绘制并注册 listener。** View 读取 `cx.theme().primary`，把这个 `Hsla` 传给 Element；`paint_quad(fill(bounds, self.color))` 再使用当前主题的颜色画出同一矩形。`on_mouse_event` 按绘制注册顺序把当前帧的 listener 加入窗口鼠标监听列表，并不会将其挂在当前 dispatch node 上；`&MouseDownEvent` 决定事件类型，冒泡阶段、鼠标左键和 hitbox 判断共同排除无关输入。闭包捕获克隆后的 handle，因为它需要在这次 `paint` 返回后继续存在。
4. **修改长期状态。** `Entity<SurfaceDemo>` 在 Element 树重建后仍然存在。`owner.update` 修改 `presses`，`cx.notify()` 使 View 再次渲染。下一次 `render` 会生成新的 `EventSurface` 与计数文字；两个阶段关联状态都不是应用计数器。

学习时可以在同一个文件中逐项改动，每次都重新运行 `cargo run -p hello_world`：

1. 注释掉 `window.paint_quad(...)`。矩形变得不可见，但原区域内的按下事件仍会增加计数。输入由 hitbox 而非像素决定。
2. 恢复绘制。要移除 hitbox，需**同时**把 `PrepaintState` 改为 `()`，并从 `paint` 中移除 listener。此时矩形可见，但不会响应输入。绘制本身不提供命中区域或回调。
3. 恢复完整示例，把父节点的 `.w(px(240.))` 改成 `.w(px(120.))`。可见区域和可点击区域会一起变窄，因为两者都使用布局求出的 bounds。

| 观察结果 | 排查位置 |
| --- | --- |
| 看不到彩色区域 | 父节点是否给出确定尺寸、`request_layout` 是否返回布局 ID，以及 `paint_quad` 是否还在。 |
| 区域可见，但按下无效 | `prepaint` 是否插入相同 bounds 的 hitbox；`paint` 是否注册 `on_mouse_event`；listener 是否检查冒泡阶段和鼠标左键。还需检查前方 hitbox 是否使用 `BlockMouse` 或 `BlockMouseExceptScroll`；普通 `Normal` hitbox 本身不会阻断后方 hitbox。 |
| 事件已触发，但文字不更新 | listener 是否更新 `Entity<SurfaceDemo>`，并在 update 闭包内调用 `cx.notify()`。 |

前两项改动是观察现象的实验，不是另两种完整实现；继续阅读前请恢复完整代码。

这个例子不需要 `ElementId`：计数由 Entity 持有，而 hitbox 和 listener 每帧重建。只有底层 Element 自身需要 keyed state 或无障碍 identity 时才加入稳定 ID。带子元素的区域还要把子布局 handle 放入 `RequestLayoutState`，在父元素 bounds 确定后 prepaint 子元素，再按顺序绘制；真实实现可参考 [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs)。可复用的输入原语还必须明确 Focus、键盘与无障碍契约；单靠鼠标 listener 不会自动具备这些能力。

### 在自定义 Element 中组合一个子元素

上面的指针区域没有子布局需要传递。接下来的完整示例增加**一个**子元素。把以下代码放入现有 [`examples/hello_world/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/hello_world/src/main.rs)，替换上一个练习的代码，然后在仓库根目录运行 `cargo run -p hello_world`。它继续使用原来的 package 和依赖。

```rust
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

struct ChildDemo;

impl Render for ChildDemo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().items_start().gap_2().child(
            ChildFrame {
                child: Some(div()
                    .w(px(160.))
                    .h(px(32.))
                    .bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
                    .child("Child: 160 x 32")
                    .into_any_element()),
                background: cx.theme().secondary,
                accent: cx.theme().primary,
            },
        )
    }
}

struct ChildFrame {
    child: Option<AnyElement>,
    background: Hsla,
    accent: Hsla,
}

impl IntoElement for ChildFrame {
    type Element = Self;

    fn into_element(self) -> Self::Element { self }
}

impl Element for ChildFrame {
    type RequestLayoutState = AnyElement;
    type PrepaintState = Bounds<Pixels>;

    fn id(&self) -> Option<ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.child.take().expect("layout requested once per frame");
        let child_layout = child.request_layout(window, cx);
        let mut style = Style::default();
        style.padding = Edges::all(px(12.).into());
        let layout = window.request_layout(style, [child_layout], cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        child.prepaint(window, cx);
        Bounds {
            origin: bounds.origin,
            size: size(px(4.), bounds.size.height),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        accent_bounds: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.paint_quad(fill(bounds, self.background));
        window.paint_quad(fill(*accent_bounds, self.accent));
        child.paint(window, cx);
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| ChildDemo)
        })
        .expect("failed to open window");
    });
}
```

窗口中会出现标有 `Child: 160 x 32` 的彩色子元素，四周留出 12 像素内边距，外框左侧还有一条窄色条。外框没有固定高度：Taffy 测量 32 像素高的子元素后，加上上下内边距。在这个不受约束的例子中，外框高度为 56 像素。把子元素的 `.h(px(32.))` 改为 `.h(px(64.))`，重新运行，外框就会增高到 88 像素。如果祖先节点限制高度，结果也会受该限制影响。

沿着所有权与阶段的顺序阅读代码：

1. `Render::render` 创建含有一个 `AnyElement` 的 `ChildFrame`。`Option` 允许 `request_layout` 把子元素移入返回的 `RequestLayoutState`；该 Element 在一帧内只请求一次布局。下一次渲染会创建新的 `ChildFrame`。
2. 子元素先请求自己的 `LayoutId`。父元素再把它传给 `window.request_layout(style, [child_layout], cx)`，从而在布局树中连接这两个节点。若传入空的子节点列表，父元素测量和定位时就不会考虑这个彩色子元素。
3. Taffy 完成布局后，`prepaint` 调用 `child.prepaint(window, cx)`，让子元素获得求出的最终位置，并准备自己的 hitbox 或几何。父元素同时根据自身 bounds 计算一条 4 像素宽的色条，把该矩形作为 `PrepaintState` 返回。
4. `paint` 先画外框背景和色条，再调用 `child.paint(window, cx)`，让子元素显示在背景上方。`AnyElement` 保存其内部阶段状态，但父元素仍须依序调用它的各阶段。

外框不处理输入，因此没有自己的 hitbox。内置的子 `div` 负责绘制自己的背景和文字。任何一次绘制调用都无法补回缺失的子布局；布局、prepaint 与 paint 必须都包含它。

| 观察结果 | 排查位置 |
| --- | --- |
| 看到了外框，却没有子元素 | 是否把子布局 ID 传给 `window.request_layout`，并依次调用了子元素的 `prepaint` 和 `paint`。若外框背景盖住子元素，还要检查绘制顺序。 |
| 子元素没有处于内边距以内 | 父元素的 style 是否包含 `Edges::all(px(12.).into())`，并使用返回的子布局 ID。不要在 `paint` 中手动偏移子元素；位置由 Taffy 计算。 |
| 外框高度不随子元素改变 | 检查外框或祖先节点是否固定高度或裁剪内容，以及是否将子布局 ID 连接到父布局。 |

这个例子把子元素保存在 `RequestLayoutState` 中，因为后续两个阶段还要使用它；`PrepaintState` 只保存计算好的色条矩形。两者都不是持久化应用状态；这类数据应交给 [Entity](./entity)。

### `GlobalElementId` 与跨帧状态

`Element::id()` 返回局部 `ElementId`。GPUI 将它与上层 keyed 元素的 ID 组合成 `GlobalElementId`，传入三个阶段。底层 Element 可以用它和 `window.with_element_state` 保留少量跨帧状态；CarouselScrollMask 就这样保存连续滚动信息。ID 在最近的 keyed ancestor 下必须唯一；可重排的项目应使用领域 ID，不能用列表下标。没有 ID 时三个阶段收到 `None`。应用数据和订阅依然应由 Entity 持有。

### TextSystem 与低阶文字绘制

[`TextSystem`](./text-system) 负责字体查询、字形 shaping、度量和缓存。可通过 `cx.text_system()` 获取，Window 也有针对窗口的文字系统。文字宽度受到字体、字号、shaping 和可用宽度影响，因此复杂文本 Element 可能在布局阶段使用测量闭包，在 `prepaint` 算行、选区及 hitbox，在 `paint` 绘制准备好的文字。三个阶段要使用相同字体参数，否则光标和选区会错位。普通文字应直接使用现有文字 Element；只有选区、内联对象或编辑器等需求才需要下沉。[GPUI Base TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) 和 [GPUI Kit 输入框底层 Element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) 分别展示这条管线在富文本和编辑器中的应用。

## 什么时候实现 `Element`

只有现有 Element 无法表达需求时才实现 `Element`，例如：

- 需要进行文字 shaping，并绘制选区和光标的代码编辑器或文本输入框；
- 具有自定义几何图形的图表或 canvas；
- 自定义布局算法；
- 需要直接控制布局、hitbox 与绘制的高性能底层原语。

GPUI Kit 的输入框使用自定义 `Element`，因为它需要对文字进行 shaping，注册输入 handler，并绘制选区和光标。GPUI 的 `Svg`、`Img`、列表和 canvas 也使用同一套生命周期。大部分应用 UI 则通过组合已有 Element 完成，只在不同分支返回不同类型时转换成 `AnyElement`。

编写可复用 UI 组件时，先使用 [`RenderOnce`](./render-once)。编写由 Entity 持有状态的 UI 时，使用 [`Render`](./render)。只有需要直接控制渲染管线时，才下沉到 `Element`。

### GPUI Kit 源码中的选择

| 案例 | 为什么仅靠组合不够 | Element 负责什么 |
| --- | --- | --- |
| [Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) | 光标、选区、折行和指针到文字位置的映射必须使用同一份文字度量结果。 | 文字布局、hitbox、输入处理和绘制顺序。 |
| [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) | 富文本和可选择范围的测量、裁剪都依赖最终 bounds。 | 文字布局状态、hitbox、选区表面与准备好的绘制数据。 |
| [CarouselScrollMask](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/carousel/scroll_mask.rs) | 手势区域必须独立占据视口，不能跟着被滚动的内容移动。 | 占满视口的布局、hitbox、鼠标与滚轮派发；它本身不画任何东西。 |
| [Plot 折线](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/shape/line.rs) | 数据点需要直接构建并三角化 Path，但无须自定义布局和输入。 | 在上层图表中绘制 Path；折线本身不必实现完整 `Element`。 |

这些案例说明 `Element` 的价值在于控制某个阶段边界，并不等于“有自定义绘图就要实现 Element”。Carousel 表明 Element 可以什么也不画；Plot 则表明自定义绘图可以由 canvas 或上层 Element 完成，无须让每条曲线都是 Element。

### 性能取决于各阶段的职责

**让高效路径成为 API 的自然用法。**GPUI 将 Element 分成布局、预绘制和绘制阶段，让作者在尺寸明确时测量、一次准备几何，并在绘制时复用。GPUI Kit 也按这个原则设计组件：可能在大量元素中重复的成本，应有明确的所有者和失效规则。目标是让普通组件用法在设计上避免重复工作，而不是要求每个调用者事后自行补上缓存。

每个需要绘制的帧都可能重建树并再次执行这些方法。`request_layout` 应集中处理 Style 与布局节点，除非固有尺寸的测量确实需要，否则不要在这里 shaping 文字或三角化路径。`prepaint` 计算一次几何，借 `PrepaintState` 交给 `paint`；不要在 paint 重算。对未变化的 Path，GPUI Kit Plot 用 keyed window state 和 shape key 保留三角化结果。订阅和长期应用数据属于 Entity，不属于帧级 associated state。最后，昂贵绘制之前要裁剪与剔除：TextView 和 Input 根据最终 bounds 计算可见内容，而不是盲目绘制整份文档。

## GPUI 如何驱动 Element

底层调用约束由 `Drawable<E>` wrapper 保证。它按 `Start → RequestLayout → LayoutComputed → Prepaint → Painted` 迁移，乱序调用就是错误。这解释了为什么 GPUI 会向后传递两类 associated state，而不是让 Element 在 paint 时重新发现全部数据；也解释了为什么不能在 `request_layout` 中调用绘制方法：Window 此时还没有进入绘制阶段。

### 布局树、派发树与 Scene 是不同的结构

`request_layout` 向布局引擎提交节点并返回 `LayoutId`。Taffy 求出最终布局后，`prepaint` 得到像素 bounds。GPUI 在 prepaint 时给 Element 建立 dispatch node；hitbox 则单独记录，供命中测试使用。`paint` 会激活对应的 dispatch node，让在这条路径上注册的 Action 与键盘 listener 派发。`window.on_mouse_event` 注册的鼠标 listener 则进入当前帧的窗口列表：捕获阶段按注册顺序调用，冒泡阶段反向调用；handler 需检查相关 hitbox。绘制命令进入 Scene。调用 `paint_path` 只会改变 Scene，不会自动创建派发节点或 hitbox；反过来，Carousel 的 mask 建立输入区域和 listener，却不绘制可见图形。

这种分离让自定义 Element 只承担真正需要的工作。装饰图形可能只需 canvas 和 Scene 命令；可聚焦控件还需协调派发、hitbox、Focus 和无障碍。虚拟列表要决定测量和绘制哪些子元素，而不只是画一个巨大的矩形。

### 身份是一条路径，而不是当前帧值的指针

`Element::id()` 返回 `ElementId` 时，GPUI 把它追加到当前 keyed ancestor 路径，生成传入三个阶段的 `GlobalElementId`。Rust Element 值在后续帧会重建，它的地址并非稳定身份。`window.with_element_state(global_id, ...)` 可以在连续绘制的帧之间传递少量有类型的状态；`window.use_keyed_state(key, cx, init)` 则在当前 Element 命名空间的 key 下创建 Entity，并观察它，使其变化可通知所属 View。这些机制都要求 key 稳定；项目可能移动时不能用列表下标。

### 无障碍也依赖最终几何

无障碍功能启用时，同时具有 ID 和 `a11y_role()` 的 Element 可以建立 AccessKit 节点。GPUI 在 prepaint 根据最终布局设置节点 bounds，调用 `write_a11y_info`，并可以加入 synthetic children。单纯绘制像素不会提供角色、名称、值或操作。GPUI Kit 标准交互组件已提供相应语义；自定义底层控件需要自行定义这份契约。具体 API 见 [Accessibility](./accessibility)。

## Identity 与交互

下面几个概念处在不同边界：

- [`ElementId`](./element_id) 在所在的 keyed scope 内标识一个 Element。GPUI 使用它的全局形式把跨帧状态和 retained work 连接起来。
- 对 `InteractiveElement` 调用 `.id(...)` 会返回 `Stateful<E>`。这个 wrapper 让需要稳定 Element identity 的 API 可以使用对应状态。
- `InteractiveElement` 提供 GPUI 的标准交互机制，包括 hitbox、鼠标 listener、Focus 跟踪、Key Context 与 Action handler。自定义 `Element` 不会自动获得这些能力。

`InteractiveElement` 要求实现者暴露 `Interactivity`，提供 `on_mouse_down`、`on_key_down`、`track_focus`、`key_context`、`on_action` 及 hover 样式等方法。调用 `.id(...)` 后获得 `Stateful<Self>`，它实现 `StatefulInteractiveElement`，提供依赖稳定身份的 role、无障碍名称、tooltip 和 click 等 API。`IntoElement` 只负责把值转成 Element，不会自动获得上述交互能力。普通组件通常使用 `#[derive(IntoElement)]` 与 `RenderOnce`；自行实现底层 `Element` 时，`IntoElement` 通常直接返回 `self`。

获得这些 API 有两条路径。组合普通交互区域时，直接使用已有元素：

```rust
div().id("result-row")
    .aria_label("Search result")
    .on_click(|_, _, _| {})
```

`.id(...)` 将 Rust 类型从 `Div` 变成 `Stateful<Div>`，依赖身份的方法才随之可用。重复行应按业务对象生成各自的 ID，而不是共用这里的字面量。`on_click` 只注册回调，不会自动更新 Entity 状态。

若 GPUI Kit 组件本身也要公开同一套 fluent 方法，应把交互状态委托给内部基础元素。[Radio 的实现](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/radio.rs)采用如下模式（省略无关字段）：

```rust
impl InteractiveElement for Radio {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Radio {}
```

`StatefulInteractiveElement` 是标记 trait：默认方法写入返回的 `Interactivity`；这个空实现本身并不会创建 ID 或 hitbox。`Radio::new(id)` 把稳定 ID 传给 Base 基础元素，由它渲染真正的交互区域。如果组件无法保证稳定身份和底层交互元素，就只应公开实际支持的方法。前面 `EventSurface` 那样的原始自定义 `Element` 不会自动获得这些 trait；要么明确提供并驱动 `Interactivity`，要么组合已有交互元素。

## `canvas`：用两个回调进入绘制阶段

GPUI 的 `canvas(prepaint, paint)` 本身就是一个 `Element`。它根据自己的 `Styled` 属性请求布局，再用最终 bounds 调用一次 `FnOnce` prepaint callback。这个 callback 返回任意临时值 `T`，GPUI 随后把它传给 `FnOnce` paint callback。这是进入 Element 生命周期的轻量方式，不是 HTML canvas，也不是跨帧保留的绘图表面。

```rust
canvas(
    move |bounds, _, _| {
        let mut line = PathBuilder::stroke(px(1.));
        line.move_to(bounds.origin);
        line.line_to(point(bounds.right(), bounds.top()));
        line.build().ok()
    },
    move |_, path, window, _| {
        if let Some(path) = path {
            window.paint_path(path, color);
        }
    },
)
.w_full()
.h(px(1.))
```

这里的 prepaint 结果是 `Option<Path<Pixels>>`，只在当前帧存在。[GPUI Kit 虚线 Separator](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/separator.rs) 在 canvas 的最终 bounds 中绘制路径；[环形 Progress](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/progress/progress_circle.rs) 则把 prepaint 计算的半径交给 paint。在 `Render` 或 `RenderOnce` 中仅需添加一处绘图 callback 时使用它。`PathBuilder`、SVG Path 写法和路径缓存的例子见 [Paint](./paint)。`canvas` 自身没有子元素、hitbox、Focus 跟踪或稳定 Element ID；如果这些职责必须协同，应实现 `Element`，或在 canvas 外组合标准交互 Element。

如果 `div()` 等现有 Interactive Element 已能满足需要，直接组合它。自定义底层原语需要标准交互时，可以像 GPUI 内置 Element 一样，嵌入或委托给 GPUI 的 `Interactivity`。如果自行实现 hitbox 与输入事件注册，也要自行正确处理派发、裁剪、cursor 行为和无障碍信息。

:::info
`Element::id()` 返回 `ElementId` 不只是给像素加一个标签，它会建立跨帧的稳定 identity。ID 在最近的 keyed ancestor 中必须唯一；只有 Element 或附加行为确实需要 identity 时才添加 ID。
:::

[Element]: https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/trait.Element.html
[IntoElement]: https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/trait.IntoElement.html
[AnyElement]: https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.AnyElement.html

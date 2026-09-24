---
title: View Cache
description: 复用未变化的 GPUI View 子树，并区分 View 缓存、元素状态、绘图缓存与虚拟列表。
order: -2.633
---

# View Cache

GPUI 把应用状态保存在 [Entity](./entity) 中，但窗口每次绘制通常会重新构建[元素树](./element)。保留 `Entity<T>` 并不等于保留上次的元素树。窗口重绘时，普通子 View 即使自身状态没变，也可能再次执行 `Render::render`。缓存 View 则会复用上一帧选定的记录，其中包括输入处理器。

当前 GPUI 没有需要构造的公开 `ViewCache` 类型。真正的 View 缓存 API 是 **`Entity<T>::cached(style)`**（或 **`AnyView::cached(style)`**），它为一个由 Entity 支撑的子树建立缓存边界。GPUI Kit 还使用范围更窄的缓存，它们节省的是不同的成本。

| 机制 | 复用或省去的工作 | 生命周期与所有者 |
| --- | --- | --- |
| `Entity::cached(style)` | 未变化 View 的 render、子元素 layout/prepaint 与 paint 工作 | GPUI 的窗口缓存，按 Entity View 及元素路径索引 |
| [`Window::use_keyed_state`](./window) | 重建后的元素所需的少量状态或计算结果 | 稳定 [`ElementId`](./element_id) 下的窗口元素状态 |
| 模型持有的缓存 | 派生值、测量结果或绘图资源 | 所属 `Entity<T>`；由应用定义失效条件 |
| `VirtualList` | 构建屏幕外的行 | 只处理可见范围；滚动位置由单独的 handle 保留 |

这些机制可以组合。例如缓存的面板里可以包含虚拟列表，可见的图表行也可以复用已三角化的路径。虚拟列表**不会**缓存所有行的 View；`use_keyed_state` 也**不会**跳过 View 的 `render`。

## 缓存由 Entity 支撑的子树

把子 Entity 保存在父 View 中，让它跨 render 存活；嵌入时传入布局样式：

```rust
use gpui_kit::*;

struct Workspace {
    panel: Entity<ResultsPanel>,
}

struct ResultsPanel {
    title: SharedString,
}

impl Render for ResultsPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.title.clone())
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.panel.clone().cached(StyleRefinement::default().size_full()))
    }
}
```

构造 `Workspace` 时用 `cx.new(...)` 创建 `panel`（详见 [Context](./context)），然后将它保存在字段中。如果在 `render` 里每次创建新 Entity，它就会获得新身份，状态和热缓存都会丢失。`style` 是缓存 View 的**外层布局约定**。GPUI 要先布局这个盒子，才能决定是否复用内容；它无法向尚未渲染的子树询问固有尺寸。因此要让父级提供确定尺寸：在有边界的父级中使用 `size_full()`，或给出明确宽高。需要由内容决定尺寸时，直接用 `.child(self.panel.clone())` 嵌入，不使用缓存。

父级保存类型擦除的面板时，可用 `AnyView::cached(style)`，GPUI Kit 的 Dock 就采用这种形式。[`RenderOnce`](./render-once) 值和任意 `ViewElement` 不能使用这个公开 API：它们没有 Entity 的通知机制来使冻结的子树失效。

## GPUI 何时复用？

命中缓存时，GPUI 不会调用该子 View 的 `render`，而是将上一帧子树的 prepaint 和 paint 记录重放到当前帧。其中包括命中区域、事件派发节点、焦点状态、鼠标监听器、输入处理器和绘制场景，因此缓存区域仍可交互。原来的元素值并不会作为一棵永久元素树保存。重用子树中的事件处理器仍会在输入发生时执行；省去的是重新构建和绘制它们的工作。

命中要求 Entity 保持干净，同时缓存 View 的**边界、内容裁剪区域及继承的文本样式**与记录的帧一致。强制刷新时 GPUI 也会绕过复用；检查器选取元素时可以禁用缓存。条件不满足，就会重新 render、布局子树，并记录新缓存。

| 变化 | 这个缓存边界的行为 |
| --- | --- |
| 兄弟元素或父 View 重绘，而此子 View 及其盒子没变 | 父级仍构建自己的树；缓存子级可被重放。 |
| 子 View 改变并调用 `cx.notify()` | GPUI 标记该 View 为脏，重建并更新缓存。 |
| 后代 View 改变并发出通知 | GPUI 将其祖先 View 路径标脏，使缓存边界重新构建。 |
| 缓存盒子改尺寸、裁剪区域改变，或继承的文本样式改变 | 此缓存条目不命中，GPUI 重建子树。 |
| 子 View 被移除，或其身份、路径改变 | 旧位置的缓存子树不能用于新位置。 |

在事件处理器或[任务](./task)中修改状态；影响可见输出时，对相应 Entity 调用 `cx.notify()`。如果子 View 的输出依赖 [Global](./global)，应观察该 Global 并通知子 View；仅修改 Global 不会把读取它的缓存 View 标脏。如果依赖另一个 Entity，则用观察或其他明确的失效路径。不要以为仅仅重新 render 父级，就一定会刷新仍然干净的缓存子级。`window.refresh()` 用于有意进行整窗刷新，不是通常的状态更新方式。

缓存有明确范围：它能跳过子级边界**内部**的工作，但窗口仍然要绘制一帧，父级也仍会按需运行。第一帧和每次未命中都要付出正常的 render/layout/paint 成本。适合在测量表明子树昂贵、周围内容变化时它却经常保持不变的地方使用。

## 元素状态是另一种缓存

GPUI 在后续 render 中会重新创建值类型的元素。如果元素需要让少量状态跨连续帧存活，`Window::use_keyed_state` 会按当前元素路径加指定 key 保存 `Entity<S>`。它还会观察该状态 Entity；状态变化时，当前 View 会收到通知。只要这个路径在连续帧中被访问，状态就会保留；缓存子树重放其元素状态访问的帧也算在内。路径消失且没有其他强引用时，状态会释放。`Window::use_state` 使用调用位置作为 key，只有这个位置能唯一标识状态时才合适。重复或可重排行的 `ElementId` 应从稳定的领域 ID 生成：改 ID 会重置状态；不相关的兄弟元素共用 ID 则可能碰撞。

GPUI Kit 的 [`Plot` 路径缓存](../component/plot) 是具体例子。Plot 及各条 `Line` 都作为值重新构建，因此存在 `Line` 上的路径会随该值消失。`PathCaches::for_paint("lines", window, cx)` 把缓存放在 Plot 的元素 ID 下的窗口键控状态中。`ShapeKey` 涵盖投影后的点和影响几何形状的描边设置；`PathCache::get` 只在 key 改变时重新三角化。路径以零原点构建，绘制时平移到当前原点，所以图表移动仍能复用几何形状。只变颜色时，可以在绘制时应用新颜色，而无需重建未变化的路径几何。

这个缓存节省的是**路径构建**，不是 Plot View 的 `render`，也不是当前帧提交绘图命令的工作。它的 slot 按位置编号：series 可以重排时，应将稳定的 series 身份映射到 slot，或确保 shape key 能安全地使变化的 slot 失效。源码分析见 [Paint](./paint#plot每帧重建的值使用-keyed-window-state)。

## 虚拟化通过少做工作提升性能

长集合即使放在缓存 View 中，第一帧和缓存失效时仍要处理所有行。GPUI Kit 的 [`VirtualList`](../base/virtual-list) 接收逐项尺寸，针对可见范围（外加少量预绘范围）调用渲染闭包；还可能为测量交叉轴尺寸而渲染一个代表项。该帧的大部分屏幕外行元素根本不会创建。它的 `VirtualListScrollHandle` 由所有者单独保留，使滚动位置跨元素重建存活。

```rust
use std::rc::Rc;
use gpui_kit::*;
use gpui_kit::base::{v_virtual_list, VirtualListScrollHandle};

struct Row {
    id: u64,
    name: SharedString,
}

struct ResultsList {
    rows: Vec<Row>,
    sizes: Rc<Vec<Size<Pixels>>>,
    scroll: VirtualListScrollHandle,
}

impl Render for ResultsList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_virtual_list(cx.entity(), "results", self.sizes.clone(), |this, range, _, _| {
            range
                .map(|ix| {
                    div()
                        .id(("result", this.rows[ix].id))
                        .child(this.rows[ix].name.clone())
                })
                .collect::<Vec<_>>()
        })
        .track_scroll(&self.scroll)
        .size_full()
    }
}
```

闭包应读取已经准备好的数据；不要在里面排序、加载数据，或给每一行都创建长期存活的 Entity。重复的可交互行应从行数据获得稳定 ID。虚拟化与 View 缓存分别解决两个问题：创建**多少**元素，以及是否重放**未变化的子树**。

## 选择最小的有效边界

先使用普通 Entity 和声明式 render。如果性能分析表明，某个稳定面板只是因为旁边的 UI 变化而反复构建，就在面板周围加 `cached(style)`，并给它可靠的布局盒子。如果每帧可见的绘图操作仍然昂贵，用明确的 key 和失效规则缓存派生几何。若成本随集合长度增长，则使用虚拟列表。不要用一个大缓存去掩盖本该移出 `render` 的计算，或本该由 Entity 保留的数据。

关于 View 何时重新构建元素树，见 [Render](./render)。

---
title: Task
description: 使用 GPUI Task 执行异步工作、控制生命周期，并将结果更新到界面。
order: -2.631
---

# Task

在 GPUI 中，[`Task<T>`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Task.html) 是 GPUI 执行器所调度工作的 handle。它最重要的性质是**所有权**：handle 被 drop 时，尚未完成的工作会取消。只有保存、等待，或者明确 detach 这个 handle，任务才会持续运行。因此，Task 的生命周期属于 View 的状态设计，而不只是 Rust `Future` trait 的细节。

**启动任务的 API** 决定工作在哪里运行；返回的 `Task` 控制它的生命周期。前台任务可以通过异步 [Context](./context) 重新进入 GPUI，更新 [Entity]。后台任务在 UI 线程之外运行，只能返回自有数据，不能直接修改 Entity 状态。

| 启动方式 | 运行位置 | 异步 callback 收到 | 适用场景 |
| --- | --- | --- | --- |
| 在 `Context<T>` 中调用 `cx.spawn(...)` | 前台线程 | `WeakEntity<T>`、`&mut AsyncApp` | 等待 I/O、定时器，然后更新 Entity |
| `cx.spawn_in(window, ...)` | 前台线程 | `WeakEntity<T>`、`&mut AsyncWindowContext` | 完成时还需要同一个 [Window](./window) 的工作 |
| 在 `App` 中调用 `cx.spawn(...)` | 前台线程 | `&mut AsyncApp` | 没有当前 Entity 的应用级工作 |
| `cx.background_spawn(...)` | 后台执行器 | 不提供 GPUI Context | 使用自有 `Send` 数据进行耗时解析或计算 |

## 任务如何在线程之间切换

在桌面应用中，GPUI 的**前台执行器**在主线程／UI 线程 poll `cx.spawn` 和 `cx.spawn_in` 创建的 future。多个前台任务可以*并发*：一个任务等待未完成的操作时，poll 返回 `Pending`，UI 线程便可处理输入、渲染或 poll 其他任务。它们不会在多个 UI 线程上并行运行。如果 `await` 的值已经 ready，任务可能在同一次 poll 中继续执行；前台任务中的耗时同步函数仍会占住 UI 线程，直到函数返回。

**后台执行器**把实现 `Send` 的工作交给平台后台调度队列或 worker pool。在桌面平台上，worker 可以与 UI 线程以及其他后台任务并行执行，具体取决于可用 worker。GPUI **不会为每个 `Task` 新建一个 OS 线程**。worker 数量和调度方式随平台而异；测试执行器也可能在没有并行 OS 线程的情况下模拟调度。后台 future 在其生命周期内可能由不同 worker poll，不能依赖固定的 worker 线程。

<figure class="task-flow-figure">
  <svg class="task-flow-desktop" viewBox="0 0 800 500" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="task-flow-title-zh task-flow-desc-zh">
    <title id="task-flow-title-zh">GPUI 前台与后台任务流程</title>
    <desc id="task-flow-desc-zh">两列分别表示主线程和后台执行器。事件启动前台任务，任务派发实现 Send 的工作并在等待期间让出 UI 线程。结果就绪后，前台任务在 UI 线程恢复，更新 Entity 并请求稍后渲染。</desc>
    <defs><marker id="task-arrow-zh" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M1 1 L7 4 L1 7" fill="none" stroke="var(--muted-foreground)" stroke-width="1.5" /></marker></defs>
    <rect class="tf-panel" x="12" y="12" width="376" height="476" rx="14" />
    <rect class="tf-panel" x="412" y="12" width="376" height="476" rx="14" />
    <text class="tf-heading" x="36" y="46">主线程 / UI 线程</text>
    <text class="tf-heading" x="436" y="46">后台执行器</text>
    <rect class="tf-ui" x="36" y="76" width="328" height="70" rx="10" />
    <text class="tf-title" x="54" y="106">1 · 用户事件</text><text class="tf-code" x="54" y="130">cx.spawn(...)</text>
    <rect class="tf-ui" x="36" y="169" width="328" height="74" rx="10" />
    <text class="tf-title" x="54" y="198">2 · 前台 poll</text><text class="tf-code" x="54" y="222">cx.background_spawn(work)</text>
    <rect class="tf-worker" x="436" y="169" width="328" height="74" rx="10" />
    <text class="tf-title" x="454" y="198">Send future 入队</text><text class="tf-detail" x="454" y="222">平台 worker pool / 调度队列</text>
    <rect class="tf-ui" x="36" y="270" width="328" height="82" rx="10" />
    <text class="tf-title" x="54" y="302">3 · await 后台 Task</text><text class="tf-detail" x="54" y="327">若返回 Pending，UI 可继续处理输入与渲染</text>
    <rect class="tf-worker" x="436" y="270" width="328" height="82" rx="10" />
    <text class="tf-title" x="454" y="302">worker poll / 计算</text><text class="tf-detail" x="454" y="327">可与 UI 工作并行</text>
    <rect class="tf-ui" x="36" y="380" width="328" height="80" rx="10" />
    <text class="tf-title" x="54" y="411">4 · 回到 UI 线程</text><text class="tf-code" x="54" y="435">WeakEntity::update · cx.notify()</text>
    <rect class="tf-worker" x="436" y="380" width="328" height="80" rx="10" />
    <text class="tf-title" x="454" y="411">Send 结果就绪</text><text class="tf-detail" x="454" y="436">唤醒前台 Task</text>
    <path class="tf-arrow" d="M200 147 V165" marker-end="url(#task-arrow-zh)" />
    <path class="tf-arrow" d="M365 206 H432" marker-end="url(#task-arrow-zh)" />
    <path class="tf-arrow" d="M200 244 V266" marker-end="url(#task-arrow-zh)" />
    <path class="tf-arrow" d="M600 244 V266" marker-end="url(#task-arrow-zh)" />
    <path class="tf-arrow" d="M600 353 V376" marker-end="url(#task-arrow-zh)" />
    <path class="tf-arrow" d="M435 420 H368" marker-end="url(#task-arrow-zh)" />
  </svg>
  <svg class="task-flow-mobile" viewBox="0 0 360 680" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="task-flow-mobile-title-zh task-flow-mobile-desc-zh">
    <title id="task-flow-mobile-title-zh">窄屏下的 GPUI 任务流程</title>
    <desc id="task-flow-mobile-desc-zh">同一流程改为纵向排列：UI 线程启动并 poll 任务；后台 worker 计算自有的 Send 数据；UI 线程恢复后更新 Entity 并请求渲染。</desc>
    <defs><marker id="task-arrow-mobile-zh" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M1 1 L7 4 L1 7" fill="none" stroke="var(--muted-foreground)" stroke-width="1.5" /></marker></defs>
    <rect class="tf-panel" x="8" y="8" width="344" height="260" rx="14" />
    <text class="tf-heading" x="24" y="34">主线程 / UI 线程</text>
    <rect class="tf-ui" x="24" y="50" width="312" height="62" rx="9" /><text class="tf-title" x="40" y="76">1 · 用户事件</text><text class="tf-code" x="40" y="98">cx.spawn(...)</text>
    <rect class="tf-ui" x="24" y="128" width="312" height="62" rx="9" /><text class="tf-title" x="40" y="154">2 · 前台 poll</text><text class="tf-code" x="40" y="176">background_spawn(work)</text>
    <rect class="tf-ui" x="24" y="206" width="312" height="51" rx="9" /><text class="tf-title" x="40" y="231">3 · await Task</text><text class="tf-detail" x="163" y="231">Pending → UI 继续运行</text>
    <rect class="tf-panel" x="8" y="282" width="344" height="251" rx="14" />
    <text class="tf-heading" x="24" y="308">后台执行器</text>
    <rect class="tf-worker" x="24" y="324" width="312" height="63" rx="9" /><text class="tf-title" x="40" y="350">Send future 入队</text><text class="tf-detail" x="40" y="373">平台 worker pool / 调度队列</text>
    <rect class="tf-worker" x="24" y="403" width="312" height="50" rx="9" /><text class="tf-title" x="40" y="434">worker poll / 计算</text>
    <rect class="tf-worker" x="24" y="469" width="312" height="51" rx="9" /><text class="tf-title" x="40" y="500">Send 结果 → 唤醒 UI</text>
    <rect class="tf-panel" x="8" y="548" width="344" height="124" rx="14" />
    <text class="tf-heading" x="24" y="575">主线程 / UI 线程</text>
    <rect class="tf-ui" x="24" y="590" width="312" height="69" rx="9" /><text class="tf-title" x="40" y="617">4 · 恢复并更新 Entity</text><text class="tf-code" x="40" y="642">WeakEntity::update · cx.notify()</text>
    <path class="tf-arrow" d="M180 113 V124" marker-end="url(#task-arrow-mobile-zh)" />
    <path class="tf-arrow" d="M180 191 V202" marker-end="url(#task-arrow-mobile-zh)" />
    <path class="tf-arrow" d="M180 258 V320" marker-end="url(#task-arrow-mobile-zh)" />
    <path class="tf-arrow" d="M180 388 V399" marker-end="url(#task-arrow-mobile-zh)" />
    <path class="tf-arrow" d="M180 454 V465" marker-end="url(#task-arrow-mobile-zh)" />
    <path class="tf-arrow" d="M180 521 V586" marker-end="url(#task-arrow-mobile-zh)" />
  </svg>
  <figcaption>图示后台结果尚未就绪时的典型流程。worker 返回自有数据；只有前台更新会修改 GPUI 状态。配色随站点深浅主题切换。</figcaption>
</figure>

`cx.spawn` 的前台 future 不要求实现 `Send`，因此可持有只能在主线程使用的 GPUI handle，但仍要求 `'static`：应把自有输入移入 future，而不是借用 `self`。`background_spawn` 则要求 future 和输出都实现 `Send + 'static`。将自有数据交给 worker，让它返回可跨线程的结果；`Entity`、`Window` 和 `Context<T>` 的更新留在前台。在前台任务中 `await` 后台 `Task`，结果就绪后会把**后续执行**排回前台执行器。`cx.notify()` 随后将 View 标记为待渲染，并不会同步绘制一帧。

## 从 owner 启动任务

在具名方法、事件处理器或生命周期钩子中启动任务。不要在 [`render`](./render) 中无条件启动，否则每次 render 都可能再启动一份。启动前先取出输入值，避免 `self` 或 `cx` 的借用跨过 `await`。

```rust
struct SearchView {
    query: String,
    results: Vec<SearchResult>,
    _search_task: Option<Task<()>>,
}

impl SearchView {
    fn search(&mut self, cx: &mut Context<Self>) {
        let query = self.query.clone();
        self._search_task = Some(cx.spawn(async move |this, cx| {
            let results = search_index(query).await;
            _ = this.update(cx, |view, cx| {
                view.results = results;
                cx.notify();
            });
        }));
    }
}
```

callback 收到名为 `this` 的 `WeakEntity<SearchView>`。它不会让 View 一直存活。`await` 结束后，`this.update` 在前台线程重新访问 Entity；若 View 已释放，则返回错误。可以用 `?`、`if let` 处理；若 View 消失是正常情况，也可以有意写成 `_ =`。修改 View 状态后调用 `cx.notify()`，依赖它的 UI 才会重新 render。

给 `_search_task` 赋予新 `Task` 会 drop 旧 handle，取消上一次搜索。这个字段使用 `Option`，因为 View 在用户发起搜索前没有任务。如果构造 View 时就启动任务，可以直接保存 `Task<()>`。

:::info
取消操作遵循异步执行的协作机制。drop Task 会阻止后续 poll，但不能撤回已发生的外部副作用，也不能从中途停止正在执行的阻塞函数。如果旧请求的结果仍可能在新请求后到达，还要在应用结果前检查请求 ID 或修订号。
:::

## 保存、等待或 detach

创建任务时就决定其生命周期：

- **保存 handle**：工作应随所属 Entity/View 或窗口级 owner 一同结束时使用。替换 `Option<Task<_>>` 适合搜索、刷新、debounce 和持续流。
- **等待 handle**：后续步骤依赖任务结果时，从另一个任务中 `await`。等待期间，外层任务拥有这个 handle。
- **调用 `.detach()`**：一次性工作应脱离当前 owner、独立完成时使用。它消耗 handle，让任务继续运行到结束；owner 此后无法通过 drop 字段取消任务。如果 View 可能先关闭，callback 应使用弱 Entity，并处理更新失败。

单独写一句 `cx.spawn(...);` 会在语句结束时 drop 返回的 handle，任务可能尚未执行就取消。如果 detach 的任务返回 `Result`，而任务本身没有报告错误，这个结果也会被丢弃。`Task::detach()` 与 `Subscription::detach()` 不同：前者让工作独立继续运行，直到它完成；后者让 callback 持续订阅，直到源 Entity 被释放。对用户可见的操作，应在 View 中保存 loading 和 failure 状态，并在完成时更新。

若在 render 路径或重复执行的 callback 中不断 spawn 并 detach，多个独立任务可能同时持续运行；有限任务正常完成，因此 `.detach()` 本身不等于泄漏。持续或长期工作应由 owner 保存 `Task`，需要停止时替换或 drop handle。若 owner 保存 Task，而任务 future 又强持有同一个 Entity，就会形成保留环。希望 owner 释放时取消任务，应使用 `Context<T>::spawn` 已提供的 `WeakEntity`，或捕获 `cx.weak_entity()`。详见 [Entity 的循环引用](./entity#使用-weakentity-表示反向引用和-callback)。

## 在 `await` 后重新进入 GPUI

从 `Context<T>` 调用 `spawn` 会得到 `AsyncApp`，它可以访问应用，却没有当前的 `&mut Window`。如果完成时需要改变 Focus、显示提示，或使用原来的 Window，就调用 `spawn_in`：

```rust
fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let draft = self.draft.clone();
    self._submit_task = Some(cx.spawn_in(window, async move |this, cx| {
        let message = send_message(draft).await;
        _ = this.update_in(cx, |view, window, cx| {
            view.messages.push(message);
            view.input_focus.focus(window, cx);
            cx.notify();
        });
    }));
}
```

`update_in` 在一次同步更新中重新提供 `&mut Self`、`&mut Window` 和 `&mut Context<Self>`。此时 Window 或 Entity 可能已经不存在，需要处理返回结果。不需要 Window 时使用 `update`。从应用级 `App::spawn` 启动时，callback 只收到 `AsyncApp`；`await` 后调用 `cx.update(|cx| { ... })`，完成简短的应用级修改。

## 将耗时工作移出 UI 线程

前台任务等待非阻塞 I/O 且结果尚未就绪时不会占住 UI 线程，但一次 poll 中的大量 CPU 计算仍会占用它。将自有且实现 `Send` 的输入移入 `background_spawn`，从前台任务 `await` 它的 `Task`，再回到 UI 线程应用结果：

```rust
struct DocumentView {
    source: String, // 可变编辑缓冲。
    revision: u64,
    parsed: Option<ParsedDocument>,
    _parse_task: Option<Task<()>>,
}

impl DocumentView {
    fn parse(&mut self, cx: &mut Context<Self>) {
        self.revision = self.revision.wrapping_add(1);
        let revision = self.revision;
        let source = self.source.clone();
        self._parse_task = Some(cx.spawn(async move |this, cx| {
            let parsed = cx.background_spawn(async move {
                parse_document(source)
            }).await;

            _ = this.update(cx, |view, cx| {
                if view.revision != revision {
                    return; // 旧结果不能覆盖新内容。
                }
                view.parsed = Some(parsed);
                cx.notify();
            });
        }));
    }
}
```

worker 接收 `String`，返回实现 `Send` 的 `ParsedDocument`；它拿不到 `App`、`Window` 或 `Context<T>`。离开 Entity 更新之前，只 clone 工作所需的输入。替换 `_parse_task` 会取消上一个外层任务及它正在等待的后台任务，但不能中途打断一次 poll 中正在执行的同步解析。修订号检查还能阻止前台更新前已经过期的结果覆盖新状态。

## GPUI Kit 中的流式处理实例

GPUI Kit 的[流式 Markdown 示例](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/src/main.rs)使用两个由 View 持有的任务和一个 channel。后台生产者生成文本片段；前台接收者负责更新 Entity，检查 replay ID，再将有效片段送进 `TextViewState`。View 保存两个 `Task<()>` handle，关闭 View 就会取消流；再次 replay 会替换生产者任务。replay ID 也能过滤旧生产者已经排入 channel 的片段。

```rust
// View 中的接收任务：
self._receiver_task = Some(cx.spawn(async move |this, cx| {
    while let Ok((replay_id, chunk)) = rx.recv().await {
        if this.update(cx, |view, cx| {
            if replay_id != view.replay_id {
                return;
            }
            view.markdown_state.update(cx, |state, cx| {
                state.push_str(&chunk, cx);
            });
            view.scroll_handle.scroll_to_bottom();
        }).is_err() {
            break; // View 已释放。
        }
    }
}));

// View 的 replay 方法中，增加 replay_id 后：
self._producer_task = cx.background_spawn(async move {
    for chunk in chunks {
        if tx.send((replay_id, chunk)).await.is_err() {
            break; // 接收者已消失。
        }
    }
});
```

这是 GPUI 特有的任务模式：Task handle 表达所有权，`WeakEntity` 保护 View 的生命周期，channel 连接两个执行器，replay ID 防止旧结果污染状态。Entity 的所有权和更新方式见 [Entity](./entity)。

[Entity]: /zh-CN/docs/entity

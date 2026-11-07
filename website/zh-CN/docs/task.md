---
title: Task
description: 使用 GPUI Task 执行异步工作、控制生命周期，并将结果更新到界面。
order: -2.631
---

# Task

在 GPUI 中，[`Task<T>`](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.Task.html) 是 GPUI 执行器所调度工作的 handle。它最重要的性质是**所有权**：handle 被 drop 时，尚未完成的工作会取消。只有保存、等待，或者明确 detach 这个 handle，任务才会持续运行。因此，Task 的生命周期属于 View 的状态设计，而不只是 Rust `Future` trait 的细节。

**启动任务的 API** 决定工作在哪里运行；返回的 `Task` 控制它的生命周期。前台任务可以通过异步 [Context](./context) 重新进入 GPUI，更新 [Entity]。后台任务在 UI 线程之外运行，只能返回自有数据，不能直接修改 Entity 状态。

| 启动方式 | 运行位置 | 异步 callback 收到 | 适用场景 |
| --- | --- | --- | --- |
| 在 `Context<T>` 中调用 `cx.spawn(...)` | 前台线程 | `WeakEntity<T>`、`&mut AsyncApp` | 等待 I/O、定时器，然后更新 Entity |
| `cx.spawn_in(window, ...)` | 前台线程 | `WeakEntity<T>`、`&mut AsyncWindowContext` | 完成时还需要同一个 [Window](./window) 的工作 |
| 在 `App` 中调用 `cx.spawn(...)` | 前台线程 | `&mut AsyncApp` | 没有当前 Entity 的应用级工作 |
| `cx.background_spawn(...)` | 后台执行器 | 不提供 GPUI Context | 使用自有 `Send` 数据进行耗时解析或计算 |

## 运行仓库中的示例

按[安装指南](./installation.md)安装平台依赖后，在仓库根目录运行：

```sh
cargo run -p example-stream-markdown
```

窗口中有 **Replay** 按钮和 **Fade in streamed text** 开关。点击 Replay，文字先清空，再分片出现。在流结束前再次点击，会开始新一轮播放；界面只显示带有当前 replay ID 的分片。关闭窗口会 drop View 持有的任务 handle，但已经进入同步循环的生产者可能要等循环跑完才停止。完整源码见 [`examples/stream-markdown/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/src/main.rs)；package 和依赖见 [`examples/stream-markdown/Cargo.toml`](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/Cargo.toml)。这是可直接运行的 workspace 示例；只依赖 `gpui-kit` 的应用骨架见[入门指南](./getting-started.md)。

可沿源码追踪一次点击：`main` 创建窗口；`Example::new` 创建 channel 并保存前台接收任务的 `Task`；按钮调用 `Example::replay`，增加 `replay_id`、清空文字、替换后台生产任务的 `Task`；接收任务检查 ID，随后更新 `TextViewState`。此例用 worker 模拟数据到来。实际网络流应异步等待数据读取，并等待有界 channel 的发送，以免 UI 较慢时数据无限排队。

## 动手做：启动、取消和失败的任务

流式示例说明了 Task 所有权，但没有失败操作。下面的小应用把三种结果都显示在界面上。在现有仓库中，将代码保存为 `examples/hello_world/src/bin/task_lab.rs`（如有需要，先创建 `bin` 目录），然后从仓库根目录执行 `cargo run -p hello_world --bin task_lab`。它复用已有的 `hello_world` package，无需新增依赖或 workspace member。

```rust
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use std::time::Duration;

struct TaskLab {
    status: String,
    request_id: u64,
    task: Option<Task<()>>,
}

impl TaskLab {
    fn start(&mut self, fail: bool, cx: &mut Context<Self>) {
        self.request_id = self.request_id.wrapping_add(1);
        let request_id = self.request_id;
        self.task = None; // Drop the previous handle before starting again.
        self.status = format!("Loading request {request_id}...");
        cx.notify();

        self.task = Some(cx.spawn(async move |this, cx| {
            let result: Result<String, String> = cx
                .background_spawn(async move {
                    // Stand in for expensive work; this blocks a worker, not the UI thread.
                    std::thread::sleep(Duration::from_millis(800));
                    if fail {
                        Err("Simulated failure".into())
                    } else {
                        Ok(format!("Completed request {request_id}"))
                    }
                })
                .await;

            _ = this.update(cx, |view, cx| {
                if view.request_id != request_id {
                    return; // A newer start or cancel owns the displayed state.
                }
                view.status = match result {
                    Ok(value) => value,
                    Err(error) => format!("Failed: {error}"),
                };
                cx.notify();
            });
        }));
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.request_id = self.request_id.wrapping_add(1);
        self.task = None; // Dropping the handle prevents future polls.
        self.status = "Cancelled".into();
        cx.notify();
    }
}

impl Render for TaskLab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(self.status.clone())
            .child(
                Button::new("start")
                    .primary()
                    .label("Start")
                    .on_click(cx.listener(|view, _, _, cx| view.start(false, cx))),
            )
            .child(
                Button::new("fail")
                    .label("Fail")
                    .on_click(cx.listener(|view, _, _, cx| view.start(true, cx))),
            )
            .child(
                Button::new("cancel")
                    .label("Cancel")
                    .on_click(cx.listener(|view, _, _, cx| view.cancel(cx))),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| TaskLab {
                status: "Idle".into(),
                request_id: 0,
                task: None,
            })
        })
        .expect("Failed to open window");
    });
}
```

点击 **Start**，观察 **Loading** 约 800 毫秒后变成 **Completed**；等待期间窗口仍应能响应点击。点击 **Fail**，会稳定地看到 **Failed**。点击 **Start**，在完成前点击 **Cancel**，**Cancelled** 应保持不变。快速依次点击 **Start** 和 **Fail**，只有最后一次请求能更新标签。若被取消的 worker 已经进入 `sleep`，它可能先把这次阻塞调用执行完；已丢弃的 `Task` 和请求 ID 会阻止结果改动 View。练习中的 sleep 只在后台 worker 中执行。真实 I/O 应使用异步 API；若长时间 CPU 工作需要及时停止，应拆成有界步骤或使用其他取消机制。

练习结束后，只删除 `examples/hello_world/src/bin/task_lab.rs`，保留 `src/bin` 中的其他文件。在未自行增加其他二进制目标的检出中，这会让 package 只剩原来的 `hello_world` 目标，其他指南中的 `cargo run -p hello_world` 就能再次运行。若该 package 中还有你自己的二进制目标，运行原示例时请指定 `--bin hello_world`。

View 持有前台 `Task`；它等待后台的 `Task<Result<...>>`，再通过 `cx.spawn` 提供的弱 Entity 更新 View。每次状态变化后，`cx.notify()` 让界面显示新状态。完成后的 handle 留在字段中，直到下一次启动或取消；丢弃已完成的任务没有影响。`Result` 的错误分支会把错误显示在界面上，不会默默丢弃。

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
    <text class="tf-heading" x="36" y="46">Main / UI thread</text>
    <text class="tf-heading" x="436" y="46">Background executor</text>
    <rect class="tf-ui" x="36" y="76" width="328" height="70" rx="10" />
    <text class="tf-title" x="54" y="106">1 · User event</text><text class="tf-code" x="54" y="130">cx.spawn(...)</text>
    <rect class="tf-ui" x="36" y="169" width="328" height="74" rx="10" />
    <text class="tf-title" x="54" y="198">2 · Foreground poll</text><text class="tf-code" x="54" y="222">cx.background_spawn(work)</text>
    <rect class="tf-worker" x="436" y="169" width="328" height="74" rx="10" />
    <text class="tf-title" x="454" y="198">Send future queued</text><text class="tf-detail" x="454" y="222">Platform workers / dispatch queue</text>
    <rect class="tf-ui" x="36" y="270" width="328" height="82" rx="10" />
    <text class="tf-title" x="54" y="302">3 · Await background Task</text><text class="tf-detail" x="54" y="327">If Pending, UI can handle input/render</text>
    <rect class="tf-worker" x="436" y="270" width="328" height="82" rx="10" />
    <text class="tf-title" x="454" y="302">Worker polls / computes</text><text class="tf-detail" x="454" y="327">May run in parallel with UI work</text>
    <rect class="tf-ui" x="36" y="380" width="328" height="80" rx="10" />
    <text class="tf-title" x="54" y="411">4 · Resume on UI thread</text><text class="tf-code" x="54" y="435">WeakEntity::update · cx.notify()</text>
    <rect class="tf-worker" x="436" y="380" width="328" height="80" rx="10" />
    <text class="tf-title" x="454" y="411">Send result ready</text><text class="tf-detail" x="454" y="436">Wake the foreground Task</text>
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
    <text class="tf-heading" x="24" y="34">Main / UI thread</text>
    <rect class="tf-ui" x="24" y="50" width="312" height="62" rx="9" /><text class="tf-title" x="40" y="76">1 · User event</text><text class="tf-code" x="40" y="98">cx.spawn(...)</text>
    <rect class="tf-ui" x="24" y="128" width="312" height="62" rx="9" /><text class="tf-title" x="40" y="154">2 · Foreground poll</text><text class="tf-code" x="40" y="176">background_spawn(work)</text>
    <rect class="tf-ui" x="24" y="206" width="312" height="51" rx="9" /><text class="tf-title" x="40" y="231">3 · await Task</text><text class="tf-detail" x="163" y="231">Pending → UI free</text>
    <rect class="tf-panel" x="8" y="282" width="344" height="251" rx="14" />
    <text class="tf-heading" x="24" y="308">Background executor</text>
    <rect class="tf-worker" x="24" y="324" width="312" height="63" rx="9" /><text class="tf-title" x="40" y="350">Queue Send future</text><text class="tf-detail" x="40" y="373">Platform workers / dispatch queue</text>
    <rect class="tf-worker" x="24" y="403" width="312" height="50" rx="9" /><text class="tf-title" x="40" y="434">Worker poll / compute</text>
    <rect class="tf-worker" x="24" y="469" width="312" height="51" rx="9" /><text class="tf-title" x="40" y="500">Send result → wake UI</text>
    <rect class="tf-panel" x="8" y="548" width="344" height="124" rx="14" />
    <text class="tf-heading" x="24" y="575">Main / UI thread</text>
    <rect class="tf-ui" x="24" y="590" width="312" height="69" rx="9" /><text class="tf-title" x="40" y="617">4 · Resume and update Entity</text><text class="tf-code" x="40" y="642">WeakEntity::update · cx.notify()</text>
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

异步 GPUI context 用于在 `await` 后重新进入 GPUI，并不表示能让 `Context<T>` 或 `Window` 的可变借用跨越等待点。启动前复制或移走自有输入；结果到来后用 `WeakEntity::update` 做简短的同步状态更新，需要同一个窗口时用 `update_in`。不要在 `background_spawn` 的 future 内修改 UI 状态。

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

上面的 `search_index` 和 `SearchResult` 代表应用自己的代码，片段用来说明所有权，并非完整应用。若要直接编译运行，请使用上面的仓库示例。

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
    source: String, // Mutable edit buffer.
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
                    return; // An older result must not overwrite newer content.
                }
                view.parsed = Some(parsed);
                cx.notify();
            });
        }));
    }
}
```

worker 接收 `String`，返回实现 `Send` 的 `ParsedDocument`；它拿不到 `App`、`Window` 或 `Context<T>`。离开 Entity 更新之前，只 clone 工作所需的输入。替换 `_parse_task` 会取消上一个外层任务及它正在等待的后台任务，但不能中途打断一次 poll 中正在执行的同步解析。修订号检查还能阻止前台更新前已经过期的结果覆盖新状态。

## 处理完成、失败和取消

用户能看到的操作，应在所属 View 中保存明确的 idle、loading、loaded、failed 等状态。启动任务前设置 loading 并 notify。让 worker 返回自有且实现 `Send` 的 `Result<Data, Error>`。`await` 后先拒绝过期请求 ID，再分别处理 `Ok`（保存数据）与 `Err`（保存可读错误）；清除 loading，并在这次完整状态更新后 notify 一次。若界面允许，刷新时可保留此前可用的数据。仅记录日志或在 `.detach()` 后丢弃错误，会使用户一直看到加载状态。

`WeakEntity` 或原窗口已释放，是正常的取消路径：`update` 或 `update_in` 失败表示界面已不存在。drop 任务 handle 可阻止后续 poll，但不能撤回 I/O，也不能中止 worker 正在执行的同步代码。若操作有外部副作用，还需另行决定它是否必须完成、如何报告结果。替换任务后，仍要用请求 ID 防止排队中的消息或迟到结果污染状态。

不要写 `while !ready {}` 或反复立即 poll 来等待别的任务；这样的循环会占住 UI 线程，甚至让等待的工作无法继续。应 `await` Task、定时器、I/O future 或 channel 接收。周期性工作要在迭代之间等待定时器，并由 owner 保存 handle。大量 CPU 工作放到后台执行器；若要求及时取消很长的计算，可把它拆成有界步骤。前台任务不能用阻塞 sleep 或阻塞 I/O。

## 检查与排查任务

运行上面的示例，检查可观察行为：Replay 使文字流式出现；再次 Replay 清空文字且不显示旧片段。关闭窗口会 drop View 及其任务 handle，但不能据此断定正在运行的生产者会立即停止。要区分 UI 卡死与 worker 较慢，检查前台 future 是否走到了尚未就绪的 `await`；首次返回 `Pending` 前的代码仍在 UI 线程执行。若没有任何内容，检查 `Task` handle 是否被保存、接收端是否还活着，以及更新失败是否得到处理。若出现旧文字，检查 UI 更新处的请求 ID。若 loading 一直不结束，检查成功与失败两条路径，并确认等待的操作能完成。

应用测试中，纯解析与请求顺序可用普通 Rust 测试；owner 生命周期和过期结果检查可用 GPUI context 测试。UI 交互测试应触发操作，并从渲染状态观察 loading、success 和 failure。使用测试执行器推进异步任务，避免 sleep 和忙等。仓库示例可用 `cargo check -p example-stream-markdown` 检查构建，并用上面的命令手动体验。

## GPUI Kit 中的流式处理实例

GPUI Kit 的[流式 Markdown 示例](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/src/main.rs)使用两个由 View 持有的任务和一个 channel。后台生产者生成文本片段；前台接收者负责更新 Entity，检查 replay ID，再将有效片段送进 `TextViewState`。View 保存两个 `Task<()>` handle；关闭 View 会 drop 这些 handle，再次 replay 会替换生产者 handle。生产者的 async 块没有 `await`，因此一旦其同步循环进入 poll，drop handle 不能中途打断该次 poll。replay ID 会过滤旧生产者的片段，包括替换前已经排入队列的片段。

```rust
// From Example::new; keep the returned Task in _task.
let _task = cx.spawn(async move |weak_self, cx| {
    while let Ok((replay_id, chunk)) = rx.recv().await {
        _ = weak_self.update(cx, |this, cx| {
            if replay_id != this.replay_id {
                return;
            }
            this.markdown_state.update(cx, |state, cx| {
                state.push_str(&chunk, cx);
            });
            this.scroll_handle.scroll_to_bottom();
        });
    }
});

// From Example::replay; replacing the field drops the old handle.
self._update_task = cx.background_executor().spawn(async move {
    let chars: Vec<char> = EXAMPLE.chars().collect();
    while current < chars.len() {
        let chunk_size = (5 + rand::random::<usize>() % 15).min(chars.len() - current);
        let chunk: String = chars[current..current + chunk_size].iter().collect();
        _ = tx.try_send((replay_id, chunk));
        current += chunk_size;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
});
```

摘录展示所有权和更新位置；初始化与渲染见链接中的完整源码。示例在后台生产任务中用 `std::thread::sleep` 模拟节奏，使用**无界 channel** 的 `try_send` 仅供演示。无界 channel 不提供背压；生产速度超过 UI 处理速度时，队列可能不断增长。替换任务后，生产者仍可能继续发送，直到同步循环返回；replay ID 防止旧片段进入 UI。真实流应异步等待，使用有界 channel 提供背压，处理发送和更新失败，并在 View 消失时结束接收任务。`WeakEntity` 保护 View 生命周期，channel 连接两个执行器。Entity 的所有权和更新方式见 [Entity](./entity)。

[Entity]: ./entity.md

---
title: Task
description: Run asynchronous work with GPUI Task, control its lifetime, and return results to the UI.
order: -2.631
---

# Task

In GPUI, a [`Task<T>`](https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.Task.html) is the handle to work scheduled by a GPUI executor. Its most important property is **ownership**: dropping the handle cancels unfinished work. A task runs only while its handle is stored, awaited, or explicitly detached. This makes the task's lifetime part of the View's state design, not just a detail of Rust's `Future` trait.

The **spawn API** chooses where work runs; the returned `Task` controls its lifetime. A foreground task can re-enter GPUI through an async [Context](./context) and update an [Entity]. A background task runs away from the UI thread and returns owned data; it cannot mutate Entity state there.

| Start from | Runs on | Async callback receives | Use for |
| --- | --- | --- | --- |
| `cx.spawn(...)` in `Context<T>` | Foreground thread | `WeakEntity<T>`, `&mut AsyncApp` | Awaiting I/O, timers, then updating an Entity |
| `cx.spawn_in(window, ...)` | Foreground thread | `WeakEntity<T>`, `&mut AsyncWindowContext` | Work whose completion needs the same [Window](./window) |
| `cx.spawn(...)` in `App` | Foreground thread | `&mut AsyncApp` | Application-level work without a current Entity |
| `cx.background_spawn(...)` | Background executor | No GPUI context | Expensive parsing or computation on owned `Send` data |

## Run the repository example

From the repository root, after installing the platform prerequisites in [Installation](./installation.md), run:

```sh
cargo run -p example-stream-markdown
```

The window opens with a **Replay** button and a **Fade in streamed text** switch. Click Replay: the text clears, then appears in chunks. Click it again before the stream finishes to start a new replay; only chunks tagged with the current replay ID are displayed. Closing the window drops the View's task handles, but a producer already running its synchronous loop can finish that loop before stopping. The complete source is [`examples/stream-markdown/src/main.rs`](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/src/main.rs); its package and dependencies are in [`examples/stream-markdown/Cargo.toml`](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/Cargo.toml). This is a runnable workspace example; for a one-dependency app skeleton, start with [Getting Started](./getting-started.md).

Follow one click through the source: `main` creates the window; `Example::new` creates the channel and saves a foreground receiver `Task`; the button calls `Example::replay`, which increments `replay_id`, clears the text and replaces the background producer `Task`; the receiver checks that ID before updating `TextViewState`. The example uses a worker to simulate incoming chunks. In an actual network stream, await the source's asynchronous read and await a bounded channel send so a slow UI cannot cause unbounded queued data.

## Build a task you can start, cancel, and fail

The streaming example shows task ownership but has no failure control. This small app makes all three outcomes visible. In the existing repository checkout, save the following as `examples/hello_world/src/bin/task_lab.rs` (create the `bin` directory if needed), then run `cargo run -p hello_world --bin task_lab` from the repository root. It uses the existing `hello_world` package and adds no dependency or workspace member.

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

Click **Start** and watch **Loading** become **Completed** after about 800 ms; the window should still accept clicks during the wait. Click **Fail** to see a deterministic **Failed** state. Click **Start**, then **Cancel** before it completes: **Cancelled** should remain visible. Click **Start** and immediately **Fail**: only the latest request may update the label. A cancelled worker that is already inside `sleep` may finish that blocking call; the dropped `Task` and request ID prevent its result from changing the View. This sleep is deliberately confined to a background worker for the exercise. Real I/O should use an async API, and long CPU jobs need bounded steps or another cancellation mechanism if prompt stopping matters.

After the exercise, delete only `examples/hello_world/src/bin/task_lab.rs`; keep any other files in `src/bin`. In an unmodified checkout, this leaves only the original `hello_world` binary, so the `cargo run -p hello_world` commands in other guides work again. If you already have other binaries in that package, specify `--bin hello_world` when running its original example.

The View owns the foreground `Task`. That task awaits a background `Task<Result<...>>`, then updates the View through the weak Entity supplied by `cx.spawn`. `cx.notify()` makes each status transition visible. Keep the completed handle in the field until the next start or cancel; dropping a completed task is harmless. The `Result` branch shows the error in the UI instead of silently discarding it.

## How execution moves between threads

On a desktop app, GPUI's **foreground executor** polls `cx.spawn` and `cx.spawn_in` futures on the main/UI thread. Several foreground tasks can be *concurrent*: when one awaits an unfinished operation, its poll returns `Pending`, and the UI thread can handle input, render, or poll another task. They do not run in parallel with each other on separate UI threads. An `await` whose value is already ready may continue in the same poll; an expensive synchronous function inside a foreground task still blocks the UI until it returns.

The **background executor** schedules `Send` work through the platform's background dispatch queue or worker pool. On desktop, worker polls may run in parallel with the UI thread and with other background tasks, subject to available workers. GPUI does **not** create an OS thread for every `Task`. The actual worker count and scheduling depend on the platform; test executors may simulate the scheduling without parallel OS threads. A background future may be polled on different workers over its lifetime, so do not rely on worker thread affinity.

<figure class="task-flow-figure">
  <svg class="task-flow-desktop" viewBox="0 0 800 500" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="task-flow-title-en task-flow-desc-en">
    <title id="task-flow-title-en">GPUI foreground and background task flow</title>
    <desc id="task-flow-desc-en">Two columns show the main UI thread and the background executor. An event queues a foreground task. It dispatches Send work, yields while awaiting a pending result, and resumes on the UI thread to update an Entity and request a later render.</desc>
    <defs><marker id="task-arrow-en" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M1 1 L7 4 L1 7" fill="none" stroke="var(--muted-foreground)" stroke-width="1.5" /></marker></defs>
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
    <path class="tf-arrow" d="M200 147 V165" marker-end="url(#task-arrow-en)" />
    <path class="tf-arrow" d="M365 206 H432" marker-end="url(#task-arrow-en)" />
    <path class="tf-arrow" d="M200 244 V266" marker-end="url(#task-arrow-en)" />
    <path class="tf-arrow" d="M600 244 V266" marker-end="url(#task-arrow-en)" />
    <path class="tf-arrow" d="M600 353 V376" marker-end="url(#task-arrow-en)" />
    <path class="tf-arrow" d="M435 420 H368" marker-end="url(#task-arrow-en)" />
  </svg>
  <svg class="task-flow-mobile" viewBox="0 0 360 680" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="task-flow-mobile-title-en task-flow-mobile-desc-en">
    <title id="task-flow-mobile-title-en">GPUI task flow on a narrow screen</title>
    <desc id="task-flow-mobile-desc-en">The same flow stacked vertically: the UI thread starts and polls a task, background workers compute owned Send data, and the UI thread resumes to update an Entity and request rendering.</desc>
    <defs><marker id="task-arrow-mobile-en" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M1 1 L7 4 L1 7" fill="none" stroke="var(--muted-foreground)" stroke-width="1.5" /></marker></defs>
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
    <path class="tf-arrow" d="M180 113 V124" marker-end="url(#task-arrow-mobile-en)" />
    <path class="tf-arrow" d="M180 191 V202" marker-end="url(#task-arrow-mobile-en)" />
    <path class="tf-arrow" d="M180 258 V320" marker-end="url(#task-arrow-mobile-en)" />
    <path class="tf-arrow" d="M180 388 V399" marker-end="url(#task-arrow-mobile-en)" />
    <path class="tf-arrow" d="M180 454 V465" marker-end="url(#task-arrow-mobile-en)" />
    <path class="tf-arrow" d="M180 521 V586" marker-end="url(#task-arrow-mobile-en)" />
  </svg>
  <figcaption>A typical pending path. The worker returns owned data; only the foreground update mutates GPUI state. Colors and text adapt to the site's light and dark themes.</figcaption>
</figure>

`cx.spawn` accepts a foreground future that need not be `Send`, so it can hold main-thread-only GPUI handles, but it still must be `'static`: move owned inputs into it instead of borrowing `self`. `background_spawn` requires both its future and its output to be `Send + 'static`. Move owned data into the worker and return a `Send` result; keep `Entity`, `Window`, and `Context<T>` updates on the foreground side. Awaiting the background `Task` from the foreground task schedules the **continuation** back on the foreground executor when the result is ready. `cx.notify()` then invalidates the View for a later render; it does not synchronously draw a frame.

An async GPUI context is a way back into GPUI after `await`, not permission to keep a mutable `Context<T>` or `Window` borrowed across suspension. Capture owned inputs before spawning. Use `WeakEntity::update` for a short, synchronous state change after the result arrives; use `update_in` when the same window is required. No UI mutation belongs inside a `background_spawn` future.

## Start work from an owner

Start a task in a named method, event handler, or lifecycle hook. Do not start one unconditionally in [`render`](./render): every render could launch another copy. Extract the input before spawning, so no borrow of `self` or `cx` crosses an `await`.

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

The callback gets a `WeakEntity<SearchView>` named `this`. It does not keep the View alive. After the `await`, `this.update` reacquires the Entity on the foreground thread and returns an error if the View has gone away. Handle that case with `?`, `if let`, or an intentional `_ =` when disappearance is normal. Call `cx.notify()` after changing View state so dependent UI renders again.

Assigning a new `Task` to `_search_task` drops the old handle and cancels the previous search. The field is an `Option` because this View has no task until the user starts one. A View that starts work during construction can store a plain `Task<()>` instead.

`search_index` and `SearchResult` above stand for application code; this snippet illustrates ownership, rather than defining a complete application. For a complete buildable workflow, run the repository example above.

:::info
Cancellation is cooperative with async execution. Dropping a task prevents further polling; it cannot undo an external side effect that already happened or stop a blocking function in the middle of a call. For results that may arrive after a newer request, also check a request ID or revision before applying them.
:::

## Keep, await, or detach

Choose the lifetime when you create the task:

- **Store it** on the owning Entity/View or a window-scoped owner when the work should end with that owner. Replacing an `Option<Task<_>>` is useful for search, refresh, debounce, and ongoing streams.
- **Await it** from another task when the next step needs its result. The awaiting task owns the handle until completion.
- **Detach it** with `.detach()` for one-off work that should finish independently of the current owner. This consumes the handle and lets the task run to completion; the owner can no longer cancel it by dropping a field. Give its callback a weak Entity and handle failed updates if the View may close first.

Calling `cx.spawn(...);` as a bare statement drops the returned handle at the end of the statement and may cancel the task before it does useful work. Detaching a task that returns `Result` also discards that result unless the task itself reports an error. `Task::detach()` is different from `Subscription::detach()`: the former lets work continue independently until it completes; the latter leaves a callback subscribed until its source Entity is dropped. For user-facing operations, keep loading and failure state in the View and update it on completion.

Repeatedly spawning and detaching from a render path or recurring callback can leave many independent tasks running at once; a finite detached task completes normally, so `.detach()` alone is not a leak. Keep recurring or long-lived work under an owner-held `Task`, and replace or drop that handle when the work should stop. If the owner stores the Task while its future captures a strong handle back to that same Entity, they form a retention cycle. Use the `WeakEntity` supplied by `Context<T>::spawn`, or capture `cx.weak_entity()`, when owner release should cancel the work. See [Entity ownership cycles](./entity#use-a-weakentity-for-back-references-and-callbacks).

## Re-enter GPUI after `await`

From `Context<T>`, `spawn` supplies `AsyncApp`, which has application access but no current `&mut Window`. Use `spawn_in` when the completion must change Focus, show a prompt, or otherwise use the originating Window:

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

`update_in` restores `&mut Self`, `&mut Window`, and `&mut Context<Self>` for one synchronous update. The Window or Entity might be gone by then, so handle its result. Use `update` when the Window is irrelevant. From an application-level `App::spawn`, the callback receives only `AsyncApp`; call `cx.update(|cx| { ... })` for a short application mutation after awaiting.

## Move heavy work off the UI thread

A foreground task can await nonblocking I/O without occupying the UI thread while pending, but CPU-intensive work inside a poll still occupies it. Move owned, `Send` input into `background_spawn`, await its `Task` from a foreground task, and apply the result back on the UI thread:

```rust
struct DocumentView {
    source: String, // Mutable editing buffer.
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
                    return; // An older result must not replace newer content.
                }
                view.parsed = Some(parsed);
                cx.notify();
            });
        }));
    }
}
```

The worker receives the `String` and returns a `Send` `ParsedDocument`; it has no `App`, `Window`, or `Context<T>`. Clone only the input it needs before leaving the Entity update. Replacing `_parse_task` cancels the previous outer task and its awaited worker task, but cannot interrupt a synchronous parse already executing inside one poll. The revision check also rejects a result that became stale before the foreground update.

## Handle completion, failure, and cancellation

For a user-visible operation, keep an explicit state such as idle, loading, loaded, or failed in the owning View. Set loading and notify before spawning. Have the worker return `Result<Data, Error>` as owned, `Send` data. After `await`, first reject an outdated request ID, then handle `Ok` by storing data or `Err` by storing a readable error; clear loading and notify once for the complete change. Keep useful previous data visible during a refresh if the UI allows it. A failure that is only logged or dropped by `.detach()` leaves users looking at a perpetual spinner.

Treat release of the `WeakEntity` or originating window as an ordinary cancellation path: a failed `update` or `update_in` means there is no UI left to change. Dropping a task handle prevents future polls but does not undo I/O or stop synchronous worker code already running. If the operation has an external side effect, decide separately whether it must finish and how to report its outcome. A request ID still guards against queued messages or a late result after replacement.

Never wait for another task with `while !ready {}` or repeated immediate polls. Such a loop occupies the UI thread and can prevent the awaited work from advancing. Await the `Task`, a timer, an I/O future, or a channel receive. For periodic work, await a timer between iterations and keep its handle with the owner. Large CPU work belongs on the background executor; break very long computation into bounded units if prompt cancellation matters. Avoid blocking sleep or blocking I/O in a foreground task.

## Check and diagnose task behavior

Run the example above and check these observable cases: Replay streams text, and a second Replay clears it and excludes old chunks. Closing the window drops the View and its task handles; it does not prove that an already running producer stops immediately. To distinguish a frozen UI from a slow worker, inspect whether the foreground future reaches a pending `await`; code before the first pending point runs on the UI thread. If nothing appears, check that the `Task` handle was retained, that the receiver is still alive, and that an update failure is handled. If stale text appears, inspect the request ID at the point of the UI update. If loading never ends, inspect both the success and error branches and whether the awaited operation can complete.

For app tests, put pure parsing and request ordering in ordinary Rust tests; use GPUI context tests for the owner lifecycle and stale-result guard. A UI interaction test should trigger the action and observe loading, success, and failure through rendered state. Drive async work with the test executor rather than sleeps or a busy loop. The repository example itself can be checked with `cargo check -p example-stream-markdown` and manually exercised with the run command above.

## A GPUI Kit streaming example

GPUI Kit's [streaming Markdown example](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/src/main.rs) uses two owned tasks and a channel. A background producer generates text chunks. A foreground receiver owns the Entity update, checks a replay ID, and pushes accepted chunks into `TextViewState`. The View keeps both `Task<()>` handles; closing it drops those handles, and another replay replaces the producer handle. The producer's async block contains no `await`, so once its synchronous loop is being polled, dropping its handle cannot interrupt that poll. The replay ID rejects chunks from an older producer, including ones queued before replacement.

```rust
// Condensed from Example::new; the returned Task is stored as _task.
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

// Condensed from Example::replay; replacement drops the prior handle.
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

The excerpt shows the ownership and update points; see the linked source for setup and rendering. The example uses `std::thread::sleep` in its background producer to simulate pacing and an **unbounded channel** with `try_send` only for this demonstration. An unbounded channel does not provide backpressure and can grow when a producer outpaces the UI. Its producer can continue sending after replacement until the synchronous loop returns; the replay ID keeps those chunks out of the UI. For a real stream, use asynchronous waiting and a bounded channel with backpressure, handle send and update errors, and end the receiver when its View is gone. `WeakEntity` protects the View lifetime, and the channel crosses executors. See [Entity](./entity) for Entity ownership and updates.

[Entity]: ./entity.md

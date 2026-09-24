---
title: Task
description: Run asynchronous work with GPUI Task, control its lifetime, and return results to the UI.
order: -2.631
---

# Task

In GPUI, a [`Task<T>`](https://docs.rs/gpui-pre/0.3.6/gpui/struct.Task.html) is the handle to work scheduled by a GPUI executor. Its most important property is **ownership**: dropping the handle cancels unfinished work. A task runs only while its handle is stored, awaited, or explicitly detached. This makes the task's lifetime part of the View's state design, not just a detail of Rust's `Future` trait.

The **spawn API** chooses where work runs; the returned `Task` controls its lifetime. A foreground task can re-enter GPUI through an async [Context](./context) and update an [Entity]. A background task runs away from the UI thread and returns owned data; it cannot mutate Entity state there.

| Start from | Runs on | Async callback receives | Use for |
| --- | --- | --- | --- |
| `cx.spawn(...)` in `Context<T>` | Foreground thread | `WeakEntity<T>`, `&mut AsyncApp` | Awaiting I/O, timers, then updating an Entity |
| `cx.spawn_in(window, ...)` | Foreground thread | `WeakEntity<T>`, `&mut AsyncWindowContext` | Work whose completion needs the same [Window](./window) |
| `cx.spawn(...)` in `App` | Foreground thread | `&mut AsyncApp` | Application-level work without a current Entity |
| `cx.background_spawn(...)` | Background executor | No GPUI context | Expensive parsing or computation on owned `Send` data |

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

A foreground task can await I/O without blocking the UI, but CPU-intensive work inside it still occupies the foreground thread. Move owned, `Send` input into `background_spawn`, await its `Task` from a foreground task, and apply the result there:

```rust
fn parse(&mut self, cx: &mut Context<Self>) {
    let source = self.source.clone();
    self._parse_task = Some(cx.spawn(async move |this, cx| {
        let parsed = cx.background_spawn(async move {
            parse_document(source)
        }).await;

        _ = this.update(cx, |view, cx| {
            view.parsed = Some(parsed);
            cx.notify();
        });
    }));
}
```

The background closure has no `App`, `Window`, or `Context<T>`. Clone only the input it needs before leaving the Entity update. If parsing can outlive a changed document, capture a revision alongside `source` and compare it inside `this.update` before assigning `parsed`.

## A GPUI Kit streaming example

GPUI Kit's [streaming Markdown example](https://github.com/longbridge/gpui-kit/blob/main/examples/stream-markdown/src/main.rs) uses two owned tasks and a channel. A background producer generates text chunks. A foreground receiver owns the Entity update, checks a replay ID, and pushes accepted chunks into `TextViewState`. The View keeps both `Task<()>` handles, so closing it cancels the stream; starting another replay replaces the producer task. The replay ID also rejects chunks already queued by an older producer.

```rust
// From the View's receiver task:
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
            break; // The View has been released.
        }
    }
}));

// From the View's replay method, after incrementing replay_id:
self._producer_task = cx.background_spawn(async move {
    for chunk in chunks {
        if tx.send((replay_id, chunk)).await.is_err() {
            break; // The receiver has gone away.
        }
    }
});
```

This is a GPUI task pattern: task handles express ownership, `WeakEntity` protects the View lifetime, the channel crosses executors, and the replay ID protects state from stale results. See [Entity](./entity) for Entity ownership and updates.

[Entity]: /docs/entity

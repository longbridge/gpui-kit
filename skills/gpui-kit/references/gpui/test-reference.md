# Test Reference

**Contents:** [Testing Patterns](#testing-patterns) · [Testing Crash-Free State Management (Re-entrancy)](#testing-crash-free-state-management-re-entrancy) · [Property Testing](#property-testing) · [Distributed Systems Testing](#distributed-systems-testing) · [Mocking and Isolation](#mocking-and-isolation)

## Testing Patterns

### Basic Entity Testing

Test entity creation, updates, and reads:

```rust
#[gpui_kit::test]
fn test_counter_entity(cx: &mut TestAppContext) {
    let counter = cx.new(|cx| Counter::new(cx));

    // Test initial state
    let initial_count = counter.read_with(cx, |counter, _| counter.count);
    assert_eq!(initial_count, 0);

    // Test updates
    counter.update(cx, |counter, cx| {
        counter.count = 42;
        cx.notify();
    });

    let updated_count = counter.read_with(cx, |counter, _| counter.count);
    assert_eq!(updated_count, 42);
}
```

### Event Testing

Test event emission and handling:

```rust
#[derive(Clone)]
struct ValueChanged {
    new_value: i32,
}

impl EventEmitter<ValueChanged> for MyComponent {}

#[gpui_kit::test]
fn test_event_emission(cx: &mut TestAppContext) {
    let component = cx.new(|cx| {
        let mut comp = MyComponent::default();

        // Subscribe to self
        cx.subscribe_self(|this, event: &ValueChanged, cx| {
            this.received_value = event.new_value;
            cx.notify();
        });

        comp
    });

    // Emit event
    component.update(cx, |_, cx| {
        cx.emit(ValueChanged { new_value: 123 });
    });

    // Verify event was handled
    let received = component.read_with(cx, |comp, _| comp.received_value);
    assert_eq!(received, 123);
}
```

### Action Testing

Test action dispatching and handling:

```rust
actions!(my_app, [Increment, Decrement]);

#[gpui_kit::test]
fn test_action_dispatch(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|cx| MyComponent::new(cx))
        }).unwrap()
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let counter = window.root(&mut cx).unwrap();

    // Dispatch action via focus handle
    let focus_handle = counter.read_with(&cx, |counter, _| counter.focus_handle.clone());
    cx.update(|window, cx| {
        focus_handle.dispatch_action(&Increment, window, cx);
    });

    let count = counter.read_with(&cx, |counter, _| counter.count);
    assert_eq!(count, 1);
}
```

### Async Testing

Test async operations and background tasks:

```rust
impl MyComponent {
    fn load_data(&self, cx: &mut Context<Self>) -> Task<i32> {
        cx.spawn(async move |this, cx| {
            // Simulate async work
            this.update(cx, |comp, _| comp.loading = true).await;
            // Return result
            42
        })
    }

    fn background_update(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            // Background work
            this.update(cx, |comp, _| {
                comp.value += 10;
            }).await;
        }).detach();
    }
}

#[gpui_kit::test]
async fn test_async_operations(cx: &mut TestAppContext) {
    let component = cx.new(|cx| MyComponent::new(cx));

    // Test awaited task
    let result = component.update(cx, |comp, cx| comp.load_data(cx)).await;
    assert_eq!(result, 42);

    // Test detached task
    component.update(cx, |comp, cx| comp.background_update(cx));

    // Detached tasks don't run until you yield
    let value_before = component.read_with(cx, |comp, _| comp.value);
    assert_eq!(value_before, 0);

    // Run pending tasks
    cx.run_until_parked();

    let value_after = component.read_with(cx, |comp, _| comp.value);
    assert_eq!(value_after, 10);
}
```

### Timer Testing

Test timer-based operations:

```rust
impl MyComponent {
    fn delayed_action(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;

            this.update(cx, |comp, cx| {
                comp.action_performed = true;
                cx.notify();
            }).await;
        }).detach();
    }
}

#[gpui_kit::test]
async fn test_timers(cx: &mut TestAppContext) {
    let component = cx.new(|cx| MyComponent::new(cx));

    component.update(cx, |comp, cx| comp.delayed_action(cx));

    // Action shouldn't have completed yet
    let performed = component.read_with(cx, |comp, _| comp.action_performed);
    assert!(!performed);

    // Run until parked (timers complete)
    cx.run_until_parked();

    let performed = component.read_with(cx, |comp, _| comp.action_performed);
    assert!(performed);
}
```

### External I/O Testing

For tests involving external systems, use `allow_parking()`:

```rust
#[gpui_kit::test]
async fn test_external_io(cx: &mut TestAppContext) {
    // Allow parking for external I/O
    cx.executor().allow_parking();

    // Simulate external operation
    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        tx.send(42).ok();
    });

    let result = rx.await.unwrap();
    assert_eq!(result, 42);
}
```

## Testing Crash-Free State Management (Re-entrancy)

The most dangerous class of GPUI bugs is **entity re-entrancy**: code that tries to update or read an entity while it is already locked in a render/update pass. These bugs are invisible at compile time — they only panic at runtime, typically when the user clicks something in a list or dropdown.

**Key properties of re-entrancy panics:**
- Triggered by user interaction (click, key press), not during static rendering.
- `#[should_panic]` only confirms the bug exists — tests must pass *without* panicking.
- `Select` commits through `defer_in` after the key or click dispatch returns. Wait for the result with `cx.wait_for` instead of asserting straight after the interaction.

Drive the component the way a user does. `SelectState` keeps its list state `pub(crate)`, so a test outside `gpui-component` opens the menu, presses keys, and observes through `selected_value()`, the element snapshot, and the delegate's own hooks.

### Pattern: Confirm through the real UI

```rust
use gpui_kit::test::{TestAppContextExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, Entity, TestAppContext, Window, WindowBounds,
    WindowOptions,
    component::{
        IndexPath,
        searchable_list::SearchableListChange,
        select::{SearchableVec, Select, SelectDelegate, SelectItem, SelectState},
    },
    div,
    prelude::*,
    px, size,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

struct Form {
    language: Entity<SelectState<SearchableVec<&'static str>>>,
}

impl Render for Form {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Select::new(&self.language).id("language").w(px(240.)))
    }
}

#[gpui_kit::test]
async fn confirming_from_the_keyboard_commits_the_value(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut language = None;
    let (handle, _view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(px(640.), px(480.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                let state = cx.new(|cx| {
                    SelectState::new(
                        SearchableVec::new(vec!["Rust", "Go"]),
                        Some(IndexPath::new(0)),
                        window,
                        cx,
                    )
                });
                language = Some(state.clone());
                let view = cx.new(|_| Form { language: state });
                view
            },
        )
        .expect("open test window")
    });
    let language = language.unwrap();

    // Open the menu, move to "Go" and confirm — the same path a user takes.
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.within("language").click("input", cx);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();

    // Select commits through `defer_in` after the key dispatch returns, so
    // wait for the closed menu instead of asserting straight away.
    cx.wait_for(handle.into(), Duration::from_millis(500), |window, _| {
        window.find("language").expanded() == Some(false)
            && window.find("language").value() == Some("Go")
    })
    .await;

    cx.update(|cx| assert_eq!(language.read(cx).selected_value(), Some(&"Go")));
}
```

### Pattern: Record `on_will_change` and `on_confirm`

A recording delegate shows which changes the selection strategy proposed and what was committed. `on_will_change` receives the proposed changes and the selection to edit; the delegate applies them itself (the trait's default applies all of them). It has no `cx`, because it runs while the list entity is borrowed, so put side effects that need `cx` in `on_confirm`.

```rust
type Log = Rc<RefCell<Vec<String>>>;

/// Records each hook call; `accept: false` vetoes every change.
struct Recording {
    items: Vec<&'static str>,
    accept: bool,
    log: Log,
}

impl SelectDelegate for Recording {
    type Item = &'static str;

    fn items_count(&self, _: usize) -> usize {
        self.items.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        self.items
            .iter()
            .position(|item| item.value() == value)
            .map(IndexPath::new)
    }

    fn on_will_change(
        &mut self,
        selection: &mut Vec<(IndexPath, Self::Item)>,
        changes: &[SearchableListChange],
    ) {
        for change in changes {
            match *change {
                SearchableListChange::Select { index } => {
                    self.log.borrow_mut().push(format!("select {}", index.row));
                    if self.accept
                        && let Some(item) = self.item(index)
                    {
                        selection.push((index, *item));
                    }
                }
                SearchableListChange::Deselect { index } => {
                    self.log
                        .borrow_mut()
                        .push(format!("deselect {}", index.row));
                    if self.accept {
                        selection.retain(|(ix, _)| *ix != index);
                    }
                }
            }
        }
    }

    fn on_confirm(&mut self, final_selection: &[(IndexPath, Self::Item)]) {
        let values: Vec<_> = final_selection.iter().map(|(_, item)| *item).collect();
        self.log.borrow_mut().push(format!("confirm {values:?}"));
    }
}

struct Picker {
    language: Entity<SelectState<Recording>>,
}

impl Render for Picker {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Select::new(&self.language).id("language").w(px(240.)))
    }
}

fn open_picker(
    cx: &mut TestAppContext,
    accept: bool,
) -> (AnyWindowHandle, Entity<SelectState<Recording>>, Log) {
    cx.update(gpui_kit::init);
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut language = None;
    let (handle, _view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(px(640.), px(480.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                let delegate = Recording {
                    items: vec!["Rust", "Go"],
                    accept,
                    log: log.clone(),
                };
                let state =
                    cx.new(|cx| SelectState::new(delegate, Some(IndexPath::new(0)), window, cx));
                language = Some(state.clone());
                let view = cx.new(|_| Picker { language: state });
                view
            },
        )
        .expect("open test window")
    });
    (handle.into(), language.unwrap(), log)
}

fn confirm_next(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.within("language").click("input", cx);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
}

#[gpui_kit::test]
async fn hooks_fire_in_order_with_the_final_selection(cx: &mut TestAppContext) {
    let (handle, language, log) = open_picker(cx, true);
    confirm_next(cx, handle);
    cx.wait_for(handle, Duration::from_millis(500), |window, _| {
        window.find("language").expanded() == Some(false)
    })
    .await;
    // Single selection proposes replacing the old item, then commits once.
    assert_eq!(
        *log.borrow(),
        ["deselect 0", "select 1", r#"confirm ["Go"]"#]
    );
    cx.update(|cx| assert_eq!(language.read(cx).selected_value(), Some(&"Go")));
}
```

### Pattern: A vetoing delegate keeps the selection

The same delegate with `accept: false` applies none of the proposed changes:

```rust
#[gpui_kit::test]
async fn a_vetoing_delegate_keeps_the_selection(cx: &mut TestAppContext) {
    let (handle, language, log) = open_picker(cx, false);
    confirm_next(cx, handle);
    cx.wait_for(handle, Duration::from_millis(500), |window, _| {
        window.find("language").expanded() == Some(false)
    })
    .await;
    // The same changes are proposed, but the delegate applied none of them:
    // `on_confirm` still runs, with the selection the user started from.
    assert_eq!(*log.borrow(), ["deselect 0", "select 1", r#"confirm ["Rust"]"#]);
    cx.update(|cx| assert_eq!(language.read(cx).selected_value(), Some(&"Rust")));
    cx.update_window(handle, |_, window, _| {
        assert_eq!(window.find("language").value(), Some("Rust"));
    })
    .unwrap();
}
```

### Checklist: What to test for each component that uses `defer_in`

- [ ] `confirm` path: no panic, the value is committed, and `selected_value()` agrees with the trigger's `value()`
- [ ] `cancel` path (Escape, click outside): no panic, selection unchanged, menu closed (`expanded() == Some(false)`)
- [ ] `on_will_change` veto: selection unchanged; `on_confirm` still runs, with the unchanged selection
- [ ] `on_will_change` editing the selection: the final selection reflects the delegate's edit
- [ ] Repeated confirms: each one leaves `selected_value()` matching the displayed value
- [ ] `render_item` never panics even when called immediately after a mutation

## Property Testing

Use random data to test edge cases:

```rust
#[gpui_kit::test(iterations = 10)]
fn test_counter_random_operations(cx: &mut TestAppContext, mut rng: StdRng) {
    let counter = cx.new(|cx| Counter::new(cx));

    let mut expected = 0i32;
    for _ in 0..100 {
        let delta = rng.random_range(-10..=10);
        expected += delta;

        counter.update(cx, |counter, cx| {
            counter.count += delta;
            cx.notify();
        });
    }

    let actual = counter.read_with(cx, |counter, _| counter.count);
    assert_eq!(actual, expected);
}
```

## Distributed Systems Testing

Test multiple app contexts communicating:

```rust
#[derive(Clone)]
struct NetworkMessage {
    from: String,
    to: String,
    data: i32,
}

#[gpui_kit::test]
fn test_distributed_apps(cx_a: &mut TestAppContext, cx_b: &mut TestAppContext) {
    // Create components in different app contexts
    let comp_a = cx_a.new(|_| MyComponent::new("A".to_string()));
    let comp_b = cx_b.new(|_| MyComponent::new("B".to_string()));

    // Simulate message passing
    comp_a.update(cx_a, |comp, cx| {
        comp.send_message("B", 42, cx);
    });

    // Run async operations
    cx_a.run_until_parked();

    // Verify message received in other context
    comp_b.update(cx_b, |comp, _| {
        comp.receive_messages();
    });

    let messages = comp_b.read_with(cx_b, |comp, _| comp.messages.clone());
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].data, 42);
}
```

### Interleaving Testing

Test concurrent operations with random execution order:

```rust
#[gpui_kit::test(iterations = 10)]
fn test_concurrent_operations(
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    mut rng: StdRng,
) {
    let comp_a = cx_a.new(|_| MyComponent::new());
    let comp_b = cx_b.new(|_| MyComponent::new());

    // Perform random operations across contexts
    for i in 0..20 {
        if rng.random_bool(0.5) {
            comp_a.update(cx_a, |comp, cx| {
                comp.perform_operation(i, cx);
            });
        } else {
            comp_b.update(cx_b, |comp, cx| {
                comp.perform_operation(i, cx);
            });
        }
    }

    // Run all pending operations
    cx_a.run_until_parked();

    // Verify final state
    let state_a = comp_a.read_with(cx_a, |comp, _| comp.state);
    let state_b = comp_b.read_with(cx_b, |comp, _| comp.state);

    // Assert invariants hold despite execution order
    assert!(state_a.is_consistent());
    assert!(state_b.is_consistent());
}
```

## Mocking and Isolation

### Network Mocking

Create mock networks for testing distributed features:

```rust
struct MockNetwork {
    messages: Arc<Mutex<Vec<NetworkMessage>>>,
}

impl MockNetwork {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn send(&self, message: NetworkMessage) {
        self.messages.lock().unwrap().push(message);
    }

    fn receive_all(&self) -> Vec<NetworkMessage> {
        self.messages.lock().unwrap().drain(..).collect()
    }
}

#[gpui_kit::test]
fn test_networked_components(cx: &mut TestAppContext) {
    let network = Arc::new(MockNetwork::new());

    let sender = cx.new(|_| MessageSender::new(network.clone()));
    let receiver = cx.new(|_| MessageReceiver::new(network));

    // Send message
    sender.update(cx, |sender, _| {
        sender.send("Hello");
    });

    // Receive message
    receiver.update(cx, |receiver, _| {
        receiver.receive_all();
    });

    let received = receiver.read_with(cx, |receiver, _| receiver.messages.clone());
    assert_eq!(received, vec!["Hello"]);
}
```
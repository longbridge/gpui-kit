---
title: ElementId
description: Give GPUI elements stable identity and understand how keyed state survives frames.
order: -2.4
---

# ElementId

An `ElementId` is a **local key** for an element in GPUI's rendered tree. GPUI combines that key with the IDs of its keyed ancestors to form a `GlobalElementId`. This path lets GPUI associate interaction and element state with the same logical element when a [View renders](./render) again. It also preserves node identity in the [accessibility tree](./accessibility) when the element has a role.

An ID is not a handle to an [Entity] and is not a way to look up an element like an HTML DOM ID. Use an `Entity<T>` for shared application state. Use an `ElementId` for identity in the element tree, including keyed component state, focus or scroll behavior supplied by a component, and state retained by a custom [Element].

## Assign an ID

Calling `.id(...)` on an interactive element such as `div()` returns a `Stateful<Div>`:

```rust
let save = div()
    .id("save-button")
    .on_click(|_, window, cx| {
        // Handle the click.
    })
    .child("Save");
```

The `Stateful<E>` wrapper exposes GPUI's stateful interaction methods and carries the element's ID. A custom `Element` can return `Some(id)` from its `id()` method without using this wrapper. A plain, unkeyed `div()` can still be a layout parent; it contributes no segment to the ID path.

Strings, integers, and name plus integer tuples are common `ElementId` inputs:

```rust
use gpui_kit::component::button::Button;

div().id("search")
div().id(("message", message.id))
Button::new(("delete-project", project.id)).label("Delete")
```

Choose a key from the object's identity, not from the text displayed to the user. A translated label, current selection, or freshly generated random value can change while the object stays the same.

## GlobalElementId and the keyed ancestor path

GPUI builds a `GlobalElementId` from the IDs on the path to an element. Only keyed ancestors add path segments:

```text
div().id("workspace")
├── div().id("inbox")
│   └── div().id(("row", 42))  → ["workspace", "inbox", ("row", 42)]
└── div().id("archive")
    └── div().id(("row", 42))  → ["workspace", "archive", ("row", 42)]
```

The two rows may share a local ID because their keyed ancestor paths differ. This diagram shows only IDs written in the example: an entity-backed View also adds its `EntityId` as a path segment, and a [`RenderOnce`](./render-once) component adds a type-name namespace. The path is scoped to the [Window](./window)'s rendered tree; `GlobalElementId` is GPUI's internal path, not a process-wide string you need to construct at call sites. For a custom drawing API that needs a path for its own key, `window.with_global_id(key, |global_id, window| { … })` creates one within that callback.

The practical uniqueness rule is: **within the same nearest keyed ancestor, each keyed descendant branch needs a distinct ID**. An unkeyed container does not open a new namespace:

```rust
div().id("workspace")
    .child(div().child(div().id("item")))
    .child(div().child(div().id("item"))) // Same keyed path: collision.
```

Give the branches their own stable IDs, or make the item IDs distinct. Duplicate paths can make retained state and interaction attach to the wrong logical element.

## Stable keys in changing lists

For a list that can insert, remove, filter, or reorder rows, derive each row ID from a stable domain value:

```rust
div().id("messages").children(messages.iter().map(|message| {
    div()
        .id(("message", message.id))
        .child(message.preview.clone())
}))
```

`("message", message.id)` separates the row's purpose from other controls that may use the same numeric ID. Reordering changes the drawing position, but each message keeps its keyed path. By contrast, `.id(index)` attaches state to a *position*: after an insertion, the old first row's focus, scroll, animation, or other keyed state may be reused for a different message. An index is appropriate only when the position itself is the identity and cannot shift.

If a repeated row contains several controls, key the row and give its children distinct local keys such as `"edit"` and `"delete"`. Moving the row then carries its whole keyed subtree with it.

### Worked example: a row and its controls

The following uses the actual `div().id(...)` and `Button::new(id)` APIs. `Message::id` is a stable database ID; `preview` is display data that may change. Each row owns a namespace for its controls:

```rust
use gpui_kit::*;
use gpui_kit::component::button::Button;

struct Message {
    id: u64,
    preview: SharedString,
}

fn message_list(messages: &[Message]) -> impl IntoElement {
    div().id("messages").children(messages.iter().map(|message| {
        div()
            .id(("message", message.id))
            .child(message.preview.clone())
            .child(Button::new("archive").label("Archive"))
    }))
}
```

For message `42`, the written part of the button's path is `"messages" → ("message", 42) → "archive"` (GPUI may add View and component namespaces). Every row can call its button `"archive"` because the row IDs differ. If the list order changes from `[42, 7]` to `[7, 42]`, those paths stay with their messages. If message `42` is removed for a rendered frame, its element-local state ends; inserting it again later creates fresh state. An application-level selection or draft that must survive removal belongs in an owned `Entity<T>` or model.

The row key does not automatically key siblings *inside* the row: two `Button::new("archive")` controls under that same row would still collide. Give them distinct local IDs. Also keep the same domain ID when a message's preview or localized label changes; using that text as the key would reset its UI identity.

### Try it: reorder, hide, and restore rows

In the existing `examples/hello_world` package, replace `src/main.rs` with the complete example below and run `cargo run -p hello_world`. Click **Add to 42** twice, then **Swap rows**. Record 42 still shows `2` after moving below record 7. Click **Hide 42**, wait until that row is visibly gone, then click **Show 42**. Its count starts again at `0` because the keyed state was absent from a rendered frame. Keep a persistent count in an application-owned Entity if it must survive hiding.

```rust
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::Button;
use gpui_kit::*;

struct Example {
    reversed: bool,
    show_42: bool,
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let order = if self.reversed { [7_u64, 42] } else { [42, 7] };
        let rows = order
            .into_iter()
            .filter(|id| *id != 42 || self.show_42)
            .map(|id| {
                // This call happens while the parent View renders, before the row div is drawn.
                // Give the state its own domain-derived key; the row ID below keys its subtree.
                let count = window.use_keyed_state(("row-count", id), cx, |_, _| 0_u32);
                let value = *count.read(cx);

                div()
                    .id(("row", id))
                    .h_flex()
                    .gap_2()
                    .child(format!("Record {id}: {value}"))
                    .child(
                        Button::new("increment")
                            .label(format!("Add to {id}"))
                            .on_click(move |_, _, cx| {
                                count.update(cx, |value, cx| {
                                    *value += 1;
                                    cx.notify();
                                });
                            }),
                    )
            })
            .collect::<Vec<_>>();

        div()
            .v_flex()
            .gap_2()
            .p_4()
            .child(Button::new("swap").label("Swap rows").on_click(cx.listener(
                |this, _, _, cx| {
                    this.reversed = !this.reversed;
                    cx.notify();
                },
            )))
            .child(
                Button::new("toggle-42")
                    .label(if self.show_42 { "Hide 42" } else { "Show 42" })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_42 = !this.show_42;
                        cx.notify();
                    })),
            )
            .child(div().id("rows").v_flex().gap_2().children(rows))
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example {
                reversed: false,
                show_42: true,
            })
        })
        .expect("failed to open window");
    });
}
```

The row's `("row", id)` path keeps its controls associated with the same record when the order changes. `Button::new("increment")` can use the same local name in both rows because the row paths differ. The counter uses a *separate* `("row-count", id)` key: it is requested while the parent View builds the rows, before the row's element ID is on the path. Both keys must use the stable record ID. Hiding a row is observable only after GPUI renders a frame without that row; clicking Hide and Show before an intervening render may preserve the old state.

## How IDs retain state

GPUI reconstructs elements during rendering. The Rust value returned by `div()` or a `RenderOnce` component is temporary; assigning it an ID does not turn it into a persistent `Entity<T>`. The ID gives GPUI a way to reconnect element-local state across consecutive frames.

`window.use_keyed_state(key, cx, init)` forms a path from the current keyed ancestors plus `key`. It returns an `Entity<S>` kept for as long as that keyed state is used in consecutive rendered frames. `Window` owns this keyed state; `cx` supplies application access (see [Context](./context) for the different GPUI contexts). `init` runs when that state has no previous entry. GPUI also observes this state entity and notifies the current View when it changes:

```rust
let focus_handle = window
    .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
    .read(cx)
    .clone();
```

GPUI Kit's `Button` uses this pattern for its focus handle. Its public ID gives the recreated button instance the same state key on later frames. The focus handle still owns focus behavior; the element ID identifies where that handle's state belongs. The window retains keyed state only while its path is accessed in successive frames; a [cached View](./view-cache) replays those accesses when it reuses its subtree. A separate strong `Entity<S>` handle can keep that entity alive after its window state entry disappears, but recreating the path will run `init` again.

For a custom `Element`, GPUI passes `Option<&GlobalElementId>` into `request_layout`, `prepaint`, and `paint` when its `id()` returns a value. During drawing, `window.with_element_state(global_id, ...)` can read state from the preceding frame and return the value to store for the next:

```rust
let state = window.with_element_state(
    id.expect("this element always has an ID"),
    |previous: Option<AnimationState>, _window| {
        let state = previous.unwrap_or_default();
        (state.clone(), state)
    },
);
```

This is the pattern behind GPUI Kit's `ScrollBounce` element, which retains motion state during `prepaint`. `with_element_state` is a drawing-phase API for element authors; ordinary Views should prefer Entity state or a component's documented API. GPUI keys stored element state by global path **and state type** and drops it when the element no longer participates in the rendered frames.

:::info
`window.use_state(cx, init)` uses the call site's code location as its local key. It works when the *full path* is unique: a call inside each row is safe if every row has a stable keyed ancestor. If repeated calls share the same keyed ancestor path, use `use_keyed_state` with a stable item key or introduce a keyed namespace around each item.
:::

## Identity changes are state changes

- Changing an element's ID or a keyed ancestor's ID gives it a new path and resets its associated element state.
- Removing an element ends its consecutive-frame state lifetime. Recreating it later initializes that state again.
- An ID does not preserve a View's `Entity<T>` by itself; a strong Entity owner controls that lifetime.
- Keyed state belongs to the Window's rendering context. Do not rely on a `GlobalElementId` to transfer state between windows.

## Effects beyond element state

An ID can be one input to a component's focus, scroll, measurement, or animation state. For example, Kit's `Button` obtains a focus handle with `window.use_keyed_state(self.id.clone(), ...)`, while `ScrollBounce` uses `window.with_element_state(...)` to retain its motion state. Reusing a path for two live controls can therefore mix behavior; changing a path can restart it. A `FocusHandle` or `ScrollHandle` still owns its respective behavior, and changing an `ElementId` does not by itself reset every handle stored elsewhere.

Stable paths also matter to cached Views. A cached Entity View reuses work only while its entity and element path still match; on a cache hit, GPUI replays the subtree's element-state accesses. IDs are not a general render cache: adding `.id(...)` to a `div()` does not make its parent skip `render` (see [View Cache](./view-cache)).

For accessibility, a custom element needs both an ID and a role to become a node. A stable ID lets GPUI preserve that node's identity across redraws, but it does not supply a role, label, keyboard behavior, or focusability. Use the component's accessibility API for those properties (see [Accessibility](./accessibility)).

## Find a bad ID path

1. Identify the logical object whose focus, scroll position, animation, or other state moved or reset. Write down the object's stable domain ID and every keyed ancestor above it. Check whether a sibling now has the same **full** path, or whether an ancestor key changes when data is reordered.
2. Look for unkeyed wrappers between repeated controls. They do not distinguish paths. Replace a shared literal with a stable object-derived ID, or key the repeated parent. Do not fix a collision with a new random value on every render: that trades state sharing for state loss.
3. If state resets only after an item disappears, check whether it was absent for a rendered frame. If it was, keep long-lived state in an Entity or model. If it resets while still present, check changing ancestor IDs, a newly created View Entity, and cache invalidation separately.

A real Kit example is [`DockSkin::render_resize_handle`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/dock/dock.rs): left, right, and bottom resize handles can live below the same keyed ancestor. Giving every handle the literal `"resize-handle"` would produce one `GlobalElementId`; the dock source notes that a press on one handle could then start another handle's drag. It selects `"resize-handle-left"`, `"resize-handle-right"`, and `"resize-handle-bottom"` from the stable placement instead.

In UI integration tests, target IDs are query selectors for observed elements, not a second identity system. Import `gpui_kit::test::TestWindowExt`; `window.find("archive")` requires one matching observed target, so repeated controls need a native scope:

```rust
let archive = window.within(("message", 42_u64)).find("archive");
assert!(archive.visible());
```

`window.within(...)` follows the keyed ancestor path even when the ancestor itself is not observed. An ambiguous `find` or `try_find` asks for a scope; it does not necessarily mean GPUI state collided, since two controls can correctly share a local ID under different row paths. Test the real click or focus outcome as well as the snapshot (see [Testing](./test)).

See [Element](./element) for the layout, prepaint, and paint lifecycle, and [Entity](./entity) for state that must outlive an element's presence in the tree.

[Element]: /docs/element
[Entity]: /docs/entity

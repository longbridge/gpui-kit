---
title: Nav Stack
description: A navigation stack of views with push, pop, forward, and replace, and an animatable transition lifecycle.
order: 16
---

# Nav Stack

A last-in-first-out stack of views, one visible at a time: push a view over the current one, pop back to the one below, or replace the top. It is SwiftUI's `NavigationStack`, Qt's `StackView`, and WinUI's `Frame`. Underneath it is a [History](../history.md) whose active entries run from the root through the current page. A popped page becomes a forward entry until the next push discards that forward branch, so `forward` brings it back the way WinUI's `GoForward` does.

Like every `gpui-base` primitive, Nav Stack supplies behavior and semantic structure without imposing a product visual language. The pages are views you create, and how a change between them moves is decided by your item renderer.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- nav-stack
```

## Import

```rust
use gpui_kit::base::{NavMotion, NavOperation, NavPage, NavStack, NavStackState};
use gpui_kit::base::motion::{PresencePhase, Transition};
```

## Anatomy and API

`NavStackState` is the stack. It lives in a GPUI entity, holds `AnyView`s root first, and emits `NavStackEvent` after every change.

| Method                                                             | Does                                                                                                                                    |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `push(view, motion, cx)`                                           | Pushes over the current top. Into an empty stack it is immediate, like Qt's `initialItem`.                                              |
| `pop(motion, cx)`                                                  | Pops the top and returns it. The root is never popped, so this returns `None` at a depth of one.                                        |
| `pop_to_root(motion, cx)`                                          | Pops everything above the root in one transition and returns those views.                                                               |
| `forward(motion, cx)`                                              | Brings back the most recently popped view over the current top and returns it. `None` when nothing has been popped since the last push. |
| `replace(view, motion, cx)`                                        | Swaps the top for `view` and returns the one replaced, keeping the forward views. On an empty stack it pushes.                          |
| `clear(cx)`                                                        | Empties the stack and the forward views immediately.                                                                                    |
| `depth()`, `is_empty()`, `current()`, `views()`, `forward_views()` | Read the stack. Show a back button when `depth() > 1`, a forward button when `forward_views()` is not empty.                            |

`NavStack` is the element. It holds the entity, takes a `transition` to run each change under, and hands every mounted view to the `item` renderer as a `NavPage`. Style the element for size, background and clipping; it is positioned so that the two pages of a change can overlap.

`NavPage` is what the renderer receives. It already fills the container. Read `phase()` (`Entering`, `Present` or `Exiting`), `operation()` (`Push`, `Pop` or `Replace`, or `None` once settled) and `progress()` (eased, `0.0` to `1.0`, shared by both pages of one change), refine the page with GPUI styles, and return it.

The authoritative module is [`components/nav_stack.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/nav_stack.rs). Native and browser previews compile this same file.

## Animation

Animation is decided at two levels, and both default to none:

- **The stack.** `NavStack` without a `transition` never animates; every change switches on the spot. Give it a `Transition` to animate changes, and an `item` renderer to say how.
- **The change.** Each `push`, `pop`, `pop_to_root` and `replace` takes a `NavMotion`, as UIKit's `animated:` and Qt's `StackView.Immediate` do per call. `NavMotion::Animated` runs the stack's transition; `NavMotion::Immediate` switches on the spot even on an animated stack, which is what restoring a stack at launch or jumping to a page from a command wants.

```rust
stack.update(cx, |stack, cx| stack.push(detail, NavMotion::Animated, cx));
stack.update(cx, |stack, cx| stack.push(restored, NavMotion::Immediate, cx));
```

## Transitions

After a push, pop or replace, the outgoing view stays mounted until the element's `Transition` finishes. Paint order follows the operation: a pushed or replacing page paints over the page it covers, and a popped page paints over the page it reveals, so a slide reads correctly in both directions.

```rust
NavStack::new(&self.stack)
    .size_full()
    .overflow_hidden()
    .transition(Transition::new(Duration::from_millis(220)))
    .item(|page, _, _| {
        let offset = match (page.phase(), page.operation()) {
            (PresencePhase::Entering, Some(NavOperation::Push)) => 1.0 - page.progress(),
            (PresencePhase::Exiting, Some(NavOperation::Pop)) => page.progress(),
            _ => 0.0,
        };
        page.left(relative(offset)).into_any_element()
    })
```

The stack also switches immediately when the platform asks for reduced motion, whatever the renderer would have drawn. A new operation while a transition is running supersedes it, and the pages reverse from where they are rather than jumping. While a change runs, neither page takes pointer input.

## State and events

Keep the `NavStackState` entity on the view that renders the stack and observe it, so a push from anywhere re-renders the host. A page that needs to navigate holds a `WeakEntity` of the stack, as the showcase page does.

`views()` and `forward_views()` are enough for a history menu: list both, and pop or forward until the chosen page is current. The showcase page draws that list as a trail of page numbers, the pages ahead greyed out.

Focus is not moved by the stack. `AnyView` carries no focus handle; a page that wants focus takes it when it is pushed, as it would anywhere else.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/nav_stack.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Announce the page change in the page itself: a heading at the top of each page gives assistive technology a landmark to land on after a push. The stack keeps only the current page interactive once a transition has finished.

## Notes

Pages are entities. The stack retains the ones on it and the ones popped since the last push, which `forward` can bring back, so a page's own subscriptions and timers live until a push discards it or the stack is cleared. Verify reduced-motion behavior in the consuming design system.

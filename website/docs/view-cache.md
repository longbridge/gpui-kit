---
title: View Cache
description: Reuse clean GPUI view subtrees and distinguish view caching from element state, geometry caching, and virtualization.
order: -2.633
---

# View Cache

GPUI keeps application state in [entities](./entity), but normally builds a new [element tree](./element) for each window draw. Keeping an `Entity<T>` alive does not keep its last element tree alive. When a window redraws, an ordinary child view may run `Render::render` again even if its own state did not change. A cached view instead reuses selected records from the last frame, including its input handlers.

There is no public type named `ViewCache` to construct. The view-cache API in current GPUI is **`Entity<T>::cached(style)`** (or **`AnyView::cached(style)`**). It creates a cached view boundary for one entity-backed subtree. GPUI Kit also uses other, narrower caches; they solve different costs.

| Mechanism | Reuses or avoids | Lifetime and owner |
| --- | --- | --- |
| `Entity::cached(style)` | A clean view's render, child layout/prepaint, and paint work | GPUI's window cache, keyed by the entity view and its element path |
| `Window::use_keyed_state` | Small state or computed data used by a rebuilt element | Window element state under a stable `ElementId` |
| A model-owned cache | A derived value, measurement, or drawing resource | An owning `Entity<T>`, with application-defined invalidation |
| `VirtualList` | Building offscreen rows | Its visible range; a separate scroll handle keeps scroll position |

These mechanisms can be combined. For example, a cached panel may contain a virtual list, and a visible chart row may reuse tessellated paths. A virtual list does **not** cache all of its row views, and `use_keyed_state` does **not** skip a view's `render`.

## Cache an entity-backed subtree

Keep the child entity in the parent across renders. Pass a layout style when embedding it:

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

Create `panel` when constructing `Workspace`, such as with `cx.new(...)`, and retain it in the struct. Creating a fresh entity inside `render` gives it a new identity and loses both its state and its warm cache. The `style` argument is the cached view's **outer layout contract**. GPUI lays out that box before deciding whether to reuse its contents; it cannot ask an unrendered subtree for an intrinsic size. Give the box a definite size from the parent (`size_full()` in a bounded parent, or explicit dimensions). For content-sized views, embed the entity normally with `.child(self.panel.clone())`.

`AnyView::cached(style)` has the same behavior when a parent stores a type-erased panel, as GPUI Kit's dock does. [`RenderOnce`](./render-once) values and arbitrary `ViewElement`s cannot opt into this API: they have no entity notification contract to invalidate a frozen subtree.

## When does GPUI reuse it?

On a cache hit, GPUI does not call that child view's `render`. It replays the earlier subtree's prepaint and paint records into the current frame. These include hitboxes, dispatch nodes, focus state, mouse listeners, input handlers, and the drawing scene, so the cached area remains interactive. The original element values are not kept as a permanent tree. Event handlers in a reused subtree run again on input; only the work to rebuild and paint them is skipped.

For a hit, the entity must remain clean, and the cached view's **bounds, content mask, and inherited text style** must match the recorded frame. GPUI also bypasses reuse during a forced refresh; inspector picking can disable caching. If any of these conditions fail, it renders and lays out the child again, then records a new cache entry.

| Change | What happens at this boundary |
| --- | --- |
| A sibling or parent redraws, while this child and its box stay unchanged | The parent still builds its tree; the cached child can be replayed. |
| The child changes and calls `cx.notify()` | GPUI marks the view dirty, rebuilds it, and updates the cache. |
| A descendant view changes and notifies | GPUI marks its ancestor view path dirty so the cached boundary is rebuilt. |
| The cached box resizes, clips differently, or inherits a different text style | GPUI misses this cache entry and rebuilds it. |
| The child is removed or its identity/path changes | Its old cached subtree cannot be used at the new position. |

Keep state mutations in event handlers or tasks and call `cx.notify()` on the affected entity when its visible output changes. If the child's output depends on a [Global](./global), observe that global and notify the child; changing a global by itself does not dirty cached readers. If it depends on another entity, use an observation or another explicit invalidation path. Do not assume that a parent re-render alone will refresh a clean cached child. Use `window.refresh()` for an intentional full refresh, not as a normal state-update mechanism.

Caching has a scope: it can skip work *inside* the child boundary, but the window still draws a frame and the parent still runs as needed. The first draw and every cache miss pay the ordinary render/layout/paint cost. Use it where a measured subtree is expensive and often stays clean while surrounding content changes.

## Element state is a different cache

GPUI recreates value-like elements on subsequent renders. If an element needs a little state across consecutive frames, `Window::use_keyed_state` stores an `Entity<S>` under the current element path plus a supplied key. It also observes that state entity and notifies the current View when the state changes. The state survives while that path is accessed on successive frames, including frames where a cached subtree replays its element-state accesses; it is released when the path disappears and no other strong handle keeps it alive. `Window::use_state` uses a call-site key, which is suitable only where that location uniquely identifies the state. An [`ElementId`](./element_id) derived from stable domain data matters for repeated or reorderable items: changing an ID resets the state; reusing one for unrelated siblings risks collision.

GPUI Kit's [`Plot` path cache](../component/plot) is a concrete example. A plot and each `Line` are rebuilt as values, so a path held on a `Line` would disappear with that value. `PathCaches::for_paint("lines", window, cx)` stores caches in keyed window state under the plot's element ID. A `ShapeKey` covers projected points and geometry-affecting stroke settings; `PathCache::get` tessellates only when that key changes. The path is built relative to zero and translated to the current origin for painting, so moving a chart can reuse the geometry. A color change can be applied while painting without rebuilding unchanged path geometry.

That cache saves **path construction**, not the plot view's `render` or the current frame's paint submission. Its slots are positional: when series can reorder, map stable series identities to slots or ensure the shape key safely invalidates the changed slot. See [Paint](./paint#plot-a-value-like-element-uses-keyed-window-state) for the source-level walkthrough.

## Virtualization avoids work instead of replaying it

For a long collection, caching a view containing every row still leaves an expensive first render and invalidations. GPUI Kit's [`VirtualList`](../base/virtual-list) accepts item sizes and calls its render closure for the visible range (with a small overdraw); it may also render one representative item for cross-axis measurement. It never constructs most offscreen row elements in that frame. Its `VirtualListScrollHandle` is retained separately in the owner so scrolling survives element rebuilds.

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

The closure should read prepared data; avoid sorting, loading, or creating one long-lived entity per row in it. Give repeated interactive rows stable IDs from the row data. Virtualization and cached views address different dimensions: how **many** elements are made, and whether an **unchanged subtree** is replayed.

## Choose the smallest useful boundary

Start with ordinary entities and declarative rendering. If profiling shows a stable panel being rebuilt because nearby UI changes, place `cached(style)` around that panel and give it a reliable layout box. If a drawing operation remains costly on every visible frame, cache its derived geometry with explicit keys and invalidation. If the cost grows with collection length, virtualize. Do not add a broad cache to hide render work that should have been moved out of `render` or data that should have been retained by an entity.

The [Render](./render) guide explains when a View creates a new tree.

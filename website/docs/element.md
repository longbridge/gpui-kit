---
title: Element
description: Understand GPUI's element tree and low-level rendering lifecycle.
order: -2.7
---

# Element

An **Element** is a node in the element tree that GPUI builds for one frame. Elements perform layout, prepare hit testing, and paint pixels into a Window. GPUI drops the tree and its frame-local callbacks before the next frame, then builds a new tree from the application's current state.

Most application code should compose the elements provided by GPUI and GPUI Kit:

```rust
div()
    .flex()
    .items_center()
    .gap_2()
    .child(Icon::new(IconName::Search))
    .child("Search")
```

This code creates an element tree. It does not implement GPUI's low-level `Element` trait.

## `Element`, `IntoElement`, and `AnyElement`

These types have different jobs:

| Type | Purpose |
| --- | --- |
| `Element` | Implements the low-level layout and painting lifecycle. |
| `IntoElement` | Converts a value into a concrete `Element`, allowing strings, components, Entities, and other values to be passed to `.child(...)`. |
| `AnyElement` | Erases the concrete element type, which is useful for heterogeneous collections, conditional branches, slots, and stored children. |

Keep the concrete type when every branch has the same type. Erase it only at a boundary that needs different element types:

```rust
fn status_icon(online: bool) -> AnyElement {
    if online {
        Icon::new(IconName::CircleCheck).into_any_element()
    } else {
        div().child("Offline").into_any_element()
    }
}
```

GPUI Kit uses `AnyElement` this way for optional slots, table cells, and functions whose branches return different UI types. `IntoElement` is the usual API boundary when the caller does not need type erasure.

## The three phases

GPUI drives an `Element` through three trait methods in order. The layout solver runs between the first and second calls:

<ol class="element-lifecycle" aria-label="GPUI Element frame lifecycle">
  <li><strong>1 · Build tree</strong><span><code>Render::render</code> creates this frame's Element values.</span></li>
  <li><strong>2 · Request layout</strong><span>GPUI calls <code>Element::request_layout</code>; return <code>LayoutId</code> and <code>RequestLayoutState</code>.</span></li>
  <li><strong>3 · Resolve bounds</strong><span>Taffy computes the layout. GPUI obtains <code>Bounds&lt;Pixels&gt;</code>.</span></li>
  <li><strong>4 · Prepaint</strong><span>GPUI calls <code>Element::prepaint</code>; prepare geometry and hitboxes, return <code>PrepaintState</code>.</span></li>
  <li><strong>5 · Paint</strong><span>GPUI calls <code>Element::paint</code>; submit drawing and current-frame input listeners.</span></li>
</ol>

The tree and frame-local listeners are discarded before the next frame. `RequestLayoutState` and `PrepaintState` move forward within this pass; they are not persistent caches.

### `request_layout`

Register the element's `Style` and child layout nodes with `window.request_layout`. GPUI's Taffy layout engine resolves their sizes and positions after layout has been requested.

Return a `LayoutId` and any `RequestLayoutState` needed by the later phases. Do not assume the final `Bounds` are available here.

### `prepaint`

GPUI now provides the resolved `Bounds`. Use this phase to shape text, calculate geometry, insert hitboxes, prepaint children, and prepare data that `paint` needs.

Return that data as `PrepaintState`. Hit testing belongs here because GPUI must establish the current frame's spatial and dispatch information before painting.

### `paint`

Paint quads, text, paths, or images with the prepared state. Register frame-local input handlers when the custom element requires them. This phase should consume the geometry prepared earlier rather than repeat layout work.

`RequestLayoutState` reaches both `prepaint` and `paint`; `PrepaintState` reaches `paint`. Persistent application state belongs in an [Entity](./entity), while small element state that must survive frames can be associated with an [ElementId](./element_id).

### A complete, minimal custom element

This invisible event surface fills its parent's bounds. It shows the exact trait signatures and why the hitbox is carried from `prepaint` into `paint`:

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
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        _: &mut Window,
        _: &mut App,
    ) {}
}
```

The element is intentionally invisible and has no handler. A hitbox is geometry for input routing, not a click callback. [GPUI Kit's `CarouselScrollMask`](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/carousel/scroll_mask.rs) starts with this pattern, then retains its `Hitbox` and installs frame-local pointer and wheel listeners in `paint`. It uses `Hitbox::should_handle_scroll` to avoid claiming gestures blocked by a surface in front. When an element has children, request their layouts first and pass their `LayoutId`s to `window.request_layout`; then prepaint and paint them in order. For intrinsic measurement, `window.request_measured_layout` takes a measurement closure instead.

`HitboxBehavior::Normal` participates in hit testing without hiding hitboxes behind it. `BlockMouse` occludes both mouse and scroll handling behind the hitbox; `BlockMouseExceptScroll` leaves scrolling available. The hitbox also carries the current content mask. In a custom element, coordinate the hitbox with any clipping and paint order; drawing a shape does not create a hitbox automatically.

### Identity and retained state

`Element::id()` returns a local `ElementId`. GPUI combines it with keyed ancestors into a `GlobalElementId` and passes that global ID into the three phases. An element can use it with `window.with_element_state` for small state that survives rebuilding the element tree. GPUI Kit's carousel surface uses that mechanism for ongoing scroll state. The ID must be unique among siblings under the nearest keyed ancestor. Use a domain ID for reorderable items; an index changes meaning after insertion or sorting. An element without an ID receives `None` and cannot use this keyed state channel. Entity state remains the right owner for application data and subscriptions.

### Text is a specialized low-level element

The `TextSystem` owns font lookup, shaping, glyph metrics, and caches. `cx.text_system()` exposes it; a window also has a window-specific text system. Text width depends on font, size, shaping, and available width, so a custom text element may need `request_measured_layout` or a measured line during layout. Its `prepaint` can then compute line geometry, selections, and hitboxes; `paint` draws the prepared text. Keep the same font parameters through measurement and paint, or caret and selection positions will drift. Use GPUI's `text(...)` and GPUI Kit's text components unless you need selection, inline objects, or a specialized editor. [GPUI Base's TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) shows measured layout, an element-owned hitbox, and painting tied to the resolved bounds. [GPUI Kit's input element](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) shows the same pipeline for editing.

## When to implement `Element`

Implement `Element` when existing elements cannot express the work, for example:

- a code editor or text input that shapes text and paints selections and cursors;
- a chart or canvas with custom geometry;
- a custom layout algorithm;
- a performance-sensitive primitive that needs direct control over layout, hitboxes, and painting.

GPUI Kit's input element uses a custom `Element` because it must shape text, register an input handler, and paint selections and cursors. GPUI's `Svg`, `Img`, lists, and canvas use the same lifecycle. Most application UI can instead compose existing elements and convert differing branches to `AnyElement`.

For a reusable UI component, start with [`RenderOnce`](./render). For stateful UI owned by an Entity, use [`Render`](./render). Drop down to `Element` only when you need to control the rendering pipeline itself.

### Decisions visible in GPUI Kit source

| Example | Why composition alone is insufficient | What the element owns |
| --- | --- | --- |
| [Input](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/input/base/element.rs) | Caret, selection, wrapping, and pointer-to-text mapping must use the same measured text geometry. | Text layout, hitboxes, input handlers, and paint order. |
| [TextView](https://github.com/longbridge/gpui-kit/blob/main/crates/base/src/text/text_view.rs) | Rich text and selectable ranges need measurement and clipping tied to resolved bounds. | Text layout state, hitbox, selection surface, and prepared painting. |
| [CarouselScrollMask](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/carousel/scroll_mask.rs) | A gesture surface must occupy the viewport independently of moving carousel content. | Full-size layout, hitbox, and pointer/wheel dispatch; it paints nothing. |
| [Plot line](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/plot/shape/line.rs) | Data points require direct path tessellation, but do not require custom layout or input. | A path painted inside a higher-level chart; this shape itself does not need a full `Element` implementation. |

These examples show that `Element` is about owning a phase boundary, not merely drawing something custom. The carousel surface demonstrates that an Element may paint nothing. Plot demonstrates the opposite: a custom drawing can use a canvas or a containing Element without making each shape an Element.

### Performance follows phase ownership

**Make the efficient path the natural API path.** GPUI divides an Element into layout, prepaint, and paint so authors can measure when dimensions are known, prepare geometry once, and reuse it during painting. GPUI Kit follows the same principle in its components: costs that repeat across a large tree should have an owner and an explicit invalidation rule. The goal is for ordinary component use to avoid repeated work by design, rather than requiring every caller to repair it with a cache.

Every rendered frame can rebuild the tree and revisit these methods. Keep `request_layout` focused on styles and layout nodes; do not shape text or tessellate paths there unless intrinsic measurement requires it. In `prepaint`, compute geometry once and pass it through `PrepaintState` to `paint`. Avoid rebuilding unchanged paths on every paint; GPUI Kit's Plot uses keyed window state and a shape key to retain tessellation across frames. Put subscriptions and long-lived application data in an Entity instead of a frame-local associated state. Finally, clip and cull before expensive painting: TextView and Input compute visible content from resolved bounds rather than blindly painting the full document.

## How GPUI drives an Element

The low-level contract is enforced by a `Drawable<E>` wrapper. It moves through `Start → RequestLayout → LayoutComputed → Prepaint → Painted`; calling the phases out of order is an error. This explains why GPUI passes two associated state values forward instead of asking the Element to rediscover everything during paint. It also explains why a custom Element should not call drawing methods during `request_layout`: the window is not in its paint phase yet.

### Layout tree, dispatch tree, and scene are different structures

`request_layout` contributes nodes to the layout engine and returns a `LayoutId`. After Taffy resolves that layout, `prepaint` obtains pixel bounds. GPUI creates a dispatch node for the Element while prepainting; its child elements and hitboxes establish the input geometry for the current frame. `paint` activates the corresponding dispatch node so Action, keyboard, and mouse listeners registered there follow the same tree path. Drawing commands go to the scene. A `paint_path` call changes the scene, but does not add a dispatch node or hitbox. Conversely, the carousel mask contributes input geometry and listeners without adding visible drawing.

This separation lets a custom Element do precisely one job. A decoration may only need a canvas and scene commands. A focusable control must coordinate dispatch, hitboxes, focus, and accessibility as well. A virtualized list must decide which children to measure and paint, not merely draw a large rectangle.

### Identity is a path, not a pointer to this frame's value

When `Element::id()` returns an `ElementId`, GPUI appends it to the current keyed ancestor path and passes the resulting `GlobalElementId` to the three methods. The Rust Element value itself is recreated for a later frame; its address is not a stable identity. `window.with_element_state(global_id, ...)` can carry a small typed value between consecutive painted frames. `window.use_keyed_state(key, cx, init)` builds an Entity at a key in the current Element namespace and observes it so changes can notify the owning view. These mechanisms require stable keys; an item index is unsafe when items can move.

### Accessibility is established with geometry

When accessibility is active, GPUI can create an AccessKit node for an Element that has both an ID and `a11y_role()`. During prepaint it sets that node's bounds from the resolved layout, calls `write_a11y_info`, and can add synthetic children. Painting pixels alone supplies no role, name, value, or action. Standard interactive GPUI Kit components already provide these semantics; a custom low-level control must supply its own contract. Continue with [Accessibility](./accessibility) for those APIs.

## Identity and interactivity

These concepts sit on separate boundaries:

- [`ElementId`](./element_id) identifies an element within its keyed scope. GPUI uses its global form to connect state and retained work across frames.
- Calling `.id(...)` on an `InteractiveElement` returns `Stateful<E>`. That wrapper enables APIs whose state must be associated with a stable element identity.
- `InteractiveElement` exposes GPUI's standard interaction machinery, including hitboxes, mouse listeners, Focus tracking, Key Context, and Action handlers. A custom `Element` does not gain those behaviors automatically.

`InteractiveElement` is implemented by types that expose an `Interactivity` field. Its methods include `on_mouse_down`, `on_key_down`, `track_focus`, `key_context`, `on_action`, and hover styles. Calling `.id(...)` returns `Stateful<Self>`, which implements `StatefulInteractiveElement` and exposes state-dependent methods such as accessibility role and label, tooltip, and click handling. The wrapper is a type-level signal: stateful interaction needs a stable identity. Implementing `IntoElement` alone only makes a value acceptable as a child; it does not implement either interactive trait. `#[derive(IntoElement)]` plus `RenderOnce` is the usual component path, while a hand-written low-level `Element` typically implements `IntoElement` by returning itself.

There are two useful ways to obtain those APIs. When composing an ordinary surface, use an existing interactive element: `div().id("result-row").aria_label("Search result").on_click(|_, _, _| {})`. The `.id(...)` call changes the Rust type from `Div` to `Stateful<Div>`, so identity-dependent methods become available. Give each repeated row a domain-derived ID rather than the same literal ID. `on_click` registers a handler; it does not automatically update an Entity's state.

When exposing the same fluent methods on a GPUI Kit component, delegate its interaction state to the underlying primitive. The component's [Radio implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/radio.rs) follows this shape (the excerpt omits unrelated fields):

```rust
impl InteractiveElement for Radio {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Radio {}
```

`StatefulInteractiveElement` is a marker trait: its default methods write into the returned `Interactivity`; the empty implementation does not create an ID or a hitbox. `Radio::new(id)` passes a stable ID to its Base primitive, which renders the actual interactive surface. If your component cannot guarantee that identity and the underlying interactive element, expose only the methods it can support. A raw custom `Element` such as `EventSurface` above has neither trait until you explicitly provide and drive that interactivity or compose an existing interactive element.

## `canvas`: two phase callbacks without a new trait implementation

GPUI's `canvas(prepaint, paint)` is itself an `Element`. It requests layout from its `Styled` properties, then invokes a `FnOnce` prepaint callback with resolved bounds. That callback returns any temporary value `T`, which GPUI passes to the `FnOnce` paint callback. This is a small bridge into the Element lifecycle, not an HTML canvas or a retained drawing surface.

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

The `prepaint` result here is `Option<Path<Pixels>>`; it exists only for this frame. [GPUI Kit's dashed Separator](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/separator.rs) uses `canvas` to draw a path in its resolved bounds, and the [circular Progress](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/progress/progress_circle.rs) returns measured radii from prepaint to paint. Use this for a focused drawing callback inside `Render` or `RenderOnce`. It has no built-in children, hitbox, focus tracking, or stable Element ID. When those responsibilities need to work together, implement `Element` or compose a standard interactive element around the canvas.

When an existing interactive element such as `div()` already provides the behavior you need, compose it. If a custom primitive needs standard interaction, embed or delegate to GPUI's `Interactivity`, as GPUI's built-in elements do. Implementing raw hitboxes and event registration yourself also makes you responsible for dispatch, clipping, cursor behavior, and accessibility.

:::info
`Element::id()` returning an `ElementId` does more than label pixels: it creates stable identity across frames. Keep IDs unique within their nearest keyed ancestor, and do not add an ID unless the element or an attached behavior needs identity.
:::

[Element]: https://docs.rs/gpui/latest/gpui/trait.Element.html
[IntoElement]: https://docs.rs/gpui/latest/gpui/trait.IntoElement.html
[AnyElement]: https://docs.rs/gpui/latest/gpui/struct.AnyElement.html

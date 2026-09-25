---
title: SharedString
description: Choose and use GPUI's immutable, cheaply cloned text for UI state and components.
order: -2.45
---

# SharedString

Small costs add up. In UI code, a cost built into an ordinary component call is repeated wherever that component appears and redraws. GPUI and GPUI Kit treat that as an API design constraint: retained text should be easy to own and pass around without making every caller copy its full contents. `SharedString` is one concrete choice that gives component authors this default.

`SharedString` is GPUI's owned, immutable text type. GPUI Kit uses it for labels, placeholders, titles, and other text that an element or component keeps after the builder call returns. Use it by default for **UI text** held across [render](./render) calls, component boundaries, or task closures. Borrow temporarily with [`&str`](https://doc.rust-lang.org/std/primitive.str.html). Import `SharedString` with `use gpui_kit::*;` or `use gpui_kit::SharedString;`.

## Small costs add up

Small copies and allocations are a long-term budget to control when choosing framework types, not a burden to leave at every call site. If a label API required each caller to supply a newly owned buffer, ordinary composition would repeat full text copies across many components. By accepting `SharedString` for retained text, a component can keep an owned value while callers reuse the one they already hold. Authors do not need a hand-written cache around each label for that common path.

This default avoids repeated **full text** copies, not all work: cloning inline text copies its small bytes, cloning shared heap text updates a reference count, and `format!(...).into()` still constructs new text each time it runs. Other costs have their own tools, such as [virtualization](../base/virtual-list) for offscreen rows and [view caching](./view-cache) for an unchanged subtree.

## Why use it instead of `String` for UI text?

Consider a title passed from a workspace View through a header and tab component to a final label. Each layer that keeps the title after its caller returns needs a valid ownership choice: move the value and give up the caller's copy, borrow it with a lifetime tied to the source, or clone it. Rust's [`String`](https://doc.rust-lang.org/std/string/struct.String.html) owns a mutable buffer, so cloning a nonempty `String` at each boundary creates independent buffers and copies the title's bytes. Rebuilding the title with `format!` at each layer repeats formatting and allocation instead.

`SharedString` represents that immutable value in a form that is cheap to clone. The workspace can retain one value while each layer takes an owned clone; for long heap-backed text, the clones share its bytes instead of copying them at every handoff. This is why GPUI and GPUI Kit expose it in many text properties and accept `impl Into<SharedString>` at component boundaries. The tradeoff is immutability: changing the text means constructing a new value. Sharing helps while the value stays unchanged; it is not a promise of zero cost.

<figure class="shared-string-memory">
  <svg viewBox="-16 -16 912 416" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="shared-memory-title-en shared-memory-desc-en">
    <title id="shared-memory-title-en">Three String owners compared with three SharedString owners</title>
    <desc id="shared-memory-desc-en">For long heap-backed text, three String owners each point to a separate full text buffer. Three SharedString owners hold separate small handles that point to one shared text buffer with a reference count. Animated dashed arrows show the ownership links, not elapsed time.</desc>
    <defs>
      <marker id="shared-arrow-en" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M1 1 L7 4 L1 7" /></marker>
    </defs>
    <rect class="memory-panel" x="1" y="1" width="878" height="185" rx="12" />
    <text class="memory-heading" x="24" y="29">String::clone()</text>
    <text class="memory-subtitle" x="24" y="49">Each owned clone copies the full text into its own allocation.</text>
    <rect class="memory-owner" x="24" y="65" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="83">View A · handle</text>
    <rect class="memory-owner" x="24" y="103" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="121">View B · handle</text>
    <rect class="memory-owner" x="24" y="141" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="159">View C · handle</text>
    <path class="memory-flow memory-flow-copy" d="M183 78 H451" marker-end="url(#shared-arrow-en)" />
    <path class="memory-flow memory-flow-copy" d="M183 116 H451" marker-end="url(#shared-arrow-en)" />
    <path class="memory-flow memory-flow-copy" d="M183 154 H451" marker-end="url(#shared-arrow-en)" />
    <rect class="memory-buffer memory-buffer-copy" x="463" y="65" width="190" height="27" rx="5" /><text class="memory-label" x="476" y="83">Text buffer A · full text</text>
    <rect class="memory-buffer memory-buffer-copy" x="463" y="103" width="190" height="27" rx="5" /><text class="memory-label" x="476" y="121">Text buffer B · full text</text>
    <rect class="memory-buffer memory-buffer-copy" x="463" y="141" width="190" height="27" rx="5" /><text class="memory-label" x="476" y="159">Text buffer C · full text</text>
    <text class="memory-total memory-total-copy" x="676" y="121">3 text buffers</text>
    <g transform="translate(0 12)">
    <rect class="memory-panel" x="1" y="186" width="878" height="185" rx="12" />
    <text class="memory-heading" x="24" y="214">SharedString::clone()</text>
    <text class="memory-subtitle" x="24" y="234">Each owner keeps a handle; the long text stays in one allocation.</text>
    <rect class="memory-owner" x="24" y="251" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="269">View A · handle</text>
    <rect class="memory-owner" x="24" y="289" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="307">View B · handle</text>
    <rect class="memory-owner" x="24" y="327" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="345">View C · handle</text>
    <path class="memory-flow memory-flow-share" d="M183 264 L451 303" marker-end="url(#shared-arrow-en)" />
    <path class="memory-flow memory-flow-share" d="M183 302 H451" marker-end="url(#shared-arrow-en)" />
    <path class="memory-flow memory-flow-share" d="M183 340 L451 303" marker-end="url(#shared-arrow-en)" />
    <rect class="memory-buffer memory-buffer-share" x="463" y="277" width="190" height="51" rx="6" /><text class="memory-label" x="476" y="300">One shared text buffer</text><text class="memory-detail" x="476" y="317">reference count: 3</text>
    <text class="memory-total memory-total-share" x="676" y="307">1 text buffer</text>
    </g>
  </svg>
  <figcaption>Conceptual long-text example with three owners, not a byte-accurate benchmark. <code>SharedString</code> still stores a handle per owner and updates a reference count; short inline values copy their small bytes instead.</figcaption>
</figure>

## How it stores text

GPUI currently backs `SharedString` with `SmolStr`. The representation depends on how the value is created and on its length in **UTF-8 bytes**:

| Input | Current storage | What cloning does |
| --- | --- | --- |
| `SharedString::new_static("Ready")` | References the static literal | Copies the small handle; no heap allocation |
| `SharedString::from("Ready")` | Copies this short text inline | Copies the inline bytes; no heap allocation |
| Longer dynamic text | Shared heap allocation | Clones a reference-counted handle; no copy of the text bytes |

The current inline capacity is 23 bytes. Certain runs of newlines followed by spaces also use a special allocation-free static representation. These are implementation details, not a length limit or a promise that every `SharedString` avoids allocation. In particular, `SharedString::from("a long literal ...")` uses the normal conversion path; use `new_static` when you explicitly want static storage for a literal. Neither construction nor cloning interns strings or deduplicates equal values.

For these current representations, `SharedString::clone()` does work bounded independently of the text length: a static clone copies a handle, a short inline clone copies a few bytes, and a heap-backed clone copies a handle and updates a reference count. It does not duplicate the bytes of a long heap-backed value. `String::clone()` instead allocates an independent buffer and copies its bytes. Constructing a fresh long `SharedString` can still allocate; formatting, conversion, reference-count updates, and eventually dropping shared copies still cost time.

## Ownership and access

`SharedString` owns a usable value even when its input was a temporary `String` or a borrowed `&str`. Converting a nonstatic borrow copies its content as needed; the result does not borrow from the caller. You can therefore store it in an [Entity](./entity) or move it into a callback without managing the input's lifetime.

```rust
use gpui_kit::SharedString;

let title: SharedString = "Quarterly report for the product team".into();
let header_title = title.clone(); // Retain it for one UI owner.
let tab_title = title.clone(); // Retain it for another owner.
let borrowed: &str = title.as_str(); // Read without taking ownership.

assert_eq!(borrowed, "Quarterly report for the product team");
assert_eq!(header_title, tab_title);
```

Here `.into()` converts the literal to an owned `SharedString`; for a literal that should use static storage, use `SharedString::new_static(...)` instead. Because this conversion uses the normal path and the title is longer than the current inline capacity, `header_title` and `tab_title` share its heap-backed text while unchanged. Their clones still update reference counts. `as_str()`, `AsRef<str>`, and dereferencing provide a `&str` without creating another owner; that borrow lasts only as long as the `SharedString` it came from.

`SharedString` cannot be edited in place. If a task truly needs a mutable construction or editing buffer, use a local `String` and convert it once when the result is ready. Converting an owned `String` to a long `SharedString` can still copy into shared storage; do not assume the `String` buffer is reused.

## Convert at the ownership boundary

Choose the operation according to who needs the text next:

| Source and next use | Pattern | Ownership result |
| --- | --- | --- |
| Fixed literal retained by UI | `SharedString::new_static("Ready")` | Owns a value backed by static text |
| Temporary `&str` retained by UI | `SharedString::from(text)` | Owns a value independent of the borrowed source |
| Finished `String` retained by UI | `let label: SharedString = text.into();` | Moves the `String` into conversion; it may still copy or allocate |
| Existing `SharedString`, both owners need it | `let label = title.clone();` | Both keep owned values; long heap text stays shared |
| Existing `SharedString`, only the receiver needs it | `Label::new(title)` | Moves the value; no extra clone |
| Existing `SharedString`, callee reads only | `read_text(title.as_str())` | Borrows `&str` for the call |

For a builder taking `impl Into<SharedString>`, passing `&title` also works because GPUI implements conversion from `&SharedString` by cloning it. Passing `title.as_str()` instead converts borrowed text into a **new** `SharedString`; use `title.clone()` (or `&title`) when you mean to share the existing value. If an API specifically requires `String`, `title.to_string()` creates a separate mutable string; only do this at that API boundary.

```rust
use gpui_kit::SharedString;
use gpui_kit::component::label::Label;

let title = SharedString::new_static("Downloads");
let heading = Label::new(title.clone()); // Keep title for another owner.
let tab = Label::new(title); // Last use: move it.
```

The `heading` and `tab` values each own their text. The move makes `title` unavailable afterward; clone first only where another owner still needs it.

## From an API response to a View

Design immutable text fields in API response snapshots as `SharedString` by default. Deserialize a JSON `title` directly into that type with Serde, then carry the response into application state. If the UI needs a formatted title, derive a separate presentation value when the response arrives instead of changing the transport response:

```rust
use gpui_kit::SharedString;
use serde::Deserialize;

#[derive(Deserialize)]
struct UserResponse {
    title: SharedString,
}

struct Workspace {
    response: UserResponse,
    display_title: SharedString,
}

impl From<UserResponse> for Workspace {
    fn from(response: UserResponse) -> Self {
        let display_title = format!("Profile: {}", response.title).into();
        Self { response, display_title }
    }
}
```

Moving `response` into `Workspace` keeps its original `title` without cloning it. The separate `display_title` is computed once here; nested Views and components can clone either value when they need owned text. For long heap-backed text, those later clones share the value's text allocation rather than copying its bytes; short text is stored inline. Initial JSON parsing still costs work: the current `Deserialize` implementation reads a `String` and constructs a `SharedString` from it, which may copy or allocate. `Serialize` writes the text as an ordinary string; sharing is an in-memory property, not part of the JSON value. `format!` also constructs a new value for the derived title, so derive it when data changes rather than on every render. Use a mutable text buffer only when the response text truly needs editing; an ordinary read-only response field need not start as `String`.

## Use it in GPUI Kit

GPUI Kit component builders often accept `impl Into<SharedString>`, so callers can pass a literal or an existing `SharedString`. For example, [`Button::label`](../component/button) and [`Label::new`](../component/label) accept that form:

```rust
use gpui_kit::*;
use gpui_kit::component::{button::Button, label::Label};

let title = SharedString::new_static("Downloads");
let heading = Label::new(title.clone());
let button = Button::new("open-downloads").label(title);
```

For text kept in a view and used on many renders, store one `SharedString` in the view and clone it into each element. The view's [Context](./context) is available alongside `Window` in `Render::render`:

```rust
use gpui_kit::*;
use gpui_kit::component::label::Label;

struct Header {
    title: SharedString,
}

impl Render for Header {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Label::new(self.title.clone())
    }
}
```

The clone gives the new element its own handle while the view retains its copy. For a long title, it does not copy all the characters on every render. Constructing the long title anew with `format!(...).into()` inside every render would still format and allocate on every render. Keep stable text in the state that owns it, and replace the value when it changes.

## Choose text for its lifetime

For text that a UI element, View, callback, or several owners will retain, prefer `SharedString`. For a fixed literal that needs ownership, `SharedString::new_static` explicitly uses static storage. Component builders that accept `impl Into<SharedString>` let callers pass these values without building a fresh `String` first.

When an API only reads text during the current call, pass `&str` or borrow from an existing `SharedString` with `.as_str()`. This creates no additional text owner.

Avoid introducing `String` for ordinary UI text. Use it when text actually needs **mutable construction or editing**, such as assembling a draft, and convert the finished value at the ownership boundary:

```rust
let mut draft = String::from("Quarterly report");
draft.push_str(" for the product team");
let title: SharedString = draft.into();
```

This `.into()` consumes the `String` and builds a `SharedString`; it may copy or allocate and does not promise to reuse the original buffer. GPUI Kit's [`InputState`](../component/input) shows why an editing buffer and an exposed value need not have the same type: it stores editable text in a `Rope`, while `value()` materializes a `SharedString` snapshot on each call. Avoid repeatedly calling `value()` just to read an unchanged field. Ordinary UI code often needs no `String` at all, but it remains useful for real construction and editing work.

:::note Source snapshot (2026-09-24)
In GPUI Kit's production library source under `crates/{kit,base,component,assets}/src`, **34 named struct storage fields** have a type containing `String`, versus **375** with a type containing `SharedString`. The `String` fields include valid exceptions such as editable text, search state, and theme schema data. This supports the default for retained UI text; it does not mean the repository contains only 34 uses of `String`.

To reproduce the count, scan `*.rs` named struct bodies under those four `src` directories, skip test-only files and content after `#[cfg(test)]`, then count field types containing the distinct Rust tokens `String` and `SharedString`. This is a lightweight source scan rather than a Rust semantic parse: it excludes local variables, function signatures, and enum fields, includes both native and WebAssembly `cfg` variants present in source, and says nothing about runtime allocations.
:::

## Common compiler errors and surprises

| Symptom | Cause and fix |
| --- | --- |
| “use of moved value” after `Label::new(title)` | The builder consumes its argument. Pass `title.clone()` when the view or another element still needs the value; move it only on the last use. |
| “borrowed data escapes” or a callback must be `'static` | A callback cannot retain `title.as_str()` borrowed from a view. Clone the `SharedString` into the `move` closure, then call `.as_str()` inside the closure if needed. |
| A method expects `&str`, but it receives `SharedString` | Pass `title.as_str()` (or `&title` where deref coercion applies). This borrows without copying text. |
| `push_str` or another mutation method is unavailable | `SharedString` is immutable. Build or edit in a `String`, then convert the completed text into `SharedString`. |
| `.into()` has an ambiguous destination type | Specify it: `let title: SharedString = source.into();` or call `SharedString::from(source)`. |

The callback case follows the same ownership rule as an element: the closure must own anything it keeps after `render` returns. For example, `let title_for_click = self.title.clone();` before an `on_click(move |_, _, _| { /* use title_for_click here */ })` gives the handler an independent value. Clone once when building the callback, rather than converting `self.title.as_str()` into a fresh value on each render.

## Related: `Cow<str>`

Rust's [`Cow<'a, str>`](https://doc.rust-lang.org/std/borrow/enum.Cow.html) can avoid a copy by borrowing existing text, but its `Borrowed` variant is tied to the source's lifetime. Its `Owned` variant contains a `String`; cloning a nonempty owned value copies those bytes. Calling `to_mut()` on a borrowed value copies it into an owned `String` before editing:

```rust
use std::borrow::Cow;

let mut text: Cow<'_, str> = Cow::Borrowed("Ready");
text.to_mut().push('!'); // Now owned and editable.
```

`SharedString` is an independently owned, immutable value with static, inline, or shared heap storage. Cloning a long heap-backed value shares its bytes; changing the text requires a new value. It is not an alias for `Cow` and does not offer `Cow`'s write-on-mutation behavior.

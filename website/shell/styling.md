---
title: Styling
description: The fluent style surface, length and colour grammars, semantic theme tokens, and hover / active / focus styles.
order: 5
---

# Styling

The script owns presentation, so this is where most of an application's code goes. Every element accepts the same style surface, written as one fluent chain — exactly what the Rust side writes:

```js
render(cx) {
  return v_flex().size_full().bg(cx.theme().colors.surface).p(12).gap(8).rounded(6);
}
```

```rust
// The same thing in Rust, on gpui-base.
v_flex().size_full().bg(surface).p(px(12.)).gap(px(8.)).rounded(px(6.))
```

## Two halves, one surface

The style surface has two halves, and they exist for different reasons.

**No-argument methods come from GPUI's reflection table.** `flex_col`, `items_center`, `gap_2`, `rounded_md`, `text_sm`, `size_full`, `font_semibold`, `truncate`, `cursor_pointer` — the whole family, obtained from `gpui_kit::base::styled_ext_reflection_methods` and `gpui_kit::styled_reflection::methods` with no maintenance at all. Not one of these names is written down anywhere in the runtime. When upstream GPUI adds a style method, the script surface has it, and so does the generated `gpui-kit.d.ts`.

The build these pages were written against exposes **3,148** of them. It is however many `fn(self) -> Self` style methods GPUI currently has, and it moves when GPUI moves. `gpui-shell types` prints the exact figure for your build.

**Methods that take arguments cannot be reflected**, so there are **57** of them bound by hand. That list is the one hand-maintained table in the styling layer, and it is deliberately small.

The two halves never overlap: a name is in one or the other, and a test fails the build if a name ever lands in both.

## Lengths

A bare number is pixels. A string carries its unit.

```js
.p(12)          // 12px
.w("50%")       // half the parent
.h("auto")
.gap("0.5rem")
```

Which of those a given method accepts follows **its Rust signature**, because that signature is what rejects the bad ones. GPUI has three length types nested inside each other, and the runtime keeps the distinction rather than flattening it:

| Type             | Accepts                                           | Rejects               |
| ---------------- | ------------------------------------------------- | --------------------- |
| `Length`         | a number, `"12px"`, `"1.5rem"`, `"50%"`, `"auto"` | —                     |
| `DefiniteLength` | a number, `"12px"`, `"1.5rem"`, `"50%"`           | `"auto"`              |
| `AbsoluteLength` | a number, `"12px"`, `"1.5rem"`                    | percentages, `"auto"` |

```text
`p` cannot be "auto"; it expects a definite length such as 12 or "50%"
```

```text
`rounded` expects an absolute length such as 8 or "0.5rem";
percentages and "auto" are not allowed here
```

`"auto"` padding and a percentage radius have no meaning in the layout engine underneath, and a runtime that accepted them would have to invent one.

### The parametric methods

| Family     | Methods                                                                    | Argument         |
| ---------- | -------------------------------------------------------------------------- | ---------------- |
| Size       | `w` `h` `size` `min_w` `min_h` `min_size` `max_w` `max_h` `max_size`       | `Length`         |
| Padding    | `p` `px` `py` `pt` `pb` `pl` `pr`                                          | `DefiniteLength` |
| Margin     | `m` `mx` `my` `mt` `mb` `ml` `mr`                                          | `Length`         |
| Position   | `inset` `top` `bottom` `left` `right`                                      | `Length`         |
| Flex       | `gap` `gap_x` `gap_y`                                                      | `DefiniteLength` |
| Flex       | `flex_basis`                                                               | `Length`         |
| Flex       | `flex_grow` `flex_shrink`                                                  | number           |
| Border     | `border` `border_t` `border_b` `border_l` `border_r` `border_x` `border_y` | `AbsoluteLength` |
| Radius     | `rounded` and the `_t` `_b` `_l` `_r` `_tl` `_tr` `_bl` `_br` forms        | `AbsoluteLength` |
| Paint      | `bg` `text_color` `text_bg` `border_color`                                 | colour           |
| Paint      | `text_size`                                                                | `AbsoluteLength` |
| Paint      | `line_height`                                                              | `DefiniteLength` |
| Typography | `font_family`                                                              | string           |
| Paint      | `opacity`                                                                  | number           |

`line_height` is the one exception worth memorizing: a **bare number is a multiplier**, not pixels. `line_height(1.45)` means 1.45× the font size, because that is what it means everywhere else in the industry and 1.45px is never what anyone meant. A string still follows the ordinary grammar.

### What is deliberately not bound

`shadow`, `cursor`, `text_align`, `text_overflow`, `font_weight` and `scrollbar_width` take Rust structs or enums rather than scalars, and are not exposed as parametric methods. Every one of them has a no-argument form that is reflected and works today: `shadow_sm`, `cursor_pointer`, `text_center`, `truncate`, `font_bold`. A real shadow API belongs with the token work, not as a positional argument list.

## Colours

A colour is normally read from the call-scoped theme. Semantic token name strings remain accepted for compatibility, and hex literals are available for fixed colours:

```js
render(cx) {
  return element
    .bg(cx.theme().colors.surface)         // follows the theme
    .text_color("#1e88e5");    // does not
}
```

The palette defines seventeen tokens:

|           |                                                                      |
| --------- | -------------------------------------------------------------------- |
| Ground    | `background`, `foreground`                                           |
| Surfaces  | `surface`, `surface_foreground`                                      |
| Emphasis  | `primary`, `primary_foreground`, `secondary`, `secondary_foreground` |
| Recessive | `muted`, `muted_foreground`                                          |
| Highlight | `accent`, `accent_foreground`, `selection`                           |
| Danger    | `destructive`, `destructive_foreground`                              |
| Chrome    | `border`, `input`, `ring`                                            |

Hex literals accept `#rgb`, `#rrggbb` and `#rrggbbaa`.

**Prefer a value from `cx.theme().colors`.** A literal bypasses the theme, and a theme switch will not reach it. The example application makes exactly this point: it follows the visual language of `crates/base/examples/showcase`, which has to write literal colours because Base ships no palette, and reads semantic tokens instead — so the same code follows a theme that the Rust showcase cannot.

A mistyped token names the whole set rather than failing vaguely:

```text
unknown color token `surfacee`; expected one of: background, foreground, surface, … —
or a #rrggbb literal
```

### Where the active tokens come from

gpui-shell does not own a palette or theme file format. It reads the active
`gpui_kit::base::Theme` supplied by the host. A JavaScript application may replace
that same Base Snapshot with `set_theme({ appearance, tokens })`; theme names
and any registry remain application state.

## State styles

`hover`, `active` and `focus` take a function, which receives a detached element that collects the declarations:

```js
renderSave(cx) {
  return Button.new("save")
    .bg(cx.theme().colors.surface)
    .border(1)
    .border_color(cx.theme().colors.border)
    .hover((style) => style.bg(cx.theme().colors.muted).border_color(cx.theme().colors.foreground))
    .active((style) => style.bg(cx.theme().colors.border))
    .focus((style) => style.border_color(cx.theme().colors.ring))
    .child("Save");
}
```

The function's return value is ignored, so a chain and a block body both work. The declarations inside are the **ordinary style methods** — there is no second grammar for "what a style is", and every length and colour rule above applies unchanged.

Two implementation facts leak far enough to be worth knowing:

- **`active` and `focus` need a stable element identity.** A plain `div` acquires one lazily, derived from its position in the description, which is stable across renders for a stable tree. `Button`, `Checkbox` and `Input` already have one.
- **A `Switch` ignores state styles.** The switch root is not the interactive element — its track is — so a state style on it has nowhere to land. The runtime logs a warning saying to style the row around it instead, rather than dropping the declaration silently.

## Scrolling overflow

Scrolling is element behavior rather than a style declaration. Give the viewport
a bounded width or height, then choose the axes it owns:

```js
v_flex()
  .id("activity")
  .h(240)
  .overflow_y_scroll()
  .children(this.rows.map((row) => row));
```

Use `.overflow_scroll()` for both axes, `.overflow_x_scroll()` for horizontal
scrolling, or `.overflow_y_scroll()` for vertical scrolling. A stable `.id(...)`
keeps the native scroll position attached to the same viewport across script
renders.

The corresponding `.overflow_scrollbar()`, `.overflow_x_scrollbar()` and
`.overflow_y_scrollbar()` methods keep the same scrolling behavior and also
paint gpui-component's native scrollbars. They require a stable `.id(...)` so
each viewport keeps independent scrollbar and scroll-position state.

## Theme values

Read semantic values from the context that is rendering or handling the event:

```js
render(cx) {
  return v_flex()
    .gap(cx.theme().spacing.md)
    .rounded(cx.theme().radius.lg)
    .bg(cx.theme().colors.surface)
    .child(`${cx.theme().appearance}: ${cx.theme().is_dark ? "dark" : "light"}`);
}
```

The Snapshot is deeply read-only. `theme()` remains as a compatibility accessor, but `cx.theme()` is preferred. An application may call `set_theme({ appearance, tokens })` from an event or task with its own complete color, spacing, and radius token Snapshot. gpui-shell writes that Snapshot into gpui-base and rebuilds token-backed script Views; it does not own theme names, palettes, or a file format.

## Native motion

`.transition(property, policy)` and `.spring(property, policy?)` animate later target changes for `opacity`, `width`, `height`, `left`, and `top`. Motion is retained and advanced by native GPUI frames: after the script changes the target and calls `cx.notify()`, animation frames do **not** re-enter JavaScript.

```js
div()
  .id("drawer")
  .left(this.open ? 320 : 16)
  .opacity(this.open ? 1 : 0.5)
  .transition("left", { duration: 220, easing: "ease-out" })
  .spring("opacity", { response: 260, damping: 0.85 });
```

Animated length targets are **numeric pixels only**. Relative values such as `"50%"`, `"1rem"`, and `"auto"` cannot be sampled into a stable native channel and are rejected. Give the element a stable `.id(...)` (controls already use their constructor id), otherwise a changing tree position changes the motion identity.

## Unknown methods

```text
unknown style method `text_colour` (did you mean: text_color?)
```

The suggestion is a Levenshtein match against the full name list, with a tight budget — two edits, relaxed to a third of the name for longer identifiers. A wrong suggestion is worse than none.

There is a nice piece of machinery behind that message, and it explains a number in the source. QuickJS reports a missing method as a bare `TypeError: not a function` **without naming the property**, so a mistyped style name would otherwise arrive with no clue at all. Wrapping the element prototype in a `Proxy` fixes that — and measured at roughly 30% of the entire description pass (1.09 ms → 1.42 ms for 443 nodes).

So the runtime keeps a fast plain prototype as the default, and when a render fails with "not a function" it **re-runs that render once** against a diagnostic `Proxy` prototype, purely to produce the message. Errors are rare; a 30% tax on every render is not.

## Not there yet

- **Semantic state styles.** `gpui-base` has a `state_style` layer with a defined priority order for checked, selected and disabled. It is not bound; use `.when(condition, …)` for those states today.
- **Keyframe animation.** Target-value transitions and springs exist; arbitrary keyframes and per-frame JavaScript callbacks do not.
- **Spacing and radius tokens in styles.** The palette carries spacing and radius scales, but style methods take lengths, not token names — only colours resolve a token. Applications define their own scale as a constant, the way the example's `SPACE` object does.

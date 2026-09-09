---
title: Combobox
description: An autocomplete input paired with a searchable dropdown list.
---

# Combobox

A searchable dropdown for selecting one or multiple values from a list.

## Select vs Combobox

| Feature | Select | Combobox |
| --- | --- | --- |
| Searchable | ✓ (optional) | ✓ (optional) |
| Multi-select | — | ✓ (`.multiple(true)`) |
| Custom trigger rendering | — | ✓ |
| Custom item rendering | — | ✓ |
| Footer action slot | — | ✓ |

Use `Select` for simple single-value picking. Use `Combobox` when you need multi-select, a fully custom trigger, or custom item rendering.

## Import

```rust
use gpui_kit::component::combobox::{
    Combobox, ComboboxState, ComboboxEvent, ComboboxTriggerCtx,
};
use gpui_kit::component::searchable_list::{
    SearchableListItem, SearchableVec, SearchableGroup,
};
```

## Usage

### Basic Single-Select

```rust
let state = cx.new(|cx| {
    ComboboxState::new(
        SearchableVec::new(vec!["Next.js", "SvelteKit", "Nuxt.js"]),
        vec![], // no initial selection
        window,
        cx,
    )
    .searchable(true)
});

Combobox::new(&state)
    .placeholder("Select framework...")
    .search_placeholder("Search...")
    .w_full()
```

### Multi-Select

Pass `.multiple(true)` to enable multi-select mode. Clicking an item toggles it; the dropdown stays open until the user presses Escape or clicks outside.

```rust
let state = cx.new(|cx| {
    ComboboxState::new(
        SearchableVec::new(vec!["React", "Vue", "Angular"]),
        vec![IndexPath::new(0)], // pre-selected
        window,
        cx,
    )
    .multiple(true)
    .searchable(true)
});

Combobox::new(&state).placeholder("Select frameworks")
```

### Pre-selected Item

Pass index paths of items to pre-select:

```rust
let state = cx.new(|cx| {
    ComboboxState::new(items, vec![IndexPath::new(0)], window, cx)
});
```

### Grouped Items

Use `SearchableGroup` to group items under a heading:

```rust
let grouped = SearchableVec::new(vec![
    SearchableGroup::new("Fruits").items(vec![
        FoodItem::new("Apples"),
        FoodItem::new("Bananas"),
    ]),
    SearchableGroup::new("Vegetables").items(vec![
        FoodItem::new("Carrots"),
        FoodItem::new("Spinach"),
    ]),
]);

let state = cx.new(|cx| {
    ComboboxState::new(grouped, vec![], window, cx).searchable(true)
});

Combobox::new(&state)
```

### Implementing `SearchableListItem`

Built-in implementations exist for `String`, `SharedString`, and `&'static str`. For custom types implement the trait:

```rust
#[derive(Clone)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl SearchableListItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &SharedString {
        &self.code
    }

    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(query)
            || self.code.to_lowercase().contains(query)
    }
}
```

### Disabled Items

Return `true` from `disabled()` on items that should not be selectable:

```rust
impl SearchableListItem for MyItem {
    // ...
    fn disabled(&self) -> bool {
        self.is_unavailable
    }
}
```

### Custom Check Icon

```rust
Combobox::new(&state)
    .check_icon(Icon::new(IconName::CircleCheck))
```

### Footer Action

Render a persistent action at the bottom of the dropdown (e.g. an "Add new" button):

```rust
Combobox::new(&state)
    .footer(|_, cx| {
        Button::new("add-new")
            .ghost()
            .label("New item")
            .icon(Icon::new(IconName::Plus))
            .w_full()
            .justify_start()
            .into_any_element()
    })
```

### Custom Trigger

Override the entire trigger element. `ComboboxTriggerCtx` exposes the current selection, open/disabled flags, and size:

```rust
Combobox::new(&state)
    .render_trigger(|ctx, _, cx| {
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .when(ctx.selection.is_empty(), |this| {
                this.text_color(cx.theme().muted_foreground)
                    .child("Select...")
            })
            .children(ctx.selection.iter().map(|(_, item)| {
                div()
                    .bg(cx.theme().accent)
                    .rounded_sm()
                    .px_1p5()
                    .py_0p5()
                    .text_sm()
                    .child(item.title())
            }))
            .into_any_element()
    })
```

### Sizes

```rust
Combobox::new(&state).large()
Combobox::new(&state)  // medium (default)
Combobox::new(&state).small()
```

### Cleanable

```rust
Combobox::new(&state).cleanable(true) // show clear button when a value is selected
```

### Disabled

```rust
Combobox::new(&state).disabled(true)
```

### Events

Both `Change` (fired on every toggle) and `Confirm` (fired when the dropdown closes) carry the full selection as `Vec<Value>`.

```rust
cx.subscribe_in(&state, window, |view, _, event, window, cx| {
    match event {
        ComboboxEvent::Change(values) => {
            // fired on every toggle
        }
        ComboboxEvent::Confirm(values) => {
            // fired when the dropdown closes
        }
    }
});
```

### Mutating Programmatically

Values are resolved through the current delegate. Values that cannot be found are ignored.

`set_selected_values` clears the search query first, so an active search never decides which
values can be selected. Index paths address the list as it is currently displayed, so
`set_selected_indices`, `add_selected_index` and `remove_selected_index` act on the visible rows
and leave the query alone.

```rust
// Replace the entire selection by value
state.update(cx, |s, cx| {
    s.set_selected_values(&["React", "Angular"], window, cx);
});

// Replace the entire selection by index path
state.update(cx, |s, cx| {
    s.set_selected_indices(vec![IndexPath::new(0), IndexPath::new(2)], window, cx);
});

// Add / remove individual items
state.update(cx, |s, cx| {
    s.add_selected_index(IndexPath::new(1), cx);
    s.remove_selected_index(IndexPath::new(0), cx);
});

// Clear all selections
state.update(cx, |s, cx| {
    s.clear_selection(cx);
});

// Read all selected values (multi-select)
let values = state.read(cx).selected_values(); // Vec<Value>

// Read the first selected value (single-select convenience)
let value = state.read(cx).selected_value(); // Option<Value>
```

## Keyboard Shortcuts

| Key       | Action                                   |
| --------- | ---------------------------------------- |
| `Tab`     | Focus trigger                            |
| `Enter`   | Open menu or confirm highlighted item    |
| `Up/Down` | Navigate options (opens menu if closed)  |
| `Escape`  | Close menu                               |

## Theming

- `background` — Dropdown input background
- `input` — Trigger border color
- `foreground` — Text color
- `muted_foreground` — Placeholder and disabled text
- `border` — Menu border
- `radius` — Border radius

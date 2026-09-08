---
title: Select
description: Displays a list of options for the user to pick from—triggered by a button.
---

# Select

:::info
This component was named `Dropdown` in `<= 0.3.x`.

It has been renamed to `Select` to better reflect its purpose.
:::

A select component that allows users to choose from a list of options.

Supports search functionality, grouped items, custom rendering, and various states. Built with keyboard navigation and accessibility in mind.

:::tip
For richer selection UIs with custom trigger rendering or multi-select, see [Combobox](combobox).
:::

## Import

```rust
use gpui_kit::component::select::{
    Select, SelectState, SelectItem, SelectDelegate,
    SelectEvent, SearchableVec, SelectGroup
};
```

## Usage

### Basic Select

You can create a basic select dropdown by initializing a `SelectState` with a list of items.

The first type parameter of `SelectState` is the items for the state, which must implement the [SelectItem] trait.

The built-in implementations of `SelectItem` include common types like `String`, `SharedString`, and `&'static str`.

```rust
let state = cx.new(|cx| {
    SelectState::new(
        vec!["Apple", "Orange", "Banana"],
        Some(IndexPath::default()), // Select first item
        window,
        cx,
    )
});

Select::new(&state)
```

### Placeholder

```rust
let state = cx.new(|cx| {
    SelectState::new(
        vec!["Rust", "Go", "JavaScript"],
        None, // No initial selection
        window,
        cx,
    )
});

Select::new(&state)
    .placeholder("Select a language...")
```

### Accessibility

Give the control a name that stays the same when the selection changes:

```rust
Select::new(&state)
    .accessibility_label("Programming language")
    .placeholder("Choose a language")
```

The accessible value uses the committed item's `title()` and any `title_prefix`.
A custom `display_title()` remains visual presentation. Searching does not change
that committed value. With no selection, the accessible value uses the placeholder.
Enabled controls expose accessible activation to open or close the popup.

### Searchable

Use `searchable(true)` to enable search functionality within the dropdown.

```rust
let fruits = SearchableVec::new(vec![
    "Apple", "Orange", "Banana", "Grape", "Pineapple",
]);

let state = cx.new(|cx| {
    SelectState::new(fruits, None, window, cx).searchable(true)
});

Select::new(&state)
    .icon(IconName::Search) // Shows search icon
```

### Impl SelectItem

By default, we have implmemented `SelectItem` for common types like `String`, `SharedString` and `&'static str`. You can also create your own item types by implementing the `SelectItem` trait.

This is useful when you want to display complex data structures, and also want get that data type from `select_value` method.

You can also customize the search logic by overriding the `matches` method.

```rust
#[derive(Debug, Clone)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl SelectItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<gpui_kit::AnyElement> {
        // Custom display for selected item
        Some(format!("{} ({})", self.name, self.code).into_any_element())
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }

    fn matches(&self, query: &str) -> bool {
        // Custom search logic
        self.name.to_lowercase().contains(&query.to_lowercase()) ||
        self.code.to_lowercase().contains(&query.to_lowercase())
    }
}
```

### Group Items

```rust
let mut grouped_items = SearchableVec::new(vec![]);

// Group countries by first letter
grouped_items.push(
    SelectGroup::new("A")
        .items(vec![
            Country { name: "Australia".into(), code: "AU".into() },
            Country { name: "Austria".into(), code: "AT".into() },
        ])
);
grouped_items.push(
    SelectGroup::new("B")
        .items(vec![
            Country { name: "Brazil".into(), code: "BR".into() },
            Country { name: "Belgium".into(), code: "BE".into() },
        ])
);

let state = cx.new(|cx| {
    SelectState::new(grouped_items, None, window, cx)
});

Select::new(&state)
```

### Sizes

```rust
Select::new(&state).large()
Select::new(&state) // medium (default)
Select::new(&state).small()
```

### Disabled State

```rust
Select::new(&state).disabled(true)
```

### Cleanable

```rust
Select::new(&state)
    .cleanable(true) // Show clear button when item is selected
```

### Custom Appearance

```rust
Select::new(&state)
    .w(px(320.))                    // Set dropdown width
    .menu_width(px(400.))           // Set menu popup width
    .menu_max_h(rems(10.))          // Set menu max height (default: 20rem)
    .appearance(false)              // Remove default styling
    .title_prefix("Country: ")      // Add prefix to selected title
```

### Empty State

```rust
let state = cx.new(|cx| {
    SelectState::new(Vec::<SharedString>::new(), None, window, cx)
});

Select::new(&state)
    .empty(
        h_flex()
            .h_24()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child("No options available")
    )
```

### Events

```rust
cx.subscribe_in(&state, window, |view, state, event, window, cx| {
    match event {
        SelectEvent::Confirm(value) => {
            if let Some(selected_value) = value {
                println!("Selected: {:?}", selected_value);
            } else {
                println!("Selection cleared");
            }
        }
    }
});
```

### Mutating

```rust
// Set by index
state.update(cx, |state, cx| {
    state.set_selected_index(Some(IndexPath::default().row(2)), window, cx);
});

// Set by value (requires PartialEq on Value type)
state.update(cx, |state, cx| {
    state.set_selected_value(&"US".into(), window, cx);
});

// Get current selection
let current_value = state.read(cx).selected_value();
```

Update items:

```rust
state.update(cx, |state, cx| {
    let new_items = vec!["New Option 1".into(), "New Option 2".into()];
    state.set_items(new_items, window, cx);
});
```

## Examples

### Language Selector

```rust
let languages = SearchableVec::new(vec![
    "Rust".into(),
    "TypeScript".into(),
    "Go".into(),
    "Python".into(),
    "JavaScript".into(),
]);

let state = cx.new(|cx| {
    SelectState::new(languages, None, window, cx)
});

Select::new(&state)
    .placeholder("Select language...")
    .title_prefix("Language: ")
```

### Country/Region Selector

```rust
#[derive(Debug, Clone)]
struct Region {
    name: SharedString,
    code: SharedString,
    flag: SharedString,
}

impl SelectItem for Region {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<gpui_kit::AnyElement> {
        Some(
            h_flex()
                .items_center()
                .gap_2()
                .child(self.flag.clone())
                .child(format!("{} ({})", self.name, self.code))
                .into_any_element()
        )
    }

    fn value(&self) -> &Self::Value {
        &self.code
    }
}

let regions = vec![
    Region {
        name: "United States".into(),
        code: "US".into(),
        flag: "🇺🇸".into()
    },
    Region {
        name: "Canada".into(),
        code: "CA".into(),
        flag: "🇨🇦".into()
    },
];

let state = cx.new(|cx| {
    SelectState::new(regions, None, window, cx)
});

Select::new(&state)
    .placeholder("Select country...")
```

### Integrated with Input Field

```rust
// Combined country code + phone input
h_flex()
    .border_1()
    .border_color(cx.theme().input)
    .rounded(cx.theme().radius_lg)
    .w_full()
    .gap_1()
    .child(
        div().w(px(140.)).child(
            Select::new(&country_state)
                .appearance(false) // No border/background
                .py_2()
                .pl_3()
        )
    )
    .child(Separator::vertical())
    .child(
        div().flex_1().child(
            Input::new(&phone_input)
                .appearance(false)
                .placeholder("Phone number")
                .pr_3()
                .py_2()
        )
    )
```

### Multi-level Grouped Select

```rust
let mut grouped_countries = SearchableVec::new(vec![]);

for (continent, countries) in countries_by_continent {
    grouped_countries.push(
        SelectGroup::new(continent)
            .items(countries)
    );
}

let state = cx.new(|cx| {
    SelectState::new(grouped_countries, None, window, cx)
});

Select::new(&state)
    .menu_width(px(350.))
    .placeholder("Select country...")
```

## Keyboard Shortcuts

| Key       | Action                                  |
| --------- | --------------------------------------- |
| `Tab`     | Focus dropdown                          |
| `Enter`   | Open menu or select current item        |
| `Up/Down` | Navigate options (opens menu if closed) |
| `Escape`  | Close menu                              |
| `Space`   | Open menu                               |

## Theming

The dropdown respects the current theme and uses the following theme tokens:

- `background` - Dropdown input background
- `input` - Border color
- `foreground` - Text color
- `muted_foreground` - Placeholder and disabled text
- `accent` - Selected item background
- `accent_foreground` - Placeholder text color
- `border` - Menu border
- `radius` - Border radius

[SelectItem]: https://docs.rs/gpui-component/latest/gpui_component/select/trait.SelectItem.html

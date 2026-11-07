# GPUI Component Testing Rules

## Testing Principles

### 1. **Simplicity First**

- Avoid excessive simple tests
- Focus on complex logic and core functionality

### 2. **Builder Pattern Testing**

- Every component should have a `test_*_builder` test for coverage of the builder pattern
- Tests should cover all major configuration options
- Use method chaining to demonstrate complete API usage

#### Example:

```rust
#[gpui::test]
fn test_button_builder(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("complex-button")
        .label("Save Changes")
        .primary()
        .outline()
        .large()
        .tooltip("Click to save")
        .compact()
        .loading(false)
        .disabled(false)
        .selected(false)
        .on_click(|_, _, _| {});

    // Assert all key properties
    assert_eq!(button.label, Some("Save Changes".into()));
    assert_eq!(button.variant, ButtonVariant::Primary);
    assert!(button.outline);
    assert_eq!(button.size, Size::Large);
}
```

### 3. **Complex Logic Testing**

- Test conditional branching logic
- Test state transitions and interactions
- Test edge cases

#### Example:

```rust
#[gpui::test]
fn test_button_clickable_logic(_cx: &mut gpui::TestAppContext) {
    // Test behavior under multiple conditions
    let clickable = Button::new("test").on_click(|_, _, _| {});
    assert!(clickable.clickable());

    let disabled = Button::new("test").disabled(true).on_click(|_, _, _| {});
    assert!(!disabled.clickable());

    let loading = Button::new("test").loading(true).on_click(|_, _, _| {});
    assert!(!loading.clickable());
}
```

### 4. **Helper Method Testing**

- Test component helper methods and validation logic
- Combine related tests into a single function

#### Example:

```rust
#[gpui::test]
fn test_button_variant_methods(_cx: &mut gpui::TestAppContext) {
    // Test variant check methods
    assert!(ButtonVariant::Link.is_link());
    assert!(ButtonVariant::Text.is_text());
    assert!(ButtonVariant::Ghost.is_ghost());

    // Test related logic
    assert!(ButtonVariant::Link.no_padding());
    assert!(ButtonVariant::Text.no_padding());
}
```

## What NOT to Test

### ❌ Anti-patterns to Avoid

1. **Simple getter/setter tests**

```rust
// ❌ Don't write tests like this
#[gpui::test]
fn test_button_with_label(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("test").label("Click Me");
    assert_eq!(button.label, Some("Click Me".into()));
}
```

2. **Individual property tests**

```rust
// ❌ Don't write separate tests for each property
#[gpui::test]
fn test_button_disabled(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("test").disabled(true);
    assert!(button.disabled);
}

#[gpui::test]
fn test_button_selected(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("test").selected(true);
    assert!(button.selected);
}
```

_These should be merged into the builder pattern test_

3. **Individual size/variant tests**

```rust
// ❌ Don't write separate tests for each size
#[gpui::test]
fn test_button_xsmall(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("test").xsmall();
    assert_eq!(button.size, Size::XSmall);
}

#[gpui::test]
fn test_button_small(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("test").small();
    assert_eq!(button.size, Size::Small);
}
```

## Test File Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // 1. Builder pattern test (required)
    #[gpui::test]
    fn test_component_builder(_cx: &mut gpui::TestAppContext) {
        // Test complete method chaining
    }

    // 2. Complex logic test (if applicable)
    #[gpui::test]
    fn test_component_complex_logic(_cx: &mut gpui::TestAppContext) {
        // Test conditional branches, state transitions, etc.
    }

    // 3. Helper method test (if applicable)
    #[gpui::test]
    fn test_component_helper_methods(_cx: &mut gpui::TestAppContext) {
        // Test helper methods
    }
}
```

## Test Count Guidelines

| Component Type    | Recommended Tests | Notes                            |
| ----------------- | ----------------- | -------------------------------- |
| Simple component  | 1-2 tests         | Builder + complex logic (if any) |
| Medium component  | 2-3 tests         | Builder + logic + helper methods |
| Complex component | 3-5 tests         | Based on actual complexity       |

## Real-world Examples

### Button Component (3 tests)

- `test_button_builder` - Complete configuration test
- `test_button_clickable_logic` - Click logic test
- `test_button_variant_methods` - Variant method test

### ButtonIcon Component (2 tests)

- `test_button_icon_builder` - Complete configuration test
- `test_button_icon_variant_types` - Variant type test

### ButtonGroup Component (1 test)

- `test_button_group_builder` - Complete configuration test (covers all important features)

### DropdownButton Component (1 test)

- `test_dropdown_button_builder` - Complete configuration test

### Toggle Component (2 tests)

- `test_toggle_builder` - Toggle configuration test
- `test_toggle_group_builder` - ToggleGroup configuration test

## GPUI Test Usage

### When to Use `#[gpui::test]`

- When testing UI component rendering
- When testing window-dependent behavior
- When testing interactive elements that require event handling

### When NOT to Use `#[gpui::test]`

- For pure logic tests that don't involve rendering
- For utility function tests
- For simple data structure tests
- For validation logic that doesn't require app context

#### Example:

```rust
// ✅ Use regular Rust test for simple logic
#[test]
fn test_button_variant_conversion() {
    let rounded: ButtonRounded = px(5.0).into();
    assert!(matches!(rounded, ButtonRounded::Size(_)));
}

// ✅ Use gpui::test for component behavior
#[gpui::test]
fn test_button_builder(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("test").large();
    assert_eq!(button.size, Size::Large);
}
```

## Summary

✅ **DO**:

- Test complete builder patterns
- Test complex business logic
- Test conditional branches and state transitions
- Combine related tests
- Use regular `#[test]` when GPUI context is not needed

❌ **DON'T**:

- Test simple property setters
- Write separate tests for each property/size/variant
- Test obvious functionality
- Over-fragment tests
- Use `#[gpui::test]` unnecessarily

**Goal**: Cover the most critical functionality with minimal tests while keeping code clean and maintainable.

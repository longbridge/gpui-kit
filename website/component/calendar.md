---
title: Calendar
description: A flexible calendar component for displaying months, navigating dates, and selecting single dates or date ranges.
---

# Calendar

A standalone calendar component that provides a rich interface for date selection and navigation. The Calendar component supports single date selection, date range selection, multiple month views, custom disabled dates, and comprehensive keyboard navigation.

- [CalendarState]: For managing calendar state and selection.
- [Calendar]: For rendering the calendar UI.

## Import

```rust
use gpui_kit::component::{
    calendar::{Calendar, CalendarState, CalendarEvent, Date, Matcher},
};
```

## Usage

### Basic Calendar

```rust
let state = cx.new(|cx| CalendarState::new(window, cx));
Calendar::new(&state)
```

### Calendar with Initial Date

```rust
use jiff::ZonedDateTime;

let state = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(ZonedDateTime::now().date(), window, cx);
    state
});

Calendar::new(&state)
```

### Date Range Calendar

```rust
use jiff::{ToSpan as _, ZonedDateTime};

let state = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    let now = ZonedDateTime::now().date();
    state.set_date(
        Date::Range(Some(now), now.checked_add(7.days()).ok()),
        window,
        cx
    );
    state
});

Calendar::new(&state)
```

### Multiple Months Display

```rust
// Show 2 months side by side
Calendar::new(&state)
    .number_of_months(2)

// Show 3 months
Calendar::new(&state)
    .number_of_months(3)
```

### Calendar Sizes

```rust
Calendar::new(&state).large()
Calendar::new(&state) // medium (default)
Calendar::new(&state).small()
```

## Date Restrictions

### Disabled Weekends

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(vec![0, 6]) // Sunday=0, Saturday=6
});

Calendar::new(&state)
```

### Disabled Specific Weekdays

```rust
// Disable Sundays, Wednesdays, and Saturdays
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(vec![0, 3, 6])
});

Calendar::new(&state)
```

### Disabled Date Range

```rust
use jiff::{ToSpan as _, ZonedDateTime};

let now = ZonedDateTime::now().date();

let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::range(
            Some(now),
            now.checked_add(7.days()).ok(),
        ))
});

Calendar::new(&state)
```

### Disabled Date Interval

```rust
// Disable dates outside the interval (before/after specified dates)
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::interval(
            Some(now - 30.days()),
            now.checked_add(30.days()).ok()
        ))
});

Calendar::new(&state)
```

### Custom Disabled Dates

```rust
// Disable first 5 days of each month
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            date.day0() < 5 // day0() returns 0-based day
        }))
});

Calendar::new(&state)

// Disable all Mondays
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            date.weekday() == jiff::civil::Weekday::Monday
        }))
});

Calendar::new(&state)

// Disable past dates
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            *date < ZonedDateTime::now().date()
        }))
});

Calendar::new(&state)
```

## Month/Year Navigation

The Calendar automatically provides navigation controls:

- **Previous/Next Month**: Arrow buttons in the header
- **Month Selection**: Click on month name to open month picker
- **Year Selection**: Click on year to open year picker
- **Year Pages**: Navigate through 20-year pages in year view

### Custom Year Range

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .year_range((2020, 2030)) // Limit to specific year range
});

Calendar::new(&state)
```

## Handle Selection Events

```rust
let state = cx.new(|cx| CalendarState::new(window, cx));

cx.subscribe(&state, |view, _, event, _| {
    match event {
        CalendarEvent::Selected(date) => {
            match date {
                Date::Single(Some(selected_date)) => {
                    println!("Date selected: {}", selected_date);
                }
                Date::Range(Some(start), Some(end)) => {
                    println!("Range selected: {} to {}", start, end);
                }
                Date::Range(Some(start), None) => {
                    println!("Range start: {}", start);
                }
                _ => {
                    println!("Selection cleared");
                }
            }
        }
    }
});

Calendar::new(&state)
```

## Advanced Examples

### Business Days Only Calendar

```rust
use jiff::civil::Weekday;

let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday)
        }))
});

Calendar::new(&state)
```

### Holiday Calendar

```rust
use jiff::civil;
use std::collections::HashSet;

// Define holidays
let holidays: HashSet<civil::Date> = [
    civil::date(2024, 1, 1), // New Year
    civil::date(2024, 7, 4), // Independence Day
    civil::date(2024, 12, 25), // Christmas
].into_iter().collect();

let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(move |date| {
            holidays.contains(date)
        }))
});

Calendar::new(&state)
```

### Multi-Month Range Selector

```rust
let state = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(Date::Range(None, None), window, cx); // Range mode
    state
});

Calendar::new(&state)
    .number_of_months(3) // Show 3 months for easier range selection
```

### Quarterly View Calendar

```rust
let state = cx.new(|cx| CalendarState::new(window, cx));

// Update to show current quarter's months
Calendar::new(&state)
    .number_of_months(3)
```

## Custom Styling

```rust
use gpui_kit::{px, relative};

Calendar::new(&calendar)
    .p_4() // Custom padding
    .bg(cx.theme().secondary) // Custom background
    .border_2() // Custom border
    .border_color(cx.theme().primary) // Custom border color
    .rounded(px(12.)) // Custom border radius
    .w(px(400.)) // Custom width
    .h(px(350.)) // Custom height
```

## API Reference

- [Calendar]
- [CalendarState]
- [RangeMatcher]

## Examples

### Event Planning Calendar

```rust
let event_calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    // Disable past dates and weekends
    state = state.disabled_matcher(Matcher::custom(|date| {
        let now = ZonedDateTime::now().date();
        *date < now || matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday)
    }));
    state
});

Calendar::new(&event_calendar)
    .large() // Easier to see and interact with
```

### Vacation Booking Calendar

```rust
let vacation_calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(Date::Range(None, None), window, cx); // Range mode
    state
});

Calendar::new(&vacation_calendar)
    .number_of_months(2) // Show 2 months for range selection
```

### Report Date Range Selector

```rust
let report_calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx)
        .year_range((2020, 2025)); // Limit to business years

    state.set_date(Date::Range(None, None), window, cx);
    state
});

Calendar::new(&report_calendar)
    .number_of_months(3)
    .small() // Compact for dashboard use
```

### Availability Calendar

```rust
use std::collections::HashSet;

let unavailable_dates: HashSet<civil::Date> = get_unavailable_dates();

let availability_calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(move |date| {
            unavailable_dates.contains(date)
        }))
});

Calendar::new(&availability_calendar)
    .number_of_months(2)
```

The Calendar component provides a foundation for any date-related UI requirements, from simple date pickers to complex scheduling interfaces.

[Calendar]: https://docs.rs/gpui-component/latest/gpui_component/calendar/struct.Calendar.html
[CalendarState]: https://docs.rs/gpui-component/latest/gpui_component/calendar/struct.CalendarState.html
[RangeMatcher]: https://docs.rs/gpui-component/latest/gpui_component/calendar/struct.RangeMatcher.html

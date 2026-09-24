---
title: TimeField
description: A segmented input for a time of day, on a 24-hour or 12-hour clock.
---

# TimeField

A segmented input for a time of day. Each part — hour, minute, optional second and optional AM/PM — is edited on its own with the keyboard, and the field keeps its width while digits are typed. [DatePicker](date-picker) uses it to edit the time of a date.

## Import

```rust
use gpui_kit::component::time_field::{
    HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision,
};
```

## Usage

### Basic Time Field

```rust
let time = cx.new(|cx| TimeFieldState::new(window, cx));

TimeField::new(&time)
```

The field starts at `00:00`. Set a value with `set_time`, which does not emit an event:

```rust
use chrono::NaiveTime;

time.update(cx, |state, cx| {
    state.set_time(NaiveTime::from_hms_opt(9, 30, 0).unwrap(), window, cx);
});
```

### With Seconds

```rust
let time = cx.new(|cx| {
    TimeFieldState::new(window, cx).precision(TimePrecision::Second)
});

TimeField::new(&time) // 09:30:15
```

### 12-Hour Clock

The field uses a 24-hour clock by default. `HourCycle::H12` shows hours from `12` to `11` followed by an AM/PM segment:

```rust
let time = cx.new(|cx| {
    TimeFieldState::new(window, cx).hour_cycle(HourCycle::H12)
});

TimeField::new(&time) // 09:30 PM
```

### Sizes, Disabled and Invalid

```rust
TimeField::new(&time).small()
TimeField::new(&time).large()
TimeField::new(&time).disabled(true)

// Show the owner's validation result; edits are not rejected.
TimeField::new(&time).invalid(true)
```

## Handle Changes

User edits emit `TimeFieldEvent::Change` with the new time:

```rust
cx.subscribe(&time, |this, _, event, cx| match event {
    TimeFieldEvent::Change(time) => {
        this.reminder = *time;
        cx.notify();
    }
});
```

## Keyboard

| Key | Action |
| --- | --- |
| `Up` / `Down` | Step the selected segment; it wraps without changing the next unit |
| `Left` / `Right` | Move between segments |
| `Tab` / `Shift-Tab` | Move between segments, then leave the field |
| `0`–`9` | Type the selected segment and move on once it is complete |
| `a` / `p` | Set AM or PM on a 12-hour clock |
| `Backspace` / `Delete` | Reset the selected segment |

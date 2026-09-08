---
title: Calendar
description: 用于展示月份、浏览日期和选择单日或区间的灵活日历组件。
---

# Calendar

Calendar 是一个独立的日历组件，支持单日选择、日期区间选择、多月视图、禁用日期规则以及完整的键盘导航能力。

- [CalendarState] 负责状态与选择管理
- [Calendar] 负责渲染日历界面

## 导入

```rust
use gpui_kit::component::{
    calendar::{Calendar, CalendarState, CalendarEvent, Date, Matcher},
};
```

## 用法

### 基础日历

```rust
let state = cx.new(|cx| CalendarState::new(window, cx));
Calendar::new(&state)
```

### 初始日期

```rust
use chrono::Local;

let state = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(Local::now().naive_local().date(), window, cx);
    state
});

Calendar::new(&state)
```

### 日期区间

```rust
use chrono::{Local, Days};

let state = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    let now = Local::now().naive_local().date();
    state.set_date(
        Date::Range(Some(now), now.checked_add_days(Days::new(7))),
        window,
        cx
    );
    state
});

Calendar::new(&state)
```

### 多月显示

```rust
Calendar::new(&state)
    .number_of_months(2)

Calendar::new(&state)
    .number_of_months(3)
```

### 尺寸

```rust
Calendar::new(&state).large()
Calendar::new(&state)
Calendar::new(&state).small()
```

## 日期限制

### 禁用周末

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(vec![0, 6])
});
```

### 禁用日期区间

```rust
use chrono::{Local, Days};

let now = Local::now().naive_local().date();

let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::range(
            Some(now),
            now.checked_add_days(Days::new(7)),
        ))
});
```

### 自定义禁用规则

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            date.weekday() == chrono::Weekday::Mon
        }))
});
```

## 月份与年份导航

Calendar 自带这些导航能力：

- 上一月 / 下一月按钮
- 点击月份切换月视图
- 点击年份切换年视图
- 在年视图中按页浏览年份

### 自定义年份范围

```rust
let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .year_range((2020, 2030))
});
```

## 监听选择事件

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
                _ => {}
            }
        }
    }
});
```

## 示例

### 仅工作日

```rust
use chrono::Weekday;

let state = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
        }))
});
```

### 假期禁用

```rust
use chrono::NaiveDate;
use std::collections::HashSet;

let holidays: HashSet<NaiveDate> = [
    NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    NaiveDate::from_ymd_opt(2024, 7, 4).unwrap(),
    NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(),
].into_iter().collect();
```

### 多月区间选择

```rust
let state = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(Date::Range(None, None), window, cx);
    state
});

Calendar::new(&state)
    .number_of_months(3)
```

[Calendar]: https://docs.rs/gpui-component/latest/gpui_component/calendar/struct.Calendar.html
[CalendarState]: https://docs.rs/gpui-component/latest/gpui_component/calendar/struct.CalendarState.html
[RangeMatcher]: https://docs.rs/gpui-component/latest/gpui_component/calendar/struct.RangeMatcher.html

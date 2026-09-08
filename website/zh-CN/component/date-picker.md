---
title: DatePicker
description: 支持单日期和日期范围选择的日历选择器组件。
---

# DatePicker

DatePicker 是一个灵活的日期选择组件，内置日历界面，支持单日期选择、日期范围选择、自定义格式、禁用日期和预设范围。

## 导入

```rust
use gpui_kit::component::{
    date_picker::{DatePicker, DatePickerState, DateRangePreset, DatePickerEvent},
    calendar::{Date, Matcher},
};
```

## 用法

### 基础 Date Picker

```rust
let date_picker = cx.new(|cx| DatePickerState::new(window, cx));

DatePicker::new(&date_picker)
```

### 设置初始日期

```rust
use chrono::Local;

let date_picker = cx.new(|cx| {
    let mut picker = DatePickerState::new(window, cx);
    picker.set_date(Local::now().naive_local().date(), window, cx);
    picker
});

DatePicker::new(&date_picker)
```

### 日期范围选择

```rust
use chrono::{Local, Days};

let range_picker = cx.new(|cx| DatePickerState::range(window, cx));

DatePicker::new(&range_picker)
    .number_of_months(2)

let range_picker = cx.new(|cx| {
    let now = Local::now().naive_local().date();
    let mut picker = DatePickerState::new(window, cx);
    picker.set_date(
        (now, now.checked_add_days(Days::new(7)).unwrap()),
        window,
        cx,
    );
    picker
});

DatePicker::new(&range_picker)
    .number_of_months(2)
```

### 自定义日期格式

```rust
let date_picker = cx.new(|cx| {
    DatePickerState::new(window, cx)
        .date_format("%Y-%m-%d")
});

DatePicker::new(&date_picker)

// Other format examples:
// "%m/%d/%Y" -> 12/25/2023
// "%B %d, %Y" -> December 25, 2023
// "%d %b %Y" -> 25 Dec 2023
```

### 占位文本

```rust
DatePicker::new(&date_picker)
    .placeholder("Select a date...")
```

### 可清空

```rust
DatePicker::new(&date_picker)
    .cleanable(true)
```

### 不同尺寸

```rust
DatePicker::new(&date_picker).large()
DatePicker::new(&date_picker)
DatePicker::new(&date_picker).small()
```

### 禁用状态

```rust
DatePicker::new(&date_picker).disabled(true)
```

### 自定义外观

```rust
DatePicker::new(&date_picker).appearance(false)

div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(DatePicker::new(&date_picker).appearance(false))
```

## 日期限制

### 禁用周末

```rust
use gpui_kit::component::calendar;

let date_picker = cx.new(|cx| {
    DatePickerState::new(window, cx)
        .disabled_matcher(vec![0, 6])
});

DatePicker::new(&date_picker)
```

### 禁用日期区间

```rust
use chrono::{Local, Days};

let now = Local::now().naive_local().date();

let date_picker = cx.new(|cx| {
    DatePickerState::new(window, cx)
        .disabled_matcher(calendar::Matcher::range(
            Some(now),
            now.checked_add_days(Days::new(7)),
        ))
});

DatePicker::new(&date_picker)
```

### 禁用日期间隔

```rust
let date_picker = cx.new(|cx| {
    DatePickerState::new(window, cx)
        .disabled_matcher(calendar::Matcher::interval(
            Some(now),
            now.checked_add_days(Days::new(5))
        ))
});

DatePicker::new(&date_picker)
```

### 自定义禁用规则

```rust
let date_picker = cx.new(|cx| {
    DatePickerState::new(window, cx)
        .disabled_matcher(calendar::Matcher::custom(|date| {
            date.day0() < 5
        }))
});

DatePicker::new(&date_picker)

let date_picker = cx.new(|cx| {
    DatePickerState::new(window, cx)
        .disabled_matcher(calendar::Matcher::custom(|date| {
            date.weekday() == chrono::Weekday::Mon
        }))
});
```

## 自定义年份范围

默认情况下，日期选择器在年份选择模式下会显示以今天为中心前后各 50 年。可通过 `set_year_range` 配置更大的范围——例如用于生日选择，需要回溯到 1900 年。

`range` 参数使用**半开区间** `(start, end)`，`end` **不包含**在内。若要包含当前年份，传入 `(1900, current_year + 1)`。

```rust
use chrono::Datelike;

// 生日选择器：允许选择 1900 年到当前年（含）
let birthday_picker = cx.new(|cx| {
    let current_year = chrono::Local::now().year();
    let mut picker = DatePickerState::new(window, cx)
        .date_format("%Y-%m-%d");
    picker.set_year_range((1900, current_year + 1), window, cx);
    picker
});

DatePicker::new(&birthday_picker)
    .cleanable(true)
    .placeholder("选择生日")
```

`set_year_range` 对单日期和范围两种模式均有效。

## 预设范围

### 单日期预设

```rust
use chrono::{Utc, Duration};

let presets = vec![
    DateRangePreset::single(
        "Yesterday",
        (Utc::now() - Duration::days(1)).naive_local().date(),
    ),
    DateRangePreset::single(
        "Last Week",
        (Utc::now() - Duration::weeks(1)).naive_local().date(),
    ),
    DateRangePreset::single(
        "Last Month",
        (Utc::now() - Duration::days(30)).naive_local().date(),
    ),
];

DatePicker::new(&date_picker)
    .presets(presets)
```

### 日期范围预设

```rust
let range_presets = vec![
    DateRangePreset::range(
        "Last 7 Days",
        (Utc::now() - Duration::days(7)).naive_local().date(),
        Utc::now().naive_local().date(),
    ),
    DateRangePreset::range(
        "Last 30 Days",
        (Utc::now() - Duration::days(30)).naive_local().date(),
        Utc::now().naive_local().date(),
    ),
    DateRangePreset::range(
        "Last 90 Days",
        (Utc::now() - Duration::days(90)).naive_local().date(),
        Utc::now().naive_local().date(),
    ),
];

DatePicker::new(&date_picker)
    .number_of_months(2)
    .presets(range_presets)
```

## 处理选择事件

```rust
let date_picker = cx.new(|cx| DatePickerState::new(window, cx));

cx.subscribe(&date_picker, |view, _, event, _| {
    match event {
        DatePickerEvent::Change(date) => {
            match date {
                Date::Single(Some(selected_date)) => {
                    println!("Single date selected: {}", selected_date);
                }
                Date::Range(Some(start), Some(end)) => {
                    println!("Date range selected: {} to {}", start, end);
                }
                Date::Range(Some(start), None) => {
                    println!("Range start selected: {}", start);
                }
                _ => {
                    println!("Date cleared");
                }
            }
        }
    }
});
```

## 显示多个月份

```rust
DatePicker::new(&date_picker)
    .number_of_months(2)

DatePicker::new(&date_picker)
    .number_of_months(3)
```

## 高级示例

### 仅工作日可选

```rust
use chrono::Weekday;

let business_days_picker = cx.new(|cx| {
    DatePickerState::new(window, cx)
        .disabled_matcher(calendar::Matcher::custom(|date| {
            matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
        }))
});

DatePicker::new(&business_days_picker)
    .placeholder("Select business day")
```

### 限制最大范围

```rust
use chrono::Days;

let max_30_days_picker = cx.new(|cx| DatePickerState::range(window, cx));

cx.subscribe(&max_30_days_picker, |view, picker, event, _| {
    match event {
        DatePickerEvent::Change(Date::Range(Some(start), Some(end))) => {
            let duration = end.signed_duration_since(*start).num_days();
            if duration > 30 {
                picker.update(cx, |state, cx| {
                    state.set_date(Date::Range(Some(*start), None), window, cx);
                });
            }
        }
        _ => {}
    }
});

DatePicker::new(&max_30_days_picker)
    .number_of_months(2)
    .placeholder("Select up to 30 days")
```

### 季度预设

```rust
use chrono::{NaiveDate, Datelike};

fn quarter_start(year: i32, quarter: u32) -> NaiveDate {
    let month = (quarter - 1) * 3 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
}

fn quarter_end(year: i32, quarter: u32) -> NaiveDate {
    let month = quarter * 3;
    let start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    NaiveDate::from_ymd_opt(year, month, start.days_in_month()).unwrap()
}

let year = Local::now().year();
let quarterly_presets = vec![
    DateRangePreset::range("Q1", quarter_start(year, 1), quarter_end(year, 1)),
    DateRangePreset::range("Q2", quarter_start(year, 2), quarter_end(year, 2)),
    DateRangePreset::range("Q3", quarter_start(year, 3), quarter_end(year, 3)),
    DateRangePreset::range("Q4", quarter_start(year, 4), quarter_end(year, 4)),
];

DatePicker::new(&date_picker)
    .presets(quarterly_presets)
```

## 示例

### 事件日期选择

```rust
let event_date = cx.new(|cx| {
    let mut picker = DatePickerState::new(window, cx)
        .date_format("%B %d, %Y")
        .disabled_matcher(calendar::Matcher::custom(|date| {
            *date < Local::now().naive_local().date()
        }));
    picker
});

DatePicker::new(&event_date)
    .placeholder("Choose event date")
    .cleanable(true)
```

### 预订系统日期范围

```rust
let booking_range = cx.new(|cx| DatePickerState::range(window, cx));

let booking_presets = vec![
    DateRangePreset::range("This Weekend", /* weekend dates */),
    DateRangePreset::range("Next Week", /* next week dates */),
    DateRangePreset::range("This Month", /* this month dates */),
];

DatePicker::new(&booking_range)
    .number_of_months(2)
    .presets(booking_presets)
    .placeholder("Select check-in and check-out dates")
```

### 财务周期选择

```rust
let financial_period = cx.new(|cx| {
    DatePickerState::range(window, cx)
        .date_format("%Y-%m-%d")
});

DatePicker::new(&financial_period)
    .number_of_months(3)
    .presets(quarterly_presets)
    .placeholder("Select reporting period")
```

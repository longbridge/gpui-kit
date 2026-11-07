---
title: TimeField
description: 分段输入一天中的时间，支持 24 小时制和 12 小时制。
---

# TimeField

TimeField 用于分段输入一天中的时间。时、分以及可选的秒和上午/下午各自独立，用键盘逐段编辑；输入数字时字段宽度保持不变。[DatePicker](date-picker) 也用它来编辑日期对应的时间。

## 导入

```rust
use gpui_kit::component::time_field::{
    HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision,
};
```

## 用法

### 基础用法

```rust
let time = cx.new(|cx| TimeFieldState::new(window, cx));

TimeField::new(&time)
```

字段初始值为 `00:00`。用 `set_time` 设置值，这个方法不会触发事件：

```rust
use chrono::NaiveTime;

time.update(cx, |state, cx| {
    state.set_time(NaiveTime::from_hms_opt(9, 30, 0).unwrap(), window, cx);
});
```

### 精确到秒

```rust
let time = cx.new(|cx| {
    TimeFieldState::new(window, cx).precision(TimePrecision::Second)
});

TimeField::new(&time) // 09:30:15
```

### 12 小时制

默认使用 24 小时制。`HourCycle::H12` 会把小时显示为 `12` 到 `11`，并在后面加一个上午/下午（AM/PM）段：

```rust
let time = cx.new(|cx| {
    TimeFieldState::new(window, cx).hour_cycle(HourCycle::H12)
});

TimeField::new(&time) // 09:30 PM
```

### 尺寸、禁用与校验状态

```rust
TimeField::new(&time).small()
TimeField::new(&time).large()
TimeField::new(&time).disabled(true)

// 显示业务层的校验结果，不会拒绝用户的编辑。
TimeField::new(&time).invalid(true)
```

## 处理变更

用户编辑后会触发 `TimeFieldEvent::Change`，携带新的时间：

```rust
cx.subscribe(&time, |this, _, event, cx| match event {
    TimeFieldEvent::Change(time) => {
        this.reminder = *time;
        cx.notify();
    }
});
```

## 键盘操作

| 按键 | 行为 |
| --- | --- |
| `Up` / `Down` | 调整当前选中的段，在段内循环，不会改动上一级 |
| `Left` / `Right` | 在段之间移动 |
| `Tab` / `Shift-Tab` | 在段之间移动，到头后离开字段 |
| `0`–`9` | 输入当前段，填满后自动跳到下一段 |
| `a` / `p` | 12 小时制下切换上午或下午 |
| `Backspace` / `Delete` | 重置当前段 |

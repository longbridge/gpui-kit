---
title: Sheet
description: 从屏幕边缘滑出的内容面板组件。
---

# Sheet

Sheet 是一种从屏幕边缘滑出的面板组件，也常被用作侧栏、抽屉或临时内容面板。它适合承载导航菜单、表单、设置项和辅助信息，而不会直接占用主视图空间。

## 导入

```rust
use gpui_kit::component::WindowExt;
use gpui_kit::component::Placement;
```

## 用法

### 在根视图中渲染 Sheet 图层

如果应用要支持 Sheet，需要在根视图中渲染 sheet layer。

[Root::render_sheet_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_sheet_layer) 会把当前激活的 Sheet 渲染到应用内容之上。

```rust
use gpui_kit::component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            .children(sheet_layer)
    }
}
```

### 基础 Sheet

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Navigation")
        .child("Sheet content goes here")
})
```

### 不同方向

```rust
window.open_sheet_at(Placement::Left, cx, |sheet, _, _| {
    sheet.title("Left Sheet")
})

window.open_sheet_at(Placement::Right, cx, |sheet, _, _| {
    sheet.title("Right Sheet")
})
```

### 自定义尺寸

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Wide Sheet")
        .size(px(500.))
        .child("This sheet is 500px wide")
})
```

### 表单内容

```rust
let input = cx.new(|cx| InputState::new(window, cx));
let date = cx.new(|cx| DatePickerState::new(window, cx));

window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("User Profile")
        .child(
            v_flex()
                .gap_4()
                .child("Enter your information:")
                .child(Input::new(&input).placeholder("Full Name"))
                .child(DatePicker::new(&date).placeholder("Date of Birth"))
        )
        .footer(
            h_flex()
                .gap_3()
                .child(Button::new("save").primary().label("Save"))
                .child(Button::new("cancel").label("Cancel"))
        )
})
```

### Overlay 与关闭行为

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Settings")
        .overlay(true)
        .overlay_closable(true)
        .child("Sheet settings content")
})
```

### 可调整大小

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Resizable Panel")
        .resizable(true)
        .size(px(300.))
        .child("You can resize this sheet by dragging the edge")
})
```

### 自定义位置偏移

```rust
window.open_sheet(cx, |sheet, _, _| {
    sheet
        .title("Below Title Bar")
        .margin_top(px(32.))
        .child("This sheet appears below the title bar")
})
```

### 自定义样式

```rust
window.open_sheet(cx, |sheet, _, cx| {
    sheet
        .title("Styled Sheet")
        .bg(cx.theme().accent)
        .text_color(cx.theme().accent_foreground)
        .border_color(cx.theme().primary)
        .child("Custom styled sheet content")
})
```

### 主动关闭

```rust
Button::new("close")
    .label("Close Sheet")
    .on_click(|_, window, cx| {
        window.close_sheet(cx);
    })

window.close_sheet(cx);
```

## API 参考

### Window 扩展

- `open_sheet(cx, fn)` 默认在右侧打开
- `open_sheet_at(placement, cx, fn)` 在指定方向打开
- `close_sheet(cx)` 关闭当前 Sheet

### 常用 Builder 方法

- `title(str)`
- `child(el)`
- `footer(el)`
- `size(px)`
- `margin_top(px)`
- `resizable(bool)`
- `overlay(bool)`
- `overlay_closable(bool)`
- `on_close(fn)`

## 最佳实践

1. 左右方向更适合导航和设置面板
2. 上下方向更适合临时辅助内容
3. 尽量提供清晰标题和明显关闭路径
4. 长内容建议分组排版
5. 对于复杂内容，尽量延迟加载内部数据

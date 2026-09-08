---
title: Rating
description: 简单的交互式星级评分组件。
---

# Rating

Rating 是一个星级评分组件，允许用户选择评分值。它支持不同尺寸、自定义颜色、禁用状态以及点击事件处理。

## 导入

```rust
use gpui_kit::component::rating::Rating;
```

## 用法

### 基础评分

```rust
Rating::new("my-rating")
    .value(3)
    .max(5)
    .on_click(|value, _, _| {
        println!("Rating changed to: {}", value);
    })
```

### 受控评分

```rust
struct MyView {
    rating: usize,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Rating::new("rating")
            .value(self.rating)
            .max(5)
            .on_click(cx.listener(|view, value: &usize, _, cx| {
                view.rating = *value;
                cx.notify();
            }))
    }
}
```

### 不同尺寸

Rating 实现了 [Sizable] trait：

```rust
Rating::new("rating").xsmall().value(3).max(5)
Rating::new("rating").small().value(3).max(5)
Rating::new("rating").value(3).max(5)
Rating::new("rating").large().value(3).max(5)
```

### 自定义颜色

默认使用主题中的 `yellow` 颜色。你也可以通过 `color` 方法覆盖：

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .color(cx.theme().green)
```

### 禁用状态

```rust
Rating::new("rating")
    .value(2)
    .max(5)
    .disabled(true)
```

### 自定义最大值

默认最大值为 5 星，也可以设置为任意数量：

```rust
Rating::new("rating")
    .value(7)
    .max(10)
```

### 点击行为

Rating 的点击行为有两个规则：

- 点击已点亮的星星，会将评分减少 1。
- 点击未点亮的星星，会将评分设置为该星星对应的值。

`on_click` 回调接收到的新值类型为 `&usize`。

```rust
Rating::new("rating")
    .value(3)
    .max(5)
    .on_click(|new_value, _, _| {
        println!("New rating: {}", new_value);
    })
```

## API 参考

- [Rating]

### 方法

- `new(id: impl Into<ElementId>)`：创建新的 Rating 组件。
- `with_size(size: impl Into<Size>)`：设置星星尺寸，支持 [Sizable]。
- `value(value: usize)`：设置当前评分值，范围 `0..=max`。
- `max(max: usize)`：设置最大星数，默认值为 5。
- `color(color: impl Into<Hsla>)`：设置激活颜色，默认使用主题黄色。
- `disabled(disabled: bool)`：禁用交互，支持 [Disableable]。
- `on_click(handler: Fn(&usize, &mut Window, &mut App))`：设置点击处理函数。

## 示例

### 只读展示

```rust
Rating::new("rating")
    .value(4)
    .max(5)
    .disabled(true)
```

### 带状态的交互评分

```rust
struct ProductView {
    user_rating: usize,
}

impl Render for ProductView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                Rating::new("product-rating")
                    .value(self.user_rating)
                    .max(5)
                    .on_click(cx.listener(|view, value: &usize, _, cx| {
                        view.user_rating = *value;
                        cx.notify();
                    }))
            )
            .child(format!("Your rating: {}/5", self.user_rating))
    }
}
```

### 自定义颜色的大尺寸评分

```rust
Rating::new("rating")
    .large()
    .value(5)
    .max(5)
    .color(cx.theme().orange)
```

[Rating]: https://docs.rs/gpui-component/latest/gpui_component/rating/struct.Rating.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
[Disableable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Disableable.html

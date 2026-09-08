---
title: Avatar
description: 支持图片、姓名首字母和占位图标的头像组件。
---

# Avatar

Avatar 用于显示用户头像图片，并在无图片时自动回退为姓名首字母或占位图标。组件支持多种尺寸，也可以通过 AvatarGroup 组合展示团队或成员列表。

## 导入

```rust
use gpui_kit::component::avatar::{Avatar, AvatarGroup};
```

## 用法

### 基础 Avatar

通过图片地址和用户名创建头像：

```rust
Avatar::new()
    .name("John Doe")
    .src("https://example.com/avatar.jpg")
```

### 使用首字母回退

当未提供图片时，Avatar 会显示用户名首字母，并自动生成背景颜色：

```rust
Avatar::new()
    .name("John Doe")

Avatar::new()
    .name("Jane Smith")
```

### 占位头像

适用于匿名用户或没有姓名的场景：

```rust
use gpui_kit::component::IconName;

Avatar::new()

Avatar::new()
    .placeholder(IconName::Building2)
```

### 不同尺寸

```rust
Avatar::new()
    .name("John Doe")
    .xsmall()

Avatar::new()
    .name("John Doe")
    .small()

Avatar::new()
    .name("John Doe")

Avatar::new()
    .name("John Doe")
    .large()

Avatar::new()
    .name("John Doe")
    .with_size(px(100.))
```

### 自定义样式

```rust
Avatar::new()
    .src("https://example.com/avatar.jpg")
    .with_size(px(100.))
    .border_3()
    .border_color(cx.theme().foreground)
    .shadow_sm()
    .rounded(px(20.))
```

## AvatarGroup

[AvatarGroup] 可以以紧凑、重叠的方式显示多个头像。

### 基础分组

```rust
AvatarGroup::new()
    .child(Avatar::new().src("https://example.com/user1.jpg"))
    .child(Avatar::new().src("https://example.com/user2.jpg"))
    .child(Avatar::new().src("https://example.com/user3.jpg"))
    .child(Avatar::new().name("John Doe"))
```

### 限制数量

```rust
AvatarGroup::new()
    .limit(3)
    .child(Avatar::new().src("https://example.com/user1.jpg"))
    .child(Avatar::new().src("https://example.com/user2.jpg"))
    .child(Avatar::new().src("https://example.com/user3.jpg"))
    .child(Avatar::new().src("https://example.com/user4.jpg"))
    .child(Avatar::new().src("https://example.com/user5.jpg"))
```

### 使用省略标记

当超过限制数量时，可显示 `...` 提示还有更多成员：

```rust
AvatarGroup::new()
    .limit(3)
    .ellipsis()
    .child(Avatar::new().src("https://example.com/user1.jpg"))
    .child(Avatar::new().src("https://example.com/user2.jpg"))
    .child(Avatar::new().src("https://example.com/user3.jpg"))
    .child(Avatar::new().src("https://example.com/user4.jpg"))
    .child(Avatar::new().src("https://example.com/user5.jpg"))
```

### 分组尺寸

[Sizable] trait 也可用于 AvatarGroup，并会作用于内部所有头像：

```rust
AvatarGroup::new()
    .xsmall()
    .child(Avatar::new().name("A"))
    .child(Avatar::new().name("B"))
    .child(Avatar::new().name("C"))

AvatarGroup::new()
    .small()
    .child(Avatar::new().name("A"))
    .child(Avatar::new().name("B"))

AvatarGroup::new()
    .child(Avatar::new().name("A"))
    .child(Avatar::new().name("B"))

AvatarGroup::new()
    .large()
    .child(Avatar::new().name("A"))
    .child(Avatar::new().name("B"))
```

### 批量添加头像

```rust
let avatars = vec![
    Avatar::new().src("https://example.com/user1.jpg"),
    Avatar::new().src("https://example.com/user2.jpg"),
    Avatar::new().name("John Doe"),
];

AvatarGroup::new()
    .children(avatars)
    .limit(5)
    .ellipsis()
```

## API 参考

- [Avatar]
- [AvatarGroup]

## 示例

### 团队成员展示

```rust
use gpui_kit::component::{h_flex, v_flex};

v_flex()
    .gap_4()
    .child("Development Team")
    .child(
        AvatarGroup::new()
            .limit(4)
            .ellipsis()
            .child(Avatar::new().name("Alice Johnson").src("https://example.com/alice.jpg"))
            .child(Avatar::new().name("Bob Smith").src("https://example.com/bob.jpg"))
            .child(Avatar::new().name("Charlie Brown"))
            .child(Avatar::new().name("Diana Prince"))
            .child(Avatar::new().name("Eve Wilson"))
    )
```

### 用户资料头部

```rust
h_flex()
    .items_center()
    .gap_4()
    .child(
        Avatar::new()
            .src("https://example.com/profile.jpg")
            .name("John Doe")
            .large()
            .border_2()
            .border_color(cx.theme().primary)
    )
    .child(
        v_flex()
            .child("John Doe")
            .child("Software Engineer")
    )
```

### 匿名用户

```rust
use gpui_kit::component::IconName;

Avatar::new()
    .placeholder(IconName::UserCircle)
    .medium()
```

### 自动配色

```rust
Avatar::new().name("Alice")
Avatar::new().name("Bob")
Avatar::new().name("Charlie")
```

[Avatar]: https://docs.rs/gpui-component/latest/gpui_component/avatar/struct.Avatar.html
[AvatarGroup]: https://docs.rs/gpui-component/latest/gpui_component/avatar/struct.AvatarGroup.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

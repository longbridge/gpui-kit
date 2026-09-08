---
title: Combobox
description: 带有可搜索下拉列表的自动补全输入组件。
---

# Combobox

支持从可搜索列表中选择一个或多个值的下拉选择组件。

## Select 与 Combobox 的区别

| 功能 | Select | Combobox |
| --- | --- | --- |
| 可搜索 | ✓（可选） | ✓（可选） |
| 多选 | — | ✓（`.multiple(true)`） |
| 自定义触发器渲染 | — | ✓ |
| 自定义列表项渲染 | — | ✓ |
| 底部操作插槽 | — | ✓ |

需要简单单选时用 `Select`；需要多选、完全自定义触发器或自定义列表项渲染时用 `Combobox`。

## 导入

```rust
use gpui_kit::component::combobox::{
    Combobox, ComboboxState, ComboboxEvent, ComboboxTriggerCtx,
};
use gpui_kit::component::searchable_list::{
    SearchableListItem, SearchableVec, SearchableGroup,
};
```

## 用法

### 基础单选

```rust
let state = cx.new(|cx| {
    ComboboxState::new(
        SearchableVec::new(vec!["Next.js", "SvelteKit", "Nuxt.js"]),
        vec![], // 无初始选中
        window,
        cx,
    )
    .searchable(true)
});

Combobox::new(&state)
    .placeholder("选择框架...")
    .search_placeholder("搜索...")
    .w_full()
```

### 多选

通过 `.multiple(true)` 开启多选模式。点击列表项会切换其选中状态，下拉菜单保持展开直到按下 Escape 或点击外部。

```rust
let state = cx.new(|cx| {
    ComboboxState::new(
        SearchableVec::new(vec!["React", "Vue", "Angular"]),
        vec![IndexPath::new(0)], // 预选项
        window,
        cx,
    )
    .multiple(true)
    .searchable(true)
});

Combobox::new(&state).placeholder("选择框架")
```

### 预选项

通过索引路径指定预选的列表项：

```rust
let state = cx.new(|cx| {
    ComboboxState::new(items, vec![IndexPath::new(0)], window, cx)
});
```

### 分组列表项

使用 `SearchableGroup` 对列表项进行分组：

```rust
let grouped = SearchableVec::new(vec![
    SearchableGroup::new("水果").items(vec![
        FoodItem::new("苹果"),
        FoodItem::new("香蕉"),
    ]),
    SearchableGroup::new("蔬菜").items(vec![
        FoodItem::new("胡萝卜"),
        FoodItem::new("菠菜"),
    ]),
]);

let state = cx.new(|cx| {
    ComboboxState::new(grouped, vec![], window, cx).searchable(true)
});

Combobox::new(&state)
```

### 实现 `SearchableListItem`

`String`、`SharedString` 和 `&'static str` 已内置实现了 `SearchableListItem`。自定义类型需手动实现该 trait：

```rust
#[derive(Clone)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl SearchableListItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &SharedString {
        &self.code
    }

    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(query)
            || self.code.to_lowercase().contains(query)
    }
}
```

### 禁用列表项

在列表项的 `disabled()` 方法中返回 `true` 即可将该项设为不可选：

```rust
impl SearchableListItem for MyItem {
    // ...
    fn disabled(&self) -> bool {
        self.is_unavailable
    }
}
```

### 自定义勾选图标

```rust
Combobox::new(&state)
    .check_icon(Icon::new(IconName::CircleCheck))
```

### 底部操作按钮

在下拉菜单底部渲染一个固定操作项（如"新建"按钮）：

```rust
Combobox::new(&state)
    .footer(|_, cx| {
        Button::new("add-new")
            .ghost()
            .label("新建项目")
            .icon(Icon::new(IconName::Plus))
            .w_full()
            .justify_start()
            .into_any_element()
    })
```

### 自定义触发器

完全覆盖触发器元素的渲染。`ComboboxTriggerCtx` 包含当前选中状态、开关标志和尺寸信息：

```rust
Combobox::new(&state)
    .render_trigger(|ctx, _, cx| {
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .when(ctx.selection.is_empty(), |this| {
                this.text_color(cx.theme().muted_foreground)
                    .child("请选择...")
            })
            .children(ctx.selection.iter().map(|(_, item)| {
                div()
                    .bg(cx.theme().accent)
                    .rounded_sm()
                    .px_1p5()
                    .py_0p5()
                    .text_sm()
                    .child(item.title())
            }))
            .into_any_element()
    })
```

### 尺寸

```rust
Combobox::new(&state).large()
Combobox::new(&state)  // 默认（medium）
Combobox::new(&state).small()
```

### 可清除

```rust
Combobox::new(&state).cleanable(true) // 有选中值时显示清除按钮
```

### 禁用状态

```rust
Combobox::new(&state).disabled(true)
```

### 事件监听

`Change`（每次切换时触发）和 `Confirm`（下拉菜单关闭时触发）均携带完整的选中值列表 `Vec<Value>`。

```rust
cx.subscribe_in(&state, window, |view, _, event, window, cx| {
    match event {
        ComboboxEvent::Change(values) => {
            // 每次切换时触发
        }
        ComboboxEvent::Confirm(values) => {
            // 下拉菜单关闭时触发
        }
    }
});
```

### 程序化操控

值会通过当前 delegate 解析，无法找到的值会被忽略。

`set_selected_values` 会先清除搜索关键词，因此正在进行的搜索不会决定哪些值可以被选中。
index path 定位的是列表当前显示的内容，所以 `set_selected_indices`、`add_selected_index`
和 `remove_selected_index` 作用于可见行，不会改动搜索关键词。

```rust
// 按值替换整个选中集合
state.update(cx, |s, cx| {
    s.set_selected_values(&["React", "Angular"], window, cx);
});

// 按索引路径替换整个选中集合
state.update(cx, |s, cx| {
    s.set_selected_indices(vec![IndexPath::new(0), IndexPath::new(2)], window, cx);
});

// 增加 / 移除单个选项
state.update(cx, |s, cx| {
    s.add_selected_index(IndexPath::new(1), cx);
    s.remove_selected_index(IndexPath::new(0), cx);
});

// 清空选中
state.update(cx, |s, cx| {
    s.clear_selection(cx);
});

// 读取所有选中值（多选）
let values = state.read(cx).selected_values(); // Vec<Value>

// 读取第一个选中值（单选便利方法）
let value = state.read(cx).selected_value(); // Option<Value>
```

## 键盘快捷键

| 按键       | 操作                             |
| ---------- | -------------------------------- |
| `Tab`      | 聚焦触发器                       |
| `Enter`    | 打开菜单或确认当前高亮项         |
| `↑ / ↓`   | 在选项间导航（未打开时自动打开） |
| `Escape`   | 关闭菜单                         |

## 主题样式

- `background` — 触发器背景
- `input` — 触发器边框颜色
- `foreground` — 文字颜色
- `muted_foreground` — 占位符和禁用文字颜色
- `border` — 菜单边框颜色
- `radius` — 圆角

---
title: History
description: 用于应用状态的浏览器式导航轨迹和分组 undo/redo 事务。
order: 7
---

# History

`History<T>` 与 `UndoHistory<T>` 分别保存两种不同的应用状态。两者都不持有 GPUI 状态，且都由调用方把返回的值应用到模型；但它们的操作含义不同：

- `History<T>` 是浏览器式的线性导航轨迹，支持后退和前进。
- `UndoHistory<T>` 将改动记录为 undo 事务，并能把多个改动合成一次用户操作。

## 引入

```rust
use gpui_kit::base::{History, UndoHistory};
```

## 如何选择

应根据状态的含义选择类型，而不是根据操作它的 UI 命令名称选择：

- 当每个条目表示一个位置，后退或前进需要返回到达的位置，并且必须保留当前根条目时，使用 `History<T>`。
- 当每个条目表示一项可逆改动，并且 undo 或 redo 需要返回一次用户事务中的全部改动时，使用 `UndoHistory<T>`。
- 当分组依赖比时间或显式边界更丰富的领域语义时，使用领域专用的管理器。例如 Input 使用私有事务管理器来理解输入、删除、选择区和 IME 组合输入。

在 gpui-component 内部，`NavStack` 使用 `History<NavEntry>` 进行页面导航；Dock 的 tiles canvas 使用 `UndoHistory<TileChange>` 回退分组的移动和缩放改动；Input 则有意保留其专用的私有 undo manager。

## `History`：导航轨迹

每到一个位置就 push 一条。当前条目是轨迹中的最后一个值。例如访问完 `A -> B -> C` 后，当前是 `C`；后退会返回新的当前条目 `B`：

```rust
let mut history = History::new();
history.push("A");
history.push("B");
history.push("C");

assert_eq!(history.back(), Some("B"));
assert_eq!(history.current(), Some(&"B"));
```

`back()` 不会越过根条目，到根时返回 `None`。`forward()` 会恢复最近一个此前离开的条目。后退后再 push 新条目会丢弃前进分支，和浏览器打开新页面时的行为相同。`max_entries` 限制从根到当前的活动条目：降低上限会立即删除最旧的多余活动条目；达到上限时前进，会先删除最旧的活动条目，再恢复下一个条目。

`entries()` 按从根到当前条目的顺序迭代。完整的 `A -> B -> C` 轨迹会依次得到 `A`、`B`、`C`；`entries().rev()` 则得到 `C`、`B`、`A`。`forward_entries()` 从最近的前进条目到最远的前进条目迭代。用 `retain` 删除已失效的位置，用 `replace_current` 原地更新当前的位置，用 `remove_current` 删除当前条目而不丢弃前进分支。

| 方法                                         | 作用                                               |
| -------------------------------------------- | -------------------------------------------------- |
| `new()`                                      | 创建空轨迹。`max_entries` 默认是 1000。            |
| `max_entries(n)`                             | 限制从根到当前的条目数，并立即删除最旧的多余条目。 |
| `push(entry)`                                | 让 `entry` 成为当前条目，并丢弃前进分支。          |
| `back()`、`forward()`                        | 在轨迹中移动，返回移动后的当前条目。               |
| `current()`                                  | 返回当前条目。                                     |
| `can_back()`、`can_forward()`                | 判断对应方向是否可以移动。                         |
| `entries()`、`forward_entries()`             | 按导航顺序迭代当前轨迹和前进分支。                 |
| `replace_current(entry)`、`remove_current()` | 更新或删除当前条目。                               |
| `retain(keep)`、`clear()`                    | 从两侧删除不保留的条目，或清空轨迹。               |

## `UndoHistory`：分组 undo 与 redo

每次需要由应用回退的改动都 push 一条。要把一次拖拽作为一项可撤销操作，请显式地把它的所有更新分组。`undo()` 以最新在前的顺序返回一个组里的改动，确保最近的改动先被回退；`redo()` 按最旧在前的顺序返回同一组，以原始顺序重新应用：

```rust
let mut history = UndoHistory::new();
history.start_grouping();
history.push("从 x=0 移到 x=10");
history.push("从 x=10 移到 x=20");
history.end_grouping();

assert_eq!(
    history.undo(),
    Some(vec!["从 x=10 移到 x=20", "从 x=0 移到 x=10"]),
);
assert_eq!(
    history.redo(),
    Some(vec!["从 x=0 移到 x=10", "从 x=10 移到 x=20"]),
);
```

对于边界不明确的改动，`group_interval` 会把时间间隔足够短的连续 push 合成一个事务。成功 undo 或 redo 会结束这段定时分组窗口，下一次 push 会创建新事务。显式分组与此独立：只要显式分组仍在进行，push 就会继续追加到当前事务，包括刚完成 undo 之后。新的 push 会清空 redo 事务。回放改动时，用 `set_ignoring(true)` 防止回放本身被再次记录。

| 方法                                  | 作用                                                                                                         |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `new()`                               | 创建空 undo 历史。`max_undos` 默认是 1000。                                                                  |
| `max_undos(n)`                        | 限制 undo 事务数并立即删除最旧的多余事务；redo 也会遵守该上限。                                              |
| `group_interval(duration)`            | 将相隔很近的连续 push 合并为一个事务。                                                                       |
| `start_grouping()`、`end_grouping()`  | 让后续 push 追加到当前事务；结束分组会停止这种显式追加行为。和上例一样，空历史中的第一个 push 会创建该事务。 |
| `push(change)`                        | 在当前或一个新事务中记录改动，并清空 redo。                                                                  |
| `undo()`、`redo()`                    | undo 时最新改动在前，redo 时最旧改动在前地返回最新事务。                                                     |
| `can_undo()`、`can_redo()`            | 判断是否有可用事务。                                                                                         |
| `set_ignoring(bool)`、`is_ignoring()` | 控制是否记录 push。                                                                                          |
| `clear()`                             | 清空 undo 与 redo 事务。                                                                                     |

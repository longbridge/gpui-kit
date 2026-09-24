---
title: SharedString
description: 了解如何选择并使用 GPUI 中不可变、易于 clone 的 UI 文本。
order: -2.45
---

# SharedString

微小成本会累积。UI 中一次普通的组件调用所带的成本，会随组件数量和重绘次数反复出现。GPUI 和 GPUI Kit 因此把它当作 API 设计约束：长期持有的文本应易于拥有和传递，不让每位调用方都完整复制其内容。`SharedString` 就是为组件作者提供这一默认路径的具体选择。

`SharedString` 是 GPUI 用于保存文本的自有、不可变类型。GPUI Kit 用它存储 label、placeholder、title 等内容；组件或元素在 builder 调用结束后仍需持有这些文本。对于跨多次 [render](./render)、组件边界或 task 闭包持有的 **UI 文本**，默认使用它。临时读取可以借用 [`&str`](https://doc.rust-lang.org/std/primitive.str.html)。可以通过 `use gpui_kit::*;` 或 `use gpui_kit::SharedString;` 导入。

## 微小成本会累积

选择框架类型时，微小的复制与分配是一项需要长期主动控制的预算，不应把负担留给每一个调用位置。若标签 API 要求调用方每次都提供新建的自有缓冲区，大量普通组件组合就会反复完整复制文本。以 `SharedString` 接收长期持有的文本，组件可以拥有自己的值，调用方也能复用已经保存的值。常见路径无需给每个标签手写缓存。

这种默认设计避免的是反复**完整复制文本**，并非消除全部工作：clone 内联文本会复制少量字节，clone 共享堆文本会更新引用计数，`format!(...).into()` 每次运行仍会构造新文本。其他成本有各自的工具，例如用[虚拟列表](../base/virtual-list)跳过屏幕外行，用[视图缓存](./view-cache)复用未变化的子树。

## 为何 UI 文本优先用它而不是 `String`

设想工作区 View 将标题依次传给标题栏、标签页组件，最后交给标签。每层若要在调用方返回后继续持有标题，就得遵循 Rust 的所有权规则：移动值并放弃上游那份、借用并受源数据生命周期约束，或 clone。Rust 的 [`String`](https://doc.rust-lang.org/std/string/struct.String.html) 拥有可变缓冲区；在每个边界 clone 非空 `String`，就会创建独立缓冲区并复制标题字节。若每层改用 `format!` 重新构造标题，则会反复格式化和分配。

`SharedString` 以容易 clone 的形式表示这段不可变文本。工作区可以保留一份值，每层再取得自有的 clone；对于堆上长文本，这些 clone 共享字节，无需在每次透传时完整复制。这就是 GPUI 与 GPUI Kit 在许多文本属性中使用它、并在组件边界接收 `impl Into<SharedString>` 的原因。代价是不可变：改动文本就要构建新值。只要内容不变，多个副本才得以共享；这并不意味着绝对零开销。

<figure class="shared-string-memory">
  <svg viewBox="0 0 880 360" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="shared-memory-title-zh shared-memory-desc-zh">
    <title id="shared-memory-title-zh">三个 String 所有者与三个 SharedString 所有者的内存关系</title>
    <desc id="shared-memory-desc-zh">对于存于堆上的长文本，三个 String 所有者各自指向完整文本缓冲区。三个 SharedString 所有者分别持有小型句柄，指向一份带引用计数的共享文本。虚线动效只表示所有权连接，不表示耗时。</desc>
    <defs>
      <marker id="shared-arrow-zh" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M1 1 L7 4 L1 7" /></marker>
    </defs>
    <rect class="memory-panel" x="1" y="1" width="878" height="173" rx="12" />
    <text class="memory-heading" x="24" y="29">String::clone()</text>
    <text class="memory-subtitle" x="24" y="49">每次取得自有副本，都要把完整文本复制到另一份分配中。</text>
    <rect class="memory-owner" x="24" y="65" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="83">View A · 句柄</text>
    <rect class="memory-owner" x="24" y="103" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="121">View B · 句柄</text>
    <rect class="memory-owner" x="24" y="141" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="159">View C · 句柄</text>
    <path class="memory-flow memory-flow-copy" d="M183 78 H451" marker-end="url(#shared-arrow-zh)" />
    <path class="memory-flow memory-flow-copy" d="M183 116 H451" marker-end="url(#shared-arrow-zh)" />
    <path class="memory-flow memory-flow-copy" d="M183 154 H451" marker-end="url(#shared-arrow-zh)" />
    <rect class="memory-buffer memory-buffer-copy" x="463" y="65" width="190" height="27" rx="5" /><text class="memory-label" x="476" y="83">文本缓冲区 A · 完整文本</text>
    <rect class="memory-buffer memory-buffer-copy" x="463" y="103" width="190" height="27" rx="5" /><text class="memory-label" x="476" y="121">文本缓冲区 B · 完整文本</text>
    <rect class="memory-buffer memory-buffer-copy" x="463" y="141" width="190" height="27" rx="5" /><text class="memory-label" x="476" y="159">文本缓冲区 C · 完整文本</text>
    <text class="memory-total memory-total-copy" x="676" y="121">3 份文本缓冲区</text>
    <rect class="memory-panel" x="1" y="186" width="878" height="173" rx="12" />
    <text class="memory-heading" x="24" y="214">SharedString::clone()</text>
    <text class="memory-subtitle" x="24" y="234">每位所有者都持有句柄；长文本只保留一份分配。</text>
    <rect class="memory-owner" x="24" y="251" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="269">View A · 句柄</text>
    <rect class="memory-owner" x="24" y="289" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="307">View B · 句柄</text>
    <rect class="memory-owner" x="24" y="327" width="154" height="27" rx="5" /><text class="memory-label" x="36" y="345">View C · 句柄</text>
    <path class="memory-flow memory-flow-share" d="M183 264 L451 303" marker-end="url(#shared-arrow-zh)" />
    <path class="memory-flow memory-flow-share" d="M183 302 H451" marker-end="url(#shared-arrow-zh)" />
    <path class="memory-flow memory-flow-share" d="M183 340 L451 303" marker-end="url(#shared-arrow-zh)" />
    <rect class="memory-buffer memory-buffer-share" x="463" y="277" width="190" height="51" rx="6" /><text class="memory-label" x="476" y="300">一份共享文本缓冲区</text><text class="memory-detail" x="476" y="317">引用计数：3</text>
    <text class="memory-total memory-total-share" x="676" y="307">1 份文本缓冲区</text>
  </svg>
  <figcaption>这是三个所有者持有长文本的概念图，不是按字节精确缩放的性能基准。`SharedString` 每位所有者仍持有句柄，clone 也会更新引用计数；短内联文本则会复制少量字节。</figcaption>
</figure>

## 文本如何存储

GPUI 目前使用 `SmolStr` 作为 `SharedString` 的底层实现。存储形式取决于构造方式和文本的 **UTF-8 字节数**：

| 输入 | 当前存储形式 | clone 时发生什么 |
| --- | --- | --- |
| `SharedString::new_static("Ready")` | 引用静态字面量 | 复制小型 handle，不分配堆内存 |
| `SharedString::from("Ready")` | 将这段短文本复制到值内部 | 复制内联字节，不分配堆内存 |
| 较长的动态文本 | 共享的堆内存 | clone 带引用计数的 handle，不复制文本字节 |

当前的内联容量是 23 字节。某些由换行符后接空格组成的文本也会使用无需分配的特殊静态表示。这些属于实现细节，不是长度限制，也不表示每个 `SharedString` 都不会分配内存。尤其是，`SharedString::from("较长的字面量...")` 走普通转换路径；如果明确需要字面量的静态存储，应使用 `new_static`。构造和 clone 都不会对相同内容做字符串驻留或自动去重。

对于这些当前的存储形式，`SharedString::clone()` 的工作量不随文本长度增长：静态值复制 handle，短内联值复制少量字节，堆上值复制 handle 并更新引用计数。它不会复制堆上长文本的字节。`String::clone()` 则会分配独立缓冲区并复制文本字节。重新构造长 `SharedString` 仍可能分配；格式化、转换、引用计数更新，以及最终释放共享副本，都需要时间。

## 所有权与读取

即使输入来自临时 `String` 或借用的 `&str`，`SharedString` 得到的仍是可独立持有的值。转换非静态借用时，会按需要复制内容；结果不会借用调用方的数据。因此可以把它存入 [Entity](./entity)，或移入 callback，而无需维持输入的生命周期。

```rust
use gpui_kit::SharedString;

let title: SharedString = "Quarterly report for the product team".into();
let header_title = title.clone(); // 给一个 UI 所有者保留副本。
let tab_title = title.clone(); // 给另一个所有者保留副本。
let borrowed: &str = title.as_str(); // 只读借用，不取得所有权。

assert_eq!(borrowed, "Quarterly report for the product team");
assert_eq!(header_title, tab_title);
```

这里的 `.into()` 把字面量转换为自有的 `SharedString`；字面量若应使用静态存储，则改用 `SharedString::new_static(...)`。这里走的是普通转换路径，标题也超过当前内联容量，因此 `header_title` 与 `tab_title` 在内容不变时共享其堆上文本；它们的 clone 仍要更新引用计数。可以通过 `as_str()`、`AsRef<str>` 或解引用取得 `&str`，无需创建另一位所有者；这个借用不能超过原 `SharedString` 的生命周期。

`SharedString` 本身不能原地修改。如果确实需要可变的构造或编辑缓冲区，可在局部使用 `String`，完成后转换一次。将自有的长 `String` 转成 `SharedString` 时仍可能复制到共享存储中，不要假设会复用原缓冲区。

## 从 API 响应进入 View

对于 API 响应快照中的不可变文本字段，默认设计为 `SharedString`。用 Serde 将 JSON `title` 直接反序列化为这种类型，再把响应带入应用状态。若 UI 需要格式化后的标题，应在响应到达时另行生成展示值，而不是修改传输层响应：

```rust
use gpui_kit::SharedString;
use serde::Deserialize;

#[derive(Deserialize)]
struct UserResponse {
    title: SharedString,
}

struct Workspace {
    response: UserResponse,
    display_title: SharedString,
}

impl From<UserResponse> for Workspace {
    fn from(response: UserResponse) -> Self {
        let display_title = format!("Profile: {}", response.title).into();
        Self { response, display_title }
    }
}
```

把 `response` 移入 `Workspace`，其原始 `title` 不需要 clone。这里另行计算 `display_title` 一次；之后嵌套的 View 和组件需要自有文本时，可以 clone 任一值。若文本较长并存于堆上，这些后续 clone 会共享该值的文本分配，而不复制字节；短文本则存于内联空间。最初的 JSON 解析仍有成本：当前 `Deserialize` 实现先读取一个 `String`，再由它构造 `SharedString`，期间可能复制或分配。`Serialize` 输出普通字符串；共享是内存中的属性，不是 JSON 值的一部分。`format!` 也会为展示标题构造新值，因此应在数据变化时计算，而不是每次 render 时重新格式化。只有响应文本确实需要编辑时才使用可变文本缓冲区；普通只读响应字段无需从 `String` 开始。

## 在 GPUI Kit 中使用

GPUI Kit 的组件 builder 经常接收 `impl Into<SharedString>`，调用方可以传入字面量或已有的 `SharedString`。例如，[`Button::label`](../component/button) 与 [`Label::new`](../component/label) 都采用这种形式：

```rust
use gpui_kit::*;
use gpui_kit::component::{button::Button, label::Label};

let title = SharedString::new_static("Downloads");
let heading = Label::new(title.clone());
let button = Button::new("open-downloads").label(title);
```

对于存放在 View 中、会在多次 render 时使用的文本，在 View 中保存一个 `SharedString`，再 clone 给每次创建的元素。`Render::render` 会同时提供该 View 的 [Context](./context) 和 `Window`：

```rust
use gpui_kit::*;
use gpui_kit::component::label::Label;

struct Header {
    title: SharedString,
}

impl Render for Header {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Label::new(self.title.clone())
    }
}
```

clone 后，新元素有自己的 handle，View 也保留原值。对于长标题，这不会在每次 render 时复制全部字符。反过来，如果每次 render 都重新执行 `format!(...).into()`，仍会反复格式化和分配。将稳定文本保存在拥有它的状态中，只在内容变化时替换。

## 按文本的生命周期选择

对于 UI 元素、View、callback 或多个 owner 会长期持有的文本，优先使用 `SharedString`。需要自有值的固定字面量用 `SharedString::new_static`，明确采用静态存储。组件 builder 接收 `impl Into<SharedString>` 时，可以直接传这些值，不必先构造新的 `String`。

如果 API 只在当前调用中读取文本，传 `&str` 即可；已有 `SharedString` 也可用 `.as_str()` 借出。这样不会再创建文本所有者。

普通 UI 文本尽量不要引入 `String`。确实需要**可变构造或编辑**文本时，例如组装草稿，可以使用它，并在所有权边界将完成的内容转换一次：

```rust
let mut draft = String::from("Quarterly report");
draft.push_str(" for the product team");
let title: SharedString = draft.into();
```

这个 `.into()` 消费 `String` 并构造 `SharedString`；它可能复制或分配，不保证复用原缓冲区。GPUI Kit 的 [`InputState`](../component/input) 说明编辑缓冲区和对外值不必同型：它把可编辑文本存入 `Rope`，而 `value()` 每次调用都会物化一个 `SharedString` 快照。不要为了读取未变化的字段而反复调用 `value()`。日常 UI 代码往往不需要 `String`，但真正的文本构造和编辑工作仍可使用它。

:::note 源码快照（2026-09-24）
在 GPUI Kit 的 `crates/{kit,base,component,assets}/src` 正式库源码中，类型包含 `String` 的**具名 struct 存储字段有 34 个**，类型包含 `SharedString` 的有 **375 个**。这些 `String` 字段包括可编辑文本、搜索状态和主题 schema 数据等合理例外。这支持长期持有的 UI 文本默认使用 `SharedString`，并不表示整个仓库只有 34 处 `String` 用法。

复核时，扫描上述四个 `src` 目录中 `*.rs` 的具名 struct 主体，跳过仅用于测试的文件及 `#[cfg(test)]` 之后的内容，再统计字段类型中出现独立 Rust token `String` 与 `SharedString` 的数量。这是轻量源码扫描，不是 Rust 语义解析：它不含局部变量、函数签名和 enum 字段，包含源码中同时存在的原生与 WebAssembly `cfg` 变体，也不能推断运行时分配次数。
:::

## 与 `Cow<str>` 的关系

Rust 的 [`Cow<'a, str>`](https://doc.rust-lang.org/std/borrow/enum.Cow.html) 可以借用现有文本，避免当下复制，但其 `Borrowed` 形式受源数据生命周期约束。`Owned` 形式持有 `String`；clone 非空的 owned 值会复制文本字节。对 borrowed 值调用 `to_mut()`，则会先复制成自有的 `String`，再供修改：

```rust
use std::borrow::Cow;

let mut text: Cow<'_, str> = Cow::Borrowed("Ready");
text.to_mut().push('!'); // 现在是自有且可编辑的值。
```

`SharedString` 是独立持有的不可变值，具有静态、内联或共享堆存储形式。clone 堆上长文本会共享字节；修改内容需要构建新值。它不是 `Cow` 的别名，也没有 `Cow` 那种修改时才复制的行为。

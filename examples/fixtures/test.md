# Hello, **World**!

Inline image mix: larger PNG avatars <img src="https://avatars.githubusercontent.com/u/5518" alt="Jason Lee avatar" width="32" height="32" /> and <img src="https://avatars.githubusercontent.com/u/28998859" alt="GitHub avatar" width="32" height="32" /> stay inside the same text flow, and another SVG badge ![Rust](https://rust-lang.org/static/images/rust-logo-blk.svg) should wrap with nearby text when the window is resized.

This is first paragraph, there have **BOLD**, _italic_, and ~~strikethrough~~, `code` text [^1] [^2].

This is an additional demonstration paragraph in English demonstrating more content for [Markdown GFM]. It includes various stylistic elements and plain text.

Link click handling: [open this link](https://github.com/longbridge/gpui-kit) to see the custom `TextView` callback in the application log.

![Img](https://miro.medium.com/v2/resize:fit:1400/format:webp/1*WgEz5f3n3lD7MfC7NeQGOA.jpeg)

[Markdown GFM]: https://github.github.com/gfm/

[^1]: This is a footnote example.

[^2]: Here is another footnote.

## Basic formatting

### **Bold** text

You can mark some text as bold with **two asterisks**
or **two underscores**.

### **Italic** text

You can mark some text as italic with _asterisks_
or _underscores_.

### **_Bold and italic_**

Three stars gives **_bold and italic_**

### ~~Strikethrough~~

Using `~~two tildes~~` will strikethrough: ~~two tildes~~

## Blockquotes

> Blockquote: More complex nested inline style like **bold: _italic_**.
> This is second paragraph, it includes a block quote.

And this is next blockquote

> Hello, world!

### Nested blockquotes

> First level
>
> > Second level
> > Third level
>
> ```rs
> const FOO: &str = "bar";
> ```

## Code block

#### Rust

```rust
struct Repository {
    /// Name of the repository.
    name: String,
}

fn main() {
    let _ = Repository {
        name: "GPUI Component".to_string(),
    };

    println!("Hello, World!");
}
```

#### Python

```python
class Repository:
    """A repository."""

    def __init__(self, name: str):
        """Initialize the repository.

        Args:
            name: Name of the repository.
        """
        self.name = name
```

---

## Heading for [Links](https://www.google.com)

Here is a link to [Google](https://www.google.com), and another to [Rust](https://www.rust-lang.org).

## Image

![](https://miro.medium.com/v2/resize:fit:1400/format:webp/1*sOTh1aAl32jxKNuGO0TOcA.png)

### SVG

![Rust](https://rust-lang.org/static/images/rust-logo-blk.svg)

## Table

| Header 1 | Centered | Header 3                             | Align Right |
| -------- | :------: | ------------------------------------ | ----------: |
| Cell 0   |  Cell 1  | This is a long cell with line break. |      Cell 3 |
| Row 2    |  Row 2   | Row 2<br>[Link](https://github.com)  |       Row 2 |
| Row 3    | **Bold** | Row 3                                |       Row 3 |

See the way the text is aligned, depending on the position of `':'`

| Syntax    | Description |   Test Text |
| :-------- | :---------: | ----------: |
| Header    |    Title    | Here's this |
| Paragraph |    Text     |    And more |

## Lists

### Bulleted List

- Bullet 1, this is **very long** and needs to be wrapped to the next line, display should be wrapped to the next line as well.
  Continuation paragraph that should appear below.
- Bullet 2, the `second` bullet item is also long and needs to be wrapped to the next line.
  - Bullet 2.1
    This is a `deepth continuation` paragraph.
    - Bullet 2.1.1
      - Bullet 2.1.1.1
    - Bullet 2.1.2

  - Bullet 2.2

- Bullet 3

### Numbered List

1. Numbered item 1
   1. Numbered item 1.1
      1. Numbered item 1.1.1
   1. Numbered item 1.2
2. Numbered item 2
3. Numbered item 3

### To-Do List

- [x] Task 1, a long long text task, this line is very long and needs to be wrapped to the next line, display should be wrapped to the next line as well.
- [ ] Task 2, going to do something if there is a long text that needs to be wrapped to the next line.
- [ ] Task 3

### Block content in list items

1. A fenced code block:

   ```rust
   fn main() {
       println!("Nested code block");
   }
   ```

2. A blockquote:

   > Blockquotes inside list items should remain visible.

3. A table:

   | Name  | Value |
   | ----- | ----- |
   | Alpha | 1     |
   | Beta  | 2     |

4. A heading:

   #### Heading inside a list item

## Heading

Add `##` at the beginning of a line to set as Heading.
You can use up to 6 `#` symbols for the corresponding Heading levels

## Heading 2

This is paragraph of the heading 2.

### Heading 3

This is paragraph of the heading 3.

#### Heading 4

This is paragraph of the heading 4.

##### Heading 5

This is paragraph of the heading 5.

###### Heading 6

This is paragraph of the heading 6.

## HTML

### Paragraph and Text

<div>
    Here is a test in div.
    <p>This is a paragraph inside a div element, have <a href="https://google.com">link</a>, <strong>bold</strong>, <em>italic</em>, and <code>code</code> text.</p>
    <div>
        <p>This is second paragraph.</p>
    </div>
    A text after div.
</div>

### List

<ol>
<li>Numbered item 1</li>
<li>Numbered item 2</li>
</ol>

<ul>
<li>Bullet 1</li>
<li>Bullet 2</li>
</ul>

### Table

<table>
<thead>
<tr>
<td>Head 1</td>
<td>Head 2</td>
</tr>
</thead>
<tbody>
<tr>
<td><strong>Cell</strong> 1</td>
<td>Cell 2</td>
</tr>
<tr>
<td>Cell 3</td>
<td>Cell 4</td>
</tr>
</tbody>
</table>

### Image

<img src="https://miro.medium.com/v2/resize:fit:1400/format:webp/1*QY36p64kSGfBQsIFci8WBw.png" alt="The Best Programming Languages to Learn in 2025" width="100%" />

## Unsupported

### HTML

<details>
<summary>Click to expand</summary>
<div>
    <p>This is a paragraph <a href="https://google.com">inside</a> a details element.</p>
    <p>This is second paragraph.</p>
</div>
</details>

### Math

Inline math renders in the same text flow, for example $e^{i\pi} + 1 = 0$ and $a^2 + b^2 = c^2$.

$$
\frac{\alpha + \beta}{\sqrt{\gamma}} = \sum_{i=1}^{n} i^2
$$

## Markers

It also catches markers inside inline `code` and fenced blocks:

```rust
fn render() {
    // TODO: cache the parsed AST between frames
    // FIXME: handle empty input without a panic
}
```

This is final paragraph, it includes a code block and a list of items.

### Custom components

A custom Markdown parser converts project-specific syntax into typed nodes,
then registered renderers turn those nodes into arbitrary interactive
components.

Ticker blocks render as compact one-line quote rows:

$AAPL.US

$TSLA.US

A `<UserCard />` block renders a user card with a 24px avatar and a follow
button:

<UserCard id="huacnlee" />

<UserCard id="madcodelife" />

## Task markers

The custom `MarkerHighlighter` (an LSP-style semantic tokens provider)
highlights these markers in the source editor on the left, each in a
different color:

- TODO: support nested task lists
- FIXME: links with parentheses break parsing
- XXX: revisit the table column-width heuristic
- HACK: temporary workaround for footnote ordering
- NOTE: math blocks require the `$$` fence

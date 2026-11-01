//! The editor and Markdown code blocks highlighted by `syntect` alone.
//!
//! Run on its own so no other workspace member brings in tree-sitter:
//!
//! ```sh
//! cargo run -p example-syntect --release
//! ```

use gpui_kit::component::{
    ActiveTheme, Selectable as _, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, TabSize},
    text::TextView,
    v_flex,
};
use gpui_kit::*;

const SAMPLES: &[(&str, &str, &str)] = &[
    (
        "Rust",
        "rust",
        r#"/// A point in two dimensions.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    pub fn distance(&self, other: &Point) -> f32 {
        let (dx, dy) = (self.x - other.x, self.y - other.y);
        (dx * dx + dy * dy).sqrt()
    }
}

fn main() {
    let origin = Point { x: 0.0, y: 0.0 };
    println!("{:?} is {} away", origin, origin.distance(&Point { x: 3.0, y: 4.0 }));
}
"#,
    ),
    (
        "TOML",
        "toml",
        r#"[package]
name = "demo"
version = "0.1.0" # bumped on release
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }

[[bin]]
name = "demo"
path = "src/main.rs"
"#,
    ),
    (
        "TypeScript",
        "typescript",
        r#"interface User {
  id: number;
  name: string;
  email?: string;
}

export async function load(id: number): Promise<User> {
  const response = await fetch(`/api/users/${id}`);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return (await response.json()) as User;
}
"#,
    ),
    (
        "Python",
        "python",
        r#"from dataclasses import dataclass


@dataclass
class Order:
    """One line of an order."""
    sku: str
    quantity: int = 1

    def total(self, price: float) -> float:
        return self.quantity * price  # no discounts yet
"#,
    ),
    (
        "Markdown",
        "markdown",
        "# Release notes\n\n- **Faster** startup\n- `syntect` highlighting\n\n> Read the [docs](https://gpui-kit.com).\n",
    ),
];

const PREVIEW: &str = r#"# Code blocks

Highlighted by `syntect`, in languages the default build has no grammar for.

```toml
[package]
name = "demo"
version = "0.1.0"
```

```tsx
export const Hello = ({ name }: { name: string }) => <p>Hello, {name}</p>;
```

```dockerfile
FROM rust:1.85
RUN cargo build --release
```

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
```

```sql
SELECT id, name FROM users WHERE active = TRUE ORDER BY name;
```

```no-such-language
renders as plain text
```
"#;

struct Example {
    editor: Entity<EditorState>,
    language: &'static str,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (_, language, code) = SAMPLES[0];
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(language)
                .line_number(true)
                .folding(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(code)
        });
        Self { editor, language }
    }

    fn show(
        &mut self,
        language: &'static str,
        code: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.language = language;
        self.editor.update(cx, |editor, cx| {
            editor.set_highlighter(language, cx);
            editor.set_value(code, window, cx);
        });
        cx.notify();
    }

    /// About 20,000 lines of Rust: past the size reparsed on every keystroke,
    /// so editing shows the background parse.
    fn large_file() -> String {
        let (_, _, code) = SAMPLES[0];
        (0..1_000)
            .map(|index| code.replace("Point", &format!("Point{index}")))
            .collect()
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let buttons = SAMPLES.iter().map(|(label, language, code)| {
            let (language, code) = (*language, *code);
            Button::new(*label)
                .small()
                .label(*label)
                .selected(self.language == language)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.show(language, code.to_owned(), window, cx)
                }))
        });

        h_flex()
            .size_full()
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_1()
                            .p_2()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .children(buttons)
                            .child(
                                Button::new("large")
                                    .small()
                                    .ghost()
                                    .label("Large file")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show("rust", Self::large_file(), window, cx)
                                    })),
                            ),
                    )
                    .child(Editor::new(&self.editor).flex_1().bordered(false)),
            )
            .child(
                div()
                    .id("preview")
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .p_4()
                    .overflow_y_scroll()
                    .child(TextView::markdown("preview", PREVIEW)),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| Example::new(window, cx))
        })
        .expect("Failed to open window");
    });
}

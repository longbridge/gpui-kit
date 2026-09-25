---
title: Getting Started
description: Build your first GPUI Kit desktop application with one dependency and one view.
order: -2
---

# Getting Started

This guide builds a small desktop window with a GPUI Kit button. You need Rust and Cargo plus the system libraries for your platform; see [Installation](./installation.md) for macOS, Windows and Linux requirements. For a browser target, start with [WebAssembly](./webassembly.md) after learning the view model here.

## Create a project

```sh
cargo new gpui-hello
cd gpui-hello
```

Add GPUI Kit to the generated `Cargo.toml`:

```toml
[dependencies]
gpui-kit = "0.6"
```

This single dependency includes GPUI, GPUI Base, the styled GPUI Component library and its default icon assets. Application code accesses GPUI through `use gpui_kit::*;` and components through `gpui_kit::component`. You can change the feature selection later; see [Icons & Assets](./assets.md).

## Add a view

Replace `src/main.rs` with:

```rust
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child("Hello, World!")
            .child(
                Button::new("hello")
                    .primary()
                    .label("Click me")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    application()
        .with_assets(assets::Assets)
        .run(|cx| {
            init(cx);

            open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| HelloWorld)
            })
            .expect("Failed to open window");
        });
}
```

Run `cargo run` from the project directory. A window shows the label and button; clicking the button prints `Clicked!` in the terminal.

The startup sequence has three parts:

1. `gpui_kit::application()` creates the desktop application; `.with_assets(...)` registers the default icon source.
2. `gpui_kit::init(cx)` initializes the enabled Kit layers, including component themes. Call it once before opening application windows or constructing components.
3. `gpui_kit::open_window(...)` creates an `Entity<HelloWorld>` from the closure and wraps it in a [`Root`](./window). `Root` owns the window's overlay layers, including dialogs, sheets and notifications. Return your content view from the closure, not another `Root`.

`HelloWorld` implements GPUI's [`Render`](./render) trait. When GPUI renders the view, `render` returns an [element tree](./element): a `div` containing text and a `Button`. The button is a value built for that render; when a control needs lasting state, such as an input's text, the owning view keeps an [Entity](./entity) for that state instead of recreating it in `render`.

## A small mental model

An [Entity<T>](./entity) holds state across frames. It can own a model without drawing anything; when `T` implements `Render` and is mounted, the entity is a persistent **View** that builds a fresh element tree each time it renders. A [RenderOnce](./render-once) component takes its inputs as a value and describes a reusable piece of that tree. Use one where the caller supplies its state and handlers; it can still use small keyed element state. Give complex state, subscriptions, and tasks a lasting owner.

```text
app shell → feature (model, commands, view)
              ├─ Entity<Model>        retained state
              └─ Entity<View>         retained view; View implements Render
                    └─ element tree   rebuilt for each render
                         └─ RenderOnce values for reusable pieces
```

As an app grows, a feature with its own workflow can keep its model and views together in a feature crate, with a private [Global](./global) only when it needs truly application-wide state. Let features cooperate through small public interfaces, events, or `Entity` handles. This keeps reusable pieces inexpensive to adopt and gives teammates or AI agents a clear boundary for parallel changes. The [Coding Guides](./coding-guides) explain when to make that split and how to keep ownership and dependencies clear.

## Where to go next

Read these in order as your app grows:

1. [Entity](./entity.md), [Context](./context.md), and [Render](./render.md): retain a value, change it from a button callback, and confirm the window shows the new value. Read [Window](./window.md) to see how `Root` hosts that view and its overlays.
2. [Element](./element.md) and [RenderOnce](./render-once.md): distinguish the rebuilt element tree from persistent state. Follow the runnable Brush exercise in [Paint](./paint.md), and confirm a pointer press changes the drawing.
3. [Focus](./focus.md), [Action](./action.md), and [Event](./event.md): use Tab to reach an interactive target, then trigger one command and observe its state change. Follow [Task](./task.md) to run the streaming example; press Replay twice and confirm old chunks do not return.
4. [Accessibility](./accessibility.md) and [Testing](./test.md): follow the Save flow with a keyboard, check its focus and visible result, then run the documented UI test and confirm both the rendered status and saved model value. Check assistive technology separately on each target platform.
5. [Component catalog](../component/index.md): choose controls for your application; then read [Icons & Assets](./assets.md) and [Fonts](./fonts.md) as your interface needs them.

For a tested example of retained input state and subscriptions, read the [application recipes](https://github.com/longbridge/gpui-kit/tree/main/examples/ai_recipes). The [Coding Guides](./coding-guides.md) explain the conventions behind those examples.

## Complete tested view

This settings view is a compiled recipe showing retained input state and subscriptions. The excerpt is synchronized with its [Rust source](https://github.com/longbridge/gpui-kit/blob/main/examples/ai_recipes/src/settings.rs). The repository's default `gpui-kit-recipes` executable opens a small bootstrap view; it does not display this settings view. The [settings interaction test](https://github.com/longbridge/gpui-kit/blob/main/examples/ai_recipes/tests/settings.rs) mounts `Settings` in a GPUI test window. From the repository root, run:

```sh
cargo test -p gpui-kit-recipes --test settings
```

The test passes when typing updates the retained preview to `a`, then to `ab` after an unrelated redraw, with change counts of 1 and 2. This verifies the input subscription and state lifetime in a test window; it does not perform a native visual review.

<!-- recipe:settings:start -->
```rust
use gpui_kit::component::{
    ActiveTheme, IconName, WindowExt,
    button::Button,
    checkbox::Checkbox,
    form::{Field, Form},
    input::{Input, InputEvent, InputState},
    radio::RadioGroup,
    switch::Switch,
};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, div,
};

pub struct Settings {
    name: Entity<InputState>,
    preview: SharedString,
    changes: usize,
    enabled: bool,
    remember: bool,
    delivery: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl Settings {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let subscription = cx.subscribe_in(&name, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.preview = state.read(cx).value().to_string().into();
                this.changes += 1;
                cx.notify();
            }
        });
        Self {
            name,
            preview: "".into(),
            changes: 0,
            enabled: false,
            remember: false,
            delivery: Some(0),
            _subscriptions: vec![subscription],
        }
    }

    pub fn input(&self) -> Entity<InputState> {
        self.name.clone()
    }

    pub fn preview(&self) -> &SharedString {
        &self.preview
    }

    pub fn changes(&self) -> usize {
        self.changes
    }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child("Profile")
            .child(
                Form::new()
                    .child(Field::new().label("Name").child(Input::new(&self.name)))
                    .child(Field::new().label("Preview").child(self.preview.clone()))
                    .child(
                        Field::new().label_indent(false).child(
                            Checkbox::new("remember")
                                .label("Remember name")
                                .checked(self.remember)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.remember = *value;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        Field::new().label_indent(false).child(
                            Switch::new("enabled")
                                .label("Enable notifications")
                                .checked(self.enabled)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.enabled = *value;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        Field::new().label("Delivery").child(
                            RadioGroup::new("delivery")
                                .children(["Immediately", "Daily summary"])
                                .selected_index(self.delivery)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.delivery = Some(*value);
                                    cx.notify();
                                })),
                        ),
                    )
                    .footer(
                        Button::new("about")
                            .label("About…")
                            .icon(IconName::Info)
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog.title("About").child("A complete GPUI Kit window")
                                });
                            }),
                    ),
            )
    }
}
```
<!-- recipe:settings:end -->

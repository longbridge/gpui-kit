---
title: Getting Started
description: Build your first GPUI Kit desktop application with one dependency and one view.
order: -2
---

# Getting Started

This guide takes you from an empty folder to a desktop window with a label and a button. Install Rust and your platform's system libraries first; see [Installation](./installation.md).

## Create a project

1. Create a new Cargo project:

   ```sh
   cargo new gpui-hello
   cd gpui-hello
   ```

2. Add GPUI Kit to the generated `Cargo.toml`:

   ```toml
   [dependencies]
   gpui-kit = "{{gpui_kit_version}}"
   ```

This one dependency brings in GPUI, GPUI Base, the styled GPUI Component library and its default icons. Your code reaches GPUI through `use gpui_kit::*;` and components through `gpui_kit::component`. To change the feature selection later, see [Icons & Assets](./assets.md).

## Add a view and run it

1. Replace `src/main.rs` with:

   ```rust
   use gpui_kit::component::{
       button::{Button, ButtonVariants},
       label::Label,
   };
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
               .child(Label::new("Hello, World!"))
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

2. Run it from the project directory:

   ```sh
   cargo run
   ```

   The first build takes a while because Cargo compiles every dependency.

A window opens with "Hello, World!" above a **Click me** button. Clicking the button prints `Clicked!` in the terminal.

### What the code does

1. `application()` creates the desktop application; `.with_assets(...)` registers the default icon source.
2. `init(cx)` initializes the enabled Kit layers, including component themes. Call it once, before opening windows or constructing components.
3. `open_window(...)` creates an `Entity<HelloWorld>` from the closure and wraps it in a [`Root`](./window). `Root` hosts the window's dialogs, sheets and notifications. Return your content view from the closure, not another `Root`.

`HelloWorld` implements GPUI's [`Render`](./render) trait. Each time GPUI renders the view, `render` returns an [element tree](./element): a `div` holding a `Label` and a `Button`.

The greeting is a `Label` rather than a bare string so that screen readers can read it. `.child("Hello")` looks the same but is not exposed to assistive technology.

## A small mental model

- An [Entity<T>](./entity) holds state across frames. It can own a model that draws nothing.
- When `T` implements `Render` and is mounted, the entity is a persistent **View**. It builds a fresh element tree each time it renders.
- Elements such as the `Button` above are values built for one render. When a control needs lasting state, such as an input's text, the owning view keeps an [Entity](./entity) for that state instead of recreating it in `render`.
- A [RenderOnce](./render-once) component takes its inputs as a value and describes a reusable piece of the tree. Use one where the caller supplies the state and handlers; it can still keep small keyed element state. Complex state, subscriptions and tasks need a lasting owner.

```text
app shell → feature (model, commands, view)
              ├─ Entity<Model>        retained state
              └─ Entity<View>         retained view; View implements Render
                    └─ element tree   rebuilt for each render
                         └─ RenderOnce values for reusable pieces
```

As an app grows, a feature with its own workflow can keep its model and views together in a feature crate. Use a private [Global](./global) only for truly application-wide state. Features cooperate through small public interfaces, events or `Entity` handles, which keeps reusable pieces cheap to adopt and gives teammates or AI agents clear boundaries for parallel changes. The [Coding Guides](./coding-guides) explain when to split and how to keep ownership and dependencies clear.

## Optional configuration

The example above needs none of this. Come back when you want a different rendering backend or depend on GPUI without Kit.

### Use GPUI Fast

By default, GPUI Kit uses a pinned snapshot of upstream GPUI. Enable `gpui-fast` to switch to GPUI Fast's retained rendering and layout engine:

```toml
[dependencies]
gpui-kit = { version = "{{gpui_kit_version}}", features = ["gpui-fast"] }
```

- The feature switches GPUI, the native platforms, the WebAssembly backend and every enabled Kit layer together.
- Your code does not change: keep using `gpui_kit::*`, `gpui_kit::component` and `#[gpui_kit::test]`. No `[patch]`, extra engine dependency or crate alias is needed.
- It works with `default-features = false` and with `assets`, `test-support`, `inspector` or `profiler`.
- GPUI Fast dependencies use `0.1.0` requirements, so `cargo update` can select compatible `0.1.x` releases.
- Cargo features are additive: enabling Fast selects its public types for the whole dependency graph, while upstream packages remain build dependencies. Do not mix Kit's Fast types with a direct dependency on upstream GPUI.
- If you use Base, Component, assets, FPS, WebView or Shell on their own, enable their `gpui-fast` feature too when selecting a backend explicitly.

In a checkout of this repository, compare the gallery with `cargo run` and `cargo run --features gpui-fast`. WebAssembly builds use the nightly toolchain described in [WebAssembly](./webassembly.md).

### macOS text rendering: `font-kit`

**If you depend on `gpui-kit`, skip this section: it already enables `font-kit`.** It applies only to applications that depend on GPUI directly.

GPUI is published on crates.io as version-pinned snapshot crates: `gpui-pre` is GPUI itself and `gpui-pre-platform` holds the native platform backends. `gpui-kit` depends on matching versions for you. On macOS, text rendering needs the `font-kit` feature of `gpui-pre-platform`, which is off by default. Without it, the backend falls back to `NoopTextSystem`: a window opens but shows no text, and `all_font_names()` returns an empty list.

To enable it in a direct GPUI application:

1. Add the feature to the `gpui_platform` dependency. For a macOS-only application:

   ```toml
   [dependencies]
   gpui = { package = "gpui-pre", version = "={{gpui_pre_version}}" }
   gpui_platform = { package = "gpui-pre-platform", version = "={{gpui_pre_version}}", features = ["font-kit"] }
   ```

   Keep any other platform features your application uses, and keep both snapshot versions aligned.

2. Rebuild with `cargo run`.
3. To confirm, run `cargo tree -e features -i gpui-pre-macos` on macOS and check that `font-kit` is listed.

The feature belongs to `gpui-pre-platform`, which enables `gpui-pre-macos/font-kit`. Do not add `features = ["font-kit"]` to `gpui` or `gpui-kit`, do not add a separate `font-kit` dependency, and do not add these entries to a project that already uses `gpui-kit`.

## Where to go next

Read these in order as your app grows:

1. [Entity](./entity.md), [Context](./context.md) and [Render](./render.md): retain a value, change it from a button callback, and confirm the window shows the new value. [Window](./window.md) shows how `Root` hosts that view and its overlays.
2. [Element](./element.md) and [RenderOnce](./render-once.md): tell the rebuilt element tree apart from persistent state. Run the Brush exercise in [Paint](./paint.md) and confirm a pointer press changes the drawing.
3. [Focus](./focus.md), [Action](./action.md) and [Event](./event.md): use Tab to reach an interactive target, then trigger a command and observe its state change. Run the streaming example in [Task](./task.md); press Replay twice and confirm old chunks do not return.
4. [Accessibility](./accessibility.md) and [Testing](./test.md): complete the Save flow with a keyboard and check its focus and visible result, then run the documented UI test and confirm both the rendered status and the saved model value. Check assistive technology separately on each target platform.
5. [Component catalog](../component/index.md): choose controls for your application, then read [Icons & Assets](./assets.md) and [Fonts](./fonts.md) as your interface needs them.

For a browser target, continue with [WebAssembly](./webassembly.md). For a tested example of retained input state and subscriptions, read the [application recipes](https://github.com/longbridge/gpui-kit/tree/main/examples/ai_recipes); the [Coding Guides](./coding-guides.md) explain the conventions behind them.

## Complete tested view

This settings view shows retained input state and subscriptions. It is a compiled recipe, kept in sync with its [Rust source](https://github.com/longbridge/gpui-kit/blob/main/examples/ai_recipes/src/settings.rs).

To check it, run the [settings interaction test](https://github.com/longbridge/gpui-kit/blob/main/examples/ai_recipes/tests/settings.rs) from the repository root. It mounts `Settings` in a GPUI test window:

```sh
cargo test -p gpui-kit-recipes --test settings
```

The test passes when typing updates the retained preview to `a`, then to `ab` after an unrelated redraw, with change counts of 1 and 2. It verifies the input subscription and state lifetime, not the native visuals. The repository's default `gpui-kit-recipes` executable opens a small bootstrap view and does not display this settings view.

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

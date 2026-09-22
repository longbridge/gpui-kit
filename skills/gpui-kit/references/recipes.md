# Tested application recipe

This complete view comes from `examples/ai_recipes/src/settings.rs`. That consumer crate depends only on `gpui-kit`; the same source is compiled and tested by `cargo test -p gpui-kit-recipes`, and `script/check-ai-recipes` keeps this copy identical to it. Pair it with that example's `src/bootstrap.rs`, which installs assets, initializes the library, and wraps the window in `Root`.

The view owns both input state and subscriptions. Rendering creates only elements. The window's `Root` renders the dialog, sheet and notification layers above the view.

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

For changes in this repository, run `script/check-ai docs`, `script/check-ai rust`, or `script/check-ai shell` for the relevant verification profile (or `script/check-ai all` for all three), and `cargo test -p gpui-kit-recipes` when a recipe source changes. For a downstream app, compile and test your own consumer, then verify keyboard/focus and visuals in a real window. Automated recipe tests do not establish a model success rate.

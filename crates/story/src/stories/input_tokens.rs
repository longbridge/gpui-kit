//! A retained example shared by the Input and Textarea stories.
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _,
    button::Button,
    checkbox::Checkbox,
    h_flex,
    input::{
        InlineToken, InlineTokenTag, Input, InputContent, InputEvent, InputGroup, InputState,
        Textarea, TextareaState,
    },
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled, Window,
    div,
};

pub(super) struct TokenExample {
    state: State,
    saved: InputContent,
    readonly: bool,
    disabled: bool,
    result: String,
    _subscription: gpui_kit::Subscription,
}
#[derive(Clone)]
enum State {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
}
macro_rules! dispatch {
    ($state:expr, |$input:ident| $body:expr) => {
        match $state {
            State::Input($input) => $body,
            State::Textarea($input) => $body,
        }
    };
}

impl TokenExample {
    pub(super) fn new(multiline: bool, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let state = if multiline { State::Textarea(cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 6))) }
                else { State::Input(cx.new(|cx| InputState::new(window, cx))) };
            let text = if multiline { "/translate @alice@bob 请检查这段文字 🙂\nTry selecting part of a reference, deleting it, and undoing." }
                else { "Ask @alice@bob to review 🙂" };
            let saved = if multiline {
                InputContent::new(text).with_token(0..10, InlineToken::new("command", "/translate").with_label("Translate"))
                    .with_token(11..17, InlineToken::new("alice", "@alice").with_label("Alice"))
                    .with_token(17..21, InlineToken::new("bob", "@bob").with_label("Bob"))
            } else {
                InputContent::new(text).with_token(4..10, InlineToken::new("alice", "@alice").with_label("Alice"))
                    .with_token(10..14, InlineToken::new("bob", "@bob").with_label("Bob"))
            };
            dispatch!(&state, |input| input.update(cx, |input, cx| input.set_content(saved.clone(), window, cx).expect("valid demo content")));
            let subscription = dispatch!(&state, |input| cx.subscribe(input, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) { cx.notify(); }
            }));
            Self { state, saved, readonly: false, disabled: false, result: String::new(), _subscription: subscription }
        })
    }
}
impl Render for TokenExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = dispatch!(&self.state, |input| input.read(cx).content());
        let input = match &self.state {
            State::Input(input) => Input::new(input)
                .aria_label("Atomic reference input")
                .readonly(self.readonly)
                .disabled(self.disabled)
                .on_token_click(cx.listener(
                    |this, event: &gpui_kit::component::input::InlineTokenClickEvent, _, cx| {
                        this.result = format!("Opened {}", event.token().label());
                        cx.notify();
                    },
                ))
                .into_any_element(),
            State::Textarea(input) => InputGroup::new("token-composer")
                .input(
                    Textarea::new(input)
                        .aria_label("Atomic reference textarea")
                        .readonly(self.readonly)
                        .disabled(self.disabled)
                        .render_token(|token, _, _| {
                            InlineTokenTag::new(token)
                                .with_icon(IconName::File)
                                .with_tooltip("Open reference")
                        })
                        .on_token_click(cx.listener(
                            |this,
                             event: &gpui_kit::component::input::InlineTokenClickEvent,
                             _,
                             cx| {
                                this.result = format!("Opened {}", event.token().label());
                                cx.notify();
                            },
                        )),
                )
                .into_any_element(),
        };
        v_flex()
            .w_full()
            .gap_2()
            .child(input)
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("insert-token")
                            .small()
                            .label("Insert reference")
                            .disabled(self.readonly || self.disabled)
                            .on_click(cx.listener(|this, _, window, cx| {
                                // The ID names the resource, so inserting the same
                                // reference twice reuses it.
                                let result = dispatch!(&this.state, |input| input.update(
                                    cx,
                                    |input, cx| input.replace_with_token(
                                        InlineToken::new("reference", "@reference")
                                            .with_label("Reference"),
                                        window,
                                        cx
                                    )
                                ));
                                if let Err(error) = result {
                                    this.result = error.to_string();
                                }
                                dispatch!(&this.state, |input| input
                                    .update(cx, |input, cx| input.focus(window, cx)));
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("save-tokens")
                            .small()
                            .label("Save draft")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.saved =
                                    dispatch!(&this.state, |input| input.read(cx).content());
                                this.result = "Draft saved".into();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("restore-tokens")
                            .small()
                            .label("Restore draft")
                            .disabled(self.readonly || self.disabled)
                            .on_click(cx.listener(|this, _, window, cx| {
                                let saved = this.saved.clone();
                                let result = dispatch!(&this.state, |input| input
                                    .update(cx, |input, cx| input.set_content(saved, window, cx)));
                                this.result = result.map_or_else(
                                    |error| error.to_string(),
                                    |_| "Draft restored".into(),
                                );
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("submit-tokens")
                            .small()
                            .label("Submit")
                            .disabled(self.disabled)
                            .on_click(cx.listener(|this, _, _, cx| {
                                let content =
                                    dispatch!(&this.state, |input| input.read(cx).content());
                                this.result = format!(
                                    "Submitted {} active references: {}",
                                    content.tokens().len(),
                                    content.text()
                                );
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Checkbox::new("tokens-readonly")
                            .label("Read-only")
                            .checked(self.readonly)
                            .on_click(cx.listener(|this, value, _, cx| {
                                this.readonly = *value;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("tokens-disabled")
                            .label("Disabled")
                            .checked(self.disabled)
                            .on_click(cx.listener(|this, value, _, cx| {
                                this.disabled = *value;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("Text: {}", content.text())),
            )
            .child(div().text_sm().child(format!(
                    "Tokens: {}",
                    content
                        .tokens()
                        .iter()
                        .map(|s| format!("{} {:?}", s.token().id(), s.range()))
                        .collect::<Vec<_>>()
                        .join(", ")
                )))
            .child(div().text_sm().child(self.result.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};
    use std::{cell::RefCell, ops::Deref as _, rc::Rc};

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn click(id: &'static str, cx: &mut VisualTestContext) {
        let bounds = cx.update(|window, _| {
            gpui_kit::base::test_support::find(window, &[], &id.into())
                .expect("story control is laid out")
                .bounds()
        });
        cx.simulate_click(bounds.center(), Modifiers::default());
        draw(cx);
    }

    fn content(story: &Entity<TokenExample>, cx: &mut VisualTestContext) -> InputContent {
        story.read_with(cx, |story, cx| {
            dispatch!(&story.state, |input| input.read(cx).content())
        })
    }

    #[gpui_kit::test]
    fn token_story_insert_delete_undo_restore_and_permissions(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        for multiline in [false, true] {
            let mounted = Rc::new(RefCell::new(None));
            let capture = mounted.clone();
            let window = cx.add_window(move |window, cx| {
                let story = TokenExample::new(multiline, window, cx);
                *capture.borrow_mut() = Some(story.clone());
                gpui_kit::component::Root::new(story, window, cx)
            });
            let story = mounted.borrow().clone().unwrap();
            let mut visual = VisualTestContext::from_window(*window.deref(), cx);
            draw(&mut visual);
            let initial = content(&story, &mut visual);
            click("insert-token", &mut visual);
            let inserted = content(&story, &mut visual);
            assert_eq!(inserted.tokens().len(), initial.tokens().len() + 1);
            click("save-tokens", &mut visual);
            let range = inserted
                .tokens()
                .iter()
                .find(|span| span.token().id().as_ref() == "reference")
                .unwrap()
                .range();
            visual.update(|window, cx| {
                story.update(cx, |story, cx| {
                    dispatch!(&story.state, |input| input.update(cx, |input, cx| {
                        input.set_selected_range(range.start + 1..range.end, cx);
                        input.focus(window, cx);
                    }));
                })
            });
            visual.simulate_keystrokes("backspace");
            draw(&mut visual);
            assert_eq!(
                content(&story, &mut visual).tokens().len(),
                initial.tokens().len()
            );
            #[cfg(target_os = "macos")]
            visual.simulate_keystrokes("cmd-z");
            #[cfg(not(target_os = "macos"))]
            visual.simulate_keystrokes("ctrl-z");
            draw(&mut visual);
            assert_eq!(content(&story, &mut visual), inserted);
            click("insert-token", &mut visual);
            click("restore-tokens", &mut visual);
            assert_eq!(content(&story, &mut visual), inserted);
            click("submit-tokens", &mut visual);
            story.read_with(&visual, |story, _| {
                assert_eq!(
                    story.result,
                    format!(
                        "Submitted {} active references: {}",
                        inserted.tokens().len(),
                        inserted.text()
                    )
                );
            });
            click("tokens-readonly", &mut visual);
            click("insert-token", &mut visual);
            assert_eq!(content(&story, &mut visual), inserted);
            click("tokens-readonly", &mut visual);
            click("tokens-disabled", &mut visual);
            click("restore-tokens", &mut visual);
            click("insert-token", &mut visual);
            assert_eq!(content(&story, &mut visual), inserted);
        }
    }
}

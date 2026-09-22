//! A chat composer shared by the Input and Textarea stories. Commands, images,
//! skills and people are inserted as atomic inline tokens; the application
//! maps a token's ID back to the resource it names.
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{
        InlineToken, InlineTokenClickEvent, InlineTokenContext, InlineTokenError, InputContent,
        InputEvent, InputGroup, InputGroupAddon, InputGroupAddonAlignment as Align,
        InputGroupButton, InputGroupInput, InputGroupTextarea, InputState, InputToken,
        TextareaState,
    },
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled, Window, div, prelude::FluentBuilder as _, rems,
};

/// What a token refers to, recovered from the prefix of its ID.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reference {
    Command,
    Image,
    Skill,
    Person,
}

impl Reference {
    const ALL: [Self; 4] = [Self::Command, Self::Image, Self::Skill, Self::Person];

    fn of(token: &InlineToken) -> Option<Self> {
        let (prefix, _) = token.id().split_once(':')?;
        Self::ALL
            .into_iter()
            .find(|reference| reference.prefix() == prefix)
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Image => "image",
            Self::Skill => "skill",
            Self::Person => "person",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Command => IconName::SquareTerminal,
            Self::Image => IconName::Image,
            Self::Skill => IconName::Sparkles,
            Self::Person => IconName::AtSign,
        }
    }

    /// The button that inserts a sample of this reference.
    fn action(self) -> (&'static str, &'static str) {
        match self {
            Self::Command => ("insert-command", "Insert command"),
            Self::Image => ("insert-image", "Attach image"),
            Self::Skill => ("insert-skill", "Insert skill"),
            Self::Person => ("insert-person", "Mention someone"),
        }
    }

    /// The sample the story inserts. A real composer resolves this from a
    /// picker or completion; the ID is what the application looks up later.
    fn sample(self) -> InlineToken {
        match self {
            Self::Command => InlineToken::new("command:commit-pr", "/commit-pr"),
            Self::Image => InlineToken::new("image:1", "[Image 1]").with_label("Image 1"),
            Self::Skill => InlineToken::new("skill:gpui-kit", "$gpui-kit").with_label("gpui-kit"),
            Self::Person => InlineToken::new("person:alice", "@alice").with_label("Alice"),
        }
    }
}

/// Build content from prose, attaching each sample token at its first
/// occurrence so the byte ranges never drift from the text.
fn content(text: &str, references: &[Reference]) -> InputContent {
    references
        .iter()
        .fold(InputContent::new(text), |content, reference| {
            let token = reference.sample();
            let start = text
                .find(token.text().as_ref())
                .expect("sample is in the text");
            content
                .with_token(start..start + token.text().len(), token)
                .expect("samples do not overlap")
        })
}

fn describe(content: &InputContent) -> String {
    content
        .tokens()
        .iter()
        .map(|span| format!("{} {:?}", span.token().id(), span.range()))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) struct TokenExample {
    state: State,
    saved: InputContent,
    readonly: bool,
    disabled: bool,
    status: SharedString,
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
            let (state, saved) = if multiline {
                let state = cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 6));
                let saved = content(
                    "/commit-pr for the token composer, then ask @alice to review [Image 1] \
                     against $gpui-kit 🙂\nSelect part of a reference, delete it, and undo.",
                    &Reference::ALL,
                );
                (State::Textarea(state), saved)
            } else {
                let state = cx.new(|cx| InputState::new(window, cx));
                let saved = content(
                    "Ask @alice to review [Image 1] 🙂",
                    &[Reference::Person, Reference::Image],
                );
                (State::Input(state), saved)
            };
            dispatch!(&state, |input| input.update(cx, |input, cx| input
                .set_value(saved.clone(), window, cx)));
            let subscription = dispatch!(&state, |input| cx.subscribe(
                input,
                |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                }
            ));
            Self {
                state,
                saved,
                readonly: false,
                disabled: false,
                status: SharedString::default(),
                _subscription: subscription,
            }
        })
    }

    fn content(&self, cx: &App) -> InputContent {
        dispatch!(&self.state, |input| input.read(cx).content())
    }

    /// Insert a reference the way a picker would: separated from the
    /// surrounding words, with the caret ready for the next one.
    fn insert(&mut self, reference: Reference, window: &mut Window, cx: &mut Context<Self>) {
        let result = dispatch!(&self.state, |input| input.update(cx, |input, cx| {
            let start = input.selected_range().start;
            let text = input.value();
            if text[..start].ends_with(|c: char| !c.is_whitespace()) {
                input.replace(" ", window, cx);
            }
            input.replace_with_token(reference.sample(), window, cx)?;
            input.replace(" ", window, cx);
            input.focus(window, cx);
            Ok::<_, InlineTokenError>(())
        }));
        if let Err(error) = result {
            self.status = error.to_string().into();
        }
        cx.notify();
    }

    fn open(&mut self, event: &InlineTokenClickEvent, cx: &mut Context<Self>) {
        let token = event.token();
        self.status = match Reference::of(token) {
            Some(Reference::Command) => format!("Would run {}", token.text()),
            Some(Reference::Image) => format!("Would preview {}", token.label()),
            Some(Reference::Skill) => format!("Would open the {} skill", token.label()),
            Some(Reference::Person) => format!("Would open {}'s profile", token.label()),
            None => format!("Unknown reference {}", token.id()),
        }
        .into();
        cx.notify();
    }

    fn send(&mut self, cx: &mut Context<Self>) {
        let content = self.content(cx);
        self.status = format!(
            "Sent {:?} with {} references",
            content.text().trim(),
            content.tokens().len()
        )
        .into();
        cx.notify();
    }

    fn token(token: &InlineTokenContext) -> InputToken {
        let reference = Reference::of(token.token());
        InputToken::new(token).when_some(reference, |element, reference| {
            element.icon(reference.icon())
        })
    }

    fn insert_button(&self, reference: Reference, cx: &Context<Self>) -> InputGroupButton {
        let (id, label) = reference.action();
        InputGroupButton::new(id)
            .icon(reference.icon())
            .accessibility_label(label)
            .tooltip(label)
            .disabled(self.readonly)
            .on_click(cx.listener(move |this, _, window, cx| this.insert(reference, window, cx)))
    }

    fn send_button(&self, cx: &Context<Self>) -> InputGroupButton {
        InputGroupButton::new("send-message")
            .primary()
            .label("Send")
            .on_click(cx.listener(|this, _, _, cx| this.send(cx)))
    }

    fn composer(&self, cx: &Context<Self>) -> InputGroup {
        let inserts = Reference::ALL
            .into_iter()
            .map(|reference| self.insert_button(reference, cx));
        let open = cx.listener(|this, event: &InlineTokenClickEvent, _, cx| this.open(event, cx));
        match &self.state {
            State::Input(input) => InputGroup::new("token-composer")
                .input(
                    InputGroupInput::new(input)
                        .aria_label("Message")
                        .token(|token, _, _| Self::token(token))
                        .on_token_click(open),
                )
                .addon(
                    InputGroupAddon::new("composer-actions")
                        .align(Align::InlineEnd)
                        .children(inserts)
                        .child(self.send_button(cx)),
                ),
            State::Textarea(input) => InputGroup::new("token-composer")
                .input(
                    InputGroupTextarea::new(input)
                        .aria_label("Message")
                        .token(|token, _, _| Self::token(token))
                        .on_token_click(open),
                )
                .addon(
                    InputGroupAddon::new("composer-actions")
                        .align(Align::BlockEnd)
                        .children(inserts)
                        .child(self.send_button(cx).ml_auto()),
                ),
        }
        .readonly(self.readonly)
        .disabled(self.disabled)
    }

    fn readout(&self, cx: &Context<Self>) -> impl IntoElement {
        let content = self.content(cx);
        let row = |label: &'static str, value: String| {
            h_flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .flex_shrink_0()
                        .w(rems(5.5))
                        .text_color(cx.theme().muted_foreground)
                        .child(label),
                )
                .child(div().min_w_0().flex_1().child(value))
        };
        v_flex()
            .w_full()
            .gap_1()
            .text_sm()
            .child(row("Text", content.text().to_string()))
            .child(row("References", describe(&content)))
            .when(!self.status.is_empty(), |this| {
                this.child(row("Status", self.status.to_string()))
            })
    }
}

impl Render for TokenExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_4()
            .child(self.composer(cx))
            .child(
                h_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("save-draft")
                                    .outline()
                                    .small()
                                    .label("Save draft")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.saved = this.content(cx);
                                        this.status = "Draft saved".into();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("restore-draft")
                                    .outline()
                                    .small()
                                    .label("Restore draft")
                                    .disabled(self.readonly || self.disabled)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        let saved = this.saved.clone();
                                        dispatch!(&this.state, |input| input
                                            .update(cx, |input, cx| input
                                                .set_value(saved, window, cx)));
                                        this.status = "Draft restored".into();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                Checkbox::new("tokens-readonly")
                                    .small()
                                    .label("Readonly")
                                    .checked(self.readonly)
                                    .on_click(cx.listener(|this, value, _, cx| {
                                        this.readonly = *value;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("tokens-disabled")
                                    .small()
                                    .label("Disabled")
                                    .checked(self.disabled)
                                    .on_click(cx.listener(|this, value, _, cx| {
                                        this.disabled = *value;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(self.readout(cx))
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
        story.read_with(cx, |story, cx| story.content(cx))
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
            let end = initial.text().len();
            visual.update(|_, cx| {
                story.update(cx, |story, cx| {
                    dispatch!(&story.state, |input| input
                        .update(cx, |input, cx| input.set_selected_range(end..end, cx)));
                })
            });
            click("insert-command", &mut visual);
            let inserted = content(&story, &mut visual);
            assert_eq!(inserted.tokens().len(), initial.tokens().len() + 1);
            assert!(
                inserted.text().ends_with(" /commit-pr "),
                "a picker separates the reference from its neighbors: {:?}",
                inserted.text()
            );
            click("save-draft", &mut visual);
            let range = inserted
                .tokens()
                .iter()
                .find(|span| span.token().id().as_ref() == "command:commit-pr")
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
            click("insert-person", &mut visual);
            click("restore-draft", &mut visual);
            assert_eq!(content(&story, &mut visual), inserted);
            click("send-message", &mut visual);
            story.read_with(&visual, |story, _| {
                assert_eq!(
                    story.status,
                    format!(
                        "Sent {:?} with {} references",
                        inserted.text().trim(),
                        inserted.tokens().len()
                    )
                );
            });
            click("tokens-readonly", &mut visual);
            click("insert-command", &mut visual);
            assert_eq!(content(&story, &mut visual), inserted);
            click("tokens-readonly", &mut visual);
            click("tokens-disabled", &mut visual);
            click("restore-draft", &mut visual);
            click("insert-command", &mut visual);
            assert_eq!(content(&story, &mut visual), inserted);
        }
    }
}

use std::collections::BTreeMap;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, Size,
    button::ButtonVariants as _,
    input::{InputContentType, InputEvent, InputState, TextareaState},
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlignment as Align, InputGroupButton,
        InputGroupButtonSize as ButtonSize, InputGroupInput, InputGroupText, InputGroupTextarea,
    },
    kbd::Kbd,
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, ClipboardItem, Context, Entity, FocusHandle, Focusable, IntoElement,
    Keystroke, ParentElement as _, Render, SharedString, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, rems,
};

use crate::{Story, section};

mod examples;

pub struct InputGroupStory {
    focus_handle: FocusHandle,
    search: Entity<InputState>,
    url: Entity<InputState>,
    amount: Entity<InputState>,
    password: Entity<InputState>,
    email: Entity<InputState>,
    disabled: Entity<InputState>,
    disabled_invalid: Entity<InputState>,
    readonly: Entity<InputState>,
    loading: Entity<InputState>,
    shortcut: Entity<InputState>,
    notes: Entity<TextareaState>,
    message: Entity<TextareaState>,
    sizes: Vec<(Size, Entity<InputState>)>,
    copied: bool,
    starred: bool,
    runs: usize,
    attached: bool,
    last_message: Option<SharedString>,
    last_search: Option<SharedString>,
    example_inputs: BTreeMap<&'static str, Entity<InputState>>,
    example_textareas: BTreeMap<&'static str, Entity<TextareaState>>,
    search_scope: &'static str,
    country_code: &'static str,
    posted_comment: Option<SharedString>,
    submitted_custom: Option<SharedString>,
    saved_profile: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl InputGroupStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search components…"));
            let url = cx.new(|cx| InputState::new(window, cx).default_value("gpui-kit.com"));
            let amount = cx.new(|cx| InputState::new(window, cx).placeholder("0.00"));
            let password = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Enter password")
                    .masked(true)
            });
            let email = cx.new(|cx| InputState::new(window, cx).placeholder("you@example.com"));
            let disabled = cx.new(|cx| InputState::new(window, cx).default_value("Unavailable"));
            let disabled_invalid =
                cx.new(|cx| InputState::new(window, cx).default_value("Invalid saved value"));
            let readonly =
                cx.new(|cx| InputState::new(window, cx).default_value("https://gpui-kit.com"));
            let loading = cx.new(|cx| InputState::new(window, cx).placeholder("Searching…"));
            let shortcut = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
            let notes = cx.new(|cx| {
                TextareaState::new(window, cx)
                    .default_value("console.log('Hello, GPUI Kit!');")
                    .rows(4)
            });
            let message = cx.new(|cx| {
                TextareaState::new(window, cx)
                    .placeholder("Write a message…")
                    .auto_grow(2, 6)
            });
            let sizes = [Size::XSmall, Size::Small, Size::Medium, Size::Large]
                .into_iter()
                .map(|size| {
                    let state = cx.new(|cx| {
                        InputState::new(window, cx).placeholder(format!("{} input", size.as_str()))
                    });
                    (size, state)
                })
                .collect();
            let example_inputs = examples::create_inputs(window, cx);
            let example_textareas = examples::create_textareas(window, cx);
            let mut subscriptions = [&search, &url, &email]
                .into_iter()
                .map(|state| {
                    cx.subscribe(state, |_: &mut Self, _, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            cx.notify();
                        }
                    })
                })
                .collect::<Vec<_>>();
            subscriptions.push(cx.subscribe(&message, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }));
            subscriptions.push(
                cx.subscribe(&shortcut, |this, state, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.last_search = Some(state.read(cx).value());
                        cx.notify();
                    }
                }),
            );
            for state in example_inputs.values() {
                subscriptions.push(cx.subscribe(state, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                }));
            }
            for state in example_textareas.values() {
                subscriptions.push(cx.subscribe(state, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                }));
            }
            Self {
                focus_handle: cx.focus_handle(),
                search,
                url,
                amount,
                password,
                email,
                disabled,
                disabled_invalid,
                readonly,
                loading,
                shortcut,
                notes,
                message,
                sizes,
                copied: false,
                starred: false,
                runs: 0,
                attached: false,
                last_message: None,
                last_search: None,
                example_inputs,
                example_textareas,
                search_scope: "Documentation",
                country_code: "+1",
                posted_comment: None,
                submitted_custom: None,
                saved_profile: None,
                _subscriptions: subscriptions,
            }
        })
    }

    fn copy_url(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.readonly.read(cx).value().to_string(),
        ));
        self.copied = true;
        cx.notify();
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let message = self.message.read(cx).value();
        if message.trim().is_empty() || message.chars().count() > 280 {
            return;
        }
        self.last_message = Some(message);
        self.message.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        self.attached = false;
        cx.notify();
    }
}

impl Story for InputGroupStory {
    fn title() -> &'static str {
        "Input Group"
    }

    fn description() -> &'static str {
        "Compose inputs and textareas with icons, text, actions, and toolbars in one frame."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for InputGroupStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InputGroupStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search.read(cx).value().to_lowercase();
        let count = [
            "Button",
            "Input",
            "Textarea",
            "Input Group",
            "Select",
            "Combobox",
            "Checkbox",
            "Radio",
            "Slider",
            "Switch",
            "Calendar",
            "Color Picker",
        ]
        .into_iter()
        .filter(|name| name.to_lowercase().contains(&query))
        .count();
        let email = self.email.read(cx).value();
        let invalid = !email.is_empty() && (!email.contains('@') || !email.contains('.'));
        let message = self.message.read(cx).value();
        let characters = message.chars().count();
        let icon = |name| Icon::new(name).size_4();
        let column = || v_flex().w_full().max_w(rems(24.)).gap_4();
        v_flex().w_full().gap_4()
            .child(section("Default").child(
                InputGroup::new("input-group-search").max_w(rems(24.))
                    .input(InputGroupInput::new(&self.search).aria_label("Search components"))
                    .addon(InputGroupAddon::new("search-icon").child(icon(IconName::Search)))
                    .addon(InputGroupAddon::new("search-results").align(Align::InlineEnd)
                        .child(InputGroupText::new().child(format!("{count} results")))),
            ))
            .child(self.render_alignment())
            .child(self.render_icons())
            .child(section("Text").description("Leading and trailing text share the input frame.").child(
                column()
                    .child(InputGroup::new("input-group-url")
                        .input(InputGroupInput::new(&self.url).aria_label("Website").content_type(InputContentType::Url))
                        .addon(InputGroupAddon::new("url-scheme").child(InputGroupText::new().child("https://")))
                        .addon(InputGroupAddon::new("url-action").align(Align::InlineEnd)
                            .child(InputGroupButton::new("favorite-url")
                                .with_size(ButtonSize::IconXSmall).icon(IconName::Star)
                                .with_button(|button| button.accessibility_label("Favorite website").tooltip("Favorite website"))
                                .when(self.starred, |button| button.text_color(cx.theme().primary))
                                .on_click(cx.listener(|this, _, _, cx| { this.starred = !this.starred; cx.notify(); })))))
                    .child(InputGroup::new("input-group-amount")
                        .input(InputGroupInput::new(&self.amount).aria_label("Amount"))
                        .addon(InputGroupAddon::new("currency-symbol").child(InputGroupText::new().child("$")))
                        .addon(InputGroupAddon::new("currency-code").align(Align::InlineEnd).child(InputGroupText::new().child("USD"))))
                    .child(self.extra_input("domain", "Domain")
                        .addon(InputGroupAddon::new("domain-protocol").child(InputGroupText::new().child("https://")))
                        .addon(InputGroupAddon::new("domain-suffix").align(Align::InlineEnd).child(InputGroupText::new().child(".com"))))
                    .child(self.extra_input("username", "Work username")
                        .addon(InputGroupAddon::new("username-domain").align(Align::InlineEnd)
                            .child(InputGroupText::new().child("@company.com")))),
            ))
            .child(section("Buttons").description("Multiple native actions retain their own behavior and focus.").child(
                column()
                    .child(InputGroup::new("input-group-copy").readonly(true)
                        .input(InputGroupInput::new(&self.readonly).aria_label("Documentation URL"))
                        .addon(InputGroupAddon::new("copy-actions").align(Align::InlineEnd)
                            .child(InputGroupButton::new("copy-url").with_size(ButtonSize::IconXSmall)
                                .icon(if self.copied { IconName::Check } else { IconName::Copy })
                                .with_button(|button| button.accessibility_label("Copy URL").tooltip("Copy URL"))
                                .on_click(cx.listener(|this, _, window, cx| this.copy_url(window, cx))))
                            .child(InputGroupButton::new("select-url").label("Select all")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.readonly.update(cx, |state, cx| { state.focus(window, cx); state.select_all(window, cx); });
                                })))))
                    .child(InputGroup::new("input-group-password")
                        .input(InputGroupInput::new(&self.password).aria_label("Password").content_type(InputContentType::Password))
                        .addon(InputGroupAddon::new("password-action").align(Align::InlineEnd)
                            .child(InputGroupButton::new("toggle-password").with_size(ButtonSize::IconXSmall)
                                .icon(if self.password.read(cx).presentation().is_masked() { IconName::Eye } else { IconName::EyeOff })
                                .with_button(|button| button.accessibility_label("Toggle password visibility").tooltip("Toggle password visibility"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.password.update(cx, |state, cx| { state.toggle_masked(window, cx); });
                                    cx.notify();
                                }))))),
            ))
            .child(self.render_tooltips(cx))
            .child(self.render_dropdowns(cx))
            .child(self.render_popover(cx))
            .child(self.render_labels(cx))
            .child(self.render_button_actions(cx))
            .child(section("Keyboard shortcut and loading").child(
                column()
                    .child(InputGroup::new("input-group-shortcut")
                        .input(InputGroupInput::new(&self.shortcut).aria_label("Search"))
                        .addon(InputGroupAddon::new("shortcut-icon").child(icon(IconName::Search)))
                        .addon(InputGroupAddon::new("shortcut-key").align(Align::InlineEnd)
                            .child(Kbd::new(Keystroke::parse("enter").unwrap()))))
                    .when_some(self.last_search.clone(), |this, query| {
                        this.child(div().text_sm().text_color(cx.theme().muted_foreground)
                            .child(format!("Submitted search: {query}")))
                    })
                    .child(InputGroup::new("input-group-loading").readonly(true)
                        .input(InputGroupInput::new(&self.loading).aria_label("Search in progress"))
                        .addon(InputGroupAddon::new("loading-spinner").child(Spinner::new().small()))
                        .addon(InputGroupAddon::new("loading-text").align(Align::InlineEnd).child(InputGroupText::new().child("Please wait…")))),
            ))
            .child(self.render_loading())
            .child(section("Validation and disabled").child(
                column()
                    .child(v_flex().gap_2()
                        .child(InputGroup::new("input-group-email").invalid(invalid)
                            .input(InputGroupInput::new(&self.email).aria_label("Email address").content_type(InputContentType::EmailAddress))
                            .addon(InputGroupAddon::new("email-icon").child(icon(IconName::Info))))
                        .child(div().text_sm()
                            .text_color(if invalid { cx.theme().danger } else { cx.theme().muted_foreground })
                            .child(if invalid { "Enter an email address such as you@example.com." } else { "Validation comes from the application's field value." })))
                    .child(InputGroup::new("input-group-disabled").disabled(true)
                        .input(InputGroupInput::new(&self.disabled).aria_label("Disabled input"))
                        .addon(InputGroupAddon::new("disabled-icon").child(icon(IconName::Search)))
                        .addon(InputGroupAddon::new("disabled-action").align(Align::InlineEnd)
                            .child(InputGroupButton::new("disabled-search").label("Search"))))
                    .child(v_flex().gap_2()
                        .child(InputGroup::new("input-group-disabled-invalid").disabled(true).invalid(true)
                            .input(InputGroupInput::new(&self.disabled_invalid).aria_label("Disabled invalid input")))
                        .child(div().text_sm().text_color(cx.theme().danger)
                            .child("The error remains visible while editing is unavailable."))),
            ))
            .child(self.render_textarea_examples(cx))
            .child(section("Textarea toolbars").child(
                column()
                    .child(InputGroup::new("input-group-notes")
                        .input(InputGroupTextarea::new(&self.notes).aria_label("Script text"))
                        .addon(InputGroupAddon::new("notes-header").align(Align::BlockStart)
                            .border_b_1().border_color(cx.theme().border)
                            .child(InputGroupText::new().child(icon(IconName::File)).child("script.js"))
                            .child(InputGroupButton::new("copy-notes").ml_auto().with_size(ButtonSize::IconXSmall)
                                .icon(IconName::Copy).with_button(|button| button.accessibility_label("Copy script").tooltip("Copy script"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(this.notes.read(cx).value().to_string()));
                                }))))
                        .addon(InputGroupAddon::new("notes-footer").align(Align::BlockEnd)
                            .border_t_1().border_color(cx.theme().border)
                            .child(InputGroupText::new().child(format!("Run count: {}", self.runs)))
                            .child(InputGroupButton::new("run-notes").ml_auto().secondary().label("Run")
                                .on_click(cx.listener(|this, _, _, cx| { this.runs += 1; cx.notify(); }))))),
            ))
            .child(self.render_comment(cx))
            .child(self.render_custom_textarea(cx))
            .child(self.render_profile(cx))
            .child(section("Chat textarea").description("The retained textarea grows with the message; its actions remain below it.").child(
                column()
                    .child(InputGroup::new("input-group-message").invalid(characters > 280)
                        .input(InputGroupTextarea::new(&self.message).aria_label("Message"))
                        .when(self.attached, |group| group.addon(InputGroupAddon::new("message-attachment")
                            .align(Align::BlockStart).child(InputGroupText::new().child(icon(IconName::File)).child("notes.txt"))))
                        .addon(InputGroupAddon::new("message-actions").align(Align::BlockEnd)
                            .child(InputGroupText::new().child(format!("{characters}/280")))
                            .child(InputGroupButton::new("attach-message").ml_auto()
                                .with_size(ButtonSize::IconXSmall).icon(IconName::Plus)
                                .with_button(|button| button.accessibility_label("Toggle sample attachment").tooltip("Toggle sample attachment"))
                                .on_click(cx.listener(|this, _, _, cx| { this.attached = !this.attached; cx.notify(); })))
                            .child(InputGroupButton::new("send-message").primary().label("Send")
                                .disabled(message.trim().is_empty() || characters > 280)
                                .on_click(cx.listener(|this, _, window, cx| this.send(window, cx))))))
                    .when_some(self.last_message.clone(), |this, message| {
                        this.child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!("Sent: {message}")))
                    }),
            ))
            .child(section("Sizes").child(
                column().children(self.sizes.iter().map(|(size, state)| {
                    InputGroup::new(("input-group-size", state.entity_id())).with_size(*size)
                        .input(InputGroupInput::new(state).aria_label(format!("{} input", size.as_str())))
                        .addon(InputGroupAddon::new(("size-icon", state.entity_id())).child(icon(IconName::Search)))
                })),
            ))
    }
}

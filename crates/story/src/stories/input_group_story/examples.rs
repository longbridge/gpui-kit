use super::*;
use gpui_kit::component::{
    StyledExt as _,
    button::Button,
    form::{Field, Form},
    group_box::GroupBox,
    h_flex,
    input::Input,
    label::Label,
    menu::{DropdownMenu as _, PopupMenuItem},
    popover::Popover,
};

pub(super) fn create_inputs(
    window: &mut Window,
    cx: &mut Context<InputGroupStory>,
) -> BTreeMap<&'static str, Entity<InputState>> {
    [
        ("align-start", "Search…", ""),
        ("align-end", "Enter password", ""),
        ("align-top", "Enter your full name", ""),
        ("align-bottom", "0.00", ""),
        ("icon-email", "Enter your email", ""),
        ("icon-verified", "Username", "ada"),
        ("icon-multiple", "Website", "gpui-kit.com"),
        ("domain", "example", ""),
        ("username", "Enter your username", ""),
        ("tooltip-password", "Enter password", ""),
        ("tooltip-email", "Your email address", ""),
        ("dropdown-file", "Enter file name", "notes.txt"),
        ("dropdown-search", "Enter search query", ""),
        ("phone", "Phone number", ""),
        ("popover-url", "example.com", "gpui-kit.com"),
        ("label-username", "username", ""),
        ("label-email", "you@example.com", ""),
        ("button-actions", "Enter a project name", "Input Group"),
        ("loading-end", "Searching…", ""),
        ("loading-start", "Processing…", ""),
        ("loading-text", "Saving changes…", ""),
        ("profile-name", "Your name", "Ada Lovelace"),
        ("profile-email", "you@example.com", "ada@example.com"),
    ]
    .into_iter()
    .map(|(id, placeholder, value)| {
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(value)
                .masked(matches!(id, "align-end" | "tooltip-password"))
        });
        (id, state)
    })
    .collect()
}

pub(super) fn create_textareas(
    window: &mut Window,
    cx: &mut Context<InputGroupStory>,
) -> BTreeMap<&'static str, Entity<TextareaState>> {
    [
        ("textarea-plain", "Enter your text here…", ""),
        ("textarea-header", "Write your question…", ""),
        ("textarea-footer", "Enter your message", ""),
        ("textarea-disabled", "", "This textarea is disabled."),
        ("textarea-invalid", "Write a short summary…", ""),
        ("comment", "Share your thoughts…", ""),
        ("custom", "An automatically growing textarea…", ""),
    ]
    .into_iter()
    .map(|(id, placeholder, value)| {
        let state = cx.new(|cx| {
            let state = TextareaState::new(window, cx)
                .placeholder(placeholder)
                .default_value(value)
                .rows(3);
            if id == "custom" {
                state.auto_grow(1, 8)
            } else {
                state
            }
        });
        (id, state)
    })
    .collect()
}

fn column() -> impl IntoElement + gpui_kit::ParentElement + gpui_kit::Styled {
    v_flex().w_full().max_w(rems(24.)).gap_4()
}

fn labeled(label: &'static str, description: &'static str, control: impl IntoElement) -> Field {
    Field::new()
        .label(label)
        .description(description)
        .child(control)
}

fn compact_trigger(id: &'static str, label: impl Into<SharedString>) -> InputGroupButton {
    InputGroupButton::new(id).label(label)
}

impl InputGroupStory {
    pub(super) fn extra_input(&self, id: &'static str, label: &'static str) -> InputGroup {
        InputGroup::new(id).input(
            InputGroupInput::new(&self.example_inputs[id])
                .aria_label(label)
                .when(matches!(id, "align-end" | "tooltip-password"), |input| {
                    input.content_type(InputContentType::Password)
                }),
        )
    }

    fn extra_textarea(&self, id: &'static str, label: &'static str) -> InputGroup {
        InputGroup::new(id)
            .input(InputGroupTextarea::new(&self.example_textareas[id]).aria_label(label))
    }

    pub(super) fn render_alignment(&self) -> impl IntoElement {
        section("Alignment")
            .description("Place addons before, after, above, or below the control.")
            .child(
                column()
                    .child(labeled(
                        "Inline start",
                        "A leading search icon.",
                        self.extra_input("align-start", "Leading icon search")
                            .addon(
                                InputGroupAddon::new("align-start-addon")
                                    .child(Icon::new(IconName::Search).size_4()),
                            ),
                    ))
                    .child(labeled(
                        "Inline end",
                        "A trailing icon with a masked input.",
                        self.extra_input("align-end", "Trailing icon password")
                            .addon(
                                InputGroupAddon::new("align-end-addon")
                                    .align(Align::InlineEnd)
                                    .child(Icon::new(IconName::EyeOff).size_4()),
                            ),
                    ))
                    .child(labeled(
                        "Block start",
                        "The header is inside the shared frame.",
                        self.extra_input("align-top", "Full name").addon(
                            InputGroupAddon::new("align-top-addon")
                                .align(Align::BlockStart)
                                .child(InputGroupText::new().child("Full Name")),
                        ),
                    ))
                    .child(labeled(
                        "Block end",
                        "The unit sits below the single-line input.",
                        self.extra_input("align-bottom", "Amount with footer")
                            .addon(
                                InputGroupAddon::new("align-bottom-addon")
                                    .align(Align::BlockEnd)
                                    .child(InputGroupText::new().child("USD")),
                            ),
                    )),
            )
    }

    pub(super) fn render_icons(&self) -> impl IntoElement {
        section("Icons")
            .description("Leading, paired, and multiple trailing icons.")
            .child(
                column()
                    .child(
                        self.extra_input("icon-email", "Email with icon").addon(
                            InputGroupAddon::new("icon-email-addon")
                                .child(Icon::new(IconName::Inbox).size_4()),
                        ),
                    )
                    .child(
                        self.extra_input("icon-verified", "Verified username")
                            .addon(
                                InputGroupAddon::new("icon-user-addon")
                                    .child(Icon::new(IconName::User).size_4()),
                            )
                            .addon(
                                InputGroupAddon::new("icon-check-addon")
                                    .align(Align::InlineEnd)
                                    .child(Icon::new(IconName::Check).size_4()),
                            ),
                    )
                    .child(
                        self.extra_input("icon-multiple", "Website with multiple icons")
                            .addon(
                                InputGroupAddon::new("icon-multiple-addon")
                                    .align(Align::InlineEnd)
                                    .child(Icon::new(IconName::Star).size_4())
                                    .child(Icon::new(IconName::Info).size_4()),
                            ),
                    ),
            )
    }

    pub(super) fn render_tooltips(&self, _: &Context<Self>) -> impl IntoElement {
        section("Tooltips")
            .description("Compact help triggers keep the explanation beside its field.")
            .child(
                column().children(
                    [
                        (
                            "tooltip-password",
                            "Password help",
                            "Use at least 8 characters.",
                        ),
                        (
                            "tooltip-email",
                            "Email help",
                            "Used for notifications about this workspace.",
                        ),
                    ]
                    .map(|(id, label, help)| {
                        self.extra_input(id, label).addon(
                            InputGroupAddon::new(format!("tooltip-addon-{id}"))
                                .align(Align::InlineEnd)
                                .child(
                                    InputGroupButton::new(format!("tooltip-button-{id}"))
                                        .with_size(ButtonSize::IconXSmall)
                                        .icon(IconName::Info)
                                        .with_button(move |button| {
                                            button.accessibility_label(label).tooltip(help)
                                        }),
                                ),
                        )
                    }),
                ),
            )
    }

    pub(super) fn render_dropdowns(&self, cx: &Context<Self>) -> impl IntoElement {
        let file_view = cx.entity().downgrade();
        let scope_view = cx.entity().downgrade();
        let phone_view = cx.entity().downgrade();
        let scope = self.search_scope;
        let country = self.country_code;
        section("Dropdown menus").description("Menus act on the filename or choose the search scope and country code.")
            .child(column()
                .child(self.extra_input("dropdown-file", "File name")
                    .addon(InputGroupAddon::new("file-menu-addon").align(Align::InlineEnd)
                        .child(compact_trigger("file-menu", "More").dropdown_menu(move |menu, _, _| {
                            ["Copy filename", "Reset filename", "Clear filename"].into_iter().fold(menu, |menu, label| {
                                let view = file_view.clone();
                                menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        let state = &this.example_inputs["dropdown-file"];
                                        if label == "Copy filename" {
                                            cx.write_to_clipboard(ClipboardItem::new_string(state.read(cx).value().to_string()));
                                        } else {
                                            state.update(cx, |state, cx| {
                                                state.set_value(if label == "Reset filename" { "notes.txt" } else { "" }, window, cx);
                                            });
                                        }
                                        cx.notify();
                                    });
                                }))
                            })
                        }))))
                .child(self.extra_input("dropdown-search", "Scoped search")
                    .addon(InputGroupAddon::new("scope-menu-addon").align(Align::InlineEnd)
                        .child(compact_trigger("scope-menu", scope).icon(IconName::ChevronDown)
                            .dropdown_menu(move |menu, _, _| {
                                ["Documentation", "Blog posts", "Changelog"].into_iter().fold(menu, |menu, label| {
                                    let view = scope_view.clone();
                                    menu.item(PopupMenuItem::new(label).checked(label == scope).on_click(move |_, _, cx| {
                                        let _ = view.update(cx, |this, cx| { this.search_scope = label; cx.notify(); });
                                    }))
                                })
                            }))))
                .child(self.extra_input("phone", "Phone number")
                    .addon(InputGroupAddon::new("country-menu-addon")
                        .child(compact_trigger("country-menu", country).icon(IconName::ChevronDown)
                            .dropdown_menu(move |menu, _, _| {
                                ["+1", "+44", "+46"].into_iter().fold(menu, |menu, label| {
                                    let view = phone_view.clone();
                                    menu.item(PopupMenuItem::new(label).checked(label == country).on_click(move |_, _, cx| {
                                        let _ = view.update(cx, |this, cx| { this.country_code = label; cx.notify(); });
                                    }))
                                })
                            })))))
    }

    pub(super) fn render_popover(&self, cx: &Context<Self>) -> impl IntoElement {
        let address = self.example_inputs["popover-url"].read(cx).value();
        section("Popover").description("A native popover keeps contextual details attached to its trigger.")
            .child(column().child(self.extra_input("popover-url", "Website with details")
                .addon(InputGroupAddon::new("address-details-addon")
                    .child(Popover::new("address-details")
                        .trigger(InputGroupButton::new("address-details-trigger")
                            .with_size(ButtonSize::IconXSmall)
                            .icon(IconName::Info).aria_label("Address details").tooltip("Address details"))
                        .w(rems(18.)).gap_2().text_sm()
                        .child(div().font_semibold().child("Address details"))
                        .child(format!("https://{address}"))
                        .child("The protocol prefix stays separate from the editable hostname."))
                    .child(InputGroupText::new().child("https://")))))
    }

    pub(super) fn render_labels(&self, cx: &Context<Self>) -> impl IntoElement {
        section("Labels and descriptions").child(
            column()
                .child(labeled(
                    "Username",
                    "Clicking the @ addon focuses the input.",
                    self.extra_input("label-username", "Username with label")
                        .addon(InputGroupAddon::new("label-username-addon").child(Label::new("@"))),
                ))
                .child(
                    self.extra_input("label-email", "Notification email").addon(
                        InputGroupAddon::new("label-email-addon")
                            .align(Align::BlockStart)
                            .child(Label::new("Email").text_color(cx.theme().foreground))
                            .child(
                                InputGroupButton::new("label-email-help")
                                    .ml_auto()
                                    .with_size(ButtonSize::IconXSmall)
                                    .icon(IconName::Info)
                                    .with_button(|button| {
                                        button.accessibility_label("Notification email help")
                            .tooltip("We'll use this address for workspace notifications.")
                                    }),
                            ),
                    ),
                ),
        )
    }

    pub(super) fn render_button_actions(&self, cx: &Context<Self>) -> impl IntoElement {
        section("Text and icon actions")
            .description(
                "Use larger buttons for a clear text action; keep secondary actions compact.",
            )
            .child(
                column().child(
                    self.extra_input("button-actions", "Project name").addon(
                        InputGroupAddon::new("project-actions")
                            .align(Align::BlockEnd)
                            .child(
                                InputGroupButton::new("project-clear")
                                    .with_size(ButtonSize::Small)
                                    .label("Clear")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.example_inputs["button-actions"]
                                            .update(cx, |state, cx| {
                                                state.set_value("", window, cx)
                                            });
                                    })),
                            )
                            .child(
                                InputGroupButton::new("project-reset")
                                    .with_size(ButtonSize::Small)
                                    .secondary()
                                    .label("Reset")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.example_inputs["button-actions"]
                                            .update(cx, |state, cx| {
                                                state.set_value("Input Group", window, cx)
                                            });
                                    })),
                            )
                            .child(
                                InputGroupButton::new("project-copy")
                                    .ml_auto()
                                    .with_size(ButtonSize::IconSmall)
                                    .icon(IconName::Copy)
                                    .with_button(|button| {
                                        button
                                            .accessibility_label("Copy project name")
                                            .tooltip("Copy project name")
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            this.example_inputs["button-actions"]
                                                .read(cx)
                                                .value()
                                                .to_string(),
                                        ));
                                    })),
                            ),
                    ),
                ),
            )
    }

    pub(super) fn render_loading(&self) -> impl IntoElement {
        section("Spinner placement")
            .description("Progress can lead, trail, or sit next to status text.")
            .child(
                column()
                    .child(
                        self.extra_input("loading-end", "Search loading state")
                            .readonly(true)
                            .addon(
                                InputGroupAddon::new("spinner-end")
                                    .align(Align::InlineEnd)
                                    .child(Spinner::new().small()),
                            ),
                    )
                    .child(
                        self.extra_input("loading-start", "Processing loading state")
                            .readonly(true)
                            .addon(
                                InputGroupAddon::new("spinner-start").child(Spinner::new().small()),
                            ),
                    )
                    .child(
                        self.extra_input("loading-text", "Saving loading state")
                            .readonly(true)
                            .addon(
                                InputGroupAddon::new("spinner-text")
                                    .align(Align::InlineEnd)
                                    .child(InputGroupText::new().child("Saving…"))
                                    .child(Spinner::new().small()),
                            ),
                    ),
            )
    }

    pub(super) fn render_textarea_examples(&self, cx: &Context<Self>) -> impl IntoElement {
        let remaining = 120isize
            - self.example_textareas["textarea-footer"]
                .read(cx)
                .value()
                .chars()
                .count() as isize;
        let summary = self.example_textareas["textarea-invalid"].read(cx).value();
        section("Textarea variants").child(
            column()
                .child(labeled(
                    "Without addons",
                    "The text viewport owns wrapping and scrolling.",
                    self.extra_textarea("textarea-plain", "Plain grouped textarea"),
                ))
                .child(
                    self.extra_textarea("textarea-header", "Textarea with header")
                        .addon(
                            InputGroupAddon::new("textarea-header-addon")
                                .align(Align::BlockStart)
                                .child(InputGroupText::new().child("Ask, search, or chat…")),
                        ),
                )
                .child(
                    self.extra_textarea("textarea-footer", "Textarea with remaining count")
                        .invalid(remaining < 0)
                        .addon(
                            InputGroupAddon::new("textarea-footer-addon")
                                .align(Align::BlockEnd)
                                .child(
                                    InputGroupText::new()
                                        .child(format!("{remaining} characters left")),
                                ),
                        ),
                )
                .child(labeled(
                    "Invalid",
                    "Enter a summary to clear the error.",
                    self.extra_textarea("textarea-invalid", "Required summary")
                        .invalid(summary.trim().is_empty()),
                ))
                .child(labeled(
                    "Disabled",
                    "Text and addon actions are unavailable.",
                    self.extra_textarea("textarea-disabled", "Disabled textarea")
                        .disabled(true)
                        .addon(
                            InputGroupAddon::new("textarea-disabled-addon")
                                .align(Align::BlockEnd)
                                .child(
                                    InputGroupButton::new("textarea-disabled-post").label("Post"),
                                ),
                        ),
                )),
        )
    }

    pub(super) fn render_comment(&self, cx: &Context<Self>) -> impl IntoElement {
        let value = self.example_textareas["comment"].read(cx).value();
        section("Comment composer")
            .description(
                "Cancel clears the draft; Post keeps the submitted text below the composer.",
            )
            .child(
                column()
                    .child(
                        self.extra_textarea("comment", "Comment draft").addon(
                            InputGroupAddon::new("comment-actions")
                                .align(Align::BlockEnd)
                                .child(
                                    InputGroupText::new()
                                        .child(format!("{} characters", value.chars().count())),
                                )
                                .child(
                                    InputGroupButton::new("comment-cancel")
                                        .ml_auto()
                                        .with_size(ButtonSize::Small)
                                        .label("Cancel")
                                        .disabled(value.is_empty())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.clear_comment(window, cx)
                                        })),
                                )
                                .child(
                                    InputGroupButton::new("comment-post")
                                        .with_size(ButtonSize::Small)
                                        .primary()
                                        .label("Post")
                                        .disabled(value.trim().is_empty())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.posted_comment = Some(
                                                this.example_textareas["comment"].read(cx).value(),
                                            );
                                            this.clear_comment(window, cx);
                                        })),
                                ),
                        ),
                    )
                    .when_some(self.posted_comment.clone(), |this, value| {
                        this.child(div().text_sm().child(format!("Posted: {value}")))
                    }),
            )
    }

    fn clear_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.example_textareas["comment"].update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn render_custom_textarea(&self, cx: &Context<Self>) -> impl IntoElement {
        let state = &self.example_textareas["custom"];
        section("Auto-growing textarea").description("Editor padding and typography, the focused frame, and the submit label and icon each have their own style overrides.")
            .child(column()
                .child(InputGroup::new("custom")
                    .focused_style(|style| style.border_color(cx.theme().primary))
                    .input(InputGroupTextarea::new(state).aria_label("Auto-growing draft")
                        .editor_style(|style| style.p_3().text_base().font_family(cx.theme().mono_font_family.clone())))
                    .addon(InputGroupAddon::new("custom-footer").align(Align::BlockEnd)
                        .child(InputGroupText::new().child("Plain text"))
                        .child(InputGroupButton::new("custom-submit").ml_auto().primary().label("Submit")
                            .icon(IconName::ArrowUp)
                            .label_style(|style| style.font_semibold())
                            .icon_style(|style| style.size_4())
                            .disabled(state.read(cx).value().trim().is_empty())
                            .on_click(cx.listener(|this, _, window, cx| {
                                let state = &this.example_textareas["custom"];
                                this.submitted_custom = Some(state.read(cx).value());
                                state.update(cx, |state, cx| state.set_value("", window, cx));
                                cx.notify();
                            })))))
                .when_some(self.submitted_custom.clone(), |this, value| this.child(div().text_sm().child(format!("Submitted: {value}")))))
    }

    pub(super) fn render_profile(&self, cx: &Context<Self>) -> impl IntoElement {
        section("Form composition").description("Use Field and GroupBox to keep labels, descriptions, and the save action together.")
            .child(column()
                .child(GroupBox::new().title("Contact details")
                    .child(Form::new()
                        .child(Field::new().label("Display name").child(Input::new(&self.example_inputs["profile-name"]).aria_label("Profile display name")))
                        .child(Field::new().label("Email").description("Shown in this example only.")
                            .child(self.extra_input("profile-email", "Profile email")
                                .addon(InputGroupAddon::new("profile-email-icon").child(Icon::new(IconName::Inbox).size_4()))))
                        .footer(h_flex().justify_end().child(Button::new("profile-save").primary().label("Save contact")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.saved_profile = Some(format!("{} — {}", this.example_inputs["profile-name"].read(cx).value(), this.example_inputs["profile-email"].read(cx).value()).into());
                                cx.notify();
                            }))))))
                .when_some(self.saved_profile.clone(), |this, value| this.child(div().text_sm().child(format!("Saved: {value}")))))
    }
}

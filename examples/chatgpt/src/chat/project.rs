//! Projects: a page listing a project's chats and instructions, and the
//! dialog that creates or edits one.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
    form::{field, v_form},
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::{ChatView, Page, READING_WIDTH};
use crate::store::{Project, new_id};

impl ChatView {
    pub(super) fn open_project(&mut self, id: &str, cx: &mut Context<Self>) {
        self.stop(cx);
        self.page = Page::Project(id.to_string());
        // The composer on a project page starts a chat inside it.
        self.current = None;
        self.new_chat_project = Some(id.to_string());
        self.messages.clear();
        self.usage = None;
        self.scroller.update(cx, |state, cx| state.reset(0, cx));
        cx.notify();
    }

    /// Create a project, or edit the one with `id`.
    pub(super) fn open_project_dialog(
        &mut self,
        id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let existing = id
            .as_ref()
            .and_then(|id| self.projects.iter().find(|project| &project.id == id))
            .cloned();
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Project name")
                .default_value(
                    existing
                        .as_ref()
                        .map(|p| p.name.clone())
                        .unwrap_or_default(),
                )
        });
        let instructions = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(4, 10)
                .placeholder("How ChatGPT should respond in this project")
                .default_value(
                    existing
                        .as_ref()
                        .map(|p| p.instructions.clone())
                        .unwrap_or_default(),
                )
        });
        name.update(cx, |state, cx| state.focus(window, cx));

        let view = cx.entity().downgrade();
        let editing = existing.is_some();
        window.open_dialog(cx, move |dialog, _, _| {
            let (view, name, instructions, id) =
                (view.clone(), name.clone(), instructions.clone(), id.clone());
            dialog
                .title(if editing {
                    "Edit project"
                } else {
                    "New project"
                })
                .w(px(480.))
                .child(
                    v_form()
                        .child(field().label("Name").child(Input::new(&name)))
                        .child(
                            field()
                                .label("Instructions")
                                .description("Added to every chat in this project.")
                                .child(Textarea::new(&instructions)),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().child(Button::new("cancel").label("Cancel")))
                        .child(DialogAction::new().child(
                            Button::new("save").primary().label(if editing {
                                "Save"
                            } else {
                                "Create project"
                            }),
                        )),
                )
                .on_ok(move |_, _, cx| {
                    let name_value = name.read(cx).value().trim().to_string();
                    if name_value.is_empty() {
                        return false;
                    }
                    let instructions_value = instructions.read(cx).value().to_string();
                    _ = view.update(cx, |this, cx| {
                        this.save_project(id.clone(), name_value, instructions_value, cx)
                    });
                    true
                })
        });
    }

    fn save_project(
        &mut self,
        id: Option<String>,
        name: String,
        instructions: String,
        cx: &mut Context<Self>,
    ) {
        let id = match id.and_then(|id| self.projects.iter_mut().find(|p| p.id == id)) {
            Some(project) => {
                project.name = name;
                project.instructions = instructions;
                project.id.clone()
            }
            None => {
                let project = Project {
                    id: new_id(),
                    name,
                    instructions,
                };
                let id = project.id.clone();
                self.projects.push(project);
                id
            }
        };
        if let Some(store) = &self.store {
            _ = store.save_projects(&self.projects);
        }
        self.open_project(&id, cx);
    }

    fn confirm_delete_project(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.projects.iter().find(|p| p.id == id).cloned() else {
            return;
        };
        let view = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, cx| {
            let (view, id) = (view.clone(), project.id.clone());
            alert
                .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
                .title(format!("Delete “{}”?", project.name))
                .description("Its chats move back to your chat history.")
                .confirm()
                .ok_text("Delete project")
                .ok_variant(ButtonVariant::Danger)
                .on_ok(move |_, _, cx| {
                    _ = view.update(cx, |this, cx| this.delete_project(&id, cx));
                    true
                })
        });
    }

    fn delete_project(&mut self, id: &str, cx: &mut Context<Self>) {
        self.projects.retain(|project| project.id != id);
        let moved: Vec<String> = self
            .conversations
            .iter()
            .filter(|c| c.project_id.as_deref() == Some(id))
            .map(|c| c.id.clone())
            .collect();
        for conversation in moved {
            self.move_conversation(&conversation, None, cx);
        }
        if let Some(store) = &self.store {
            _ = store.save_projects(&self.projects);
        }
        self.new_chat(None, cx);
    }

    pub(super) fn render_project_page(&self, id: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let Some(project) = self.projects.iter().find(|p| p.id == id) else {
            return div().into_any_element();
        };
        let chats: Vec<_> = self
            .conversations
            .iter()
            .filter(|c| c.project_id.as_deref() == Some(id))
            .collect();
        let edit_id = project.id.clone();
        let delete_id = project.id.clone();

        div()
            .id("project-page")
            .size_full()
            .overflow_y_scroll()
            .child(
                v_flex()
                    .w_full()
                    .max_w(rems(READING_WIDTH))
                    .mx_auto()
                    .px_6()
                    .py_8()
                    .gap_6()
                    .child(
                        h_flex()
                            .gap_3()
                            .child(Icon::new(IconName::Folder).text_color(theme.muted_foreground))
                            .child(
                                div()
                                    .flex_1()
                                    .text_2xl()
                                    .font_semibold()
                                    .child(project.name.clone()),
                            )
                            .child(Button::new("edit-project").small().label("Edit…").on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.open_project_dialog(Some(edit_id.clone()), window, cx)
                                }),
                            ))
                            .child(
                                Button::new("delete-project")
                                    .small()
                                    .ghost()
                                    .label("Delete…")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.confirm_delete_project(&delete_id, window, cx)
                                    })),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(theme.muted_foreground)
                                    .child("Instructions"),
                            )
                            .child(div().text_sm().map(|this| {
                                if project.instructions.trim().is_empty() {
                                    this.text_color(theme.muted_foreground)
                                        .child("None yet. Add some with Edit….")
                                } else {
                                    this.child(project.instructions.clone())
                                }
                            })),
                    )
                    .child(
                        v_flex()
                            .items_start()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(theme.muted_foreground)
                                    .child("Chats"),
                            )
                            .when(chats.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("Start a chat below; it is saved in this project."),
                                )
                            })
                            .children(chats.into_iter().map(|conversation| {
                                let id = conversation.id.clone();
                                Button::new(SharedString::from(format!("project-chat-{id}")))
                                    .ghost()
                                    .icon(Icon::new(gpui_kit::assets::IconName::MessageSquare))
                                    .label(conversation.title.clone())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_conversation(&id, cx)
                                    }))
                            })),
                    ),
            )
            .into_any_element()
    }
}

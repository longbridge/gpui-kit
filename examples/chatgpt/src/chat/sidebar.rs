//! The history sidebar: new chat, search, projects, chats by date and the
//! account.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::Input,
    menu::{DropdownMenu as _, PopupMenuItem},
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
        SidebarMenuItem,
    },
    v_flex,
};
use gpui_kit::{assets::IconName as LucideIcon, *};

use super::{ChatView, Page, Session};
use crate::store::{Age, Conversation, now};

impl ChatView {
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search.read(cx).value().to_lowercase();
        let matches = |conversation: &&Conversation| {
            query.is_empty() || conversation.title.to_lowercase().contains(&query)
        };
        let now = now();

        let mut groups: Vec<SidebarGroup<SidebarMenu>> = Vec::new();
        if query.is_empty() {
            groups.push(
                SidebarGroup::new("Projects").child(
                    SidebarMenu::new()
                        .children(self.projects.iter().map(|project| {
                            let id = project.id.clone();
                            SidebarMenuItem::new(project.name.clone())
                                .icon(IconName::Folder)
                                .active(self.page == Page::Project(project.id.clone()))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.open_project(&id, cx)),
                                )
                        }))
                        .child(
                            SidebarMenuItem::new("New project…")
                                .icon(IconName::Plus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_project_dialog(None, window, cx)
                                })),
                        ),
                ),
            );
        }

        // Chats in a project live on the project's page; search finds all.
        let chats: Vec<&Conversation> = self
            .conversations
            .iter()
            .filter(|conversation| !query.is_empty() || conversation.project_id.is_none())
            .filter(matches)
            .collect();
        if query.is_empty() {
            for age in [Age::Today, Age::Yesterday, Age::PreviousWeek, Age::Older] {
                let items: Vec<SidebarMenuItem> = chats
                    .iter()
                    .filter(|conversation| Age::of(conversation.updated_at, now) == age)
                    .map(|conversation| self.chat_item(conversation, cx))
                    .collect();
                if !items.is_empty() {
                    groups.push(
                        SidebarGroup::new(age.label()).child(SidebarMenu::new().children(items)),
                    );
                }
            }
        } else {
            groups.push(
                SidebarGroup::new(if chats.is_empty() {
                    "No results"
                } else {
                    "Results"
                })
                .child(
                    SidebarMenu::new().children(
                        chats
                            .iter()
                            .map(|conversation| self.chat_item(conversation, cx)),
                    ),
                ),
            );
        }

        Sidebar::new("history")
            .collapsible(SidebarCollapsible::Offcanvas)
            .collapsed(self.sidebar_collapsed)
            .w(rems(16.))
            .header(
                SidebarHeader::new().child(
                    v_flex()
                        .w_full()
                        .items_start()
                        .gap_2()
                        .child(
                            Button::new("new-chat")
                                .ghost()
                                .small()
                                .icon(Icon::new(LucideIcon::SquarePen))
                                .label("New chat")
                                .on_click(cx.listener(|this, _, _, cx| this.new_chat(None, cx))),
                        )
                        .child(
                            Input::new(&self.search)
                                .small()
                                .w_full()
                                .prefix(Icon::new(IconName::Search).xsmall())
                                .cleanable(true),
                        ),
                ),
            )
            .children(groups)
            .footer(SidebarFooter::new().child(self.render_account(cx)))
    }

    fn chat_item(&self, conversation: &Conversation, cx: &mut Context<Self>) -> SidebarMenuItem {
        let id = conversation.id.clone();
        let view = cx.entity().downgrade();
        let projects: Vec<(String, String)> = self
            .projects
            .iter()
            .filter(|project| Some(&project.id) != conversation.project_id.as_ref())
            .map(|project| (project.id.clone(), project.name.clone()))
            .collect();
        let in_project = conversation.project_id.is_some();

        SidebarMenuItem::new(conversation.title.clone())
            .active(self.page == Page::Chat && self.current.as_deref() == Some(&conversation.id))
            .on_click(cx.listener({
                let id = id.clone();
                move |this, _, _, cx| this.open_conversation(&id, cx)
            }))
            .context_menu(move |menu, _, _| {
                let menu = projects.iter().fold(menu, |menu, (project_id, name)| {
                    let (view, id, project_id) = (view.clone(), id.clone(), project_id.clone());
                    menu.item(PopupMenuItem::new(format!("Move to {name}")).on_click(
                        move |_, _, cx| {
                            _ = view.update(cx, |this, cx| {
                                this.move_conversation(&id, Some(project_id.clone()), cx)
                            });
                        },
                    ))
                });
                let menu = if in_project {
                    let (view, id) = (view.clone(), id.clone());
                    menu.item(PopupMenuItem::new("Remove from project").on_click(
                        move |_, _, cx| {
                            _ = view.update(cx, |this, cx| this.move_conversation(&id, None, cx));
                        },
                    ))
                } else {
                    menu
                };
                let (view, id) = (view.clone(), id.clone());
                menu.separator()
                    .item(
                        PopupMenuItem::new("Delete…").on_click(move |_, window, cx| {
                            _ = view.update(cx, |this, cx| this.confirm_delete(&id, window, cx));
                        }),
                    )
            })
    }

    fn render_account(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (account, signed_in_with_chatgpt): (SharedString, bool) = match &self.session {
            Session::ChatGpt(credentials) => (
                credentials
                    .email()
                    .unwrap_or("ChatGPT account")
                    .to_string()
                    .into(),
                true,
            ),
            _ => ("API key".into(), false),
        };
        let view = cx.entity().downgrade();
        let button = Button::new("account")
            .ghost()
            .small()
            .w_full()
            .icon(IconName::CircleUser)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(account),
            );

        if signed_in_with_chatgpt {
            button
                .dropdown_caret(true)
                .dropdown_menu_with_anchor(Anchor::BottomLeft, move |menu, _, _| {
                    let view = view.clone();
                    menu.item(PopupMenuItem::new("Sign out").on_click(move |_, _, cx| {
                        _ = view.update(cx, |this, cx| this.sign_out(cx));
                    }))
                })
                .into_any_element()
        } else {
            button.into_any_element()
        }
    }

    pub(super) fn move_conversation(
        &mut self,
        id: &str,
        project_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            conversation.project_id = project_id.clone();
            if let Some(store) = &self.store {
                _ = store.save_conversation(conversation);
            }
        }
        if self.current.as_deref() == Some(id) {
            self.new_chat_project = project_id;
        }
        cx.notify();
    }

    pub(super) fn confirm_delete(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(title) = self
            .conversations
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.title.clone())
        else {
            return;
        };
        let view = cx.entity().downgrade();
        let id = id.to_string();
        window.open_alert_dialog(cx, move |alert, _, cx| {
            let (view, id) = (view.clone(), id.clone());
            alert
                .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
                .title(format!("Delete “{title}”?"))
                .description("The conversation and its images are removed from this computer.")
                .confirm()
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .on_ok(move |_, _, cx| {
                    _ = view.update(cx, |this, cx| this.delete_conversation(&id, cx));
                    true
                })
        });
    }

    fn delete_conversation(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(ix) = self.conversations.iter().position(|c| c.id == id) else {
            return;
        };
        let conversation = self.conversations.remove(ix);
        if let Some(store) = &self.store {
            store.delete_conversation(&conversation);
        }
        if self.current.as_deref() == Some(id) {
            self.new_chat(conversation.project_id, cx);
        }
        cx.notify();
    }
}

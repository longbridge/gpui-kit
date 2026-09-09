use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme, Icon, IconName, StyledExt, WindowExt as _,
    button::{Button, ButtonVariants as _},
    date_picker::{DatePicker, DatePickerState},
    dialog::{
        Dialog, DialogAction, DialogClose, DialogDescription, DialogFooter, DialogHeader,
        DialogTitle,
    },
    h_flex,
    input::{Input, InputState},
    select::{Select, SelectState},
    table::{Column, DataTable, TableDelegate, TableState},
    text::{TextView, markdown},
    v_flex,
};
use serde::Deserialize;

use crate::{TestAction, section, story_toolbar_group};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = dialog_story, no_json)]
enum ToggleDialogOption {
    Overlay,
    OverlayClosable,
    CloseButton,
    Keyboard,
}

pub struct DialogStory {
    focus_handle: FocusHandle,
    selected_value: Option<SharedString>,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    date: Entity<DatePickerState>,
    select: Entity<SelectState<Vec<String>>>,
    table: Entity<TableState<MyTable>>,
    dialog_overlay: bool,
    close_button: bool,
    keyboard: bool,
    overlay_closable: bool,
}

struct MyTable {
    columns: Vec<Column>,
}
impl MyTable {
    fn new(_: &mut App) -> Self {
        let columns = vec![
            Column::new("id", "ID").width(px(50.)),
            Column::new("name", "Name").width(px(150.)),
            Column::new("email", "Email").width(px(250.)),
            Column::new("role", "Role").width(px(150.)),
            Column::new("status", "Status").width(px(100.)),
        ];

        Self { columns }
    }
}
impl TableDelegate for MyTable {
    fn columns_count(&self, _: &App) -> usize {
        5
    }

    fn rows_count(&self, _: &App) -> usize {
        200
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        match col_ix {
            0 => format!("{}", row_ix).into_any_element(),
            1 => format!("User {}", row_ix).into_any_element(),
            2 => format!("user-{}@mail.com", row_ix).into_any_element(),
            3 => "User".into_any_element(),
            4 => "Active".into_any_element(),
            _ => panic!("Invalid column index"),
        }
    }
}

impl super::Story for DialogStory {
    fn title() -> &'static str {
        "Dialog"
    }

    fn description() -> &'static str {
        "Present focused content above the current view."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DialogStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
        let input2 =
            cx.new(|cx| InputState::new(window, cx).placeholder("Type before opening a dialog"));
        let date = cx.new(|cx| DatePickerState::new(window, cx));
        let select = cx.new(|cx| {
            SelectState::new(
                vec![
                    "Option 1".to_string(),
                    "Option 2".to_string(),
                    "Option 3".to_string(),
                ],
                None,
                window,
                cx,
            )
        });

        let table = cx.new(|cx| TableState::new(MyTable::new(cx), window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            selected_value: None,
            input1,
            input2,
            date,
            select,
            dialog_overlay: true,
            close_button: true,
            keyboard: true,
            overlay_closable: true,
            table,
        }
    }

    fn on_action_test_action(
        &mut self,
        _: &TestAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.push_notification("You have clicked the TestAction.", cx);
    }

    fn render_basic_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;
        let input1 = self.input1.clone();
        let date = self.date.clone();
        let select = self.select.clone();
        let view = cx.entity();

        section("Default")
            .description("Compose form controls and footer actions.")
            .child(
                Dialog::new(cx)
                    .trigger(Button::new("show-dialog").outline().label("Open Dialog"))
                    .overlay(dialog_overlay)
                    .keyboard(self.keyboard)
                    .close_button(self.close_button)
                    .overlay_closable(overlay_closable)
                    .on_ok({
                        let view = view.clone();
                        let input1 = input1.clone();
                        let date = date.clone();
                        move |_, window, cx| {
                            view.update(cx, |view, cx| {
                                view.selected_value = Some(
                                    format!(
                                        "Hello, {}, date: {}",
                                        input1.read(cx).value(),
                                        date.read(cx).date()
                                    )
                                    .into(),
                                )
                            });
                            window.push_notification("You have pressed confirm.", cx);
                            true
                        }
                    })
                    .p_0()
                    .content({
                        move |content, _, cx| {
                            content
                                .child(
                                    DialogHeader::new()
                                        .p_4()
                                        .child(DialogTitle::new().child("Basic Dialog"))
                                        .child(DialogDescription::new().child(
                                            "This is a basic dialog created \
                                        using the declarative API.",
                                        )),
                                )
                                .child(
                                    v_flex()
                                        .px_4()
                                        .pb_4()
                                        .gap_3()
                                        .child(
                                            "This is a dialog dialog, \
                                        you can put anything here.",
                                        )
                                        .child(Input::new(&input1))
                                        .child(Select::new(&select))
                                        .child(DatePicker::new(&date).placeholder("Date of Birth")),
                                )
                                .child(
                                    DialogFooter::new()
                                        .p_4()
                                        .bg(cx.theme().muted)
                                        .justify_between()
                                        .child(
                                            Button::new("new-dialog")
                                                .label("Open Other Dialog")
                                                .outline()
                                                .on_click(move |_, window, cx| {
                                                    window.open_dialog(cx, move |dialog, _, _| {
                                                        dialog
                                                            .title("Other Dialog")
                                                            .child("This is another dialog.")
                                                            .min_h(px(100.))
                                                            .overlay_closable(overlay_closable)
                                                    });
                                                }),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(DialogClose::new().child(
                                                    Button::new("cancel").label("Cancel").outline(),
                                                ))
                                                .child(
                                                    DialogAction::new().child(
                                                        Button::new("confirm")
                                                            .primary()
                                                            .label("Confirm"),
                                                    ),
                                                ),
                                        ),
                                )
                        }
                    }),
            )
    }

    fn render_focus_return_check(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex().w_full().justify_center().child(
            v_flex()
                .w_full()
                .max_w_96()
                .gap_3()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().muted.opacity(0.45))
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().font_medium().child("Focus return check"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Type here, then open and close any dialog."),
                        ),
                )
                .child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .child(Input::new(&self.input2).w_full()),
                        )
                        .child(
                            Button::new("test-action")
                                .outline()
                                .label("Run Action")
                                .flex_shrink_0()
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(Box::new(TestAction), cx);
                                })
                                .tooltip("Verify actions still dispatch after a dialog closes."),
                        ),
                ),
        )
    }

    fn render_dialog_without_title(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("Without title")
            .description("Render content without a heading.")
            .child(
                Button::new("dialog-no-title")
                    .outline()
                    .label("Dialog without Title")
                    .on_click(cx.listener(move |_, _, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog
                                .overlay(dialog_overlay)
                                .overlay_closable(overlay_closable)
                                .child(
                                    "This is a dialog without title, \
                                you can use it when the title is not necessary.",
                                )
                        });
                    })),
            )
    }

    fn render_custom_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("Custom actions")
            .description("Replace the default footer actions.")
            .child(
                Button::new("confirm-dialog1")
                    .outline()
                    .label("Custom Buttons")
                    .on_click(cx.listener(move |_, _, window, cx| {
                        window.open_dialog(cx, move |dialog, _, cx| {
                            dialog
                                .rounded(cx.theme().radius_lg)
                                .overlay(dialog_overlay)
                                .overlay_closable(overlay_closable)
                                .child(
                                    v_flex()
                                        .gap_3()
                                        .items_center()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(cx.theme().radius_lg)
                                                .bg(cx.theme().warning.opacity(0.2))
                                                .size_12()
                                                .text_color(cx.theme().warning)
                                                .child(Icon::new(IconName::TriangleAlert).size_8()),
                                        )
                                        .child(
                                            "Update successful, \
                                        we need to restart the application.",
                                        ),
                                )
                                .footer(
                                    DialogFooter::new()
                                        .child(
                                            DialogClose::new().child(
                                                Button::new("cancel").label("Later").outline(),
                                            ),
                                        )
                                        .child(DialogAction::new().child(
                                            Button::new("ok").label("Restart Now").primary(),
                                        )),
                                )
                                .on_ok(|_, window, cx| {
                                    window.push_notification("You have pressed restart.", cx);
                                    true
                                })
                                .on_cancel(|_, window, cx| {
                                    window.push_notification("You have pressed later.", cx);
                                    true
                                })
                        });
                    })),
            )
    }

    fn render_scrollable_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("Scrollable")
            .description("Keep long content inside a fixed dialog size.")
            .child(
                Button::new("scrollable-dialog")
                    .outline()
                    .label("Scrollable Dialog")
                    .on_click(cx.listener(move |_, _, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog
                                .w(px(720.))
                                .h(px(600.))
                                .overlay(dialog_overlay)
                                .overlay_closable(overlay_closable)
                                .title("Dialog with scrollbar")
                                .child(markdown(include_str!("../../../../README.md")))
                                .footer(
                                    DialogFooter::new()
                                        .gap_2()
                                        .child(
                                            DialogClose::new().child(
                                                Button::new("cancel").label("Cancel").outline(),
                                            ),
                                        )
                                        .child(DialogAction::new().child(
                                            Button::new("confirm").label("Confirm").primary(),
                                        )),
                                )
                        });
                    })),
            )
    }

    fn render_table_in_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("Data table")
            .description("Embed a full interactive component.")
            .child(
                Button::new("table-dialog")
                    .outline()
                    .label("Table Dialog")
                    .on_click(cx.listener({
                        move |this, _, window, cx| {
                            window.open_dialog(cx, {
                                let table = this.table.clone();
                                move |dialog, _, _| {
                                    dialog
                                        .w(px(800.))
                                        .h(px(600.))
                                        .overlay(dialog_overlay)
                                        .overlay_closable(overlay_closable)
                                        .title("Dialog with Table")
                                        .child(
                                            v_flex()
                                                .size_full()
                                                .gap_3()
                                                .child(
                                                    "This is a dialog contains a table component.",
                                                )
                                                .child(DataTable::new(&table)),
                                        )
                                }
                            });
                        }
                    })),
            )
    }

    fn render_custom_paddings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Padding")
            .description("Control spacing around dialog content.")
            .child(
                Button::new("custom-dialog-paddings")
                    .outline()
                    .label("Custom Paddings")
                    .on_click(cx.listener(move |_, _, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog.p_3().title("Custom Dialog Title").child(
                                "This is a custom dialog content, we can use \
                            paddings to control the layout and spacing within \
                            the dialog.",
                            )
                        });
                    })),
            )
    }

    fn render_custom_style(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Custom style")
            .description("Customize color, radius, and foreground.")
            .child(
                Button::new("custom-dialog-style")
                    .outline()
                    .label("Custom Dialog Style")
                    .on_click(cx.listener(move |_, _, window, cx| {
                        window.open_dialog(cx, move |dialog, _, cx| {
                            dialog
                                .rounded(cx.theme().radius_lg)
                                .bg(cx.theme().cyan)
                                .text_color(cx.theme().info_foreground)
                                .title("Custom Dialog Title")
                                .child("This is a custom dialog content.")
                        });
                    })),
            )
    }

    fn render_dialog_with_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Custom content")
            .description("Compose header, body, and footer explicitly.")
            .child(
                Button::new("custom-width-dialog-btn")
                    .outline()
                    .label("Custom Width (400px)")
                    .on_click(cx.listener(move |_, _, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog.w(px(400.)).content(|content, _, _| {
                                content
                                    .child(
                                        DialogHeader::new()
                                            .child(DialogTitle::new().child("Custom Width"))
                                            .child(
                                                DialogDescription::new().child(
                                                    "This dialog has a custom width of 400px.",
                                                ),
                                            ),
                                    )
                                    .child(
                                        "Content area with custom width configuration, \
                                            and the footer is used flex 1 button widths.",
                                    )
                                    .child(
                                        DialogFooter::new()
                                            .justify_center()
                                            .child(
                                                Button::new("cancel")
                                                    .flex_1()
                                                    .outline()
                                                    .label("Cancel")
                                                    .on_click(|_, window, cx| {
                                                        window.close_dialog(cx);
                                                    }),
                                            )
                                            .child(
                                                Button::new("done")
                                                    .flex_1()
                                                    .primary()
                                                    .label("Done")
                                                    .on_click(|_, window, cx| {
                                                        window.close_dialog(cx);
                                                    }),
                                            ),
                                    )
                            })
                        })
                    })),
            )
    }

    fn render_textview_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;
        section("Selectable text")
            .description("Embed selectable rich text.")
            .child(
                Dialog::new(cx)
                    .trigger(
                        Button::new("textview-dialog-btn")
                            .outline()
                            .label("TextView Dialog"),
                    )
                    .overlay(dialog_overlay)
                    .keyboard(self.keyboard)
                    .close_button(self.close_button)
                    .overlay_closable(overlay_closable)
                    .p_0()
                    .content({
                        move |content, _, cx| {
                            content
                                .child(
                                    DialogHeader::new()
                                        .p_4()
                                        .child(DialogTitle::new().child("TextView Dialog")),
                                )
                                .child(
                                    v_flex().px_4().pb_4().gap_3().child(
                                        TextView::markdown(
                                            "dialog-textview",
                                            "This is a dialog with a selectable \
                                        TextView in it. This text should be \
                                        selectable.",
                                        )
                                        .selectable(true),
                                    ),
                                )
                                .child(
                                    DialogFooter::new()
                                        .p_4()
                                        .bg(cx.theme().muted)
                                        .child(
                                            DialogClose::new().child(
                                                Button::new("cancel").label("Cancel").outline(),
                                            ),
                                        )
                                        .child(DialogAction::new().child(
                                            Button::new("confirm").primary().label("Confirm"),
                                        )),
                                )
                        }
                    }),
            )
    }
}

impl Focusable for DialogStory {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DialogStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("dialog-story")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_test_action))
            .on_action(cx.listener(|this, action: &ToggleDialogOption, _, cx| {
                match action {
                    ToggleDialogOption::Overlay => this.dialog_overlay = !this.dialog_overlay,
                    ToggleDialogOption::OverlayClosable => {
                        this.overlay_closable = !this.overlay_closable
                    }
                    ToggleDialogOption::CloseButton => this.close_button = !this.close_button,
                    ToggleDialogOption::Keyboard => this.keyboard = !this.keyboard,
                }
                cx.notify();
            }))
            .size_full()
            .child(
                v_flex()
                    .gap_6()
                    .child(story_toolbar_group().dropdown_child(
                        Button::new("dialog-options").label("Options"),
                        {
                            let overlay = self.dialog_overlay;
                            let overlay_closable = self.overlay_closable;
                            let close_button = self.close_button;
                            let keyboard = self.keyboard;
                            move |menu, _, _| {
                                menu.menu_with_check(
                                    "Overlay",
                                    overlay,
                                    Box::new(ToggleDialogOption::Overlay),
                                )
                                .menu_with_check(
                                    "Close on overlay click",
                                    overlay_closable,
                                    Box::new(ToggleDialogOption::OverlayClosable),
                                )
                                .menu_with_check(
                                    "Close button",
                                    close_button,
                                    Box::new(ToggleDialogOption::CloseButton),
                                )
                                .menu_with_check(
                                    "Keyboard",
                                    keyboard,
                                    Box::new(ToggleDialogOption::Keyboard),
                                )
                            }
                        },
                    ))
                    .child(self.render_basic_dialog(cx))
                    .child(self.render_custom_buttons(cx))
                    .child(self.render_scrollable_dialog(cx))
                    .child(self.render_table_in_dialog(cx))
                    .child(self.render_dialog_without_title(cx))
                    .child(self.render_custom_paddings(cx))
                    .child(self.render_custom_style(cx))
                    .child(self.render_dialog_with_content(cx))
                    .child(self.render_textview_dialog(cx))
                    .when(cfg!(not(target_family = "wasm")), |this| {
                        this.child(self.render_focus_return_check(cx))
                    }),
            )
    }
}

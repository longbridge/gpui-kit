use std::{cell::Cell, rc::Rc, time::Duration};

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, WindowExt as _,
    button::Button,
    command::{Command, CommandEntry, CommandGroup, CommandItem, CommandState},
    h_flex,
    kbd::Kbd,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::section;

actions!(
    command_story,
    [
        OpenProfile,
        OpenBilling,
        OpenSettings,
        GoHome,
        OpenInbox,
        OpenDocuments,
        OpenFolders,
        NewFile,
        CopyItem,
        DeleteItem
    ]
);

const COMMAND_CONTEXT: &str = "Command";

pub struct CommandStory {
    focus_handle: FocusHandle,
    inline: Entity<CommandState>,
    dialog: Entity<CommandState>,
    quick_actions: Entity<CommandState>,
    scrollable: Entity<CommandState>,
    variable_rows: Entity<CommandState>,
    search: Entity<CommandState>,
    stock_entries: Vec<CommandEntry>,
    stock_results: Vec<Stock>,
    /// Held so that a query that arrives while the last one is still in flight
    /// cancels it, instead of racing it.
    _search_task: Option<Task<()>>,
    last_command: Option<gpui_kit::SharedString>,
}

impl super::Story for CommandStory {
    fn title() -> &'static str {
        "Command"
    }

    fn description() -> &'static str {
        "A searchable list of commands and quick actions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

/// The palette used by the inline and dialog examples: two groups of commands,
/// one of them disabled, the second group carrying shortcut hints.
fn suggestions() -> Vec<CommandEntry> {
    vec![
        CommandGroup::new()
            .label("Suggestions")
            .item(
                CommandItem::new()
                    .label("Calendar")
                    .icon(IconName::Calendar),
            )
            .item(
                CommandItem::new()
                    .label("Search Emoji")
                    .icon(IconName::Search)
                    .checked(true)
                    .keywords(["smile", "icon"]),
            )
            .item(
                CommandItem::new()
                    .label("Calculator")
                    .icon(IconName::Frame)
                    .disabled(true),
            )
            .into(),
        CommandEntry::Separator,
        CommandGroup::new()
            .label("Settings")
            .item(
                CommandItem::new()
                    .label("Profile")
                    .icon(IconName::User)
                    .action(Box::new(OpenProfile)),
            )
            .item(
                CommandItem::new()
                    .label("Billing")
                    .icon(IconName::CircleUser)
                    .action(Box::new(OpenBilling)),
            )
            .item(
                CommandItem::new()
                    .label("Settings")
                    .icon(IconName::Settings)
                    .action(Box::new(OpenSettings)),
            )
            .into(),
    ]
}

fn scrollable() -> Vec<CommandEntry> {
    vec![
        CommandGroup::new()
            .label("Navigation")
            .item(
                CommandItem::new()
                    .label("Home")
                    .icon(IconName::LayoutDashboard)
                    .action(Box::new(GoHome)),
            )
            .item(
                CommandItem::new()
                    .label("Inbox")
                    .icon(IconName::Inbox)
                    .action(Box::new(OpenInbox)),
            )
            .item(
                CommandItem::new()
                    .label("Documents")
                    .icon(IconName::File)
                    .action(Box::new(OpenDocuments)),
            )
            .item(
                CommandItem::new()
                    .label("Folders")
                    .icon(IconName::Folder)
                    .action(Box::new(OpenFolders)),
            )
            .into(),
        CommandEntry::Separator,
        CommandGroup::new()
            .label("Actions")
            .item(
                CommandItem::new()
                    .label("New File")
                    .icon(IconName::Plus)
                    .action(Box::new(NewFile)),
            )
            .item(
                CommandItem::new()
                    .label("Copy")
                    .icon(IconName::Copy)
                    .action(Box::new(CopyItem)),
            )
            .item(
                CommandItem::new()
                    .label("Delete")
                    .icon(IconName::Delete)
                    .action(Box::new(DeleteItem)),
            )
            .into(),
        CommandEntry::Separator,
        CommandGroup::new()
            .label("Account")
            .item(CommandItem::new().label("Profile").icon(IconName::User))
            .item(
                CommandItem::new()
                    .label("Notifications")
                    .icon(IconName::Bell),
            )
            .item(
                CommandItem::new()
                    .label("Help & Support")
                    .icon(IconName::Info),
            )
            .into(),
        CommandEntry::Separator,
        CommandGroup::new()
            .label("Tools")
            .item(CommandItem::new().label("Palette").icon(IconName::Palette))
            .item(
                CommandItem::new()
                    .label("Terminal")
                    .icon(IconName::SquareTerminal),
            )
            .item(CommandItem::new().label("Globe").icon(IconName::Globe))
            .into(),
    ]
}

/// Actions that can be navigated without a search field, such as a compact
/// context menu.
fn quick_actions() -> impl Iterator<Item = CommandItem> {
    [
        CommandItem::new().label("New File").icon(IconName::Plus),
        CommandItem::new().label("Duplicate").icon(IconName::Copy),
        CommandItem::new()
            .label("Move to Trash")
            .icon(IconName::Delete),
    ]
    .into_iter()
}

/// Two custom rows with different intrinsic heights. The Command list measures
/// each flattened row, so both retain their own height while virtualized.
fn variable_rows() -> impl Iterator<Item = CommandItem> {
    [
        CommandItem::new().label("small-row").child(|_, _| {
            h_flex()
                .w_full()
                .py_1()
                .child(div().text_sm().child("Compact custom row"))
        }),
        CommandItem::new().label("large-row").child(|_, cx| {
            v_flex()
                .w_full()
                .py_4()
                .gap_1()
                .child(div().text_sm().child("Expanded custom row"))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Its extra detail gives this row a different height."),
                )
        }),
    ]
    .into_iter()
}

fn with_entries(command: Command, entries: impl IntoIterator<Item = CommandEntry>) -> Command {
    entries
        .into_iter()
        .fold(command, |command, entry| match entry {
            CommandEntry::Item(item) => command.item(item),
            CommandEntry::Group(group) => command.group(group),
            CommandEntry::Separator => command.separator(),
        })
}

fn key_hint(keys: &[&str], label: &'static str) -> AnyElement {
    h_flex()
        .gap_1()
        .items_center()
        .children(
            keys.iter()
                .map(|key| Kbd::new(Keystroke::parse(key).expect("static key hint must be valid"))),
        )
        .child(label)
        .into_any_element()
}

/// The stock universe the search panel queries.
///
/// Stands in for whatever a real application would go and fetch.
type Stock = (&'static str, &'static str, &'static str, f32);

const STOCKS: [Stock; 10] = [
    ("AAPL.US", "Apple Inc.", "228.52", 1.24),
    ("NVDA.US", "NVIDIA Corporation", "134.81", -0.62),
    ("TSLA.US", "Tesla, Inc.", "251.44", 3.18),
    ("MSFT.US", "Microsoft Corporation", "428.02", 0.41),
    ("AMZN.US", "Amazon.com, Inc.", "186.33", -1.07),
    ("700.HK", "Tencent Holdings Ltd.", "412.60", 0.87),
    ("9988.HK", "Alibaba Group Holding Ltd.", "82.15", -2.31),
    ("3690.HK", "Meituan", "128.90", 1.66),
    ("600519.SH", "Kweichow Moutai Co., Ltd.", "1482.00", -0.34),
    ("000858.SZ", "Wuliangye Yibin Co., Ltd.", "142.77", 0.19),
];

impl CommandStory {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let primary = if cfg!(target_os = "macos") {
            "cmd"
        } else {
            "ctrl"
        };
        cx.bind_keys([
            KeyBinding::new(&format!("{primary}-p"), OpenProfile, Some(COMMAND_CONTEXT)),
            KeyBinding::new(&format!("{primary}-b"), OpenBilling, Some(COMMAND_CONTEXT)),
            KeyBinding::new(&format!("{primary}-s"), OpenSettings, Some(COMMAND_CONTEXT)),
            KeyBinding::new(&format!("{primary}-h"), GoHome, Some(COMMAND_CONTEXT)),
            KeyBinding::new(&format!("{primary}-i"), OpenInbox, Some(COMMAND_CONTEXT)),
            KeyBinding::new(
                &format!("{primary}-d"),
                OpenDocuments,
                Some(COMMAND_CONTEXT),
            ),
            KeyBinding::new(&format!("{primary}-f"), OpenFolders, Some(COMMAND_CONTEXT)),
            KeyBinding::new(&format!("{primary}-n"), NewFile, Some(COMMAND_CONTEXT)),
            KeyBinding::new(&format!("{primary}-c"), CopyItem, Some(COMMAND_CONTEXT)),
            KeyBinding::new("backspace", DeleteItem, Some(COMMAND_CONTEXT)),
        ]);

        let inline = cx.new(|cx| CommandState::new(window, cx));
        let dialog = cx.new(|cx| CommandState::new(window, cx));
        let quick_actions = cx.new(|cx| CommandState::new(window, cx));
        let scrollable_state = cx.new(|cx| CommandState::new(window, cx));
        let variable_rows = cx.new(|cx| CommandState::new(window, cx));
        let search = cx.new(|cx| CommandState::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            inline,
            dialog,
            quick_actions,
            scrollable: scrollable_state,
            variable_rows,
            search,
            stock_entries: popular_entries(),
            stock_results: STOCKS.iter().take(5).copied().collect(),
            _search_task: None,
            last_command: None,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn on_command_confirm(
        &mut self,
        index: gpui_kit::component::IndexPath,
        cx: &mut Context<Self>,
    ) {
        self.last_command = Some(format!("section {}, row {}", index.section, index.row).into());
        cx.notify();
    }

    fn on_dialog_confirm(
        &mut self,
        index: gpui_kit::component::IndexPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_command_confirm(index, cx);
        window.close_dialog(cx);
    }

    /// Open the stock search as a dialog, starting from an empty query.
    ///
    /// The palette keeps a fixed height so that results arriving, and the
    /// query being narrowed, do not make the dialog jump around.
    fn open_stock_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._search_task = None;
        self.stock_entries = popular_entries();
        self.stock_results = STOCKS.iter().take(5).copied().collect();
        self.search.update(cx, |search, cx| {
            search.set_query("", window, cx);
            search.set_loading(false, window, cx);
        });

        let search = self.search.clone();
        let story = cx.weak_entity();
        let focus_on_mount = Rc::new(Cell::new(true));
        window.open_dialog(cx, move |dialog, _, _| {
            let search = search.clone();
            let story = story.clone();
            let close_owner = story.clone();
            let focus_on_mount = focus_on_mount.clone();
            dialog
                .close_button(false)
                .p_0()
                .on_close(move |_, window, cx| {
                    _ = close_owner.update(cx, |story, cx| {
                        story.cancel_stock_search(window, cx);
                    });
                })
                .content(move |content, window, cx| {
                    if focus_on_mount.replace(false) {
                        let search = search.clone();
                        window.defer(cx, move |window, cx| {
                            search.read(cx).focus_handle(cx).focus(window, cx);
                        });
                    }
                    let entries = story
                        .read_with(cx, |story, _| {
                            stock_entries_for_render(&story.stock_entries)
                        })
                        .unwrap_or_default();
                    let query_owner = story.clone();
                    let confirm_owner = story.clone();
                    content.child(with_entries(
                        Command::new(&search)
                            .bordered(false)
                            .placeholder("Search stocks...")
                            .empty(|_, _, cx| {
                                v_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .py_6()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(Icon::new(IconName::Search).size_8())
                                    .child("No stock found.")
                            })
                            .min_h(px(320.))
                            .max_h(px(320.))
                            .on_query(move |query, window, cx| {
                                _ = query_owner.update(cx, |story, cx| {
                                    story.on_stock_query(query, window, cx);
                                });
                            })
                            .on_confirm(move |index, window, cx| {
                                _ = confirm_owner.update(cx, |story, cx| {
                                    story.on_stock_confirm(index, window, cx);
                                });
                            }),
                        entries,
                    ))
                })
        });
    }

    /// Answer the search panel's queries the way a remote search would: spin
    /// the field, wait, then replace the entries with the results.
    fn on_stock_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            // Nothing typed is not the same as nothing found: fall back to the
            // list people would otherwise have to search for.
            self._search_task = None;
            self.stock_entries = popular_entries();
            self.stock_results = STOCKS.iter().take(5).copied().collect();
            let search = self.search.clone();
            window.defer(cx, move |window, cx| {
                search.update(cx, |state, cx| {
                    state.set_loading(false, window, cx);
                });
            });
            cx.notify();
            return;
        }

        let search = self.search.clone();
        window.defer(cx, move |window, cx| {
            search.update(cx, |state, cx| {
                state.set_loading(true, window, cx);
            });
        });

        let search = self.search.clone();
        self._search_task = Some(cx.spawn_in(window, async move |story, cx| {
            // The round trip a real search would spend on the network.
            cx.background_executor()
                .timer(Duration::from_millis(400))
                .await;

            let results = STOCKS
                .iter()
                .filter(|(symbol, name, _, _)| {
                    symbol.to_lowercase().contains(&query) || name.to_lowercase().contains(&query)
                })
                .copied()
                .collect::<Vec<_>>();
            let entries = results
                .iter()
                .copied()
                .map(|stock| CommandEntry::Item(stock_item(stock)))
                .collect::<Vec<_>>();

            _ = story.update_in(cx, |story, window, cx| {
                search.update(cx, |state, cx| {
                    state.set_loading(false, window, cx);
                });
                story.stock_entries = entries;
                story.stock_results = results;
                story._search_task = None;
                cx.notify();
            });
        }));
    }

    fn on_stock_confirm(
        &mut self,
        index: gpui_kit::component::IndexPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let stock = (index.section == 0)
            .then(|| self.stock_results.get(index.row))
            .flatten();
        let Some((symbol, _, _, _)) = stock else {
            return;
        };
        let symbol: SharedString = (*symbol).into();
        self.cancel_stock_search(window, cx);
        self.last_command = Some(symbol);
        cx.notify();
        window.close_dialog(cx);
    }

    fn cancel_stock_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._search_task = None;
        self.search.update(cx, |search, cx| {
            search.set_loading(false, window, cx);
        });
        cx.notify();
    }
}

/// What the panel shows before anything has been typed.
fn popular_entries() -> Vec<CommandEntry> {
    vec![
        CommandGroup::new()
            .label("Popular")
            .items(STOCKS.iter().take(5).map(|stock| stock_item(*stock)))
            .into(),
    ]
}

/// `Command` consumes its entries when rendered. Rebuild the stock rows from
/// the owner-held result identities so a dialog redraw never drains the owner.
fn stock_entries_for_render(entries: &[CommandEntry]) -> Vec<CommandEntry> {
    entries.to_vec()
}

/// A two-line search result: symbol and name on the left, quote on the right.
fn stock_item(stock: (&'static str, &'static str, &'static str, f32)) -> CommandItem {
    let (symbol, name, price, change) = stock;

    CommandItem::new()
        .label(name)
        .keywords([symbol])
        .child(move |_, cx| {
            let change_color = if change < 0. {
                cx.theme().chart_bearish
            } else {
                cx.theme().chart_bullish
            };

            h_flex()
                .w_full()
                .gap_3()
                .items_center()
                .justify_between()
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(div().text_sm().child(symbol))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(name),
                        ),
                )
                .child(
                    v_flex()
                        .gap_0p5()
                        .items_end()
                        .child(div().text_sm().child(price))
                        .child(
                            div()
                                .text_xs()
                                .text_color(change_color)
                                .child(format!("{:+.2}%", change)),
                        ),
                )
        })
}

impl Focusable for CommandStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_state = self.dialog.clone();
        let inline_owner = cx.weak_entity();
        let quick_actions_owner = cx.weak_entity();
        let scrollable_owner = cx.weak_entity();
        let variable_rows_owner = cx.weak_entity();

        v_flex()
            .w_full()
            .gap_6()
            .child(
                section("Inline")
                    .description("A palette rendered in place, with groups, icons and shortcuts.")
                    .child(with_entries(
                        Command::new(&self.inline)
                            .w(px(380.))
                            .on_confirm(move |index_path, _, cx| {
                                _ = inline_owner.update(cx, |story, cx| {
                                    story.on_command_confirm(index_path, cx);
                                });
                            }),
                        suggestions(),
                    )),
            )
            .child(
                section("Dialog")
                    .description(
                        "A dialog palette with its search field focused, a live match count and key hints.",
                    )
                    .child(
                        Button::new("open-command-dialog")
                            .outline()
                            .label("Open Menu")
                            .on_click(cx.listener(move |_, _, window, cx| {
                                let command = dialog_state.clone();
                                let owner = cx.weak_entity();
                                let focus_on_mount = Rc::new(Cell::new(true));
                                window.open_dialog(cx, move |dialog, _, _| {
                                    let command = command.clone();
                                    let owner = owner.clone();
                                    let focus_on_mount = focus_on_mount.clone();
                                    dialog.close_button(false).p_0().content(
                                        move |content, window, cx| {
                                            if focus_on_mount.replace(false) {
                                                let command = command.clone();
                                                window.defer(cx, move |window, cx| {
                                                    command
                                                        .read(cx)
                                                        .focus_handle(cx)
                                                        .focus(window, cx);
                                                });
                                            }
                                            let confirm_owner = owner.clone();
                                            // Cancel intentionally has no local close callback:
                                            // the propagated action belongs to Dialog.
                                            content.child(with_entries(
                                                Command::new(&command)
                                                    .bordered(false)
                                                    .placeholder("Type a command or search...")
                                                    .on_confirm(move |index_path, window, cx| {
                                                        _ = confirm_owner.update(cx, |story, cx| {
                                                            story.on_dialog_confirm(
                                                                index_path, window, cx,
                                                            );
                                                        });
                                                    })
                                                    .header(|state, _, cx| {
                                                        h_flex()
                                                            .justify_between()
                                                            .px_3()
                                                            .py_2()
                                                            .border_b_1()
                                                            .border_color(cx.theme().border)
                                                            .text_sm()
                                                            .child("Commands")
                                                            .child(format!(
                                                                "{} matches",
                                                                state.matched_count()
                                                            ))
                                                    })
                                                    .footer(|_, _, cx| {
                                                        h_flex()
                                                            .gap_3()
                                                            .items_center()
                                                            .flex_wrap()
                                                            .px_3()
                                                            .py_2()
                                                            .border_t_1()
                                                            .border_color(cx.theme().border)
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(key_hint(
                                                                &["up", "down"],
                                                                "Navigate",
                                                            ))
                                                            .child(key_hint(&["enter"], "Select"))
                                                            .child(key_hint(&["escape"], "Close"))
                                                    }),
                                                suggestions(),
                                            ))
                                        },
                                    )
                                });
                            })),
                ),
            )
            .child(
                section("Quick actions")
                    .description("A no-search palette focused on arrow-key navigation.")
                    .child(
                        Command::new(&self.quick_actions)
                            .searchable(false)
                            .items(quick_actions())
                            .w(px(380.))
                            .on_confirm(move |index_path, _, cx| {
                                _ = quick_actions_owner.update(cx, |story, cx| {
                                    story.on_command_confirm(index_path, cx);
                                });
                            }),
                    ),
            )
            .child(
                section("Scrollable")
                    .description("More commands than fit, capped at 220px.")
                    .child(with_entries(
                        Command::new(&self.scrollable)
                            .max_h(px(220.))
                            .w(px(380.))
                            .on_confirm(move |index_path, _, cx| {
                                _ = scrollable_owner.update(cx, |story, cx| {
                                    story.on_command_confirm(index_path, cx);
                                });
                            }),
                        scrollable(),
                    )),
            )
            .child(
                section("Variable-height rows")
                    .description(
                        "Each custom row keeps its own intrinsic height while the list remains virtualized.",
                    )
                    .child(
                        Command::new(&self.variable_rows)
                            .items(variable_rows())
                            .w(px(380.))
                            .on_confirm(move |index_path, _, cx| {
                                _ = variable_rows_owner.update(cx, |story, cx| {
                                    story.on_command_confirm(index_path, cx);
                                });
                            }),
                    ),
            )
            .child(
                section("Search panel")
                    .description(
                        "A palette used as a search panel whose custom filter checks symbols \
                         before company names — try \"a\", \"hk\" or \"tesla\".",
                    )
                    .child(
                        Button::new("open-stock-search")
                            .outline()
                            .label("Search Stocks")
                            .on_click(cx.listener(|story, _, window, cx| {
                                story.open_stock_search(window, cx);
                            })),
                    ),
            )
            .when_some(self.last_command.clone(), |this, value| {
                this.child(
                    section("Last confirmed")
                        .description("The value reported by the last on_confirm callback.")
                        .child(value),
                )
            })
    }
}

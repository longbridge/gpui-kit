use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, IndexPath, StyledExt as _, ThemeColor, ThemeStyled as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::PopupMenuItem,
    scroll::ScrollableElement,
    select::{Select, SelectEvent, SelectItem, SelectState},
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder, *};
use serde::Deserialize;

use crate::stories::theme_story::checkerboard::Checkerboard;

use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = theme_colors_story, no_json)]
enum ThemeOption {
    ShowInherited,
    ExpandAll,
}

#[derive(Clone)]
struct ColorEntry {
    name: String,
    color: Hsla,
    hex: String,
    is_explicit: bool,
}

#[derive(Clone)]
struct ColorCategory {
    name: String,
    entries: Vec<ColorEntry>,
}

#[derive(Clone, PartialEq)]
struct ThemeItem {
    name: SharedString,
    is_active: bool,
}

impl ThemeItem {
    fn new(name: impl Into<SharedString>, is_active: bool) -> Self {
        Self {
            name: name.into(),
            is_active,
        }
    }
}

impl SelectItem for ThemeItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }

    fn render(&self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .child(
                div()
                    .size(rems(1.0))
                    .flex_shrink_0()
                    .when(self.is_active, |this| {
                        this.child(
                            Icon::new(IconName::Check)
                                .size(rems(1.0))
                                .text_color(cx.theme().primary),
                        )
                    }),
            )
            .child(self.name.clone())
    }
}

pub struct ThemeColorsStory {
    select_state: Entity<SelectState<Vec<ThemeItem>>>,
    selected_theme_name: SharedString,
    show_all_colors: bool,
    sidebar_render_key: usize,
    force_open_state: Option<bool>,
    filter_by_value: Option<Hsla>,
    filter_input: Entity<InputState>,
    all_categories: Vec<ColorCategory>,
    categories: Vec<ColorCategory>,
}

impl crate::stories::Story for ThemeColorsStory {
    fn title() -> &'static str {
        "Theme Colors"
    }

    fn description() -> &'static str {
        "A color theme viewer to explore colors organized by categories."
        // Themes are loaded by applying user-defined color overrides to a default base theme,
        // with inherited colors marked by an indicator dot.
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ThemeColorsStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        use gpui_kit::component::ThemeRegistry;

        let registry = ThemeRegistry::global(cx);
        let mut themes = registry.sorted_themes();

        themes.sort_by_key(|a| a.name.to_lowercase());

        let active_theme_name = cx.theme().theme_name().clone();
        let items: Vec<ThemeItem> = themes
            .iter()
            .map(|theme| ThemeItem::new(theme.name.clone(), theme.name == active_theme_name))
            .collect();

        let current_theme = active_theme_name.clone();
        let selected_index = items.iter().position(|item| item.name == current_theme);
        let selected_path = selected_index.map(|idx| IndexPath::default().row(idx));
        let select_state = cx.new(|cx| SelectState::new(items, selected_path, window, cx));

        let mut this = Self {
            select_state: select_state.clone(),
            selected_theme_name: current_theme,
            show_all_colors: false,
            sidebar_render_key: 0,
            force_open_state: None,
            filter_by_value: None,
            filter_input: cx.new(|cx| InputState::new(window, cx).placeholder("Search...")),
            all_categories: Vec::new(),
            categories: Vec::new(),
        };

        cx.subscribe(
            &select_state,
            |this, _, event: &SelectEvent<Vec<ThemeItem>>, cx| {
                let SelectEvent::Confirm(theme_name) = event;
                if let Some(theme_name) = theme_name {
                    this.selected_theme_name = theme_name.clone();
                    this.filter_by_value = None;
                    this.all_categories.clear();
                    this.compute_categories(cx);
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe(&this.filter_input, |this, _, event, cx| {
            if let InputEvent::Change = event {
                this.compute_categories(cx);
                cx.notify();
            }
        })
        .detach();

        this.compute_categories(cx);
        this
    }

    fn get_theme_colors(&self, cx: &Context<Self>) -> ThemeColor {
        use gpui_kit::component::{Theme as UITheme, ThemeRegistry};

        if let Some(theme_config) = ThemeRegistry::global(cx)
            .themes()
            .get(&self.selected_theme_name)
            .cloned()
        {
            let mut temp_theme = if theme_config.mode.is_dark() {
                UITheme::from(ThemeColor::dark().as_ref())
            } else {
                UITheme::from(ThemeColor::light().as_ref())
            };

            // Apply the config to get proper colors using the public API
            temp_theme.apply_config(&theme_config);
            temp_theme.colors
        } else {
            // Fallback to current theme if selected theme not found
            **cx.theme()
        }
    }

    fn get_isolated_theme(&self, cx: &App) -> (ThemeColor, bool) {
        use gpui_kit::component::{Theme as UITheme, ThemeRegistry};

        let registry = ThemeRegistry::global(cx);

        // Look up the selected theme configuration
        let selected_theme_config = registry.themes().get(&self.selected_theme_name);

        let is_dark = if let Some(config) = selected_theme_config {
            config.mode.is_dark()
        } else {
            // Fallback to system appearance if selected theme lookup fails
            let appearance = cx.window_appearance();
            appearance == WindowAppearance::Dark || appearance == WindowAppearance::VibrantDark
        };

        let theme_config = if is_dark {
            registry.default_dark_theme()
        } else {
            registry.default_light_theme()
        };

        let mut temp_theme = if theme_config.mode.is_dark() {
            UITheme::from(ThemeColor::dark().as_ref())
        } else {
            UITheme::from(ThemeColor::light().as_ref())
        };

        temp_theme.apply_config(theme_config);
        (temp_theme.colors, is_dark)
    }

    fn compute_categories(&mut self, cx: &Context<Self>) {
        use gpui_kit::component::ThemeRegistry;

        if self.all_categories.is_empty() {
            let theme = self.get_theme_colors(cx);
            let registry = ThemeRegistry::global(cx);
            let theme_config = registry.themes().get(&self.selected_theme_name).cloned();

            self.all_categories = format_colors(&theme, theme_config.as_ref().map(|c| &c.colors));
        }

        let mut categories = self.all_categories.clone();

        if let Some(filter_value) = self.filter_by_value {
            categories = filter_categories(categories, |entry| {
                colors_equal_u8(entry.color, filter_value)
            });
        } else if !self.show_all_colors {
            categories = filter_categories(categories, |entry| entry.is_explicit);
        }

        let query = self.filter_input.read(cx).value().trim().to_lowercase();
        if !query.is_empty() {
            let normalized_query = query.strip_prefix('#').unwrap_or(&query);
            categories = categories
                .into_iter()
                .filter_map(
                    |ColorCategory {
                         name: category,
                         entries: colors,
                     }| {
                        let category_matches = category.to_lowercase().contains(&query);
                        let filtered_colors: Vec<_> = colors
                            .into_iter()
                            .filter(|entry| {
                                if category_matches || entry.name.to_lowercase().contains(&query) {
                                    return true;
                                }

                                // Hex matching
                                entry.hex.starts_with(normalized_query)
                            })
                            .collect();

                        if filtered_colors.is_empty() {
                            None
                        } else {
                            Some(ColorCategory {
                                name: category,
                                entries: filtered_colors,
                            })
                        }
                    },
                )
                .collect();
        }

        self.categories = categories;
    }

    fn render_color_swatch(
        name: String,
        color: Hsla,
        hex: String,
        is_explicit: bool,
        isolated_theme: &ThemeColor,
        cx: &App,
    ) -> impl IntoElement {
        use gpui_kit::component::{WindowExt as _, clipboard::Clipboard};

        let rgb_str = format!("#{}", hex);
        let swatch_group = format!("swatch-{}", name);

        h_flex()
            .group(swatch_group.clone())
            .gap_3()
            .items_center()
            .child(
                div()
                    .size_16()
                    .rounded(cx.theme().radius)
                    .bg(color)
                    .border_1()
                    .border_color(isolated_theme.border)
                    .flex_shrink_0(),
            )
            .child(
                v_flex()
                    .gap_1()
                    .flex_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .when(!is_explicit, |this| {
                                this.child(
                                    div()
                                        .size_1p5()
                                        .rounded_full_style(cx)
                                        .bg(isolated_theme.foreground)
                                        .flex_shrink_0(),
                                )
                            })
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .when(!is_explicit, |this: Div| {
                                        this.text_color(isolated_theme.muted_foreground)
                                    })
                                    .when(is_explicit, |this| {
                                        this.text_color(isolated_theme.foreground)
                                    })
                                    .child(name.clone()),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(isolated_theme.muted_foreground)
                                    .child(rgb_str.clone()),
                            )
                            .child(
                                div()
                                    .invisible()
                                    .group_hover(swatch_group, |this| this.visible())
                                    .child(
                                        Clipboard::new(format!("copy-{}", name))
                                            .value(rgb_str)
                                            .on_copied(move |value, window, cx| {
                                                window.push_notification(
                                                    format!("Copied {} to clipboard", value),
                                                    cx,
                                                )
                                            }),
                                    ),
                            ),
                    ),
            )
    }

    fn render_left_panel(&self, _: &mut Window, cx: &mut Context<Self>) -> Sidebar<SidebarMenu> {
        let categories = &self.categories;
        let is_filtering = self.filter_by_value.is_some();
        let entity_ref = cx.entity().clone();

        let expand_all = Rc::new(cx.listener(
            |this: &mut Self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>| {
                this.sidebar_render_key += 1;
                this.force_open_state = Some(true);
                cx.notify();
            },
        ));

        let collapse_all = Rc::new(cx.listener(
            |this: &mut Self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>| {
                this.sidebar_render_key += 1;
                this.force_open_state = Some(false);
                cx.notify();
            },
        ));

        Sidebar::new(format!("color-theme-sidebar-{}", self.sidebar_render_key))
            .w(px(300.))
            .border_0()
            .header(Input::new(&self.filter_input).prefix(IconName::Search))
            .child(
                SidebarMenu::new().children(categories.iter().enumerate().map(
                    |(
                        idx,
                        ColorCategory {
                            name: category_name,
                            entries: colors,
                        },
                    )| {
                        let is_open = self.force_open_state.unwrap_or_else(|| idx == 0);

                        SidebarMenuItem::new(category_name.clone())
                            .default_open(is_open)
                            .click_to_open(true)
                            .context_menu({
                                let expand_all = expand_all.clone();
                                let collapse_all = collapse_all.clone();
                                move |menu, _, _| {
                                    menu.item(PopupMenuItem::new("Expand All").on_click({
                                        let expand_all = expand_all.clone();
                                        move |ev, window, cx| (expand_all)(ev, window, cx)
                                    }))
                                    .item(
                                        PopupMenuItem::new("Collapse All").on_click({
                                            let collapse_all = collapse_all.clone();
                                            move |ev, window, cx| (collapse_all)(ev, window, cx)
                                        }),
                                    )
                                }
                            })
                            .children(colors.iter().map(|entry| {
                                let color_value = entry.color;
                                let is_explicit = entry.is_explicit;
                                let color_view = entity_ref.clone();
                                SidebarMenuItem::new(entry.name.clone())
                                    .suffix(move |_, cx| {
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .when(!is_explicit, |this| {
                                                this.child(
                                                    div()
                                                        .size_1p5()
                                                        .rounded_full_style(cx)
                                                        .bg(cx.theme().foreground),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .size_4()
                                                    .rounded(cx.theme().radius.half())
                                                    .bg(color_value)
                                                    .border_1()
                                                    .border_color(cx.theme().border)
                                                    .flex_shrink_0(),
                                            )
                                    })
                                    .context_menu(move |menu, _, _| {
                                        let menu_view = color_view.clone();
                                        if is_filtering {
                                            menu.item(
                                                PopupMenuItem::new("Show All Values").on_click(
                                                    move |_, _, cx| {
                                                        menu_view.update(cx, |this, cx| {
                                                            this.filter_by_value = None;
                                                            this.compute_categories(cx);
                                                            cx.notify();
                                                        })
                                                    },
                                                ),
                                            )
                                        } else {
                                            menu.item(
                                                PopupMenuItem::new("Filter By Value").on_click(
                                                    move |_, _, cx| {
                                                        menu_view.update(cx, |this, cx| {
                                                            this.filter_by_value =
                                                                Some(color_value);
                                                            this.compute_categories(cx);
                                                            cx.notify();
                                                        })
                                                    },
                                                ),
                                            )
                                        }
                                    })
                            }))
                    },
                )),
            )
    }

    fn render_right_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (isolated_theme, is_dark) = self.get_isolated_theme(cx);

        let categories = self.categories.clone();
        let categories_count = categories.len();
        let list_state = window
            .use_keyed_state("color-theme-right-panel-list-state", cx, |_, _| {
                ListState::new(19, ListAlignment::Top, px(1000.))
            })
            .read(cx)
            .clone();
        if list_state.item_count() != categories_count {
            list_state.reset(categories_count);
        }

        div()
            .border_1()
            .border_color(isolated_theme.border)
            .rounded(cx.theme().radius_lg)
            .size_full()
            .overflow_hidden()
            .child(
                Checkerboard::new(is_dark).child(
                    v_flex()
                        .size_full()
                        .overflow_hidden()
                        .rounded(cx.theme().radius_lg)
                        .px_4()
                        .child(
                            list(list_state.clone(), {
                                move |ix, _, cx| {
                                    let ColorCategory {
                                        name: category_name,
                                        entries: colors,
                                    } = categories[ix].clone();
                                    let is_last = categories_count > 0
                                        && ix == categories_count.saturating_sub(1);

                                    v_flex()
                                        .w_full()
                                        .gap_3()
                                        .pt_4()
                                        .when(is_last, |this| this.pb_4())
                                        .child(
                                            div()
                                                .text_base()
                                                .font_semibold()
                                                .pb_2()
                                                .border_b_1()
                                                .border_color(isolated_theme.border)
                                                .text_color(isolated_theme.foreground)
                                                .child(category_name.clone()),
                                        )
                                        .child(div().flex().flex_wrap().gap_4().children(
                                            colors.iter().map(|entry| {
                                                div().w(px(220.)).child(Self::render_color_swatch(
                                                    entry.name.to_string(),
                                                    entry.color,
                                                    entry.hex.clone(),
                                                    entry.is_explicit,
                                                    &isolated_theme,
                                                    cx,
                                                ))
                                            }),
                                        ))
                                        .into_any_element()
                                }
                            })
                            .size_full(),
                        )
                        .vertical_scrollbar(&list_state),
                ),
            )
    }
}

fn format_colors(
    theme: &ThemeColor,
    config: Option<&gpui_kit::component::theme::ThemeConfigColors>,
) -> Vec<ColorCategory> {
    let json_theme = serde_json::to_value(theme).unwrap_or(serde_json::Value::Null);
    let mut categories: BTreeMap<String, Vec<ColorEntry>> = BTreeMap::new();

    // Create a set of keys present in the config (if available)
    let config_keys: Option<std::collections::HashSet<String>> = config.map(|c| {
        let json_config = serde_json::to_value(c).unwrap_or(serde_json::Value::Null);
        if let serde_json::Value::Object(map) = json_config {
            map.into_iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, _)| k)
                .collect()
        } else {
            std::collections::HashSet::new()
        }
    });

    if let serde_json::Value::Object(map) = json_theme {
        for (key, value) in map {
            if let Ok(color) = serde_json::from_value::<Hsla>(value) {
                let parsed = super::mapper::parse_theme_key(&key);
                let category = parsed.category;
                let name = parsed.name;

                // Check if this key is explicit in the user config
                let is_explicit = config_keys
                    .as_ref()
                    .map_or(false, |k| k.contains(&parsed.canonical_key));

                categories.entry(category).or_default().push(ColorEntry {
                    name,
                    color,
                    hex: hsla_to_hex(color),
                    is_explicit,
                });
            }
        }
    }

    for colors in categories.values_mut() {
        colors.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let mut categories_vec: Vec<_> = categories
        .into_iter()
        .map(|(name, entries)| ColorCategory { name, entries })
        .collect();

    // Custom sort: Global first, then Primary, then others
    categories_vec.sort_by(|a, b| {
        let priority_order = [
            "Global",
            "Primary",
            "Secondary",
            "Accent",
            "Base",
            "Background",
            "Foreground",
            "Structure",
        ];

        let a_priority = priority_order.iter().position(|&x| x == a.name.as_str());
        let b_priority = priority_order.iter().position(|&x| x == b.name.as_str());

        match (a_priority, b_priority) {
            (Some(a_pos), Some(b_pos)) => a_pos.cmp(&b_pos),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        }
    });

    categories_vec
}

fn hsla_to_hex(color: Hsla) -> String {
    let rgb = color.to_rgb();
    if color.a < 1.0 {
        format!(
            "{:02x}{:02x}{:02x}{:02x}",
            (rgb.r * 255.0) as u8,
            (rgb.g * 255.0) as u8,
            (rgb.b * 255.0) as u8,
            (color.a * 255.0) as u8
        )
    } else {
        format!(
            "{:02x}{:02x}{:02x}",
            (rgb.r * 255.0) as u8,
            (rgb.g * 255.0) as u8,
            (rgb.b * 255.0) as u8
        )
    }
}

/// Compares two HSLA colors for equality at 8-bit precision.
fn colors_equal_u8(c1: Hsla, c2: Hsla) -> bool {
    let rgb1 = c1.to_rgb();
    let rgb2 = c2.to_rgb();
    let eq = |a: f32, b: f32| (a * 255.0).round() as u8 == (b * 255.0).round() as u8;
    eq(rgb1.r, rgb2.r) && eq(rgb1.g, rgb2.g) && eq(rgb1.b, rgb2.b) && eq(c1.a, c2.a)
}

/// Filters categories by a predicate on color entries, removing empty categories.
fn filter_categories(
    categories: Vec<ColorCategory>,
    predicate: impl Fn(&ColorEntry) -> bool,
) -> Vec<ColorCategory> {
    categories
        .into_iter()
        .filter_map(
            |ColorCategory {
                 name: category,
                 entries: colors,
             }| {
                let filtered: Vec<_> = colors.into_iter().filter(|e| predicate(e)).collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(ColorCategory {
                        name: category,
                        entries: filtered,
                    })
                }
            },
        )
        .collect()
}

impl Render for ThemeColorsStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .on_action(cx.listener(|this, action: &ThemeOption, _, cx| {
                match action {
                    ThemeOption::ShowInherited => {
                        this.show_all_colors = !this.show_all_colors;
                        this.compute_categories(cx);
                    }
                    ThemeOption::ExpandAll => {
                        this.sidebar_render_key += 1;
                        this.force_open_state = Some(this.force_open_state != Some(true));
                    }
                }
                cx.notify();
            }))
            .gap_4()
            .size_full()
            .overflow_hidden()
            .child(
                // Theme selector at the top
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().w(px(300.)).child(Select::new(&self.select_state)))
                            .child(
                                Button::new("set_theme")
                                    .primary()
                                    .label("Set Theme")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        use gpui_kit::component::{Theme, ThemeRegistry};

                                        let registry = ThemeRegistry::global(cx);
                                        if let Some(theme_config) = registry
                                            .themes()
                                            .get(&this.selected_theme_name)
                                            .cloned()
                                        {
                                            let mode = theme_config.mode;
                                            let theme = Theme::global_mut(cx);
                                            if mode.is_dark() {
                                                theme.dark_theme = theme_config;
                                            } else {
                                                theme.light_theme = theme_config;
                                            }
                                            Theme::change(mode, None, cx);
                                            cx.refresh_windows();

                                            // Refresh the select items to update the active checkmark
                                            let active_theme_name = cx.theme().theme_name().clone();
                                            let themes = ThemeRegistry::global(cx).sorted_themes();

                                            // Re-create items with new active state
                                            let mut items: Vec<ThemeItem> = themes
                                                .iter()
                                                .map(|theme| {
                                                    ThemeItem::new(
                                                        theme.name.clone(),
                                                        // Note: we need to handle case sensitivity if names differ,
                                                        // but usually accurate.
                                                        theme.name == active_theme_name,
                                                    )
                                                })
                                                .collect();

                                            // Sort again to be safe/consistent
                                            items.sort_by(|a, b| {
                                                a.name.to_lowercase().cmp(&b.name.to_lowercase())
                                            });

                                            // Update the select state
                                            this.select_state.update(cx, |state, cx| {
                                                state.set_items(items, window, cx);
                                            });
                                        }
                                    })),
                            ),
                    )
                    .child(crate::story_toolbar_group().dropdown_child(
                        Button::new("theme-options").label("Options"),
                        {
                            let show_all_colors = self.show_all_colors;
                            let expand_all = self.force_open_state == Some(true);
                            move |menu, _, _| {
                                menu.menu_with_check(
                                    "Inherited Colors",
                                    show_all_colors,
                                    Box::new(ThemeOption::ShowInherited),
                                )
                                .menu_with_check(
                                    "Expand All",
                                    expand_all,
                                    Box::new(ThemeOption::ExpandAll),
                                )
                            }
                        },
                    )),
            )
            .child(
                h_flex()
                    .flex_1()
                    .items_start()
                    .gap_4()
                    .child(self.render_left_panel(window, cx))
                    .child(self.render_right_panel(window, cx)),
            )
    }
}

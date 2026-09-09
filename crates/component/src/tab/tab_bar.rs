use std::{cell::RefCell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Background, Bounds, Edges, ElementId, InteractiveElement, IntoElement,
    ParentElement, Pixels, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_base::spring;
use rust_i18n::t;
use smallvec::SmallVec;

use super::{Tab, TabVariant};
use crate::button::{Button, ButtonVariants as _};
use crate::menu::{DropdownMenu as _, PopupMenuItem};
use crate::{
    ActiveTheme, ElementExt, Icon, InteractiveElementExt as _, Selectable, Sizable, Size,
    StyledExt, h_flex, styled::raised_shadow,
};

struct TabIndicatorBounds {
    container: Bounds<Pixels>,
    tabs: Vec<Bounds<Pixels>>,
}

impl TabIndicatorBounds {
    fn new(num_tabs: usize) -> Self {
        Self {
            container: Bounds::default(),
            tabs: vec![Bounds::default(); num_tabs],
        }
    }

    fn resize(&mut self, num_tabs: usize) {
        self.tabs.resize(num_tabs, Bounds::default());
    }
}

/// A TabBar element that contains multiple [`Tab`] items.
#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    base: gpui_base::Tabs,
    style: StyleRefinement,
    scroll_handle: Option<ScrollHandle>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: SmallVec<[Tab; 2]>,
    last_empty_space: AnyElement,
    selected_index: Option<usize>,
    variant: TabVariant,
    size: Size,
    menu: bool,
    max_width: Option<Pixels>,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl TabBar {
    /// Create a new TabBar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            base: gpui_base::Tabs::new(id).px(px(-1.)),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
            scroll_handle: None,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            last_empty_space: div().w_3().into_any_element(),
            selected_index: None,
            on_click: None,
            menu: false,
            max_width: None,
        }
    }

    /// Set the Tab variant, all children will inherit the variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the Tab variant to Pill, all children will inherit the variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Set the Tab variant to Outline, all children will inherit the variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Set the Tab variant to Segmented, all children will inherit the variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Set the Tab variant to Underline, all children will inherit the variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set whether to show the menu button when tabs overflow, default is false.
    pub fn menu(mut self, menu: bool) -> Self {
        self.menu = menu;
        self
    }

    /// Set the maximum width of each tab. Labels longer than this width are
    /// truncated with an ellipsis. Does not apply to icon-only tabs. The
    /// overflow menu still shows the full label.
    pub fn max_width(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = Some(width.into());
        self
    }

    /// Track the scroll of the TabBar.
    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    /// Set the prefix element of the TabBar
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the suffix element of the TabBar
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Add children of the TabBar, all children will inherit the variant.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Add child of the TabBar, tab will inherit the variant.
    pub fn child(mut self, child: impl Into<Tab>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Set the selected index of the TabBar.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Set the last empty space element of the TabBar.
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.last_empty_space = last_empty_space.into_any_element();
        self
    }

    /// Set the on_click callback of the TabBar, the first parameter is the index of the clicked tab.
    ///
    /// When this is set, the children's on_click will be ignored.
    pub fn on_click<F>(mut self, on_click: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Render the sliding indicator element for animated tab switching.
    ///
    /// Returns the indicator element together with the current animation
    /// `epoch`, which increments on every tab switch. Tabs key their own
    /// transitions (e.g. text color fade) on this epoch so they restart in sync
    /// with the indicator slide.
    fn render_indicator(
        &self,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        inset: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<(AnyElement, u64)> {
        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();
        let selected_ix = self.selected_index.unwrap_or(usize::MAX);

        if !(has_indicator && num_tabs > 0 && selected_ix < num_tabs) {
            return None;
        }

        let prev_key = format!("{}-tab-prev", self.id);
        let anim_key = format!("{}-tab-anim", self.id);
        let init_key = format!("{}-tab-init", self.id);

        let prev_selected = window.use_keyed_state(prev_key, cx, |_, _| selected_ix);
        // (to_left, to_width, epoch)
        let anim_params = window.use_keyed_state(anim_key, cx, |_, _| (px(0.), px(0.), 0u64));
        let initialized = window.use_keyed_state(init_key, cx, |_, _| false);

        // First frame: trigger re-render to capture bounds via on_prepaint
        if !*initialized.read(cx) {
            initialized.update(cx, |v, _| *v = true);
        }

        self.update_anim_params(selected_ix, bounds_rc, &prev_selected, &anim_params, cx);

        let (to_left, to_width, epoch) = *anim_params.read(cx);
        if to_width <= px(0.) {
            return None;
        }

        // The springs hold the indicator's own position and velocity, so a tab
        // switched again mid-slide is redirected from where the indicator
        // actually is rather than restarted from the tab it left.
        let indicator_key = format!("{}-tab-indicator", self.id);
        let left = spring(
            (indicator_key.clone(), "left"),
            to_left,
            cx.theme().motion_tokens().spring_move,
            window,
            cx,
        );
        let width = spring(
            (indicator_key, "width"),
            to_width,
            cx.theme().motion_tokens().spring_move,
            window,
            cx,
        );

        let variant = self.variant;
        let size = self.size;
        let inner_height = variant.inner_height(size);
        let inner_radius = variant.inner_radius(size, cx);

        let indicator = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(left + inset)
            .w(width)
            .map(|el| match variant {
                TabVariant::Segmented => el.flex().items_center().child(
                    div()
                        .w_full()
                        .h(inner_height)
                        .bg(cx.theme().tokens.background)
                        .rounded(inner_radius)
                        .shadow(raised_shadow()),
                ),
                TabVariant::Pill => el.flex().items_center().child(
                    div()
                        .size_full()
                        .bg(cx.theme().tokens.primary)
                        .rounded(cx.theme().radius_full()),
                ),
                TabVariant::Underline => el.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(2.))
                        .bg(cx.theme().tokens.primary),
                ),
                _ => el,
            });

        Some((indicator.into_any_element(), epoch))
    }

    /// Update animation parameters based on current and previous selection.
    fn update_anim_params(
        &self,
        selected_ix: usize,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        prev_selected: &gpui::Entity<usize>,
        anim_params: &gpui::Entity<(Pixels, Pixels, u64)>,
        cx: &mut App,
    ) {
        let rc = match bounds_rc {
            Some(rc) => rc,
            None => return,
        };

        let prev_ix = *prev_selected.read(cx);
        let bounds = rc.borrow();
        let container = bounds.container;

        if container.size.width == px(0.) {
            if prev_ix != selected_ix {
                prev_selected.update(cx, |v, _| *v = selected_ix);
            }
            return;
        }

        if prev_ix != selected_ix {
            if let Some(to_b) = bounds.tabs.get(selected_ix) {
                let left = to_b.origin.x - container.origin.x;
                let width = to_b.size.width;
                // Only a switch away from a tab that still exists restarts the
                // tabs' own epoch-keyed transitions.
                let epoch = anim_params.read(cx).2;
                let epoch = match bounds.tabs.get(prev_ix) {
                    Some(_) => epoch + 1,
                    None => epoch,
                };
                anim_params.update(cx, |v, _| *v = (left, width, epoch));
            }
            drop(bounds);
            prev_selected.update(cx, |v, _| *v = selected_ix);
            return;
        }

        if let Some(to_b) = bounds.tabs.get(selected_ix) {
            let left = to_b.origin.x - container.origin.x;
            let width = to_b.size.width;
            let (to_left, to_width, epoch) = *anim_params.read(cx);

            if left != to_left || width != to_width {
                anim_params.update(cx, |v, _| *v = (left, width, epoch));
            }
        }
    }
}

impl Styled for TabBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TabBar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_gap = match self.size {
            Size::Small | Size::XSmall => px(8.),
            Size::Large => px(16.),
            _ => px(12.),
        };
        let (bg, paddings, gap): (Background, _, _) = match self.variant {
            TabVariant::Tab => {
                let padding = Edges::all(px(0.));
                (cx.theme().tokens.tab_bar.into(), padding, px(0.))
            }
            TabVariant::Outline => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent.into(), padding, default_gap)
            }
            TabVariant::Pill => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent.into(), padding, px(4.))
            }
            TabVariant::Segmented => {
                let padding_x = match self.size {
                    Size::XSmall => px(2.),
                    Size::Small => px(3.),
                    _ => px(4.),
                };
                let padding = Edges {
                    left: padding_x,
                    right: padding_x,
                    ..Default::default()
                };

                (cx.theme().tokens.tab_bar_segmented.into(), padding, px(2.))
            }
            TabVariant::Underline => {
                // This gap is same as the tab inner_paddings
                let gap = match self.size {
                    Size::XSmall => px(10.),
                    Size::Small => px(12.),
                    Size::Large => px(20.),
                    _ => px(16.),
                };

                (cx.theme().transparent.into(), Edges::all(px(0.)), gap)
            }
        };

        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();

        // Bounds tracking for tab indicator animation.
        // Uses Rc<RefCell> to avoid triggering re-renders from prepaint writes.
        let bounds_rc = if has_indicator && num_tabs > 0 {
            let rc: Rc<RefCell<TabIndicatorBounds>> = window
                .use_keyed_state(format!("{}-tab-bounds", self.id), cx, |_, _| {
                    Rc::new(RefCell::new(TabIndicatorBounds::new(num_tabs)))
                })
                .read(cx)
                .clone();
            rc.borrow_mut().resize(num_tabs);
            Some(rc)
        } else {
            None
        };

        let padding_x = paddings.left;
        let indicator = self.render_indicator(&bounds_rc, padding_x, window, cx);
        let indicator_epoch = indicator.as_ref().map(|(_, epoch)| *epoch).unwrap_or(0);
        let indicator_element = indicator.map(|(el, _)| el);
        let indicator_ready = indicator_element.is_some();

        let has_suffix_or_menu = self.suffix.is_some() || self.menu;
        let mut item_metas: Vec<(Option<SharedString>, Option<Icon>, bool)> = Vec::new();
        let selected_index = self.selected_index;
        let on_click = self.on_click.clone();
        let tabs = self.base;
        let mut rendered_tabs = Vec::with_capacity(self.children.len());
        let max_width = self.max_width;

        for (ix, child) in self.children.into_iter().enumerate() {
            item_metas.push((child.label.clone(), child.icon.clone(), child.disabled));
            let tab_bar_prefix = child.tab_bar_prefix.unwrap_or(true);
            let mut tab = child
                .ix(ix)
                .tab_bar_prefix(tab_bar_prefix)
                .max_width(max_width)
                .with_variant(self.variant)
                .with_size(self.size);
            tab.indicator_active = has_indicator;
            tab.indicator_ready = indicator_ready;
            tab.indicator_epoch = indicator_epoch;
            let tab = tab
                .when_some(selected_index, |tab, selected_index| {
                    tab.selected(selected_index == ix)
                })
                .when_some(self.on_click.clone(), move |tab, on_click| {
                    tab.on_click(move |_, window, cx| on_click(&ix, window, cx))
                });

            rendered_tabs.push(if let Some(ref rc) = bounds_rc {
                let rc = rc.clone();
                div()
                    .flex_shrink_0()
                    .on_prepaint(move |bounds, _, _| {
                        if let Some(slot) = rc.borrow_mut().tabs.get_mut(ix) {
                            *slot = bounds;
                        }
                    })
                    .child(tab)
                    .into_any_element()
            } else {
                tab.into_any_element()
            });
        }

        tabs.group("tab-bar")
            .relative()
            .flex()
            .items_center()
            .bg(bg)
            .text_color(cx.theme().tab_foreground)
            .when(
                self.variant == TabVariant::Underline || self.variant == TabVariant::Tab,
                |this| {
                    this.child(
                        div()
                            .id("border-b")
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .size_full()
                            .border_b_1()
                            .border_color(cx.theme().border),
                    )
                },
            )
            .rounded(self.variant.tab_bar_radius(self.size, cx))
            .paddings(paddings)
            .refine_style(&self.style)
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                h_flex()
                    .id("tabs")
                    .flex_1()
                    .mx(-padding_x)
                    .px(padding_x)
                    .overflow_x_hidden()
                    .child(
                        h_flex()
                            .id("tabs-inner")
                            .mx(-padding_x)
                            .px(padding_x)
                            .relative()
                            .gap(gap)
                            .overflow_x_scroll()
                            .lock_scroll_axis()
                            .when_some(self.scroll_handle, |this, scroll_handle| {
                                this.track_scroll(&scroll_handle)
                            })
                            .when_some(bounds_rc.clone(), |this, rc| {
                                this.on_prepaint(move |bounds, _, _| {
                                    rc.borrow_mut().container = bounds;
                                })
                            })
                            .when_some(indicator_element, |this, ind| this.child(ind))
                            .children(rendered_tabs)
                            .when(has_suffix_or_menu, |this| this.child(self.last_empty_space)),
                    ),
            )
            .when(self.menu, |this| {
                this.child(
                    Button::new("more")
                        .xsmall()
                        .ghost()
                        .dropdown_caret(true)
                        .dropdown_menu(move |mut this, _, _| {
                            this = this.scrollable(true);
                            for (ix, (label, icon, disabled)) in item_metas.iter().enumerate() {
                                let base = if let Some(label) = label.clone() {
                                    PopupMenuItem::new(label)
                                } else if let Some(icon) = icon.clone() {
                                    PopupMenuItem::element(move |_, _| icon.clone())
                                } else {
                                    PopupMenuItem::new(t!("Dock.Unnamed"))
                                };
                                this = this.item(
                                    base.checked(selected_index == Some(ix))
                                        .disabled(*disabled)
                                        .when_some(on_click.clone(), |this, on_click| {
                                            this.on_click(move |_, window, cx| {
                                                on_click(&ix, window, cx)
                                            })
                                        }),
                                );
                            }

                            this
                        })
                        .anchor(Anchor::TopRight),
                )
            })
            .when_some(self.suffix, |this, suffix| this.child(suffix))
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{Context, Modifiers, Render, TestAppContext};

    use super::*;

    struct Harness {
        group_handler: bool,
        disabled: bool,
        child_clicks: Rc<Cell<usize>>,
        group_clicks: Rc<Cell<usize>>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let child_clicks = self.child_clicks.clone();
            let group_clicks = self.group_clicks.clone();
            TabBar::new("tabs")
                .w(px(240.))
                .child(
                    Tab::new()
                        .debug_selector(|| "first-tab".into())
                        .disabled(self.disabled)
                        .label("First")
                        .on_click(move |_, _, _| child_clicks.set(child_clicks.get() + 1)),
                )
                .when(self.group_handler, |tabs| {
                    tabs.on_click(move |ix, _, _| group_clicks.set(*ix + 1))
                })
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        group_handler: bool,
        disabled: bool,
    ) -> (
        &mut gpui::VisualTestContext,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
    ) {
        cx.update(crate::theme::init);
        let child_clicks = Rc::new(Cell::new(0));
        let group_clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let child_clicks = child_clicks.clone();
            let group_clicks = group_clicks.clone();
            move |_, _| Harness {
                group_handler,
                disabled,
                child_clicks,
                group_clicks,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, child_clicks, group_clicks)
    }

    #[gpui::test]
    fn group_callback_overrides_child_callback(cx: &mut TestAppContext) {
        let (cx, child_clicks, group_clicks) = harness(cx, true, false);
        let position = cx.debug_bounds("first-tab").unwrap().center();
        cx.simulate_click(position, Modifiers::default());
        assert_eq!(child_clicks.get(), 0);
        assert_eq!(group_clicks.get(), 1);
    }

    #[gpui::test]
    fn child_callback_is_preserved_without_group_callback(cx: &mut TestAppContext) {
        let (cx, child_clicks, group_clicks) = harness(cx, false, false);
        let position = cx.debug_bounds("first-tab").unwrap().center();
        cx.simulate_click(position, Modifiers::default());
        assert_eq!(child_clicks.get(), 1);
        assert_eq!(group_clicks.get(), 0);
    }

    #[gpui::test]
    fn disabled_tab_suppresses_child_and_group_callbacks(cx: &mut TestAppContext) {
        let (cx, child_clicks, group_clicks) = harness(cx, true, true);
        let position = cx.debug_bounds("first-tab").unwrap().center();
        cx.simulate_click(position, Modifiers::default());
        assert_eq!(child_clicks.get(), 0);
        assert_eq!(group_clicks.get(), 0);
    }

    struct ContentHarness;

    impl Render for ContentHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            TabBar::new("content-tabs").w(px(320.)).child(
                Tab::new()
                    .prefix(div().debug_selector(|| "tab-prefix".into()).child("P"))
                    .child(div().debug_selector(|| "tab-child".into()).child("Content"))
                    .suffix(div().debug_selector(|| "tab-suffix".into()).child("S")),
            )
        }
    }

    #[gpui::test]
    fn prefix_content_and_suffix_keep_their_order(cx: &mut TestAppContext) {
        cx.update(crate::theme::init);
        let (_, cx) = cx.add_window_view(|_, _| ContentHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let prefix = cx.debug_bounds("tab-prefix").unwrap();
        let child = cx.debug_bounds("tab-child").unwrap();
        let suffix = cx.debug_bounds("tab-suffix").unwrap();
        assert!(prefix.origin.x < child.origin.x);
        assert!(child.origin.x < suffix.origin.x);
        assert!(prefix.size.width > px(0.));
        assert!(child.size.width > px(0.));
        assert!(suffix.size.width > px(0.));
    }
}

use std::rc::Rc;

use crate::{ActiveTheme, Collapsible, h_flex, sidebar::SidebarItem, v_flex};
use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, SharedString, Styled as _, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_base::LogicalStyledExt as _;

/// A group of items in the [`super::Sidebar`].
#[derive(Clone)]
pub struct SidebarGroup<E: SidebarItem + 'static> {
    label: SharedString,
    suffix: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>>,
    collapsed: bool,
    children: Vec<E>,
}

impl<E: SidebarItem> SidebarGroup<E> {
    /// Create a new [`SidebarGroup`] with the given label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            suffix: None,
            collapsed: false,
            children: Vec::new(),
        }
    }

    /// Set content to show at the end of the group's header, such as a button
    /// that adds an item to the group.
    ///
    /// The label shortens to make room for it. Like the label, it is hidden
    /// while the sidebar is collapsed.
    pub fn suffix<F, S>(mut self, builder: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> S + 'static,
        S: IntoElement,
    {
        self.suffix = Some(Rc::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Add a child to the sidebar group, the child should implement [`SidebarItem`].
    pub fn child(mut self, child: E) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children to the sidebar group.
    ///
    /// See also [`SidebarGroup::child`].
    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.children.extend(children);
        self
    }
}

impl<E: SidebarItem> Collapsible for SidebarGroup<E> {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl<E: SidebarItem> SidebarItem for SidebarGroup<E> {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let id = id.into();

        v_flex()
            .relative()
            .when(!self.collapsed, |this| {
                this.child(
                    h_flex()
                        .flex_row_dir(cx)
                        .flex_shrink_0()
                        .px_2()
                        .rounded(cx.theme().radius)
                        .text_xs()
                        .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                        .h_8()
                        .map(|this| match self.suffix {
                            Some(suffix) => this
                                .gap_2()
                                .justify_between()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(self.label),
                                )
                                .child(
                                    h_flex()
                                        .flex_shrink_0()
                                        .items_center()
                                        .child(suffix(window, cx)),
                                ),
                            None => this.child(self.label),
                        }),
                )
            })
            .child(
                div()
                    .gap_2()
                    .flex_col()
                    .children(self.children.into_iter().enumerate().map(|(ix, child)| {
                        child
                            .collapsed(self.collapsed)
                            .render(format!("{}-{}", id, ix), window, cx)
                            .into_any_element()
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, InteractiveElement as _, Render, TestAppContext, px};

    use super::*;
    use crate::sidebar::SidebarMenu;

    struct GroupHarness {
        collapsed: bool,
    }

    impl Render for GroupHarness {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(240.)).child(
                SidebarGroup::<SidebarMenu>::new("A project name long enough to shorten")
                    .suffix(|_, _| div().debug_selector(|| "group-suffix".into()).size(px(16.)))
                    .collapsed(self.collapsed)
                    .render("group", window, cx),
            )
        }
    }

    #[gpui::test]
    fn the_suffix_sits_at_the_end_of_the_header(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| GroupHarness { collapsed: false });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let suffix = cx
            .debug_bounds("group-suffix")
            .expect("the suffix is drawn in the header");
        assert_eq!(suffix.size.width, px(16.));
        assert!(
            suffix.right() <= px(240.) && suffix.right() > px(220.),
            "a long label leaves the suffix at the end of the header: {suffix:?}"
        );
    }

    #[gpui::test]
    fn a_collapsed_group_hides_the_suffix(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| GroupHarness { collapsed: true });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        assert!(cx.debug_bounds("group-suffix").is_none());
    }
}

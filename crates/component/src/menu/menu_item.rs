use crate::{ActiveTheme, Disableable, StyledExt, h_flex};
use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, accesskit::Toggled, prelude::FluentBuilder as _,
};
use gpui_base::{A11yDisabledExt as _, LogicalStyledExt as _, TestSupportExt as _};
use smallvec::SmallVec;

#[derive(IntoElement)]
pub(crate) struct MenuItemElement {
    id: ElementId,
    /// `None` leaves the row out of the accessibility tree.
    role: Option<Role>,
    aria_label: Option<SharedString>,
    /// The item's 1-based position among its menu's items, and their count.
    position: Option<(usize, usize)>,
    style: StyleRefinement,
    disabled: bool,
    selected: bool,
    checked: Option<bool>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_hover: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    children: SmallVec<[AnyElement; 2]>,
}

impl MenuItemElement {
    /// Create a new MenuItem with the given ID.
    pub(crate) fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            id,
            role: Some(Role::MenuItem),
            aria_label: None,
            position: None,
            style: StyleRefinement::default(),
            disabled: false,
            selected: false,
            checked: None,
            on_click: None,
            on_hover: None,
            children: SmallVec::new(),
        }
    }

    /// Set ListItem as the selected item style.
    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set whether the menu item is checked, for assistive technology, or
    /// `None` for an item that is not an option. The check mark itself is
    /// drawn by the menu.
    pub(crate) fn checked(mut self, checked: Option<bool>) -> Self {
        self.checked = checked;
        self
    }

    /// Set the accessible label for the menu item.
    pub(crate) fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Leave the row out of the accessibility tree, as for a separator, which
    /// only draws a line.
    pub(crate) fn presentational(mut self) -> Self {
        self.role = None;
        self
    }

    /// Set the item's 1-based position among its menu's items, and how many
    /// there are, so assistive technology can say "2 of 5".
    pub(crate) fn position_in_set(mut self, position: usize, size: usize) -> Self {
        self.position = Some((position, size));
        self
    }

    /// Set the disabled state of the MenuItem.
    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set a handler for when the MenuItem is clicked.
    pub(crate) fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Set a handler for when the mouse enters the MenuItem.
    #[allow(unused)]
    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Box::new(handler));
        self
    }
}

impl Disableable for MenuItemElement {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for MenuItemElement {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for MenuItemElement {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MenuItemElement {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .flex_row_dir(cx)
            .id(self.id)
            .test_support()
            .when_some(self.role, |this, role| this.role(role))
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.position, |this, (position, size)| {
                this.aria_position_in_set(position).aria_size_of_set(size)
            })
            .aria_selected(self.selected)
            .when_some(self.checked, |this, checked| {
                this.aria_toggled(if checked {
                    Toggled::True
                } else {
                    Toggled::False
                })
            })
            .gap_x_1()
            .py_1()
            .px_2()
            .text_base()
            .text_color(cx.theme().foreground)
            .relative()
            .items_center()
            .justify_between()
            .refine_style(&self.style)
            .when_some(self.on_hover, |this, on_hover| {
                this.on_hover(move |hovered, window, cx| (on_hover)(hovered, window, cx))
            })
            .when(!self.disabled, |this| {
                // Not `group_hover` on the item's own group: GPUI styles an
                // element before registering its group, and that fallback keeps
                // the pointer's item lit after a key moves the highlight away.
                this.hover(|this| {
                    this.bg(cx.theme().tokens.accent)
                        .text_color(cx.theme().accent_foreground)
                })
                .when(self.selected, |this| {
                    this.bg(cx.theme().tokens.accent)
                        .text_color(cx.theme().accent_foreground)
                })
                .when_some(self.on_click, |this, on_click| {
                    this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(on_click)
                })
            })
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .children(self.children)
            .a11y_disabled(self.disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Element as _;

    #[gpui::test]
    fn aria_label_sets_accessible_name(_cx: &mut gpui::TestAppContext) {
        let item = MenuItemElement::new("open").aria_label("Open");

        assert_eq!(item.aria_label, Some("Open".into()));
    }

    struct Probe {
        nodes: std::rc::Rc<std::cell::RefCell<Vec<(bool, gpui::accesskit::Node)>>>,
    }

    impl gpui::Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let nodes = self.nodes.clone();
            gpui::canvas(
                move |_, window, cx| {
                    let mut nodes = nodes.borrow_mut();
                    nodes.clear();
                    for disabled in [false, true] {
                        let element = MenuItemElement::new("open")
                            .disabled(disabled)
                            .render(window, cx)
                            .into_element();
                        let mut node = gpui::accesskit::Node::new(Role::Unknown);
                        element.write_a11y_info(&mut node);
                        nodes.push((disabled, node));
                    }
                },
                |_, _, _, _| {},
            )
        }
    }

    #[gpui::test]
    fn a_disabled_item_marks_its_node_disabled(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::init(cx));
        let nodes = std::rc::Rc::default();
        let (_, cx) = cx.add_window_view({
            let nodes = std::rc::Rc::clone(&nodes);
            move |_, _| Probe { nodes }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let nodes = nodes.borrow();
        assert_eq!(nodes.len(), 2);
        for (disabled, node) in nodes.iter() {
            assert_eq!(node.is_disabled(), *disabled, "disabled {disabled}");
        }
    }
}

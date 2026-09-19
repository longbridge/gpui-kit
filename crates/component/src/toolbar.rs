use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled,
    Window, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

pub use gpui_base::ToolbarGroup;

use gpui_base::Toolbar as BaseToolbar;

use crate::{ActiveTheme, Sizable, Size, StyledExt as _, h_flex};

/// A horizontal toolbar for commands, usually placed at the top of a window,
/// pane, or section.
///
/// The design mirrors the toolbars found in native UI frameworks (macOS
/// `NSToolbar`, Windows `ToolStrip`): a themed bar that hosts a row of actions
/// such as `Button`s, vertical `Separator`s, and short labels.
///
/// The bar exposes `Toolbar` semantics to assistive technology and owns
/// roving keyboard focus: when focus is on one of its controls, the arrow
/// keys move focus along the bar, wrapping around at the ends. Hosted inputs
/// keep their own arrow-key caret behavior; place them at the trailing end of
/// the bar.
///
/// Each region accepts any [`IntoElement`]. Prefer a ghost
/// [`Button`](crate::Button) sized to match the toolbar for commands, and pass
/// a plain string for a non-interactive label. An icon-only button must carry
/// a tooltip and an accessible name.
///
/// `left` and `right` pin items to each end; `child`/`children` add to the
/// middle, whose alignment follows the pinned ends: centered with both `left`
/// and `right`, end-aligned with only `left`, and start-aligned otherwise
/// (only `right`, or neither — like a plain bar).
///
/// Colors come from the `toolbar` (background) and `toolbar_border` theme
/// tokens, which fall back to the title bar colors.
///
/// The id keeps the toolbar's keyboard-focus state stable across frames;
/// give each toolbar in a window a distinct id.
///
/// ```
/// # mod gpui_kit { pub extern crate gpui_component as component; }
/// use gpui_kit::component::toolbar::Toolbar;
///
/// let _ = Toolbar::new("document-toolbar").left("New").right("Settings");
/// ```
#[derive(IntoElement)]
pub struct Toolbar {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    left: SmallVec<[AnyElement; 1]>,
    right: SmallVec<[AnyElement; 1]>,
    children: SmallVec<[AnyElement; 1]>,
}

impl Toolbar {
    /// Create a new, empty [`Toolbar`] at [`Size::Medium`].
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::default(),
            left: SmallVec::new(),
            right: SmallVec::new(),
            children: SmallVec::new(),
        }
    }

    /// Append an element to the left (leading) region. Call multiple times to
    /// add more.
    pub fn left(mut self, child: impl IntoElement) -> Self {
        self.left.push(child.into_any_element());
        self
    }

    /// Append an element to the right (trailing) region. Call multiple times
    /// to add more.
    pub fn right(mut self, child: impl IntoElement) -> Self {
        self.right.push(child.into_any_element());
        self
    }
}

/// `child` / `children` add to the middle region, so a `Toolbar` without
/// `left`/`right` items behaves like a plain bar container.
impl ParentElement for Toolbar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Toolbar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Toolbar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // The middle aligns by which ends are pinned: centered with both left
        // and right, end-aligned with only left, otherwise start-aligned (only
        // right, or neither) — matching `StatusBar`'s region contract. The
        // regions live inside the base toolbar so its roving arrow-key
        // navigation reaches every item across them.
        let size = self.size;
        let has_left = !self.left.is_empty();
        let has_right = !self.right.is_empty();
        let region = || h_flex().overflow_hidden().items_center();

        BaseToolbar::new(self.id)
            .flex()
            .items_center()
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().toolbar_border)
            .bg(cx.theme().toolbar)
            .text_color(cx.theme().foreground)
            .map(|this| match size {
                Size::XSmall => this.h_7().px_2().gap_1().text_xs(),
                Size::Small => this.h_8().px_2().gap_1().text_sm(),
                Size::Large => this.h_12().px_3().gap_2().text_base(),
                _ => this.h_10().px_2().gap_2().text_sm(),
            })
            .refine_style(&self.style)
            .when(has_left, |this| this.child(region().children(self.left)))
            .child(
                region()
                    .flex_1()
                    .when(has_left && has_right, |this| this.justify_center())
                    .when(has_left && !has_right, |this| this.justify_end())
                    .children(self.children),
            )
            .when(has_right, |this| this.child(region().children(self.right)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolbar_builder() {
        let toolbar = Toolbar::new("toolbar")
            .left("New")
            .left("Open")
            .child("Center")
            .right("Settings")
            .small();

        assert_eq!(toolbar.left.len(), 2);
        assert_eq!(toolbar.children.len(), 1);
        assert_eq!(toolbar.right.len(), 1);
        assert_eq!(toolbar.size, Size::Small);
    }

    #[test]
    fn test_toolbar_default() {
        let toolbar = Toolbar::new("toolbar");

        assert_eq!(toolbar.size, Size::Medium);
        assert!(toolbar.left.is_empty());
        assert!(toolbar.right.is_empty());
        assert!(toolbar.children.is_empty());
    }
}

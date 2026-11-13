use std::ops::Range;

use gpui::{
    App, BorderStyle, Bounds, Corners, Edges, Element, ElementId, GlobalElementId, HighlightStyle,
    Hitbox, HitboxBehavior, Hsla, InspectorElementId, IntoElement, LayoutId, PaintQuad, Pixels,
    Point, SharedString, StyledText, Subscription, TextStyleRefinement, Window, transparent_black,
};

use crate::{TextSelectionHandle, TextSelectionRegistration, TextSelectionRun};

/// Plain text that participates in the window-scoped [`TextSelection`](crate::TextSelection).
///
/// Use [`Self::new`] for an independent run, or [`Self::with_handle`] when
/// several elements form one selectable document.
///
/// Applications must render one [`crate::TextSelectionLayer`] above their
/// content and call [`crate::init`] during startup. Selection and copy then
/// work without depending on `gpui-component`.
pub struct SelectableText {
    id: ElementId,
    handle: Option<TextSelectionHandle>,
    text: SharedString,
    styled_text: StyledText,
    document_order: u64,
    text_style: Option<TextStyleRefinement>,
    selection_color: Option<Hsla>,
}

impl SelectableText {
    /// Creates a run that owns its own selection.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self::build(id.into(), None, text.into())
    }

    /// Creates a run that joins the document `handle` belongs to.
    pub fn with_handle(
        id: impl Into<ElementId>,
        handle: TextSelectionHandle,
        text: impl Into<SharedString>,
    ) -> Self {
        Self::build(id.into(), Some(handle), text.into())
    }

    fn build(id: ElementId, handle: Option<TextSelectionHandle>, text: SharedString) -> Self {
        Self {
            id,
            handle,
            styled_text: StyledText::new(text.clone()),
            text,
            document_order: 0,
            text_style: None,
            selection_color: None,
        }
    }

    /// Places this run in reading order among the others sharing its handle.
    pub fn document_order(mut self, order: u64) -> Self {
        self.document_order = order;
        self
    }

    /// Sets the text style the run is laid out and painted with.
    pub fn text_style(mut self, style: TextStyleRefinement) -> Self {
        self.text_style = Some(style);
        self
    }

    /// Styles ranges of the text, such as the matches of a search, as
    /// [`StyledText::with_highlights`] does. The ranges are UTF-8 offsets, in
    /// order and without overlap.
    pub fn highlights(
        mut self,
        highlights: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>,
    ) -> Self {
        self.styled_text = self.styled_text.with_highlights(highlights);
        self
    }

    /// Overrides the selection background, which defaults to the theme's
    /// `colors.selection` token.
    pub fn selection_color(mut self, color: Hsla) -> Self {
        self.selection_color = Some(color);
        self
    }

    fn paint_selection(
        layout: &gpui::TextLayout,
        range: Range<usize>,
        color: Hsla,
        window: &mut Window,
    ) {
        let (Some(start), Some(end)) = (
            layout.position_for_index(range.start),
            layout.position_for_index(range.end),
        ) else {
            return;
        };
        for bounds in selection_quad_bounds(start, end, layout.bounds(), layout.line_height()) {
            window.paint_quad(PaintQuad {
                bounds,
                background: color.into(),
                corner_radii: Corners::default(),
                border_widths: Edges::default(),
                border_color: transparent_black(),
                border_style: BorderStyle::default(),
            });
        }
    }
}

/// What a run that owns its selection keeps between frames.
struct OwnedSelection {
    handle: TextSelectionHandle,
    // A drag changes the selection without a frame of its own.
    _refresh: Subscription,
}

fn selection_quad_bounds(
    start: Point<Pixels>,
    end: Point<Pixels>,
    bounds: Bounds<Pixels>,
    line_height: Pixels,
) -> Vec<Bounds<Pixels>> {
    if start.y == end.y {
        return vec![Bounds::from_corners(
            start,
            Point::new(end.x, end.y + line_height),
        )];
    }

    let mut quads = vec![Bounds::from_corners(
        start,
        Point::new(bounds.right(), start.y + line_height),
    )];
    if end.y > start.y + line_height {
        quads.push(Bounds::from_corners(
            Point::new(bounds.left(), start.y + line_height),
            Point::new(bounds.right(), end.y),
        ));
    }
    quads.push(Bounds::from_corners(
        Point::new(bounds.left(), end.y),
        Point::new(end.x, end.y + line_height),
    ));
    quads
}

impl IntoElement for SelectableText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SelectableText {
    type RequestLayoutState = TextSelectionHandle;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let handle = self.handle.clone().unwrap_or_else(|| {
            window.with_element_state(
                global_id.expect("SelectableText must have a stable element id"),
                |retained: Option<OwnedSelection>, window| {
                    let retained = retained.unwrap_or_else(|| {
                        let handle = TextSelectionHandle::new(self.text.clone(), cx);
                        let refresh = handle.refresh_window_on_change(window, cx);
                        OwnedSelection {
                            handle,
                            _refresh: refresh,
                        }
                    });
                    (retained.handle.clone(), retained)
                },
            )
        });
        let (layout_id, ()) = if let Some(style) = self.text_style.clone() {
            window.with_text_style(Some(style), |window| {
                self.styled_text
                    .request_layout(global_id, inspector_id, window, cx)
            })
        } else {
            self.styled_text
                .request_layout(global_id, inspector_id, window, cx)
        };
        (layout_id, handle)
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        handle: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.styled_text
            .prepaint(global_id, inspector_id, bounds, &mut (), window, cx);
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        let registration = TextSelectionRegistration::new(hitbox.clone(), bounds)
            .with_document_order(self.document_order)
            .with_text_bounds(vec![bounds])
            .with_rendered_element(handle, window, cx);
        handle.register(registration, window, cx);
        hitbox
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        handle: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = self.styled_text.layout().clone();
        let copy_text_before = handle.projected_copy_text(cx);
        let projection = handle.update_runs(
            &[
                TextSelectionRun::new(self.text.clone(), layout.clone(), bounds)
                    .with_document_order(self.document_order),
            ],
            cx,
        );
        if copy_text_before != handle.projected_copy_text(cx) {
            window.refresh();
        }
        let color = self
            .selection_color
            .unwrap_or_else(|| crate::Theme::global(cx).tokens.colors.selection);
        for range in projection.ranges().iter().flatten().cloned() {
            Self::paint_selection(&layout, range, color, window);
        }
        self.styled_text.paint(
            global_id,
            inspector_id,
            bounds,
            &mut (),
            &mut (),
            window,
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext as _, Bounds, Context, IntoElement, Modifiers, MouseButton, ParentElement as _,
        Render, Styled as _, TestAppContext, Window, div, point, px, size,
    };

    use super::SelectableText;
    use crate::{TextSelection, TextSelectionHandle, TextSelectionLayer};

    struct SelectableTextTestView;

    impl Render for SelectableTextTestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(TextSelectionLayer).child(
                div()
                    .w(px(240.))
                    .h(px(32.))
                    .child(SelectableText::new("local", "alpha beta")),
            )
        }
    }

    #[gpui::test]
    fn explicit_handle_constructor_preserves_document_contract(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let handle = TextSelectionHandle::new("alpha beta", cx);
            let _ = SelectableText::with_handle("plain", handle, "alpha beta").document_order(42);
        });
    }

    #[gpui::test]
    fn local_handle_participates_in_window_selection(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| SelectableTextTestView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        cx.simulate_mouse_down(
            gpui::point(px(1.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            gpui::point(px(220.), px(12.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            gpui::point(px(220.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            assert_eq!(TextSelection::selected_text(window, cx), "alpha beta");
        });
    }

    struct HighlightedTestView;

    impl Render for HighlightedTestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mark = gpui::HighlightStyle {
                background_color: Some(gpui::red()),
                font_weight: Some(gpui::FontWeight::BOLD),
                ..Default::default()
            };
            div().size_full().child(TextSelectionLayer).child(
                div()
                    .w(px(240.))
                    .h(px(32.))
                    .child(SelectableText::new("local", "alpha beta").highlights([(6..10, mark)])),
            )
        }
    }

    #[gpui::test]
    fn highlighted_text_selects_and_copies_as_plain_text(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| HighlightedTestView);
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        cx.simulate_mouse_down(
            gpui::point(px(1.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            gpui::point(px(220.), px(12.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            gpui::point(px(220.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            assert_eq!(TextSelection::selected_text(window, cx), "alpha beta");
        });
    }

    struct CachedText {
        renders: Rc<Cell<usize>>,
    }

    impl Render for CachedText {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            SelectableText::new("local", "alpha beta")
        }
    }

    struct CachedTextParent {
        text: gpui::Entity<CachedText>,
    }

    impl Render for CachedTextParent {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(TextSelectionLayer).child(
                gpui::AnyView::from(self.text.clone())
                    .cached(gpui::StyleRefinement::default().w(px(240.)).h(px(32.))),
            )
        }
    }

    #[gpui::test]
    fn a_drag_redraws_the_text_before_the_button_is_released(cx: &mut TestAppContext) {
        let renders = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let renders = renders.clone();
            move |_, cx| CachedTextParent {
                text: gpui::AppContext::new(cx, |_| CachedText { renders }),
            }
        });
        let draw = |cx: &mut gpui::VisualTestContext| {
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
        };
        draw(cx);
        let before = renders.get();
        draw(cx);
        assert_eq!(
            renders.get(),
            before,
            "the text is cached while nothing changes"
        );

        cx.simulate_mouse_down(
            gpui::point(px(1.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        draw(cx);
        let pressed = renders.get();
        cx.simulate_mouse_move(
            gpui::point(px(220.), px(12.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        draw(cx);
        assert!(
            renders.get() > pressed,
            "the selection is drawn as the pointer moves"
        );
    }

    struct CopyCallbackTestView {
        handle: TextSelectionHandle,
    }

    impl Render for CopyCallbackTestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(TextSelectionLayer)
                .child(SelectableText::new("local", "alpha beta"))
                .child(SelectableText::with_handle(
                    "custom",
                    self.handle.clone(),
                    "gamma",
                ))
        }
    }

    #[gpui::test]
    fn paint_does_not_resolve_window_selected_text(cx: &mut TestAppContext) {
        let copies = Rc::new(Cell::new(0));
        let handle = cx.update(|cx| {
            let handle = TextSelectionHandle::new("gamma", cx);
            let copies = copies.clone();
            handle.copy_with(
                move |_| {
                    copies.set(copies.get() + 1);
                    "gamma".into()
                },
                cx,
            );
            handle.set_local_selection(true, cx);
            handle
        });
        let (_, cx) = cx.add_window_view(|_, _| CopyCallbackTestView { handle });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        copies.set(0);

        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });

        assert_eq!(copies.get(), 0);
    }

    #[test]
    fn wrapped_selection_paints_full_width_middle_lines() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(100.), px(100.)));
        let quads = super::selection_quad_bounds(
            point(px(40.), px(20.)),
            point(px(30.), px(80.)),
            bounds,
            px(20.),
        );

        assert_eq!(
            quads,
            vec![
                Bounds::from_corners(point(px(40.), px(20.)), point(px(110.), px(40.))),
                Bounds::from_corners(point(px(10.), px(40.)), point(px(110.), px(80.))),
                Bounds::from_corners(point(px(10.), px(80.)), point(px(30.), px(100.))),
            ]
        );
    }

    /// A selectable line, and under it a control that acts on the selection,
    /// drawn by a view of its own: cached when `cached`.
    struct ControlTestView {
        control: gpui::Entity<ControlView>,
        cached: bool,
    }

    struct ControlView {
        renders: Rc<Cell<usize>>,
    }

    impl Render for ControlView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            use crate::ElementExt as _;
            self.renders.set(self.renders.get() + 1);
            div().w(px(60.)).h(px(20.)).keep_text_selection("control")
        }
    }

    impl Render for ControlTestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let control = gpui::AnyView::from(self.control.clone());
            let control = if self.cached {
                control
                    .cached(gpui::StyleRefinement::default().w(px(60.)).h(px(20.)))
                    .into_any_element()
            } else {
                control.into_any_element()
            };
            div()
                .size_full()
                .child(TextSelectionLayer)
                .child(
                    div()
                        .w(px(240.))
                        .h(px(32.))
                        .child(SelectableText::new("local", "alpha beta")),
                )
                .child(control)
        }
    }

    fn press(position: gpui::Point<gpui::Pixels>, cx: &mut gpui::VisualTestContext) {
        cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    }

    fn select_line(cx: &mut gpui::VisualTestContext) {
        cx.simulate_mouse_down(
            point(px(1.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(220.), px(12.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(220.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            assert_eq!(TextSelection::selected_text(window, cx), "alpha beta");
        });
    }

    fn selected(cx: &mut gpui::VisualTestContext) -> String {
        cx.update(|window, cx| {
            let _ = window.draw(cx);
            TextSelection::selected_text(window, cx)
        })
    }

    #[gpui::test]
    fn a_press_on_an_element_marked_keep_text_selection_keeps_it(cx: &mut TestAppContext) {
        for cached in [false, true] {
            let renders = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view(|_, cx| ControlTestView {
                control: cx.new(|_| ControlView {
                    renders: renders.clone(),
                }),
                cached,
            });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            select_line(cx);
            // drawn again: a cached control is replayed, not drawn
            let drawn = renders.get();
            view.update(cx, |_, cx| cx.notify());
            assert_eq!(selected(cx), "alpha beta");
            assert_eq!(renders.get() == drawn, cached);

            press(point(px(30.), px(42.)), cx);
            assert_eq!(selected(cx), "alpha beta", "cached: {cached}");
            // beside the control, the press clears it as before
            press(point(px(120.), px(42.)), cx);
            assert_eq!(selected(cx), "", "cached: {cached}");
        }
    }
}

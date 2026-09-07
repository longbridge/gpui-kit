use std::ops::Range;

use gpui::{
    Along, App, Axis, Bounds, Context, ElementId, EventEmitter, IsZero, Pixels, Window, px,
};

mod panel;
mod resize_handle;
pub use panel::*;
#[doc(hidden)]
pub use resize_handle::*;

#[doc(hidden)]
pub const PANEL_MIN_SIZE: Pixels = px(100.);

/// Create a [`ResizablePanelGroup`] with horizontal resizing
pub fn h_resizable(id: impl Into<ElementId>) -> ResizablePanelGroup {
    ResizablePanelGroup::new(id).axis(Axis::Horizontal)
}

/// Create a [`ResizablePanelGroup`] with vertical resizing
pub fn v_resizable(id: impl Into<ElementId>) -> ResizablePanelGroup {
    ResizablePanelGroup::new(id).axis(Axis::Vertical)
}

/// Create a [`ResizablePanel`].
pub fn resizable_panel() -> ResizablePanel {
    ResizablePanel::new()
}

/// State for a [`ResizablePanel`]
#[derive(Debug, Clone)]
pub struct ResizableState {
    /// The `axis` will sync to actual axis of the ResizablePanelGroup in use.
    axis: Axis,
    panels: Vec<ResizablePanelState>,
    sizes: Vec<Pixels>,
    resizing_panel_ix: Option<usize>,
    bounds: Bounds<Pixels>,
}

impl Default for ResizableState {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            panels: vec![],
            sizes: vec![],
            resizing_panel_ix: None,
            bounds: Bounds::default(),
        }
    }
}

impl ResizableState {
    /// Get the size of the panels.
    pub fn sizes(&self) -> &Vec<Pixels> {
        &self.sizes
    }

    /// Programmatically resize the panel at `ix` to `size`, redistributing
    /// space among siblings using the same logic as a drag.
    ///
    /// Sizes are clamped to the panel's `size_range` and to the container.
    /// Emits `ResizablePanelEvent::Resized` so subscribers (e.g. preference
    /// persistence) see the change just as if the user had dragged a handle.
    ///
    /// Out-of-range indices are a no-op. For the last panel, space is taken
    /// from the previous sibling (the last panel has no handle of its own).
    pub fn resize_panel(
        &mut self,
        ix: usize,
        size: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ix >= self.sizes.len() {
            return;
        }
        if ix + 1 < self.sizes.len() {
            self.resize_panel_at_handle(ix, size, window, cx);
        } else if ix > 0 {
            // Last panel: drive its size by resizing the previous sibling so
            // the freed space lands here.
            let delta = self.sizes[ix] - size;
            let prev = self.sizes[ix - 1];
            self.resize_panel_at_handle(ix - 1, prev + delta, window, cx);
        }
        self.done_resizing(cx);
    }

    /// Insert a panel state at `ix`, or append it when no index is supplied.
    ///
    /// Existing panel sizes are redistributed so their total remains equal to
    /// the current container size.
    pub fn insert_panel(
        &mut self,
        size: Option<Pixels>,
        ix: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let panel_state = ResizablePanelState {
            size,
            ..Default::default()
        };

        let size = size.unwrap_or(PANEL_MIN_SIZE);

        // We make sure that the size always sums up to the container size
        // by reducing the size of all other panels first.
        let container_size = self.container_size().max(px(1.));
        let total_leftover_size = (container_size - size).max(px(1.));

        for (i, panel) in self.panels.iter_mut().enumerate() {
            let ratio = self.sizes[i] / container_size;
            self.sizes[i] = total_leftover_size * ratio;
            panel.size = Some(self.sizes[i]);
        }

        if let Some(ix) = ix {
            self.panels.insert(ix, panel_state);
            self.sizes.insert(ix, size);
        } else {
            self.panels.push(panel_state);
            self.sizes.push(size);
        };

        cx.notify();
    }

    /// Adopt slot sizes decided by an owner that keeps its own record of the
    /// layout — the dock's pane tree does.
    ///
    /// Unlike [`Self::insert_panel`], nothing is redistributed: the caller has
    /// already decided how the space divides, and re-normalizing here would
    /// undo exactly that decision. Slots the caller left unconstrained keep
    /// whatever they had.
    pub(crate) fn adopt_sizes(&mut self, sizes: &[Option<Pixels>], cx: &mut Context<Self>) {
        let mut changed = false;
        for (ix, size) in sizes.iter().enumerate() {
            // The preference is mirrored exactly, `None` included. That is the
            // load-bearing half: `insert_panel` resolves every existing
            // panel's `None` into a concrete value as a side effect of
            // redistributing, so after inserting one slot the caller's "these
            // two are equally unconstrained" has quietly become "that one is
            // pinned, this one is the only flexible slot" — and the flexible
            // one then swallows whatever the pinned ones leave over.
            if let Some(panel) = self.panels.get_mut(ix) {
                if panel.size != *size {
                    panel.size = *size;
                    changed = true;
                }
            }

            // The measurement only moves when the tree names a size; an
            // unconstrained slot keeps whatever it was last laid out at until
            // the next pass recomputes it.
            let Some(size) = size else { continue };
            if let Some(slot) = self.sizes.get_mut(ix) {
                if *slot != *size {
                    *slot = *size;
                    changed = true;
                }
            }
        }

        if changed {
            cx.notify();
        }
    }

    pub(crate) fn sync_panels_count(
        &mut self,
        axis: Axis,
        panels_count: usize,
        cx: &mut Context<Self>,
    ) {
        let mut changed = self.axis != axis;
        self.axis = axis;

        if panels_count > self.panels.len() {
            let diff = panels_count - self.panels.len();
            self.panels
                .extend(vec![ResizablePanelState::default(); diff]);
            self.sizes.extend(vec![PANEL_MIN_SIZE; diff]);
            changed = true;
        }

        if panels_count < self.panels.len() {
            self.panels.truncate(panels_count);
            self.sizes.truncate(panels_count);
            changed = true;
        }

        if changed {
            // We need to make sure the total size is in line with the container size.
            self.adjust_to_container_size(cx);
        }
    }

    pub(crate) fn update_panel_size(
        &mut self,
        panel_ix: usize,
        bounds: Bounds<Pixels>,
        size_range: Range<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let size = bounds.size.along(self.axis);
        // This check is only necessary to stop the very first panel from resizing on its own
        // it needs to be passed when the panel is freshly created so we get the initial size,
        // but its also fine when it sometimes passes later.
        if self.sizes[panel_ix].as_f32() == PANEL_MIN_SIZE.as_f32() {
            self.sizes[panel_ix] = size;
            self.panels[panel_ix].size = Some(size);
        }
        self.panels[panel_ix].bounds = bounds;
        self.panels[panel_ix].size_range = size_range;
        cx.notify();
    }

    /// Remove the panel at `panel_ix` and redistribute the remaining space.
    pub fn remove_panel(&mut self, panel_ix: usize, cx: &mut Context<Self>) {
        self.panels.remove(panel_ix);
        self.sizes.remove(panel_ix);
        if let Some(resizing_panel_ix) = self.resizing_panel_ix {
            if resizing_panel_ix > panel_ix {
                self.resizing_panel_ix = Some(resizing_panel_ix - 1);
            }
        }
        self.adjust_to_container_size(cx);
    }

    /// Reset the panel at `panel_ix` while preserving its current size.
    pub fn reset_panel(&mut self, panel_ix: usize, cx: &mut Context<Self>) {
        let old_size = self.sizes[panel_ix];

        self.panels[panel_ix] = ResizablePanelState::default();
        self.sizes[panel_ix] = old_size;
        self.adjust_to_container_size(cx);
    }

    /// Remove all panel state.
    pub fn clear(&mut self) {
        self.panels.clear();
        self.sizes.clear();
    }

    #[inline]
    /// Return the measured size of the group along its resize axis.
    pub fn container_size(&self) -> Pixels {
        self.bounds.size.along(self.axis)
    }

    pub(crate) fn done_resizing(&mut self, cx: &mut Context<Self>) {
        self.resizing_panel_ix = None;
        cx.emit(ResizablePanelEvent::Resized);
    }

    fn panel_size_range(&self, ix: usize) -> Range<Pixels> {
        let Some(panel) = self.panels.get(ix) else {
            return PANEL_MIN_SIZE..Pixels::MAX;
        };

        panel.size_range.clone()
    }

    fn sync_real_panel_sizes(&mut self, _: &App) {
        for (i, panel) in self.panels.iter().enumerate() {
            self.sizes[i] = panel.bounds.size.along(self.axis);
        }
    }

    /// Resize the panel at `ix` by treating `ix` as the drag-handle position
    /// (the handle that sits between panel `ix` and panel `ix + 1`). Returns
    /// early on the last panel since there is no handle below it.
    ///
    /// This is the worker behind drag interactions and the public
    /// [`Self::resize_panel`] API.
    fn resize_panel_at_handle(
        &mut self,
        ix: usize,
        size: Pixels,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let old_sizes = self.sizes.clone();

        let mut ix = ix;
        // Only resize the left panels.
        if ix >= old_sizes.len() - 1 {
            return;
        }
        let container_size = self.container_size();
        self.sync_real_panel_sizes(cx);

        let move_changed = size - old_sizes[ix];
        if move_changed == px(0.) {
            return;
        }

        let size_range = self.panel_size_range(ix);
        let new_size = size.clamp(size_range.start, size_range.end);
        let is_expand = move_changed > px(0.);

        let main_ix = ix;
        let mut new_sizes = old_sizes.clone();

        if is_expand {
            let mut changed = new_size - old_sizes[ix];
            new_sizes[ix] = new_size;

            while changed > px(0.) && ix < old_sizes.len() - 1 {
                ix += 1;
                let size_range = self.panel_size_range(ix);
                let available_size = (new_sizes[ix] - size_range.start).max(px(0.));
                let to_reduce = changed.min(available_size);
                new_sizes[ix] -= to_reduce;
                changed -= to_reduce;
            }
        } else {
            let mut changed = new_size - size;
            new_sizes[ix] = new_size;

            while changed > px(0.) && ix > 0 {
                ix -= 1;
                let size_range = self.panel_size_range(ix);
                let available_size = (new_sizes[ix] - size_range.start).max(px(0.));
                let to_reduce = changed.min(available_size);
                changed -= to_reduce;
                new_sizes[ix] -= to_reduce;
            }

            new_sizes[main_ix + 1] += old_sizes[main_ix] - size - changed;
        }

        // If total size exceeds container size, adjust the main panel
        let total_size: Pixels = new_sizes.iter().map(|s| s.as_f32()).sum::<f32>().into();
        if total_size > container_size {
            let overflow = total_size - container_size;
            new_sizes[main_ix] = (new_sizes[main_ix] - overflow).max(size_range.start);
        }

        for (i, _) in old_sizes.iter().enumerate() {
            let size = new_sizes[i];
            self.panels[i].size = Some(size);
        }
        self.sizes = new_sizes;
        cx.notify();
    }

    /// Adjust panel sizes according to the container size.
    ///
    /// When the container size changes, the panels should take up the same percentage as they did before.
    fn adjust_to_container_size(&mut self, cx: &mut Context<Self>) {
        if self.container_size().is_zero() {
            return;
        }

        // A panel with no size preference is laid out by flex, and its entry
        // in `sizes` is a placeholder until something measures it. Rescaling
        // by a ratio computed from that placeholder drags the panels that
        // *do* have a preference along with it: a 200px sidebar beside one
        // flexible panel comes back 587px wide on the frame after the first,
        // which reads as the layout jumping once for no reason. Flex already
        // fits the container, so there is nothing here to adjust.
        if self.panels.iter().any(|panel| panel.size.is_none()) {
            return;
        }

        let container_size = self.container_size();
        let total = self.sizes.iter().map(|s| s.as_f32()).sum::<f32>();
        if !total.is_finite() || total <= 0. {
            return;
        }
        let total_size = px(total);

        for i in 0..self.panels.len() {
            let size = self.sizes[i];
            let ratio = size / total_size;
            let new_size = container_size * ratio;

            self.sizes[i] = new_size;
            self.panels[i].size = Some(new_size);
        }
        cx.notify();
    }
}

impl EventEmitter<ResizablePanelEvent> for ResizableState {}

#[derive(Debug, Clone, Default)]
pub(crate) struct ResizablePanelState {
    pub size: Option<Pixels>,
    pub size_range: Range<Pixels>,
    bounds: Bounds<Pixels>,
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext as _, Context, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
        ParentElement as _, Pixels, Render, Styled as _, TestAppContext, VisualTestContext, Window,
        div, point, px, size,
    };

    use super::{ResizableState, h_resizable, resizable_panel};

    struct MixedSizingHarness {
        width: Pixels,
    }

    impl Render for MixedSizingHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(self.width).h(px(100.)).child(
                h_resizable("mixed-sizing")
                    .child(
                        resizable_panel()
                            .size(px(240.))
                            .child(div().size_full().debug_selector(|| "fixed-sidebar".into())),
                    )
                    .child(
                        resizable_panel().child(
                            div()
                                .size_full()
                                .debug_selector(|| "flexible-content".into()),
                        ),
                    ),
            )
        }
    }

    #[gpui::test]
    fn mixed_sizing_is_stable_between_resize_and_followup_frame(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| MixedSizingHarness { width: px(800.) });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.draw(cx).clear(cx);
        });
        let before = cx.debug_bounds("fixed-sidebar").unwrap().size.width;

        view.update(cx, |view, cx| {
            view.width = px(1200.);
            cx.notify();
        });
        cx.run_until_parked();
        let settled_frame = cx.debug_bounds("fixed-sidebar").unwrap().size.width;
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let followup_frame = cx.debug_bounds("fixed-sidebar").unwrap().size.width;

        // Resizable panels preserve their proportional sizing across a
        // container resize; the important invariant is that applying the
        // state on the follow-up frame does not move the divider again.
        assert_ne!(settled_frame, before);
        assert_eq!(followup_frame, settled_frame);
    }

    struct CallerStateHarness {
        width: Pixels,
        state: gpui::Entity<ResizableState>,
    }

    impl Render for CallerStateHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(self.width).h(px(100.)).child(
                h_resizable("caller-state")
                    .with_state(&self.state)
                    .child(
                        resizable_panel()
                            .size(px(240.))
                            .child(div().size_full().debug_selector(|| "cs-sidebar".into())),
                    )
                    .child(resizable_panel().child(div().size_full())),
            )
        }
    }

    /// A group whose state the caller owns (`with_state`, as the dock does)
    /// has no `use_keyed_state` observer behind it, so the settling frame has
    /// to be scheduled by the deferred notify rather than by that observer.
    #[gpui::test]
    fn caller_owned_state_settles_on_the_same_frame(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| ResizableState::default()));
        let (view, cx) = cx.add_window_view({
            let state = state.clone();
            move |_, _| CallerStateHarness {
                width: px(800.),
                state,
            }
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.draw(cx).clear(cx);
        });

        view.update(cx, |view, cx| {
            view.width = px(1200.);
            cx.notify();
        });
        cx.run_until_parked();
        let settled = cx.debug_bounds("cs-sidebar").unwrap().size.width;
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let followup = cx.debug_bounds("cs-sidebar").unwrap().size.width;

        assert_eq!(followup, settled, "settling frame must not be pending");
    }

    struct ResizableHarness {
        state: gpui::Entity<ResizableState>,
        resizes: Rc<Cell<usize>>,
    }

    impl Render for ResizableHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.)).h(px(100.)).child(
                h_resizable("resizable")
                    .with_state(&self.state)
                    .on_resize({
                        let resizes = self.resizes.clone();
                        move |_, _, _| resizes.set(resizes.get() + 1)
                    })
                    .child(
                        resizable_panel()
                            .size(px(150.))
                            .child(div().size_full().debug_selector(|| "first-panel".into())),
                    )
                    .child(
                        resizable_panel()
                            .size(px(250.))
                            .child(div().size_full().debug_selector(|| "second-panel".into())),
                    ),
            )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
    ) -> (
        &mut VisualTestContext,
        gpui::Entity<ResizableState>,
        Rc<Cell<usize>>,
    ) {
        let state = cx.update(|cx| cx.new(|_| ResizableState::default()));
        let resizes = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let state = state.clone();
            let resizes = resizes.clone();
            move |_, _| ResizableHarness { state, resizes }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, state, resizes)
    }

    #[gpui::test]
    fn dynamic_panel_lifecycle_is_owned_by_resizable_state(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| ResizableState::default()));

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                state.bounds.size = size(px(400.), px(100.));
                state.panels.push(Default::default());
                state.sizes.push(px(400.));
                state.insert_panel(Some(px(200.)), None, cx);
                assert_eq!(state.sizes(), &vec![px(200.), px(200.)]);

                state.reset_panel(0, cx);
                assert_eq!(state.sizes(), &vec![px(200.), px(200.)]);

                state.remove_panel(0, cx);
                assert_eq!(state.sizes(), &vec![px(400.)]);

                state.clear();
                assert!(state.sizes().is_empty());
            });
        });
    }

    #[gpui::test]
    fn group_measures_panels_and_programmatic_resize_uses_drag_rules(cx: &mut TestAppContext) {
        let (cx, state, _) = harness(cx);
        let first = cx.debug_bounds("first-panel").unwrap();
        let second = cx.debug_bounds("second-panel").unwrap();
        assert_eq!(first.size.width + second.size.width, px(400.));

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.resize_panel(0, px(220.), window, cx);
            });
            window.draw(cx).clear(cx);
        });

        state.read_with(cx, |state, _| {
            assert_eq!(state.sizes(), &vec![px(220.), px(180.)]);
        });
    }

    #[gpui::test]
    fn dragging_the_handle_resizes_and_emits_once(cx: &mut TestAppContext) {
        let (cx, state, resizes) = harness(cx);
        let boundary = cx.debug_bounds("second-panel").unwrap().left();

        cx.simulate_mouse_down(
            point(boundary - px(2.), px(50.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(boundary + px(10.), px(50.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(220.), px(50.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(220.), px(50.)),
            MouseButton::Left,
            Modifiers::default(),
        );

        state.read_with(cx, |state, _| {
            assert_eq!(state.sizes(), &vec![px(220.), px(180.)]);
        });
        assert_eq!(resizes.get(), 1);
    }

    struct SizedGroupHarness;

    impl Render for SizedGroupHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(400.))
                .h(px(100.))
                .child(
                    h_resizable("sized-resizable").size(px(40.)).child(
                        resizable_panel()
                            .child(div().size_full().debug_selector(|| "sized-panel".into())),
                    ),
                )
        }
    }

    #[gpui::test]
    fn a_group_size_binds_the_cross_axis(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| SizedGroupHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let panel = cx.debug_bounds("sized-panel").unwrap();
        assert_eq!(panel.size.width, px(400.));
        assert_eq!(panel.size.height, px(40.));
    }
}

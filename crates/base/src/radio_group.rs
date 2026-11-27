use std::{cell::RefCell, rc::Rc};

use gpui::{
    AnyElement, App, Axis, Bounds, ClickEvent, Div, Element, ElementId, FocusHandle,
    GlobalElementId, InspectorElementId, InteractiveElement, Interactivity, IntoElement,
    KeyDownEvent, KeyboardClickEvent, LayoutId, ParentElement, Pixels, RenderOnce, Role, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, accesskit, div,
};
use smallvec::SmallVec;

use crate::{LayoutDirection, StyledExt as _};

pub(crate) type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// An unstyled container for a set of radio elements.
///
/// Selection stays with the child radios: the application decides which radio
/// is checked and what checking one does. The group supplies the radio-group
/// semantics and the keyboard contract of the ARIA radio group pattern:
///
/// - The group is one tab stop. `Tab` enters it on the checked radio (the
///   first enabled radio when none is checked) and leaves it with the next
///   `Tab`.
/// - Down and Right move focus to the next enabled radio, Up and Left to the
///   previous one, in either axis, wrapping at both ends. In a right-to-left
///   layout Left and Right swap.
/// - Moving with an arrow also checks the radio: its `on_change(true)` runs.
/// - Space checks the focused radio when it is unchecked.
/// - Disabled radios are skipped and never take focus from the keyboard.
///
/// Inside a group, the group decides each radio's tab stop: `tab_index` and
/// `tab_stop` set on a radio apply only to a standalone radio.
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    axis: Axis,
    children: SmallVec<[AnyElement; 4]>,
}

impl RadioGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            axis: Axis::Vertical,
            children: SmallVec::new(),
        }
    }

    /// Sets the semantic axis of the group.
    ///
    /// Assistive technology hears this orientation. The arrow keys work the
    /// same way in both axes, as the radio group pattern asks.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

/// A radio registered with its group during the group's layout, in tree order.
pub(crate) struct RadioEntry {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) checked: bool,
    pub(crate) disabled: bool,
    pub(crate) on_change: Option<ChangeHandler>,
}

#[derive(Default)]
pub(crate) struct RadioRegistry {
    entries: Vec<RadioEntry>,
    /// The radio that is the group's single tab stop, chosen after the last
    /// layout. Radios read it while they render.
    stop: Option<FocusHandle>,
}

impl RadioRegistry {
    /// The registry of the radio group currently laying out, if any.
    pub(crate) fn current() -> Option<RadioRegistryRef> {
        RADIO_GROUP_STACK.with(|stack| stack.borrow().last().cloned())
    }

    pub(crate) fn is_stop(&self, focus_handle: &FocusHandle) -> bool {
        self.stop.as_ref() == Some(focus_handle)
    }

    pub(crate) fn register(&mut self, entry: RadioEntry) {
        self.entries.push(entry);
    }
}

pub(crate) type RadioRegistryRef = Rc<RefCell<RadioRegistry>>;

thread_local! {
    static RADIO_GROUP_STACK: RefCell<Vec<RadioRegistryRef>> = const { RefCell::new(Vec::new()) };
}

/// Wraps a radio group so the radios rendered while it lays out register with
/// it, the same way [`crate::Tabs`] collects its tabs: radios nested in
/// wrappers still count, and the registry matches what was rendered last.
struct RadioGroupScope {
    registry: RadioRegistryRef,
    inner: AnyElement,
}

impl Element for RadioGroupScope {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.registry.borrow_mut().entries.clear();
        RADIO_GROUP_STACK.with(|stack| stack.borrow_mut().push(self.registry.clone()));
        let layout_id = self.inner.request_layout(window, cx);
        RADIO_GROUP_STACK.with(|stack| stack.borrow_mut().pop());
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        sync_tab_stop(&self.registry, window);
        self.inner.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner.paint(window, cx);
    }
}

/// Make exactly one enabled radio the group's tab stop: the focused radio
/// while the group has focus, otherwise the checked radio, otherwise the first.
fn sync_tab_stop(registry: &RadioRegistryRef, window: &mut Window) {
    let mut registry = registry.borrow_mut();
    let enabled = |entry: &&RadioEntry| !entry.disabled;
    let stop = registry
        .entries
        .iter()
        .filter(enabled)
        .find(|entry| entry.focus_handle.is_focused(window))
        .or_else(|| registry.entries.iter().filter(enabled).find(|e| e.checked))
        .or_else(|| registry.entries.iter().find(|entry| !entry.disabled))
        .map(|entry| entry.focus_handle.clone());

    if registry.stop != stop {
        registry.stop = stop;
        // Radios pick up the new stop when they next render.
        window.refresh();
    }
}

enum Step {
    Next,
    Previous,
}

fn handle_key_down(
    registry: &RadioRegistryRef,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut App,
) {
    // A parent that already handled the key (Questionnaire moves its own
    // radio cards in the capture phase) owns it.
    if event.keystroke.modifiers.modified() || window.default_prevented() {
        return;
    }
    let rtl = LayoutDirection::global(cx).is_rtl();
    let step = match event.keystroke.key.as_str() {
        "down" => Step::Next,
        "up" => Step::Previous,
        "right" if rtl => Step::Previous,
        "right" => Step::Next,
        "left" if rtl => Step::Next,
        "left" => Step::Previous,
        _ => return,
    };

    let (target, on_change) = {
        let entries = &registry.borrow().entries;
        let Some(current) = entries
            .iter()
            .position(|entry| entry.focus_handle.is_focused(window))
        else {
            return;
        };
        let enabled: Vec<usize> = (0..entries.len())
            .filter(|&ix| !entries[ix].disabled)
            .collect();
        let (Some(&first), Some(&last)) = (enabled.first(), enabled.last()) else {
            return;
        };
        let target = match step {
            Step::Next => enabled
                .iter()
                .copied()
                .find(|&ix| ix > current)
                .unwrap_or(first),
            Step::Previous => enabled
                .iter()
                .rev()
                .copied()
                .find(|&ix| ix < current)
                .unwrap_or(last),
        };
        let entry = &entries[target];
        let on_change = (!entry.checked).then(|| entry.on_change.clone()).flatten();
        (entry.focus_handle.clone(), on_change)
    };

    cx.stop_propagation();
    window.prevent_default();
    window.focus(&target, cx);
    sync_tab_stop(registry, window);
    // Selection follows focus in a radio group. The handler runs after the
    // registry borrow ends, because it usually rerenders the group.
    if let Some(on_change) = on_change {
        let event = ClickEvent::Keyboard(KeyboardClickEvent::default());
        on_change(true, &event, window, cx);
    }
}

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for RadioGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for RadioGroup {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for RadioGroup {}

impl RenderOnce for RadioGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            base,
            style,
            axis,
            children,
        } = self;

        // The registry outlives the frame so the key handler sees the radios
        // rendered last, in the order the user sees them.
        let registry = window
            .use_keyed_state(id, cx, |_, _| RadioRegistryRef::default())
            .read(cx)
            .clone();
        let key_handler = {
            let registry = registry.clone();
            move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                handle_key_down(&registry, event, window, cx);
            }
        };

        RadioGroupScope {
            registry,
            inner: radio_group(base, axis)
                .on_key_down(key_handler)
                .children(children)
                .refine_style(&style)
                .into_any_element(),
        }
    }
}

/// The group's own semantics: its role and orientation.
fn radio_group(base: Stateful<Div>, axis: Axis) -> Stateful<Div> {
    base.role(Role::RadioGroup).aria_orientation(match axis {
        Axis::Horizontal => accesskit::Orientation::Horizontal,
        Axis::Vertical => accesskit::Orientation::Vertical,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Radio;
    use gpui::{Context, Render, TestAppContext, VisualTestContext, px};

    /// Four radios, the third disabled, between two plain focus stops.
    struct Harness {
        axis: Axis,
        checked: Option<usize>,
        changes: Rc<RefCell<Vec<usize>>>,
        before: FocusHandle,
        after: FocusHandle,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let radios = (0..4).map(|ix| {
                let changes = self.changes.clone();
                let entity = cx.entity();
                Radio::new(("radio", ix))
                    .size(px(20.))
                    .checked(self.checked == Some(ix))
                    .disabled(ix == 2)
                    // Explicit settings must not split the group's single stop.
                    .tab_index(5)
                    .tab_stop(true)
                    .on_change(move |checked, _, _, cx| {
                        assert!(checked);
                        changes.borrow_mut().push(ix);
                        entity.update(cx, |this, cx| {
                            this.checked = Some(ix);
                            cx.notify();
                        });
                    })
            });
            div()
                .child(div().id("before").size(px(10.)).track_focus(&self.before))
                .child(RadioGroup::new("group").axis(self.axis).children(radios))
                .child(div().id("after").size(px(10.)).track_focus(&self.after))
        }
    }

    struct Fixture<'a> {
        cx: &'a mut VisualTestContext,
        view: gpui::Entity<Harness>,
        changes: Rc<RefCell<Vec<usize>>>,
        after: FocusHandle,
    }

    impl Fixture<'_> {
        fn draw(&mut self) {
            self.cx.update(|window, cx| window.draw(cx).clear(cx));
        }

        fn checked(&mut self) -> Option<usize> {
            self.cx.run_until_parked();
            self.view.read_with(self.cx, |this, _| this.checked)
        }

        /// The radio that has focus, found by asking Space to check it.
        fn focused(&mut self) -> Option<usize> {
            self.draw();
            self.changes.borrow_mut().clear();
            // Force the focused radio unchecked so Space reports it.
            self.view.update(self.cx, |this, cx| {
                this.checked = None;
                cx.notify();
            });
            self.draw();
            self.cx.simulate_keystrokes("space");
            self.cx.simulate_event(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse("space").unwrap(),
            });
            self.changes.borrow().last().copied()
        }
    }

    fn fixture(cx: &mut TestAppContext, axis: Axis, checked: Option<usize>) -> Fixture<'_> {
        let changes: Rc<RefCell<Vec<usize>>> = Rc::default();
        let (view, cx) = cx.add_window_view({
            let changes = changes.clone();
            move |window, cx| {
                let before = cx.focus_handle().tab_stop(true);
                before.focus(window, cx);
                Harness {
                    axis,
                    checked,
                    changes,
                    before,
                    after: cx.focus_handle().tab_stop(true),
                }
            }
        });
        let after = view.read_with(cx, |this, _| this.after.clone());
        // The second frame applies the tab stop chosen by the first.
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| window.focus_next(cx));
        Fixture {
            cx,
            view,
            changes,
            after,
        }
    }

    #[gpui::test]
    fn tab_enters_on_the_checked_radio_and_leaves_with_one_press(cx: &mut TestAppContext) {
        let mut f = fixture(cx, Axis::Vertical, Some(1));
        assert_eq!(f.focused(), Some(1));
        f.cx.update(|window, cx| window.focus_next(cx));
        f.cx.update(|window, _| assert!(f.after.is_focused(window)));
    }

    #[gpui::test]
    fn without_a_checked_radio_the_first_enabled_radio_is_the_stop(cx: &mut TestAppContext) {
        let mut f = fixture(cx, Axis::Vertical, None);
        assert_eq!(f.focused(), Some(0));
    }

    #[gpui::test]
    fn arrows_move_check_skip_disabled_radios_and_wrap(cx: &mut TestAppContext) {
        let mut f = fixture(cx, Axis::Vertical, Some(0));

        f.cx.simulate_keystrokes("down");
        assert_eq!(*f.changes.borrow(), vec![1], "moving checks the radio");
        assert_eq!(f.checked(), Some(1));

        f.draw();
        f.cx.simulate_keystrokes("down");
        assert_eq!(f.checked(), Some(3), "the disabled radio is skipped");
        f.draw();
        f.cx.simulate_keystrokes("right");
        assert_eq!(f.checked(), Some(0), "wraps past the last radio");
        f.draw();
        f.cx.simulate_keystrokes("left");
        assert_eq!(f.checked(), Some(3), "wraps before the first radio");
        f.draw();
        f.cx.simulate_keystrokes("up");
        assert_eq!(f.checked(), Some(1), "skips the disabled radio back");
        assert_eq!(f.focused(), Some(1), "focus moved with the selection");

        // Leaving after moving still takes one press.
        f.cx.update(|window, cx| window.focus_next(cx));
        f.cx.update(|window, _| assert!(f.after.is_focused(window)));
    }

    #[gpui::test]
    fn a_horizontal_group_uses_all_four_arrows(cx: &mut TestAppContext) {
        let mut f = fixture(cx, Axis::Horizontal, Some(0));
        f.cx.simulate_keystrokes("right");
        assert_eq!(f.checked(), Some(1));
        f.draw();
        f.cx.simulate_keystrokes("up");
        assert_eq!(f.checked(), Some(0));
    }

    #[gpui::test]
    fn left_and_right_follow_the_global_rtl_direction(cx: &mut TestAppContext) {
        let mut f = fixture(cx, Axis::Horizontal, Some(0));
        f.cx.update(|_, cx| crate::LayoutDirection::Rtl.set_global(cx));
        f.cx.simulate_keystrokes("left");
        assert_eq!(f.checked(), Some(1), "the next radio sits to the left");
        f.draw();
        f.cx.simulate_keystrokes("right");
        assert_eq!(f.checked(), Some(0));
    }

    #[gpui::test]
    fn only_one_radio_is_a_tab_stop(cx: &mut TestAppContext) {
        let f = fixture(cx, Axis::Vertical, Some(3));
        f.cx.update(|window, cx| {
            window.focus_next(cx);
            assert!(f.after.is_focused(window), "one stop: Tab leaves at once");
            window.focus_prev(cx);
            window.focus_prev(cx);
            let before = f.view.read(cx).before.clone();
            assert!(before.is_focused(window), "and one stop going back");
        });
    }
}

//! The popup behind [`Input::list`](super::Input::list): a [`List`] under the
//! field that searches with the field's text, the counterpart of an HTML
//! `<input list>`.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Context, Edges, Entity, Focusable as _, InteractiveElement,
    IntoElement, MouseDownEvent, ParentElement, Pixels, Styled as _, Subscription, Task, Window,
    deferred, rems,
};
use gpui_base::{DeferredPopover, GlobalState};

use super::{Enter, Escape, InputEvent, InputState, MoveDown, MoveUp};
use crate::{
    ElementExt as _, Sizable as _, Size, StyledExt as _, ThemeStyled as _,
    list::{List, ListDelegate, ListEvent, ListState},
    sizing::DROPDOWN_LIST_PADDING,
    v_flex,
};

/// [`ListState`] with its delegate type erased, so that `Input` stays
/// non-generic.
pub(super) trait InputList {
    fn id(&self) -> u64;
    fn watch(&self, menu: &mut Context<Menu>, window: &mut Window) -> [Subscription; 2];
    fn search(&self, query: String, window: &mut Window, cx: &mut App);
    fn cancel_search(&self, cx: &mut App);
    fn is_showing(&self, cx: &App) -> bool;
    fn has_items(&self, cx: &App) -> bool;
    fn has_selection(&self, cx: &App) -> bool;
    fn select_first(&self, window: &mut Window, cx: &mut App);
    fn select_next(&self, window: &mut Window, cx: &mut App);
    fn select_prev(&self, window: &mut Window, cx: &mut App);
    fn confirm(&self, window: &mut Window, cx: &mut App);
    fn cancel(&self, window: &mut Window, cx: &mut App);
    fn render(&self, size: Size) -> AnyElement;
}

impl<D: ListDelegate> InputList for Entity<ListState<D>> {
    fn id(&self) -> u64 {
        self.entity_id().as_u64()
    }

    fn watch(&self, cx: &mut Context<Menu>, window: &mut Window) -> [Subscription; 2] {
        [
            // A search that finishes changes what shows, and re-renders only
            // the list unless the view holding the input hears of it too.
            cx.observe(self, |menu, _, cx| {
                menu.sync_token(cx);
                cx.notify();
            }),
            cx.subscribe_in(self, window, |menu, _, event: &ListEvent, _, cx| {
                match event {
                    ListEvent::Confirm(_) => menu.accept(cx),
                    ListEvent::Cancel => menu.close(cx),
                    ListEvent::Select(_) => {}
                }
                cx.notify();
            }),
        ]
    }

    fn search(&self, query: String, window: &mut Window, cx: &mut App) {
        self.update(cx, |list, cx| list.search_from_input(query, window, cx));
    }

    fn cancel_search(&self, cx: &mut App) {
        self.update(cx, |list, _| list.cancel_search());
    }

    fn is_showing(&self, cx: &App) -> bool {
        self.read(cx).delegate().loading(cx) || self.has_items(cx)
    }

    fn has_items(&self, cx: &App) -> bool {
        self.read(cx).has_items(cx)
    }

    fn has_selection(&self, cx: &App) -> bool {
        self.read(cx).selected_item_exists(cx)
    }

    fn select_first(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |list, cx| list.select_first_item(window, cx));
    }

    fn select_next(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |list, cx| {
            list.on_action_select_next(&crate::actions::SelectDown, window, cx)
        });
    }

    fn select_prev(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |list, cx| list.select_prev_or_none(window, cx));
    }

    fn confirm(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |list, cx| {
            list.on_action_confirm(&crate::actions::Confirm { secondary: false }, window, cx)
        });
    }

    fn cancel(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |list, cx| {
            list.on_action_cancel(&crate::actions::Cancel, window, cx)
        });
    }

    fn render(&self, size: Size) -> AnyElement {
        List::new(self)
            .with_size(size)
            .max_h(rems(20.))
            .paddings(Edges::all(DROPDOWN_LIST_PADDING))
            .into_any_element()
    }
}

/// What the popup remembers between frames, keyed to its input and list.
pub(super) struct Menu {
    list: Rc<dyn InputList>,
    open: bool,
    /// The query the list last searched for.
    searched: Option<String>,
    /// The input's text as last seen, so that a change event that leaves the
    /// text as it was starts nothing.
    text: Option<String>,
    /// The search the last change asked for. It starts after the change's
    /// effects are done, so that accepting a row, which writes the text before
    /// the list reports the accept, can drop it.
    pending_search: Task<()>,
    /// Set from accepting a row until its effects are done: a change in that
    /// time is the accepted value being written, wherever it is written from.
    accepting: bool,
    _end_accepting: Task<()>,
    /// The input's bounds as last laid out, which the popup is placed against
    /// and which a press may land in without closing it.
    field: Bounds<Pixels>,
    /// Held only while the popup shows, so the input's context menu keeps
    /// working when there is nothing to show.
    _deferred: Option<DeferredPopover>,
    _subscriptions: [Subscription; 3],
}

impl Menu {
    fn new(
        input: &Entity<InputState>,
        list: Rc<dyn InputList>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let [observe, events] = list.watch(cx, window);
        Self {
            list,
            open: false,
            searched: None,
            text: None,
            pending_search: Task::ready(()),
            accepting: false,
            _end_accepting: Task::ready(()),
            field: Bounds::default(),
            _deferred: None,
            _subscriptions: [
                observe,
                events,
                cx.subscribe_in(input, window, Self::on_input_event),
            ],
        }
    }

    fn on_input_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            // Only what the user types searches: a change made in code to an
            // input they are not in leaves the list alone.
            InputEvent::Change if input.focus_handle(cx).is_focused(window) => {
                let query = input.read(cx).value().trim().to_string();
                if self.text.as_ref() == Some(&query) {
                    return;
                }
                self.text = Some(query.clone());
                if self.accepting {
                    return;
                }
                self.pending_search = cx.spawn_in(window, async move |menu, cx| {
                    _ = menu.update_in(cx, |menu, window, cx| {
                        menu.open = true;
                        menu.search(query, window, cx);
                        cx.notify();
                    });
                });
                return;
            }
            InputEvent::Blur => self.close(cx),
            _ => return,
        }
        cx.notify();
    }

    /// A row was accepted, by key or click: close, and let the text it writes
    /// start no search. The write may come before this, from the delegate's
    /// `confirm`, or after it, from a `ListEvent::Confirm` subscription.
    fn accept(&mut self, cx: &mut Context<Self>) {
        self.list.cancel_search(cx);
        // The list's rows are for the text before the accept.
        self.searched = None;
        self.accepting = true;
        self._end_accepting = cx.spawn(async move |menu, cx| {
            _ = menu.update(cx, |menu, _| menu.accepting = false);
        });
        self.close(cx);
    }

    fn search(&mut self, query: String, window: &mut Window, cx: &mut Context<Self>) {
        self.searched = Some(query.clone());
        self.list.search(query, window, cx);
        self.sync_token(cx);
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.pending_search = Task::ready(());
        self.sync_token(cx);
    }

    fn is_showing(&self, cx: &App) -> bool {
        self.open && self.list.is_showing(cx)
    }

    fn sync_token(&mut self, cx: &mut Context<Self>) {
        let showing = self.is_showing(cx);
        if showing != self._deferred.is_some() {
            self._deferred = showing.then(|| GlobalState::register_deferred_popover(cx));
        }
    }

    /// Down on a closed popup: show results for the current text, searching
    /// first when the list holds results for some other text.
    fn open_with(&mut self, query: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.pending_search = Task::ready(());
        if self.searched.as_ref() == Some(&query) {
            self.list.select_first(window, cx);
            self.sync_token(cx);
        } else {
            self.search(query, window, cx);
        }
        cx.notify();
    }
}

/// One input's list popup for one frame.
pub(super) struct ListPopup {
    menu: Entity<Menu>,
    input: Entity<InputState>,
    list: Rc<dyn InputList>,
    visible: bool,
    editable: bool,
    size: Size,
    field: Bounds<Pixels>,
}

impl ListPopup {
    /// The popup for `input`. A disabled or read-only input offers none, and
    /// the popup shows only while the input has focus.
    pub(super) fn new(
        input: &Entity<InputState>,
        list: Rc<dyn InputList>,
        editable: bool,
        size: Size,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let key = format!("input-list-{}-{}", input.entity_id().as_u64(), list.id());
        let menu = window.use_keyed_state(gpui::ElementId::Name(key.into()), cx, {
            let list = list.clone();
            |window, cx| Menu::new(input, list, window, cx)
        });
        let visible =
            editable && input.focus_handle(cx).is_focused(window) && menu.read(cx).is_showing(cx);
        let field = menu.read(cx).field;
        Self {
            menu,
            input: input.clone(),
            list,
            visible,
            editable,
            size,
            field,
        }
    }

    /// Add the keys, bounds observer and popup to the input's root.
    ///
    /// The keys are captured, so they reach the list before the input's own
    /// handlers, and only while they have something to do: otherwise Up and
    /// Down move the caret, Escape and Enter reach the input as usual. The
    /// list never takes focus, so its own key bindings do not run.
    pub(super) fn attach<E>(self, element: E, cx: &App) -> E
    where
        E: InteractiveElement + ParentElement,
    {
        let Self {
            menu,
            input,
            list,
            visible,
            editable,
            size,
            field,
        } = self;

        // Read when a key arrives rather than at render: a change or a
        // search may have opened or closed the popup since.
        let showing = {
            let menu = menu.clone();
            move |cx: &App| editable && menu.read(cx).is_showing(cx)
        };

        let mut element = element
            .on_prepaint({
                let menu = menu.clone();
                move |bounds, _, cx| menu.update(cx, |menu, _| menu.field = bounds)
            })
            .capture_action({
                let (menu, input, list) = (menu.clone(), input.clone(), list.clone());
                let showing = showing.clone();
                move |_: &MoveDown, window, cx| {
                    if !editable {
                        return;
                    }
                    if showing(cx) {
                        list.select_next(window, cx);
                    } else {
                        let query = input.read(cx).value().trim().to_string();
                        menu.update(cx, |menu, cx| menu.open_with(query, window, cx));
                        // With nothing to offer, Down is the input's.
                        if !showing(cx) {
                            return;
                        }
                    }
                    cx.stop_propagation();
                }
            })
            .capture_action({
                let (list, showing) = (list.clone(), showing.clone());
                move |_: &MoveUp, window, cx| {
                    if showing(cx) {
                        list.select_prev(window, cx);
                        cx.stop_propagation();
                    }
                }
            })
            .capture_action({
                let (list, showing) = (list.clone(), showing.clone());
                move |_: &Escape, window, cx| {
                    if showing(cx) {
                        list.cancel(window, cx);
                        cx.stop_propagation();
                    }
                }
            })
            .capture_action({
                let (menu, list) = (menu.clone(), list.clone());
                move |_: &Enter, window, cx| {
                    if showing(cx) && list.has_selection(cx) {
                        list.confirm(window, cx);
                        cx.stop_propagation();
                    } else {
                        // The input submits its text. A search the typing
                        // asked for, or results still on their way, must not
                        // open the popup afterwards.
                        menu.update(cx, |menu, cx| {
                            menu.close(cx);
                            cx.notify();
                        });
                    }
                }
            });
        if visible {
            element.extend([popup(&menu, &input, &*list, size, field, cx)]);
        }
        element
    }
}

/// The list under the field, as wide as it, placed and animated as every
/// dropdown is: it flips above or stays inside the window near an edge.
fn popup(
    menu: &Entity<Menu>,
    input: &Entity<InputState>,
    list: &dyn InputList,
    size: Size,
    field: Bounds<Pixels>,
    cx: &App,
) -> AnyElement {
    deferred(crate::popover::dropdown_popup(
        ("input-list", input.entity_id()),
        field,
        v_flex()
            .occlude()
            .w(field.size.width)
            .popover_style(cx)
            // Claimed in the capture phase, before the list's focus handle
            // would take focus from the input.
            .capture_any_mouse_down(|_, window, _| window.prevent_default())
            .on_mouse_down_out({
                let menu = menu.clone();
                move |event: &MouseDownEvent, _, cx| {
                    menu.update(cx, |menu, cx| {
                        if !menu.field.contains(&event.position) {
                            menu.close(cx);
                            cx.notify();
                        }
                    })
                }
            })
            .child(list.render(size)),
        cx,
    ))
    .with_priority(gpui_base::POPUP_PRIORITY)
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, ops::Deref as _, time::Duration};

    use gpui::{
        AppContext as _, Modifiers, Render, Task, TestAppContext, VisualTestContext, div, point, px,
    };

    use super::*;
    use crate::{IndexPath, Root, input::Input, list::ListItem};

    const ALL: [&str; 4] = [
        "cpu.load",
        "cpu.temperature",
        "memory.used",
        "disk.cpu_wait",
    ];

    /// Searches `ALL`; with `latency`, after a timer, as a remote search does.
    struct Delegate {
        input: Entity<InputState>,
        items: Vec<&'static str>,
        selected: Option<IndexPath>,
        latency: Duration,
        write: Write,
        started: Rc<RefCell<Vec<String>>>,
    }

    impl ListDelegate for Delegate {
        type Item = ListItem;

        fn perform_search(
            &mut self,
            query: &str,
            window: &mut Window,
            cx: &mut Context<ListState<Self>>,
        ) -> Task<()> {
            let find = |query: &str| {
                ALL.into_iter()
                    .filter(|item| item.contains(query) && *item != query)
                    .collect::<Vec<_>>()
            };
            if self.latency.is_zero() {
                self.started.borrow_mut().push(query.to_string());
                self.items = find(query);
                return Task::ready(());
            }
            let (query, latency, started) = (query.to_string(), self.latency, self.started.clone());
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(latency).await;
                started.borrow_mut().push(query.clone());
                _ = this.update(cx, |this, cx| {
                    this.delegate_mut().items = find(&query);
                    cx.notify();
                });
            })
        }

        fn items_count(&self, _: usize, _: &App) -> usize {
            self.items.len()
        }

        fn render_item(
            &mut self,
            ix: IndexPath,
            _: &mut Window,
            _: &mut Context<ListState<Self>>,
        ) -> Option<ListItem> {
            let item = self.items.get(ix.row)?;
            Some(ListItem::new(ix.row).child(*item))
        }

        fn set_selected_index(
            &mut self,
            ix: Option<IndexPath>,
            _: &mut Window,
            _: &mut Context<ListState<Self>>,
        ) {
            self.selected = ix;
        }

        fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
            if self.write != Write::InConfirm {
                return;
            }
            let Some(item) = self.selected.and_then(|ix| self.items.get(ix.row)) else {
                return;
            };
            self.input
                .update(cx, |input, cx| input.replace_all(*item, window, cx));
        }
    }

    /// Where the accepted value is written.
    #[derive(Clone, Copy, Default, PartialEq)]
    enum Write {
        /// In the delegate's `confirm`, before the list reports the accept.
        #[default]
        InConfirm,
        /// In a `ListEvent::Confirm` subscription that hears it before the popup.
        OnEventFirst,
        /// In a `ListEvent::Confirm` subscription that hears it after the popup.
        OnEventLast,
    }

    #[derive(Clone, Copy, Default)]
    struct Mode {
        disabled: bool,
        readonly: bool,
        unfocused: bool,
        latency: Duration,
        write: Write,
    }

    struct Harness {
        input: Entity<InputState>,
        list: Entity<ListState<Delegate>>,
        mode: Mode,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                div().w(px(300.)).child(
                    Input::new(&self.input)
                        .list(&self.list)
                        .disabled(self.mode.disabled)
                        .readonly(self.mode.readonly),
                ),
            )
        }
    }

    struct Fixture {
        input: Entity<InputState>,
        started: Rc<RefCell<Vec<String>>>,
        entered: Rc<std::cell::Cell<usize>>,
        cx: VisualTestContext,
    }

    impl Fixture {
        fn draw(&mut self) {
            self.cx.run_until_parked();
            self.cx.update(|window, cx| window.draw(cx).clear(cx));
        }

        fn wait(&mut self, ms: u64) {
            self.cx.executor().advance_clock(Duration::from_millis(ms));
            self.draw();
        }

        fn typed(&mut self, text: &str) {
            self.cx.simulate_input(text);
            self.draw();
        }

        fn keys(&mut self, keys: &str) {
            for key in keys.split(' ') {
                self.cx.simulate_keystrokes(key);
                self.draw();
            }
        }

        fn value(&mut self) -> String {
            self.input
                .read_with(&self.cx, |input, _| input.value().to_string())
        }

        fn click_first_row(&mut self) {
            let field_bottom = self.cx.update(|window, _| window.rem_size() * 2.25);
            self.cx
                .simulate_click(point(px(40.), field_bottom + px(16.)), Modifiers::default());
            self.draw();
        }

        fn input_focused(&mut self) -> bool {
            let input = self.input.clone();
            self.cx
                .update(|window, cx| input.focus_handle(cx).is_focused(window))
        }
    }

    fn mount(cx: &mut TestAppContext, mode: Mode) -> Fixture {
        cx.update(crate::init);
        let started: Rc<RefCell<Vec<String>>> = Default::default();
        let slot = Rc::new(RefCell::new(None));
        let (captured, log) = (slot.clone(), started.clone());
        let window = cx.add_window(move |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            let delegate = Delegate {
                input: input.clone(),
                items: Vec::new(),
                selected: None,
                latency: mode.latency,
                write: mode.write,
                started: log,
            };
            let list = cx.new(|cx| ListState::new(delegate, window, cx));
            *captured.borrow_mut() = Some((input.clone(), list.clone()));
            let harness = cx.new(|_| Harness { input, list, mode });
            Root::new(harness, window, cx)
        });
        let (input, list): (Entity<InputState>, Entity<ListState<Delegate>>) =
            slot.borrow().clone().unwrap();
        let mut cx = VisualTestContext::from_window(*window.deref(), cx);
        if !mode.unfocused {
            cx.update(|window, cx| input.update(cx, |input, cx| input.focus(window, cx)));
        }
        let entered = Rc::new(std::cell::Cell::new(0));
        cx.update(|_, cx| {
            let entered = entered.clone();
            cx.subscribe(&input, move |_, event: &InputEvent, _| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    entered.set(entered.get() + 1);
                }
            })
            .detach();
        });
        // The popup subscribes to the list when it first renders.
        if mode.write == Write::OnEventFirst {
            write_on_confirm_event(&input, &list, &mut cx);
        }
        let mut fixture = Fixture {
            input: input.clone(),
            started,
            entered,
            cx,
        };
        fixture.draw();
        if mode.write == Write::OnEventLast {
            write_on_confirm_event(&input, &list, &mut fixture.cx);
        }
        fixture
    }

    fn write_on_confirm_event(
        input: &Entity<InputState>,
        list: &Entity<ListState<Delegate>>,
        cx: &mut VisualTestContext,
    ) {
        let input = input.clone();
        cx.update(|window, cx| {
            let window = window.window_handle();
            cx.subscribe(list, move |list, event: &ListEvent, cx| {
                let ListEvent::Confirm(ix) = event else {
                    return;
                };
                let Some(item) = list.read(cx).delegate().items.get(ix.row).copied() else {
                    return;
                };
                _ = window.update(cx, |_, window, cx| {
                    input.update(cx, |input, cx| input.replace_all(item, window, cx))
                });
            })
            .detach();
        });
    }

    #[gpui::test]
    fn free_text_stays_and_enter_submits_it(cx: &mut TestAppContext) {
        let mut f = mount(cx, Mode::default());
        // Rows are showing, and none is highlighted until Down.
        f.typed("cpu");
        f.keys("enter");
        assert_eq!(f.value(), "cpu");
        assert_eq!(f.entered.get(), 1);

        f.typed(".nothing");
        f.keys("enter");
        assert_eq!(f.value(), "cpu.nothing");
        assert_eq!(f.entered.get(), 2);
    }

    #[gpui::test]
    fn the_keyboard_accepts_a_row_without_searching_again(cx: &mut TestAppContext) {
        let mut f = mount(cx, Mode::default());
        f.typed("cpu");
        f.keys("down down enter");
        assert_eq!(f.value(), "cpu.temperature");
        assert_eq!(f.entered.get(), 0, "accepting is not submitting");

        // The popup closed, so the next Enter is the input's.
        f.keys("enter");
        assert_eq!(f.entered.get(), 1);
        assert_eq!(f.value(), "cpu.temperature");

        // Up from the first row returns to the typed text.
        f.typed("x");
        f.keys("backspace down up enter");
        assert_eq!(f.value(), "cpu.temperature");
        assert_eq!(f.entered.get(), 2);
    }

    #[gpui::test]
    fn accepting_is_one_undoable_edit(cx: &mut TestAppContext) {
        let mut f = mount(cx, Mode::default());
        f.typed("mem");
        f.keys("down enter");
        assert_eq!(f.value(), "memory.used");
        f.keys(if cfg!(target_os = "macos") {
            "cmd-z"
        } else {
            "ctrl-z"
        });
        assert_eq!(f.value(), "mem");
    }

    #[gpui::test]
    fn escape_closes_and_down_reopens(cx: &mut TestAppContext) {
        let mut f = mount(cx, Mode::default());
        f.typed("mem");
        f.keys("escape enter");
        assert_eq!(f.value(), "mem");
        assert_eq!(f.entered.get(), 1);

        // Down opens with the first row highlighted.
        f.keys("down enter");
        assert_eq!(f.value(), "memory.used");
    }

    #[gpui::test]
    fn clicking_a_row_accepts_it_and_keeps_focus(cx: &mut TestAppContext) {
        let mut f = mount(cx, Mode::default());
        f.typed("mem");
        f.click_first_row();
        assert_eq!(f.value(), "memory.used");
        assert!(f.input_focused());
    }

    #[gpui::test]
    fn clicking_away_closes_the_popup(cx: &mut TestAppContext) {
        let mut f = mount(cx, Mode::default());
        f.typed("cpu");
        f.keys("down");
        f.cx.simulate_click(point(px(600.), px(400.)), Modifiers::default());
        f.draw();
        f.keys("enter");
        assert_eq!(f.value(), "cpu");
        assert_eq!(f.entered.get(), 1);
    }

    #[gpui::test]
    fn a_newer_query_cancels_an_older_one(cx: &mut TestAppContext) {
        let mut f = mount(
            cx,
            Mode {
                latency: Duration::from_millis(200),
                ..Mode::default()
            },
        );
        f.typed("c");
        f.wait(100);
        f.typed("p");
        f.wait(100);
        f.typed("u");
        f.wait(300);
        assert_eq!(*f.started.borrow(), ["cpu"]);

        // Async results reach the view holding the input, so the row is there.
        f.click_first_row();
        assert_eq!(f.value(), "cpu.load");
    }

    /// Accepting a row searches nothing and leaves the popup closed, wherever
    /// the value is written; typing afterwards searches again.
    fn accept_and_type_on(cx: &mut TestAppContext, write: Write, latency_ms: u64) {
        let mut f = mount(
            cx,
            Mode {
                latency: Duration::from_millis(latency_ms),
                write,
                ..Mode::default()
            },
        );
        f.typed("cpu");
        f.wait(300);
        f.started.borrow_mut().clear();
        f.keys("down enter");
        f.wait(500);
        assert_eq!(f.value(), "cpu.load");
        assert!(f.started.borrow().is_empty(), "searched after accepting");

        // The popup stayed closed, so Enter is the input's.
        f.keys("enter");
        assert_eq!(f.entered.get(), 1);
        assert_eq!(f.value(), "cpu.load");

        f.typed("x");
        f.wait(300);
        assert_eq!(*f.started.borrow(), ["cpu.loadx"]);
    }

    #[gpui::test]
    fn accepting_with_a_local_search_searches_nothing(cx: &mut TestAppContext) {
        accept_and_type_on(cx, Write::InConfirm, 0);
    }

    #[gpui::test]
    fn writing_on_the_confirm_event_heard_first(cx: &mut TestAppContext) {
        accept_and_type_on(cx, Write::OnEventFirst, 0);
    }

    #[gpui::test]
    fn writing_on_the_confirm_event_heard_last(cx: &mut TestAppContext) {
        accept_and_type_on(cx, Write::OnEventLast, 0);
    }

    #[gpui::test]
    fn writing_on_the_confirm_event_with_a_remote_search(cx: &mut TestAppContext) {
        accept_and_type_on(cx, Write::OnEventFirst, 200);
    }

    #[gpui::test]
    fn accepting_starts_no_remote_search(cx: &mut TestAppContext) {
        let mut f = mount(
            cx,
            Mode {
                latency: Duration::from_millis(200),
                ..Mode::default()
            },
        );
        f.typed("mem");
        f.wait(300);
        f.started.borrow_mut().clear();
        f.keys("down enter");
        f.wait(500);
        assert_eq!(f.value(), "memory.used");
        assert!(f.started.borrow().is_empty());
    }

    #[gpui::test]
    fn results_after_submit_do_not_open_the_popup(cx: &mut TestAppContext) {
        let mut f = mount(
            cx,
            Mode {
                latency: Duration::from_millis(200),
                ..Mode::default()
            },
        );
        f.typed("mem");
        f.keys("enter");
        assert_eq!(f.entered.get(), 1);
        f.wait(300);
        f.click_first_row();
        assert_eq!(f.value(), "mem");
    }

    #[gpui::test]
    fn the_context_menu_token_is_held_only_while_rows_show(cx: &mut TestAppContext) {
        let mut f = mount(cx, Mode::default());
        let held = |f: &mut Fixture| f.cx.update(|_, cx| GlobalState::is_in_deferred_context(cx));
        f.typed("cpu");
        assert!(held(&mut f));
        f.typed(".nothing");
        assert!(!held(&mut f));
    }

    #[gpui::test]
    fn a_change_in_code_to_an_unfocused_input_searches_nothing(cx: &mut TestAppContext) {
        let mut f = mount(
            cx,
            Mode {
                unfocused: true,
                ..Mode::default()
            },
        );
        f.input.update_in(&mut f.cx, |input, window, cx| {
            input.replace("cpu", window, cx)
        });
        f.draw();
        assert!(f.started.borrow().is_empty());
    }

    #[gpui::test]
    fn disabled_and_read_only_inputs_offer_nothing(cx: &mut TestAppContext) {
        for mode in [
            Mode {
                disabled: true,
                ..Mode::default()
            },
            Mode {
                readonly: true,
                ..Mode::default()
            },
        ] {
            let mut f = mount(cx, mode);
            f.keys("down enter");
            assert!(f.started.borrow().is_empty());
            assert_eq!(f.value(), "");
        }
    }
}

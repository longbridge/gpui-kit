//! Retained state that scripts hold by handle.
//!
//! The object model has three classes (design doc §7): values are copied,
//! element descriptions live for one script render, and **entities** live across
//! frames and are owned by GPUI. A script never holds an entity directly — it
//! holds a handle into a store, so a released entity produces a clear error
//! instead of a dangling reference.
//!
//! # One store per runtime
//!
//! The store is a field of the runtime, not a process- or thread-global. Two
//! runtimes in one process — a host with two plugins, a test that builds two —
//! must not be able to reach each other's state, and the way to guarantee that
//! is for there to be no shared store to reach through.
//!
//! A handle carries the store it came from in its high bits, so the invariant is
//! also *checked* rather than merely arranged: a handle from another runtime
//! resolves to nothing and reports itself, instead of quietly resolving to
//! whatever that index happens to hold here.

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

use gpui::{App, AppContext as _, Entity, EntityId, FocusHandle, Subscription, Window};
use gpui_base::VirtualListScrollHandle;
use gpui_base::dock::{DockArea, DockEvent};
use gpui_base::input::{
    InputBaseState, InputEditorStyle, InputEvent, InputModeKind, InputState, TextareaState,
};
use gpui_base::slider::{SliderEvent, SliderScale, SliderState, SliderValue};
use gpui_base::{CalendarEvent, CalendarState, OtpEvent, OtpState};

use crate::{
    dock::{DockChromeSlots, DockContexts, ScriptDockSkin},
    engine::ShellRuntime,
    runtime::ApplicationGeneration,
    view::ScriptView,
};

/// A script-visible reference to retained state.
///
/// The high [`STORE_SHIFT`] bits name the store, the low bits are a monotonic
/// id that is never reused. The 53-bit layout stays exactly representable by a
/// JavaScript number.
pub type EntityHandle = u64;

const STORE_SHIFT: u32 = 32;
const MAX_STORE_ID: u32 = (1 << 21) - 1;
const ENTITY_ID_MASK: u64 = u32::MAX as u64;
pub(crate) const MAX_LIVE_ENTITIES: usize = 10_000;

/// The runtime-wide retained-state budget is full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EntityLimitError;

/// What a handle points at. One variant per entity type the script can create.
enum Record {
    /// A retained script view. Its entity owns the child snapshot lifecycle;
    /// this record owns the script-visible handle.
    View {
        view: Entity<ScriptView>,
        application: Option<Rc<ApplicationGeneration>>,
        runtime: Weak<ShellRuntime>,
    },
    Input {
        state: Entity<InputState>,
        application: Option<Rc<ApplicationGeneration>>,
        /// Subscriptions are stored, not returned, because a dropped
        /// `Subscription` stops delivering: a script that registers a handler
        /// and moves on would otherwise silently receive nothing.
        subscriptions: Vec<Subscription>,
    },
    /// Multi-line text state.
    ///
    /// A separate variant rather than a flag on [`Record::Input`] because
    /// `TextareaState` is a different Rust type — the same editing engine
    /// specialized on a different mode — and `Textarea::new` will not accept an
    /// `InputState`. The two share their event type and almost all of their
    /// methods, which is why everything below this point treats them together.
    Textarea {
        state: Entity<TextareaState>,
        application: Option<Rc<ApplicationGeneration>>,
        subscriptions: Vec<Subscription>,
    },
    /// A calendar's month, view and selected date.
    ///
    /// Retained because the month a script is looking at outlives the frame
    /// that drew it, and because the day grid is derived from it: `next_month`
    /// moves the state and the next `month_days()` answers a different grid.
    Calendar {
        state: Entity<CalendarState>,
        application: Option<Rc<ApplicationGeneration>>,
        subscriptions: Vec<Subscription>,
    },
    /// A slider's value, bounds and scale.
    ///
    /// Retained because it is what a drag writes to: the pointer moves, GPUI's
    /// listener updates this, and the next frame reads it back — all without
    /// the description being rebuilt, which is what keeps a drag off the VM.
    Slider {
        state: Entity<SliderState>,
        application: Option<Rc<ApplicationGeneration>>,
        subscriptions: Vec<Subscription>,
    },
    /// A one-time code's digits, focus and blink.
    ///
    /// Retained for the reason an input's text is, and for one more: the blink
    /// runs on a timer that notifies this entity twice a second. Neither the
    /// digits nor the blink ever reach the script, so neither can live in a
    /// description that only the script can rebuild.
    Otp {
        state: Entity<OtpState>,
        application: Option<Rc<ApplicationGeneration>>,
        subscriptions: HashMap<OtpEventName, OtpSubscription>,
    },
    /// A focus handle the script created and hands to elements.
    ///
    /// Retained for the same reason an input's state is: focus is a fact about
    /// the window that outlives any one render, and an element rebuilt every
    /// frame cannot own it. It is what lets a script say *which* control the
    /// keyboard is on, and what a `Select` or a `DatePicker` will be
    /// constructed from — their focus handle is a required argument, not a
    /// builder call.
    Focus {
        handle: FocusHandle,
        application: Option<Rc<ApplicationGeneration>>,
    },
    /// A dockable layout: its trees, its docks, its panel entities, and the
    /// skin drawing all of it.
    ///
    /// Retained for the reason nothing else here is quite: the layout is what
    /// the *user* changed. A drag, a resize, a closed tab and a collapsed dock
    /// all happen without a script render, and rebuilding the area from a
    /// description would put every one of them back the way the script last
    /// described it.
    Dock {
        area: Entity<DockArea>,
        /// Which script handler draws each piece of chrome for the frame being
        /// drawn. Shared with the skin installed in `area`, which reads it.
        slots: Rc<DockChromeSlots>,
        /// The contexts that skin was handed while it drew, so a command
        /// arriving from a later event handler can find its own.
        contexts: Rc<DockContexts>,
        application: Option<Rc<ApplicationGeneration>>,
        subscriptions: Vec<Subscription>,
    },
    /// The scroll position of a virtualized list, and the item it has been
    /// asked to scroll to.
    ///
    /// Retained for a reason the other variants do not share: a
    /// `VirtualListScrollHandle` is *where a request is left*.
    /// `scroll_to_item` records an index that the list consumes during its next
    /// prepaint, so a handle rebuilt each frame would drop every request made
    /// between two frames — which is every request a script can make.
    VirtualScroll {
        handle: VirtualListScrollHandle,
        application: Option<Rc<ApplicationGeneration>>,
    },
}

type OtpHandler = dyn Fn(&OtpEvent, &mut Window, &mut App);

/// One native subscription per event name. Re-registering replaces only the
/// script handler behind it, so the subscription count stays bounded and GPUI's
/// deferred activation cannot race an old subscription's cancellation.
struct OtpSubscription {
    _subscription: Subscription,
    handler: Rc<RefCell<Box<OtpHandler>>>,
}

/// One retained record together with the exact view that created it.
///
/// Application ownership answers unload. View ownership answers nested release:
/// removing one child also removes handles created by that child without
/// touching its siblings or application-owned records.
struct StoredRecord {
    record: Record,
    owner: Option<EntityId>,
}

thread_local! {
    /// Handed out at construction so two stores never share an id. Thread-local
    /// because the VM and GPUI's `App` are both main-thread only.
    static NEXT_STORE_ID: Cell<u32> = const { Cell::new(0) };
}

/// The retained state of one runtime.
///
/// Created by the runtime and dropped with it, which is what releases every
/// entity the scripts of that runtime created — a store that outlived its app
/// would show up as a leaked handle at shutdown.
pub struct EntityStore {
    id: u32,
    next_id: u32,
    records: HashMap<u32, StoredRecord>,
}

/// The store-only half of releasing retained state.
///
/// Records are removed while the store is mutably borrowed, then this plan is
/// retired after that borrow has ended. In particular, `Entity::update` must
/// never run while `EntityStore`'s `RefCell` is borrowed.
pub(crate) struct EntityRelease {
    handles: HashSet<EntityHandle>,
    _records: Vec<StoredRecord>,
    views: Vec<(Entity<ScriptView>, Weak<ShellRuntime>)>,
}

impl EntityRelease {
    fn empty() -> Self {
        Self {
            handles: HashSet::new(),
            _records: Vec::new(),
            views: Vec::new(),
        }
    }

    pub(crate) fn contains(&self, handle: EntityHandle) -> bool {
        self.handles.contains(&handle)
    }

    fn cancel_owned_work(&self) {
        for (view, runtime) in &self.views {
            if let Some(runtime) = runtime.upgrade() {
                runtime.retire_view_callbacks(view.entity_id());
                crate::engine::quickjs::cancel_view_tasks(&runtime, view.entity_id());
            }
        }
    }

    /// Makes every released view inert before the plan and its records drop.
    pub(crate) fn retire(self, cx: &mut impl gpui::AppContext) {
        self.cancel_owned_work();
        for (view, _) in &self.views {
            view.update(cx, |view, _| view.retire());
        }
        // `records` and the extra entity clones drop only after every snapshot
        // has been cleared. Script values therefore remain inside VM lifetime.
    }

    /// Best-effort cleanup for `Drop` paths where GPUI provides no context.
    /// Removing the store records and cancelling owned work is still exact;
    /// any frame-retained entity becomes unreachable when its root frame drops.
    pub(crate) fn retire_without_context(self) {
        self.cancel_owned_work();
    }
}

#[derive(Clone, Copy)]
pub(crate) struct EntityCheckpoint {
    next_id: u32,
}

impl EntityStore {
    pub(crate) fn kind(&self, handle: EntityHandle) -> Option<&'static str> {
        match self.record(handle)? {
            Record::View { .. } => Some("View"),
            Record::Input { .. } => Some("InputState"),
            Record::Textarea { .. } => Some("TextareaState"),
            Record::Calendar { .. } => Some("CalendarState"),
            Record::Slider { .. } => Some("SliderState"),
            Record::Otp { .. } => Some("OtpState"),
            Record::Focus { .. } => Some("FocusHandle"),
            Record::Dock { .. } => Some("DockArea"),
            Record::VirtualScroll { .. } => Some("VirtualListScrollHandle"),
        }
    }

    /// Creates a store with a process-unique JavaScript-safe namespace.
    ///
    /// Exhaustion is reported instead of wrapping: reusing a namespace could
    /// make a stale handle from an earlier runtime name a new runtime's entity.
    pub fn try_new() -> Option<Self> {
        let id = NEXT_STORE_ID.with(|next| {
            let (id, following) = allocate_store_id(next.get())?;
            next.set(following);
            Some(id)
        })?;
        Some(Self {
            id,
            next_id: 0,
            records: HashMap::new(),
        })
    }

    /// Retains a nested script view and returns its script-visible handle.
    pub(crate) fn create_view(
        &mut self,
        view: Entity<ScriptView>,
        application: Option<Rc<ApplicationGeneration>>,
        runtime: &Rc<ShellRuntime>,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.push(Record::View {
            view,
            application,
            runtime: Rc::downgrade(runtime),
        })
    }

    /// The nested script view behind a handle, if it is live and belongs here.
    pub(crate) fn view(&self, handle: EntityHandle) -> Option<Entity<ScriptView>> {
        match self.record(handle) {
            Some(Record::View { view, .. }) => Some(view.clone()),
            _ => None,
        }
    }

    /// Creates an input state and returns its handle.
    ///
    /// The editor style is installed here rather than left to the caller because
    /// `InputEditorStyle::default()` leaves its colors unset so Base can resolve
    /// them from the active palette at paint time. `gpui-base` installs a
    /// readable light palette by default, and embedders can replace it.
    pub fn create_input(
        &mut self,
        placeholder: Option<String>,
        value: Option<String>,
        application: Option<Rc<ApplicationGeneration>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.ensure_capacity()?;
        let state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            if let Some(placeholder) = placeholder {
                state = state.placeholder(placeholder);
            }
            state.set_editor_style(editor_style());
            state
        });

        if let Some(value) = value {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }

        self.push(Record::Input {
            state,
            application,
            subscriptions: Vec::new(),
        })
    }

    /// The entity behind an input handle, if it is still live and belongs here.
    pub fn input(&self, handle: EntityHandle) -> Option<Entity<InputState>> {
        match self.record(handle) {
            Some(Record::Input { state, .. }) => Some(state.clone()),
            _ => None,
        }
    }

    /// Typed native state access for registered component adapters. Script
    /// views and adapter-owned opaque values are deliberately not included.
    pub(crate) fn native_state<T: 'static>(&self, handle: EntityHandle) -> Option<Entity<T>> {
        let entity: &dyn std::any::Any = match self.record(handle)? {
            Record::Input { state, .. } => state,
            Record::Textarea { state, .. } => state,
            Record::Calendar { state, .. } => state,
            Record::Slider { state, .. } => state,
            Record::Otp { state, .. } => state,
            Record::Dock { area, .. } => area,
            _ => return None,
        };
        entity.downcast_ref::<Entity<T>>().cloned()
    }

    /// Creates a multi-line text state and returns its handle.
    ///
    /// `rows` is offered at construction because the layout default is a single
    /// row *even for a textarea* — being multi-line is carried by the mode
    /// rather than by the layout — so a script that asked for a textarea and
    /// said nothing else would get something the height of an input.
    ///
    /// The editor style is installed for the same reason as in
    /// [`Self::create_input`]: the default one is entirely transparent.
    pub fn create_textarea(
        &mut self,
        placeholder: Option<String>,
        value: Option<String>,
        rows: Option<usize>,
        application: Option<Rc<ApplicationGeneration>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.ensure_capacity()?;
        let state = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx);
            if let Some(placeholder) = placeholder {
                state = state.placeholder(placeholder);
            }
            if let Some(rows) = rows {
                state = state.rows(rows);
            }
            state.set_editor_style(editor_style());
            state
        });

        if let Some(value) = value {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }

        self.push(Record::Textarea {
            state,
            application,
            subscriptions: Vec::new(),
        })
    }

    /// The entity behind a textarea handle, if it is still live and belongs
    /// here.
    pub fn textarea(&self, handle: EntityHandle) -> Option<Entity<TextareaState>> {
        match self.record(handle) {
            Some(Record::Textarea { state, .. }) => Some(state.clone()),
            _ => None,
        }
    }

    /// Creates a slider state and returns its handle.
    ///
    /// Everything is set at construction because base's own builders take
    /// `self` by value, and their order matters: `scale` asserts against the
    /// bounds already set, and the defaults it would otherwise check —
    /// `0..100` — are not bounds a logarithmic slider can have. The caller has
    /// already refused anything these would assert on; reaching one of them
    /// here would abort the host rather than report a script mistake.
    ///
    /// Unlike [`Self::create_input`] this needs no window: a `SliderState` is
    /// a plain value until something draws it, and only `set_value` — which
    /// the script reaches through a live host call — asks for one.
    ///
    /// Eight arguments because a slider is defined by five numbers and there is
    /// no moment between `new` and the first read at which to set them: base's
    /// builders take `self` by value.
    #[allow(clippy::too_many_arguments)]
    pub fn create_slider(
        &mut self,
        min: f32,
        max: f32,
        step: f32,
        scale: SliderScale,
        value: SliderValue,
        application: Option<Rc<ApplicationGeneration>>,
        cx: &mut App,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.ensure_capacity()?;
        let state = cx.new(|_| {
            SliderState::new()
                .min(min)
                .max(max)
                .step(step)
                .scale(scale)
                .default_value(value)
        });

        self.push(Record::Slider {
            state,
            application,
            subscriptions: Vec::new(),
        })
    }

    /// The entity behind a slider handle, if it is still live and belongs here.
    pub fn slider(&self, handle: EntityHandle) -> Option<Entity<SliderState>> {
        match self.record(handle) {
            Some(Record::Slider { state, .. }) => Some(state.clone()),
            _ => None,
        }
    }

    /// Creates a one-time-code state and returns its handle.
    ///
    /// `length` is a constructor argument rather than a builder because it is
    /// one in base: the number of cells is what an `OtpState` *is*, and there
    /// is no setter for it.
    ///
    /// Unlike [`Self::create_slider`] this needs a window: the state observes
    /// window activation so that coming back to the application restarts the
    /// blink on a code that still holds the keyboard.
    pub fn create_otp(
        &mut self,
        length: usize,
        value: Option<String>,
        masked: bool,
        application: Option<Rc<ApplicationGeneration>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.ensure_capacity()?;
        let state = cx.new(|cx| {
            let state = OtpState::new(length, window, cx).masked(masked);
            match value {
                Some(value) => state.default_value(value),
                None => state,
            }
        });

        self.push(Record::Otp {
            state,
            application,
            subscriptions: HashMap::new(),
        })
    }

    /// Creates a calendar state and returns its handle.
    ///
    /// Needs a window for the same reason [`Self::create_otp`] does: the state
    /// builds a focus handle in its constructor.
    pub fn create_calendar(
        &mut self,
        application: Option<Rc<ApplicationGeneration>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.ensure_capacity()?;
        let state = cx.new(|cx| CalendarState::new(window, cx));
        self.push(Record::Calendar {
            state,
            application,
            subscriptions: Vec::new(),
        })
    }

    /// The entity behind a calendar handle, if it is still live and belongs
    /// here.
    pub fn calendar(&self, handle: EntityHandle) -> Option<Entity<CalendarState>> {
        match self.record(handle) {
            Some(Record::Calendar { state, .. }) => Some(state.clone()),
            _ => None,
        }
    }

    /// Subscribes to a calendar's one event: a date was selected.
    ///
    /// A `Vec` of subscriptions rather than a map keyed by event name, as the
    /// slider's is: `CalendarEvent` has one variant, so there is nothing to
    /// key by and nothing a second registration could mean but "also this".
    pub fn subscribe_calendar(
        &mut self,
        handle: EntityHandle,
        window: &mut Window,
        cx: &mut App,
        handler: impl Fn(&CalendarEvent, &mut Window, &mut App) + 'static,
    ) -> bool {
        let state = match self.record(handle) {
            Some(Record::Calendar { state, .. }) => state.clone(),
            _ => return false,
        };
        let subscription =
            window.subscribe(&state, cx, move |_, event: &CalendarEvent, window, cx| {
                handler(event, window, cx)
            });
        match self.record_mut(handle) {
            Some(Record::Calendar { subscriptions, .. }) => {
                // Replaces rather than appends, matching every other `on(...)`
                // in this API: registering twice means the second handler, not
                // both of them.
                subscriptions.clear();
                subscriptions.push(subscription);
                true
            }
            _ => false,
        }
    }

    /// Creates a dock area and returns its handle.
    ///
    /// The skin is installed here rather than left to the caller because
    /// [`DockArea::with_renderer`] is a constructor step and base offers no way
    /// to replace a renderer afterwards. What *is* replaceable is the set of
    /// script handlers the skin forwards to, which is what
    /// [`DockChromeSlots`] carries — so one skin built once serves every
    /// snapshot the script publishes.
    pub(crate) fn create_dock(
        &mut self,
        id: &str,
        version: Option<usize>,
        skin: ScriptDockSkin,
        application: Option<Rc<ApplicationGeneration>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.ensure_capacity()?;
        let contexts = skin.contexts();
        let slots = skin.slots();
        let id = id.to_owned();
        let area = cx.new(|cx| DockArea::new(id, version, window, cx).with_renderer(Rc::new(skin)));
        self.push(Record::Dock {
            area,
            slots,
            contexts,
            application,
            subscriptions: Vec::new(),
        })
    }

    /// The area behind a dock handle, if it is still live and belongs here.
    pub(crate) fn dock(&self, handle: EntityHandle) -> Option<Entity<DockArea>> {
        match self.record(handle) {
            Some(Record::Dock { area, .. }) => Some(area.clone()),
            _ => None,
        }
    }

    /// Where the next frame's chrome handlers are written.
    pub(crate) fn dock_slots(&self, handle: EntityHandle) -> Option<Rc<DockChromeSlots>> {
        match self.record(handle) {
            Some(Record::Dock { slots, .. }) => Some(slots.clone()),
            _ => None,
        }
    }

    /// The contexts the last drawn frame recorded, which is what a command from
    /// a script event handler is resolved against.
    pub(crate) fn dock_contexts(&self, handle: EntityHandle) -> Option<Rc<DockContexts>> {
        match self.record(handle) {
            Some(Record::Dock { contexts, .. }) => Some(contexts.clone()),
            _ => None,
        }
    }

    /// Subscribes to a dock area's layout changes.
    ///
    /// One `Vec` and one handler, as the calendar's is: a second registration
    /// means the second handler rather than both, which is what every other
    /// `on(...)` in this API means.
    pub(crate) fn subscribe_dock(
        &mut self,
        handle: EntityHandle,
        window: &mut Window,
        cx: &mut App,
        handler: impl Fn(&DockEvent, &mut Window, &mut App) + 'static,
    ) -> bool {
        let area = match self.record(handle) {
            Some(Record::Dock { area, .. }) => area.clone(),
            _ => return false,
        };
        let subscription = window.subscribe(&area, cx, move |_, event: &DockEvent, window, cx| {
            handler(event, window, cx)
        });
        match self.record_mut(handle) {
            Some(Record::Dock { subscriptions, .. }) => {
                subscriptions.clear();
                subscriptions.push(subscription);
                true
            }
            _ => false,
        }
    }

    /// The entity behind a one-time-code handle, if it is still live and
    /// belongs here.
    pub fn otp(&self, handle: EntityHandle) -> Option<Entity<OtpState>> {
        match self.record(handle) {
            Some(Record::Otp { state, .. }) => Some(state.clone()),
            _ => None,
        }
    }

    /// Creates a focus handle and returns its handle.
    ///
    /// Only `&App` is needed — GPUI's own [`App::focus_handle`] takes no window
    /// — but this is still refused during render for the same reason
    /// [`Self::create_input`] is: a handle created inside `render` would be a
    /// new one on every frame, so the focus a script thought it was tracking
    /// would be dropped by the next repaint.
    pub fn create_focus(
        &mut self,
        application: Option<Rc<ApplicationGeneration>>,
        cx: &mut App,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.push(Record::Focus {
            handle: cx.focus_handle(),
            application,
        })
    }

    /// The focus handle behind a handle, if it is still live and belongs here.
    pub fn focus(&self, handle: EntityHandle) -> Option<FocusHandle> {
        match self.record(handle) {
            Some(Record::Focus { handle, .. }) => Some(handle.clone()),
            _ => None,
        }
    }

    /// Creates a virtualized list's scroll position and returns its handle.
    ///
    /// Refused during render for the same reason the rest are: a handle created
    /// there would be a new one every frame, and the position the script
    /// thought it was holding — along with any pending `scroll_to_item` — would
    /// be dropped by the next repaint.
    pub fn create_virtual_scroll(
        &mut self,
        application: Option<Rc<ApplicationGeneration>>,
    ) -> Result<EntityHandle, EntityLimitError> {
        self.push(Record::VirtualScroll {
            handle: VirtualListScrollHandle::new(),
            application,
        })
    }

    /// The scroll position behind a handle, if it is still live and belongs
    /// here.
    pub fn virtual_scroll(&self, handle: EntityHandle) -> Option<VirtualListScrollHandle> {
        match self.record(handle) {
            Some(Record::VirtualScroll { handle, .. }) => Some(handle.clone()),
            _ => None,
        }
    }

    /// Subscribes to one input event for as long as the handle lives.
    ///
    /// The subscription is owned by the store rather than by the script: a
    /// script has no place to keep it, and a handler that stops firing because a
    /// value was dropped is the kind of bug nobody finds.
    ///
    /// One method serves both text states: they emit the same [`InputEvent`],
    /// so only the entity's type differs, and that difference is confined to
    /// the two arms that hand it to [`subscribe_to_events`].
    pub fn subscribe_input(
        &mut self,
        handle: EntityHandle,
        event: InputEventName,
        window: &mut Window,
        cx: &mut App,
        handler: impl Fn(&InputEvent, &mut Window, &mut App) + 'static,
    ) -> bool {
        let subscription = match self.record(handle) {
            Some(Record::Input { state, .. }) => {
                subscribe_to_events(&state.clone(), event, window, cx, handler)
            }
            Some(Record::Textarea { state, .. }) => {
                subscribe_to_events(&state.clone(), event, window, cx, handler)
            }
            _ => return false,
        };

        match self.record_mut(handle) {
            Some(Record::Input { subscriptions, .. } | Record::Textarea { subscriptions, .. }) => {
                subscriptions.push(subscription);
                true
            }
            _ => false,
        }
    }

    /// Subscribes to one slider event for as long as the handle lives.
    ///
    /// Kept apart from [`Self::subscribe_input`] rather than made generic over
    /// both: a slider event carries a value, and a handler that had to accept
    /// either payload would be a handler that names neither.
    pub fn subscribe_slider(
        &mut self,
        handle: EntityHandle,
        event: SliderEventName,
        window: &mut Window,
        cx: &mut App,
        handler: impl Fn(SliderValue, &mut Window, &mut App) + 'static,
    ) -> bool {
        // Resolved and cloned inside the match so the immutable borrow ends
        // before the subscription is stored through a mutable one.
        let state = match self.record(handle) {
            Some(Record::Slider { state, .. }) => state.clone(),
            _ => return false,
        };

        let subscription =
            window.subscribe(&state, cx, move |_, emitted: &SliderEvent, window, cx| {
                if let Some(value) = event.value(emitted) {
                    handler(value, window, cx);
                }
            });

        match self.record_mut(handle) {
            Some(Record::Slider { subscriptions, .. }) => {
                subscriptions.push(subscription);
                true
            }
            _ => false,
        }
    }

    /// Subscribes to one one-time-code event for as long as the handle lives.
    ///
    /// Kept apart from [`Self::subscribe_input`] even though both carry an
    /// [`InputEvent`]: that one is generic over the text engine's mode marker
    /// and an `OtpState` is not built on it. What a script sees differs too —
    /// there is no `submit`, and completion has its own event rather than
    /// overloading `change`.
    pub fn subscribe_otp(
        &mut self,
        handle: EntityHandle,
        event: OtpEventName,
        window: &mut Window,
        cx: &mut App,
        handler: impl Fn(&OtpEvent, &mut Window, &mut App) + 'static,
    ) -> bool {
        // Resolved and cloned inside the match so the immutable borrow ends
        // before the subscription is stored through a mutable one.
        let state = match self.record(handle) {
            Some(Record::Otp { state, .. }) => state.clone(),
            _ => return false,
        };

        if let Some(Record::Otp { subscriptions, .. }) = self.record_mut(handle) {
            if let Some(subscription) = subscriptions.get_mut(&event) {
                *subscription.handler.borrow_mut() = Box::new(handler);
                return true;
            }
        }

        let handler: Rc<RefCell<Box<OtpHandler>>> = Rc::new(RefCell::new(Box::new(handler)));
        let dispatch = handler.clone();
        let subscription =
            window.subscribe(&state, cx, move |_, emitted: &OtpEvent, window, cx| {
                if event.matches(emitted) {
                    dispatch.borrow()(emitted, window, cx);
                }
            });

        match self.record_mut(handle) {
            Some(Record::Otp { subscriptions, .. }) => {
                subscriptions.insert(
                    event,
                    OtpSubscription {
                        _subscription: subscription,
                        handler,
                    },
                );
                true
            }
            _ => false,
        }
    }

    /// Drops a handle. The entity itself is released when GPUI has no other
    /// owner.
    pub fn release(&mut self, handle: EntityHandle) -> bool {
        let Some(id) = self.entity_id(handle) else {
            return false;
        };
        let Some(record) = self.records.get(&id) else {
            return false;
        };
        if matches!(record.record, Record::View { .. }) {
            return false;
        }
        self.records.remove(&id).is_some()
    }

    /// Releases a retained view and everything created under its ownership.
    ///
    /// This is deliberately typed: retiring a view must synchronously clear
    /// its snapshots and callback generations even if GPUI still holds the
    /// entity for a rendered frame.
    pub(crate) fn release_view(&mut self, handle: EntityHandle) -> Option<EntityRelease> {
        let Some(id) = self.entity_id(handle) else {
            return None;
        };
        let Some(Record::View { view, .. }) = self.records.get(&id).map(|stored| &stored.record)
        else {
            return None;
        };

        // A child can create retained state, and eventually other child views,
        // during init, events and tasks. Discover the complete ownership tree
        // before removing anything so no drop can re-enter a half-mutated map.
        let mut owners = HashSet::from([view.entity_id()]);
        let mut removed = HashSet::from([id]);
        loop {
            let mut changed = false;
            for (candidate_id, stored) in &self.records {
                if removed.contains(candidate_id)
                    || !stored.owner.is_some_and(|owner| owners.contains(&owner))
                {
                    continue;
                }
                removed.insert(*candidate_id);
                if let Record::View { view, .. } = &stored.record {
                    owners.insert(view.entity_id());
                }
                changed = true;
            }
            if !changed {
                break;
            }
        }

        Some(self.take_records(removed))
    }

    /// Releases every handle. The runtime dropping the store does this anyway;
    /// this is for a host that wants the entities gone before the VM is.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Releases retained state created by one evaluated application.
    ///
    /// Dropping an input record also drops its GPUI subscriptions, so unload
    /// cannot leave a handler (and its persistent JavaScript function) behind
    /// in the runtime-wide store.
    pub(crate) fn release_application(
        &mut self,
        application: &Rc<ApplicationGeneration>,
    ) -> EntityRelease {
        let removed = self.records.iter().filter_map(|(id, stored)| {
            let owner = match &stored.record {
                Record::View {
                    application: owner, ..
                }
                | Record::Input {
                    application: owner, ..
                }
                | Record::Otp {
                    application: owner, ..
                }
                | Record::Calendar {
                    application: owner, ..
                }
                | Record::Textarea {
                    application: owner, ..
                }
                | Record::Slider {
                    application: owner, ..
                }
                | Record::Focus {
                    application: owner, ..
                }
                | Record::VirtualScroll {
                    application: owner, ..
                }
                | Record::Dock {
                    application: owner, ..
                } => owner,
            };
            owner
                .as_ref()
                .is_some_and(|owner| Rc::ptr_eq(owner, application))
                .then_some(*id)
        });
        self.take_records(removed.collect())
    }

    /// Marks a boundary from which newly retained records can be rolled back.
    pub(crate) fn checkpoint(&self) -> EntityCheckpoint {
        EntityCheckpoint {
            next_id: self.next_id,
        }
    }

    /// Removes records allocated after `checkpoint` without reusing their ids.
    pub(crate) fn rollback(&mut self, checkpoint: EntityCheckpoint) -> EntityRelease {
        let removed = self
            .records
            .keys()
            .copied()
            .filter(|id| *id >= checkpoint.next_id)
            .collect();
        self.take_records(removed)
    }

    fn take_records(&mut self, ids: HashSet<u32>) -> EntityRelease {
        if ids.is_empty() {
            return EntityRelease::empty();
        }
        let handles = ids
            .iter()
            .map(|id| (u64::from(self.id) << STORE_SHIFT) | u64::from(*id))
            .collect();
        let mut records = Vec::with_capacity(ids.len());
        let mut views = Vec::new();
        for id in ids {
            if let Some(stored) = self.records.remove(&id) {
                if let Record::View { view, runtime, .. } = &stored.record {
                    views.push((view.clone(), runtime.clone()));
                }
                records.push(stored);
            }
        }
        EntityRelease {
            handles,
            _records: records,
            views,
        }
    }

    /// How many handles are live, for tests that assert the store does not grow
    /// without bound. Nothing in the runtime asks: the capacity is enforced
    /// inside [`Self::push`], which is the only way a record gets in.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every focus handle this store holds, in creation order.
    ///
    /// Ordered because a test asserting *which* control the keyboard landed on
    /// has to be able to name the second one.
    #[cfg(test)]
    pub(crate) fn focus_handles(&self) -> Vec<FocusHandle> {
        let mut ids: Vec<u32> = self.records.keys().copied().collect();
        ids.sort_unstable();
        ids.iter()
            .filter_map(
                |id| match self.records.get(id).map(|stored| &stored.record) {
                    Some(Record::Focus { handle, .. }) => Some(handle.clone()),
                    _ => None,
                },
            )
            .collect()
    }

    /// The first slider state this store holds.
    ///
    /// For the test that changes a value from Rust and asserts the frame that
    /// follows never entered the VM — which is the whole claim of the slider
    /// binding, and is not observable from the script side.
    #[cfg(test)]
    pub(crate) fn first_slider(&self) -> Option<Entity<SliderState>> {
        self.records
            .values()
            .find_map(|stored| match &stored.record {
                Record::Slider { state, .. } => Some(state.clone()),
                _ => None,
            })
    }

    /// The first one-time-code state this store holds.
    ///
    /// For the test that changes the code from Rust and asserts the frame that
    /// follows never entered the VM — which is the whole claim of this
    /// binding, and is not observable from the script side.
    #[cfg(test)]
    pub(crate) fn first_otp(&self) -> Option<Entity<OtpState>> {
        self.records
            .values()
            .find_map(|stored| match &stored.record {
                Record::Otp { state, .. } => Some(state.clone()),
                _ => None,
            })
    }

    /// For the test that asserts a single date is stored as `Date::Single`.
    ///
    /// Not observable from the script side: both `Single(Some(d))` and
    /// `Range(Some(d), None)` read back as the same string, which is exactly
    /// how storing the wrong one went unnoticed.
    #[cfg(test)]
    pub(crate) fn first_calendar(&self) -> Option<Entity<CalendarState>> {
        self.records
            .values()
            .find_map(|stored| match &stored.record {
                Record::Calendar { state, .. } => Some(state.clone()),
                _ => None,
            })
    }

    #[cfg(test)]
    pub(crate) fn first_input(&self) -> Option<Entity<InputState>> {
        self.records
            .values()
            .find_map(|stored| match &stored.record {
                Record::Input { state, .. } => Some(state.clone()),
                _ => None,
            })
    }

    /// Splits a handle into its entity id, refusing one that names another store.
    ///
    /// A cross-store handle is a host bug rather than a script mistake — a
    /// script can only ever have been given handles from its own runtime — so
    /// this logs rather than throwing, and resolves to nothing.
    fn entity_id(&self, handle: EntityHandle) -> Option<u32> {
        let store = (handle >> STORE_SHIFT) as u32;
        if store != self.id {
            tracing::error!(
                "entity handle {handle} belongs to store {store}, not to store {}",
                self.id
            );
            return None;
        }
        Some((handle & ENTITY_ID_MASK) as u32)
    }

    fn record(&self, handle: EntityHandle) -> Option<&Record> {
        self.records
            .get(&self.entity_id(handle)?)
            .map(|stored| &stored.record)
    }

    fn record_mut(&mut self, handle: EntityHandle) -> Option<&mut Record> {
        let id = self.entity_id(handle)?;
        self.records.get_mut(&id).map(|stored| &mut stored.record)
    }

    fn push(&mut self, record: Record) -> Result<EntityHandle, EntityLimitError> {
        self.ensure_capacity()?;
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("a shell runtime cannot create more than 2^32 retained entities");
        self.records.insert(
            id,
            StoredRecord {
                record,
                owner: crate::scope::current_view().map(|view| view.entity_id()),
            },
        );
        Ok((u64::from(self.id) << STORE_SHIFT) | u64::from(id))
    }

    fn ensure_capacity(&self) -> Result<(), EntityLimitError> {
        (self.records.len() < MAX_LIVE_ENTITIES)
            .then_some(())
            .ok_or(EntityLimitError)
    }
}

/// Delivers one named event from any of the text states to `handler`.
///
/// Generic over the mode marker rather than written once per state: the filter
/// and the subscription are identical, and duplicating them would let the two
/// drift into answering `change` differently.
fn subscribe_to_events<M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    event: InputEventName,
    window: &mut Window,
    cx: &mut App,
    handler: impl Fn(&InputEvent, &mut Window, &mut App) + 'static,
) -> Subscription {
    window.subscribe(state, cx, move |_, emitted: &InputEvent, window, cx| {
        if event.matches(emitted) {
            handler(emitted, window, cx);
        }
    })
}

fn allocate_store_id(next: u32) -> Option<(u32, u32)> {
    (next <= MAX_STORE_ID).then(|| (next, next + 1))
}

/// The events a script can subscribe to, named for what they mean rather than
/// for the key that produced them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputEventName {
    Change,
    Submit,
    Focus,
    Blur,
}

impl InputEventName {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "change" => Some(Self::Change),
            "submit" => Some(Self::Submit),
            "focus" => Some(Self::Focus),
            "blur" => Some(Self::Blur),
            _ => None,
        }
    }

    pub const NAMES: &'static [&'static str] = &["change", "submit", "focus", "blur"];

    fn matches(self, event: &InputEvent) -> bool {
        matches!(
            (self, event),
            (Self::Change, InputEvent::Change)
                | (Self::Submit, InputEvent::PressEnter { .. })
                | (Self::Focus, InputEvent::Focus)
                | (Self::Blur, InputEvent::Blur)
        )
    }
}

/// The slider events a script can subscribe to.
///
/// Two, and the difference between them is what a script does with the value:
/// `change` arrives on every pixel of a drag and is what a live readout wants;
/// `release` arrives once and is what a commit — a request, a write, an undo
/// entry — wants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SliderEventName {
    Change,
    Release,
}

impl SliderEventName {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "change" => Some(Self::Change),
            "release" => Some(Self::Release),
            _ => None,
        }
    }

    pub const NAMES: &'static [&'static str] = &["change", "release"];

    /// The value this event carries, when it is the one subscribed to.
    fn value(self, event: &SliderEvent) -> Option<SliderValue> {
        match (self, event) {
            (Self::Change, SliderEvent::Change(value))
            | (Self::Release, SliderEvent::Release(value)) => Some(*value),
            _ => None,
        }
    }
}

/// The one-time-code events a script can subscribe to.
///
/// Four, not the text input's four. `OtpState` never emits
/// `InputEvent::PressEnter`, so a `submit` here would be a name that could
/// never fire. Completion is separate from ordinary editing so validation can
/// observe `change` without starting verification early.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum OtpEventName {
    Change,
    Complete,
    Focus,
    Blur,
}

impl OtpEventName {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "change" => Some(Self::Change),
            "complete" => Some(Self::Complete),
            "focus" => Some(Self::Focus),
            "blur" => Some(Self::Blur),
            _ => None,
        }
    }

    pub const NAMES: &'static [&'static str] = &["change", "complete", "focus", "blur"];

    fn matches(self, event: &OtpEvent) -> bool {
        matches!(
            (self, event),
            (Self::Change, OtpEvent::Change)
                | (Self::Complete, OtpEvent::Complete)
                | (Self::Focus, OtpEvent::Focus)
                | (Self::Blur, OtpEvent::Blur)
        )
    }
}

/// The editor style a script's input is given: none of it.
///
/// Base resolves every colour a text input paints with from the active palette
/// on each render, so the answer to "what colour is the caret" is now asked at
/// the moment it is painted rather than at the moment the state was built.
///
/// This used to project the palette here, with light literals behind each
/// token in case none was installed yet — and a script that builds its inputs
/// in `init` builds them before it has installed anything, so the literals won
/// every time and then never changed. The input kept light-mode ink for the
/// life of the application: right by accident under a light palette, a black
/// caret and an invisible placeholder under a dark one. Projecting nothing is
/// what lets base answer, and base answers every frame.
fn editor_style() -> InputEditorStyle {
    InputEditorStyle::default()
}

#[cfg(test)]
mod tests {
    #[gpui::test]
    fn an_input_paints_with_the_palette_that_is_current_not_the_one_it_was_built_under(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::hsla;

        // Built with nothing installed, which is what a script that creates its
        // inputs in `init` does: the palette arrives on a task afterwards.
        let style = super::editor_style();
        assert_eq!(
            style.foreground.a, 0.,
            "the shell must project nothing, or base cannot tell unset from chosen"
        );

        cx.update(|cx| {
            let mut dark = gpui_base::SemanticThemeTokens::default();
            dark.colors.foreground = hsla(0., 0., 0.98, 1.0);
            dark.colors.muted_foreground = hsla(0., 0., 0.64, 1.0);
            let resolved = style.resolved(&dark);

            // The two the caret and the placeholder are drawn from.
            assert_eq!(resolved.caret, dark.colors.foreground);
            assert_eq!(resolved.muted_foreground, dark.colors.muted_foreground);

            let mut light = gpui_base::SemanticThemeTokens::default();
            light.colors.foreground = hsla(0., 0., 0.04, 1.0);
            assert_ne!(
                style.resolved(&light).caret,
                resolved.caret,
                "the same projected style must follow whichever palette is current"
            );
            let _ = cx;
        });
    }

    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    #[test]
    fn a_released_handle_stops_resolving() {
        let mut store = EntityStore::try_new().expect("store id");
        // A handle that was never issued resolves to nothing rather than
        // panicking, which is what keeps a stale script reference reportable.
        let unissued = u64::from(store.id) << STORE_SHIFT;
        assert!(store.input(unissued).is_none());
        assert!(!store.release(unissued));
    }

    #[test]
    fn a_store_starts_empty() {
        let store = EntityStore::try_new().expect("store id");
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn live_entity_limit_is_enforced_by_the_store() {
        let mut store = EntityStore::try_new().expect("store id");
        for _ in 0..MAX_LIVE_ENTITIES {
            store
                .push(Record::VirtualScroll {
                    handle: VirtualListScrollHandle::new(),
                    application: None,
                })
                .expect("the advertised live-entity capacity");
        }

        let overflow = store.push(Record::VirtualScroll {
            handle: VirtualListScrollHandle::new(),
            application: None,
        });
        assert_eq!(overflow, Err(EntityLimitError));
        assert_eq!(store.len(), MAX_LIVE_ENTITIES);
    }

    #[test]
    fn a_handle_from_another_store_does_not_resolve() {
        let first = EntityStore::try_new().expect("first store id");
        let second = EntityStore::try_new().expect("second store id");
        assert_ne!(first.id, second.id, "stores must not share an id");

        // Slot 0 of the other store. Without the store bits this would be a
        // valid index here, which is exactly the confusion the bits prevent.
        let foreign = u64::from(second.id) << STORE_SHIFT;
        assert!(first.entity_id(foreign).is_none());
    }

    #[gpui::test]
    fn released_and_cleared_handles_are_never_reissued(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let first = context
            .update(|window, cx| store.create_input(None, None, None, window, cx))
            .expect("room");
        assert!(store.release(first));
        let second = context
            .update(|window, cx| store.create_input(None, None, None, window, cx))
            .expect("room");
        assert_ne!(first, second);
        assert!(store.input(first).is_none());
        assert!(store.input(second).is_some());

        store.clear();
        let third = context
            .update(|window, cx| store.create_input(None, None, None, window, cx))
            .expect("room");
        assert_ne!(second, third);
        assert!(store.input(second).is_none());
        assert!(store.input(third).is_some());
    }

    /// A focus handle is retained state like any other: released by handle,
    /// released with its application, and never confused with an input.
    #[gpui::test]
    fn a_focus_handle_is_retained_and_released_like_any_other_entity(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let focus = context
            .update(|_, cx| store.create_focus(None, cx))
            .expect("room");
        let input = context
            .update(|window, cx| store.create_input(None, None, None, window, cx))
            .expect("room");

        // The two kinds do not answer for each other, which is what stops a
        // script from rendering an input where it asked for a focus target.
        assert!(store.focus(focus).is_some());
        assert!(store.input(focus).is_none());
        assert!(store.focus(input).is_none());

        assert!(store.release(focus));
        assert!(store.focus(focus).is_none());
        assert!(store.input(input).is_some());
    }

    /// Single-line and multi-line state are two Rust types, and `Textarea::new`
    /// will not take an `InputState`. A handle that resolved as either would
    /// therefore be a crash waiting to be materialized, so the store keeps them
    /// apart even though everything else about them is shared.
    #[gpui::test]
    fn a_textarea_handle_is_never_mistaken_for_an_input(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let input = context
            .update(|window, cx| store.create_input(None, None, None, window, cx))
            .expect("room");
        let textarea = context
            .update(|window, cx| store.create_textarea(None, None, Some(4), None, window, cx))
            .expect("room");

        assert!(store.textarea(input).is_none());
        assert!(store.input(textarea).is_none());
        assert!(store.textarea(textarea).is_some());

        assert_eq!(store.native_state::<InputState>(input), store.input(input));
        assert_eq!(
            store.native_state::<TextareaState>(textarea),
            store.textarea(textarea)
        );
        assert!(store.native_state::<InputState>(textarea).is_none());
        assert!(store.native_state::<TextareaState>(input).is_none());
        let other_store = EntityStore::try_new().expect("another store");
        assert!(other_store.native_state::<InputState>(input).is_none());

        // Subscribing reaches both through the one method, which is the part
        // that would silently stop working if a variant were forgotten there.
        assert!(context.update(|window, cx| store.subscribe_input(
            textarea,
            InputEventName::Change,
            window,
            cx,
            |_, _, _| {}
        )));

        assert!(store.release(textarea));
        assert!(store.textarea(textarea).is_none());
        assert!(store.native_state::<TextareaState>(textarea).is_none());
        assert!(store.input(input).is_some());
    }

    #[gpui::test]
    fn releasing_an_application_drops_only_its_entities(cx: &mut TestAppContext) {
        let mut store = EntityStore::try_new().expect("store id");
        let first_application = ApplicationGeneration::new(1);
        let second_application = ApplicationGeneration::new(2);
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        let first = context
            .update(|window, cx| {
                store.create_input(None, None, Some(first_application.clone()), window, cx)
            })
            .expect("room");
        let second = context
            .update(|window, cx| {
                store.create_input(None, None, Some(second_application.clone()), window, cx)
            })
            .expect("room");
        let focus = context
            .update(|_, cx| store.create_focus(Some(first_application.clone()), cx))
            .expect("room");

        store.release_application(&first_application).retire(cx);

        assert!(store.input(first).is_none());
        assert!(store.focus(focus).is_none());
        assert!(store.input(second).is_some());
    }

    #[test]
    fn store_ids_stop_before_the_javascript_safe_namespace_would_wrap() {
        assert_eq!(
            allocate_store_id(MAX_STORE_ID),
            Some((MAX_STORE_ID, MAX_STORE_ID + 1))
        );
        assert_eq!(allocate_store_id(MAX_STORE_ID + 1), None);
    }
}

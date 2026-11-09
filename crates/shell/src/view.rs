//! The single bridge between a script view and GPUI's render loop.
//!
//! Every script-defined view, panel, or dialog body is carried by a `ScriptView`
//! entity. [`ShellRoot`](crate::root::ShellRoot) mounts the entity through
//! GPUI's cached-view path, so a clean window frame reuses its rendered subtree
//! without calling this `render` at all. When the entity or one of its retained
//! descendants is dirty, `render` remains deliberately *not* necessarily a
//! script call:
//!
//! ```text
//! dirty view render ─▶ snapshot still valid? ──yes──▶ materialize   (no VM)
//!                          │
//!                          no
//!                          ▼
//!                  script render() ──▶ publish snapshot ──▶ materialize
//! ```
//!
//! The script runs only when something invalidated its snapshot: `cx.notify()`
//! from an event or a task, a hot reload, or a palette change. Everything else
//! replays the description the script already produced. Clean frames skip both
//! operations through GPUI's subtree cache. That is what keeps script and
//! materialization cost proportional to application activity rather than frame
//! rate.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Context, Element, ElementId, Empty, EntityId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ParentElement as _, Pixels, Render, Styled as _,
    Window, div,
};

use crate::{
    engine::{ShellRuntime, ViewObject},
    materialize::materialize,
    policy::Policy,
    runtime::{ViewFailure, error_banner, error_overlay},
    snapshot::RenderSnapshot,
    spec::MAX_DESCRIPTION_DEPTH,
};

pub struct ScriptView {
    /// Declared before `runtime` because fields drop in declaration order, and
    /// a script value released after its engine aborts the process. A view that
    /// happens to hold the last reference to the runtime would otherwise free
    /// the VM first and then release this handle into it. The snapshots below
    /// are in the same position: retiring their callbacks releases script
    /// values, so they must go first too.
    ///
    /// This ordering is the only thing that retires a view's callbacks while
    /// the VM that owns them is still alive.
    object: ViewObject,
    /// The published description. `None` only before the first render.
    current: Option<RenderSnapshot>,
    /// The snapshot this one replaced, held one generation longer.
    ///
    /// GPUI can dispatch an event against the elements of a frame that has
    /// already been superseded — a click landing between a rebuild and the
    /// repaint that follows it. Keeping the previous snapshot alive keeps its
    /// callbacks resolvable for exactly that window; anything older is stale and
    /// is meant to resolve to nothing.
    previous: Option<RenderSnapshot>,
    /// Set when script-visible state may have changed, cleared by the rebuild.
    dirty: bool,
    /// Set when the script-visible handle has been released. GPUI may retain
    /// the entity for an older frame, but it must never rebuild after release.
    retired: bool,
    /// The tokens and appearance the current snapshot resolved against.
    theme: Option<crate::theme_tokens::ThemeSnapshotKey>,
    /// The failure of the most recent build, if it failed.
    ///
    /// Held rather than re-derived so a script that throws is not re-run on
    /// every frame: a broken render is exactly as frame-coupled as a working one
    /// if the failure re-triggers the build.
    error: Option<String>,
    /// Why the most recent source reload failed, if it did.
    ///
    /// Reported here rather than in a toast of its own: the view keeps running
    /// its previous code, so this is a failure *of this view*, and it belongs on
    /// the same surface as a render failure. Takes precedence over `error`,
    /// which describes code the author has already edited away.
    reload_error: Option<String>,
    /// Why this view was last drawn as nothing at a nested mount, so a mount
    /// refused on every frame is reported once per snapshot.
    refused: Option<NestingRefusal>,
    /// Whose authority this view's script runs under.
    ///
    /// Captured when the view is constructed rather than read when it is used:
    /// a callback firing three seconds later must run under the grant its own
    /// script was loaded with, and no swap made in between can change that.
    policy: Rc<Policy>,
    ownership: ViewOwnership,
    runtime: Rc<ShellRuntime>,
}

/// Which cleanup boundary this view owns.
#[derive(Clone, Copy)]
enum ViewOwnership {
    /// The application root owns application-wide retained state and tasks.
    Root,
    /// A nested view owns only work keyed to its exact GPUI entity identity.
    Nested(EntityId),
}

impl ScriptView {
    /// Under the policy in force where the view was constructed.
    #[cfg(test)]
    pub(crate) fn new(runtime: Rc<ShellRuntime>, object: ViewObject) -> Self {
        Self::with_policy(runtime, object, crate::scope::policy())
    }

    pub(crate) fn with_policy(
        runtime: Rc<ShellRuntime>,
        object: ViewObject,
        policy: Rc<Policy>,
    ) -> Self {
        Self::with_ownership(runtime, object, policy, ViewOwnership::Root)
    }

    pub(crate) fn nested(
        runtime: Rc<ShellRuntime>,
        object: ViewObject,
        policy: Rc<Policy>,
        entity_id: EntityId,
    ) -> Self {
        Self::with_ownership(runtime, object, policy, ViewOwnership::Nested(entity_id))
    }

    fn with_ownership(
        runtime: Rc<ShellRuntime>,
        object: ViewObject,
        policy: Rc<Policy>,
        ownership: ViewOwnership,
    ) -> Self {
        Self {
            object,
            current: None,
            previous: None,
            dirty: true,
            retired: false,
            policy,
            theme: None,
            error: None,
            reload_error: None,
            refused: None,
            ownership,
            runtime,
        }
    }

    /// Marks the script description as possibly out of date.
    ///
    /// This is what Shell `cx.notify()` means: *my* description may have
    /// changed. Scheduling and coalescing the actual repaint stays with GPUI —
    /// three notifies before the next frame rebuild one snapshot, not three.
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// Invalidates and notifies: the host changed something the script reads.
    ///
    /// **A host that mutates state a script reads must call this, not
    /// `cx.notify()` alone.** The two are different requests now, and the
    /// difference is the point of this type:
    ///
    /// ```text
    /// cx.notify()  ── draw this view again          (no script runs)
    /// refresh()    ── and the description is stale  (the script runs)
    /// ```
    ///
    /// A bare `notify` is still the right call for a repaint that changes
    /// nothing the script can see. Getting it wrong in the other direction is
    /// visible immediately — the interface simply does not update — which is the
    /// same failure mode as a forgotten `cx.notify()` in GPUI itself, and it is
    /// cheap to find for the same reason.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.invalidate();
        cx.notify();
    }

    /// Replaces the script instance behind this view.
    ///
    /// Hot reload keeps the entity — and therefore the window, the focus and
    /// the element identities — and swaps only what the script produced. The
    /// description that the old instance built is now meaningless, so the view
    /// is invalidated with it.
    /// Records, or clears, why the latest reload of this view's source failed.
    pub(crate) fn set_reload_error(&mut self, message: Option<String>, cx: &mut Context<Self>) {
        if self.reload_error != message {
            self.reload_error = message;
            cx.notify();
        }
    }

    pub(crate) fn replace_object(&mut self, object: ViewObject) {
        self.object = object;
        self.dirty = true;
    }

    /// The script state behind this view, for host code that needs to read it.
    pub(crate) fn object(&self) -> &ViewObject {
        &self.object
    }

    /// The published description, if one has been built.
    /// Why the most recent build failed, if it did.
    ///
    /// A view with no snapshot is not the same as a view with nothing to draw:
    /// it means the script threw and the failure was recorded here. A test that
    /// finds `snapshot()` empty should report this rather than the absence,
    /// because the absence is the symptom and this is the cause.
    pub fn build_error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn snapshot(&self) -> Option<&RenderSnapshot> {
        self.current.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn nesting_refusal(&self) -> Option<NestingRefusal> {
        self.refused
    }

    /// Whether the next GPUI render will enter the VM.
    /// The authority this view's script runs under.
    pub fn policy(&self) -> Rc<Policy> {
        self.policy.clone()
    }

    pub(crate) fn runtime(&self) -> Rc<ShellRuntime> {
        self.runtime.clone()
    }

    pub(crate) fn application_generation(
        &self,
    ) -> Option<Rc<crate::runtime::ApplicationGeneration>> {
        self.object.application_generation()
    }

    pub fn is_dirty(&self) -> bool {
        !self.retired && self.dirty
    }

    /// Makes a retained entity inert before its store handle is removed.
    /// A rendered GPUI frame may still retain the entity after script release.
    pub(crate) fn retire(&mut self) {
        self.retired = true;
        self.dirty = false;
        self.error = None;
        self.reload_error = None;
        self.previous = None;
        self.current = None;
    }

    /// Runs the script and publishes what it produced.
    ///
    /// The build is transactional. A replacement snapshot is assembled beside
    /// the live one and swapped in only after the script returns successfully;
    /// a script that throws half-way leaves the previous description, and the
    /// callbacks that belong to it, exactly as they were.
    fn rebuild(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Cleared before the script runs, not after: draining the task queue at
        // the end of a build can notify this same view, and that notify has to
        // survive into the next frame rather than be wiped by the build that
        // was already in flight.
        self.dirty = false;

        let runtime = self.runtime.clone();
        let object = self.object.clone();
        let policy = self.policy.clone();
        let entity = cx.entity();

        match runtime.build_snapshot(&object, Some(entity), policy, window, cx) {
            Ok(snapshot) => {
                // Measured here rather than anywhere else because this is the
                // only place two consecutive descriptions of one view exist at
                // the same time. Nothing acts on the answer: it counts how often
                // a rebuild produced the shape it replaced, which is what a
                // template cache would have to be able to fill instead of
                // rebuild (§20.7 of `docs/gpui-shell.md`). A first build has no
                // predecessor and is not a data point either way.
                if let Some(current) = self.current.as_ref() {
                    runtime
                        .metrics()
                        .record_structure(current.structure() == snapshot.structure());
                }

                // Assigning through `previous` is what retires the snapshot
                // before last: dropping it releases its callbacks.
                self.previous = self.current.replace(snapshot);
                self.error = None;
                self.refused = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Draws nothing at a mount that would recurse or nest too deep.
    fn refuse(&mut self, entity: EntityId, refusal: NestingRefusal) -> AnyElement {
        if self.refused != Some(refusal) {
            self.refused = Some(refusal);
            match refusal {
                NestingRefusal::Reentered => tracing::error!(
                    "script view {entity} is mounted inside itself; the inner mount draws nothing"
                ),
                NestingRefusal::TooDeep => tracing::error!(
                    "script view {entity} would nest views more than {MAX_NESTED_VIEW_DEPTH} \
                     levels deep; it draws nothing at this mount"
                ),
            }
        }
        Empty.into_any_element()
    }
}

impl Drop for ScriptView {
    fn drop(&mut self) {
        match self.ownership {
            ViewOwnership::Root => {
                if let Some(application) = self.object.application_generation() {
                    self.runtime
                        .release_application_generation_without_context(&application);
                }
            }
            ViewOwnership::Nested(entity_id) => {
                // Child-owned retained records are removed by EntityStore in
                // the same operation that removes the child handle. Reaching
                // back into that RefCell here would re-enter its mutable borrow.
                crate::engine::quickjs::cancel_view_tasks(&self.runtime, entity_id);
            }
        }
    }
}

impl Render for ScriptView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.retired {
            return div().into_any_element();
        }
        let entity = cx.entity_id();
        if DrawingViews::contains(entity) {
            return self.refuse(entity, NestingRefusal::Reentered);
        }
        let theme = crate::theme_tokens::sync(cx);
        if self.theme.as_ref() != Some(&theme) {
            self.theme = Some(theme);
            self.dirty = true;
        }
        if self.is_dirty() {
            self.rebuild(window, cx);
        }

        // The view itself is one level: the frame and GPUI's view element.
        let height = 1 + self
            .current
            .as_ref()
            .map_or(1, |snapshot| u32::from(snapshot.height()));
        if DrawingViews::depth() + height > MAX_NESTED_VIEW_DEPTH {
            return self.refuse(entity, NestingRefusal::TooDeep);
        }

        let failure = match (self.reload_error.as_deref(), self.error.as_deref()) {
            (Some(message), _) => Some((ViewFailure::Reload, message)),
            (None, Some(message)) => Some((ViewFailure::Render, message)),
            (None, None) => None,
        };
        let element = match (failure, self.current.as_ref()) {
            (None, Some(snapshot)) => materialize(&self.runtime, snapshot, window, cx),
            // A build that failed left the last good snapshot in place, so the
            // interface is still there to show. Reporting over it beats
            // replacing it: the reader keeps their scroll, their focus and
            // whatever they were reading, and still learns what broke.
            (Some((failure, message)), Some(snapshot)) => div()
                .relative()
                .size_full()
                .child(materialize(&self.runtime, snapshot, window, cx))
                .child(error_banner(failure, message, window, cx))
                .into_any_element(),
            // Nothing to keep: this view has never rendered successfully.
            (Some((failure, message)), None) => error_overlay(failure, message, window, cx),
            // Unreachable in practice: a build either publishes a snapshot or
            // records an error. An empty element is the honest answer if it ever
            // is reached, rather than a panic in a render.
            (None, None) => div().into_any_element(),
        };
        nesting_frame(entity, height, element)
    }
}

/// How many levels views drawn one inside another may span together, each
/// counting its description's height and one level for itself: what one view
/// at [`MAX_DESCRIPTION_DEPTH`] takes.
///
/// Layout, prepaint and paint recurse through a mounted view without entering
/// the script, so neither the description bound nor the interpreter's stack
/// limit sees a chain or a cycle of views. The costliest chain measured, nested
/// `Checkbox`es split across views, draws at most 73 levels in an unoptimized
/// build on a 2 MiB test thread.
pub(crate) const MAX_NESTED_VIEW_DEPTH: u32 = MAX_DESCRIPTION_DEPTH as u32 + 1;

/// What a mounted dock area counts towards [`MAX_NESTED_VIEW_DEPTH`].
///
/// Base draws several views of its own between the area and a panel's script
/// view: measured, one nesting costs the stack of about seven `Checkbox`
/// levels, three of which the descriptions on either side already count.
pub(crate) const DOCK_AREA_LEVELS: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NestingRefusal {
    Reentered,
    TooDeep,
}

/// Holds `entity` on the drawing stack while `element` is laid out, prepainted
/// and painted, counting `levels` towards [`MAX_NESTED_VIEW_DEPTH`].
pub(crate) fn nesting_frame(entity: EntityId, levels: u32, element: AnyElement) -> AnyElement {
    NestingFrame {
        entity,
        levels,
        element,
    }
    .into_any_element()
}

/// The entities whose elements are being laid out, prepainted or painted on
/// this thread, outermost first. GPUI keeps its own view stack private.
#[derive(Default)]
struct DrawingViews {
    entities: Vec<(EntityId, u32)>,
    depth: u32,
}

thread_local! {
    static DRAWING_VIEWS: RefCell<DrawingViews> = RefCell::default();
}

impl DrawingViews {
    fn contains(entity: EntityId) -> bool {
        DRAWING_VIEWS.with_borrow(|drawing| drawing.entities.iter().any(|(id, _)| *id == entity))
    }

    fn depth() -> u32 {
        DRAWING_VIEWS.with_borrow(|drawing| drawing.depth)
    }

    fn enter(entity: EntityId, levels: u32) -> DrawingGuard {
        DRAWING_VIEWS.with_borrow_mut(|drawing| {
            drawing.entities.push((entity, levels));
            drawing.depth += levels;
        });
        DrawingGuard
    }
}

/// Pops on drop, so a panic unwinding through a phase leaves the stack as it
/// found it.
struct DrawingGuard;

impl Drop for DrawingGuard {
    fn drop(&mut self) {
        DRAWING_VIEWS.with_borrow_mut(|drawing| {
            if let Some((_, levels)) = drawing.entities.pop() {
                drawing.depth -= levels;
            }
        });
    }
}

/// Every mount reaches a script view through its `render` — a child view, a
/// dock panel, a dialog or sheet, the window root — so the frame there covers
/// them all. A clean frame replays the cached subtree without visiting it.
struct NestingFrame {
    entity: EntityId,
    levels: u32,
    element: AnyElement,
}

impl IntoElement for NestingFrame {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NestingFrame {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let _drawing = DrawingViews::enter(self.entity, self.levels);
        (self.element.request_layout(window, cx), ())
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
        let _drawing = DrawingViews::enter(self.entity, self.levels);
        self.element.prepaint(window, cx);
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
        let _drawing = DrawingViews::enter(self.entity, self.levels);
        self.element.paint(window, cx);
    }
}

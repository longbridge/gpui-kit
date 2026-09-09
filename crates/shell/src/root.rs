//! The window-level overlay host.
//!
//! `gpui-base` ships the overlay *parts* — [`Dialog`] and [`Sheet`] each build
//! their own viewport-sized host, [`ToastManager`] and [`ToastStackState`] own
//! stacking geometry and lifecycle, [`gpui_base::FocusTrapElement`] owns focus
//! trapping — but nothing in Base decides what happens when two of them are
//! open at once. [`ShellRoot`] is that decision, and it is the only reason this
//! module exists: it is a stacking order plus a dismissal order, with the
//! smallest presentation that makes them visible.
//!
//! It is deliberately not `gpui_component::Root`. The shell binds to
//! `gpui-base` only (see `docs/gpui-shell.md` §4.2), so the equivalent
//! host has to be written here rather than reused.

use std::{path::PathBuf, rc::Rc, time::Duration};

use gpui::{
    Anchor, AnyElement, AnyView, App, AppContext as _, ClickEvent, ClipboardItem, Context,
    ElementId, Entity, FocusHandle, Global, Hsla, InteractiveElement as _, IntoElement, KeyBinding,
    MouseButton, MouseDownEvent, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled as _, WeakFocusHandle, Window,
    actions, deferred, div, hsla, prelude::FluentBuilder as _, px,
};
use gpui_base::{
    ColorTokens, Dialog, POPUP_PRIORITY, Placement, RadiusTokens, Sheet, SpacingTokens,
    StyledExt as _, TextSelection, TextSelectionLayer, Theme, Toast, ToastManager, ToastMotion,
    ToastOptions, ToastStack, ToastStackState, TooltipOverlay, TooltipTransition,
    active_focus_trap,
    animation::{EffectTransition, ease_in_out_cubic, ease_out_cubic},
    v_flex,
};

use crate::{scope, view::ScriptView};

actions!(shell_root, [Tab, TabPrev, Copy]);

/// The key context the root installs. Overlay hosts add their own contexts
/// (`Dialog`, `Sheet`) below this one, so a binding declared here is reachable
/// from inside an overlay while an overlay binding is not reachable outside it.
const CONTEXT: &str = "ShellRoot";

/// How often the toast lifecycle clock is advanced.
///
/// Toast timeouts are wall-clock, not frame-driven, so they have to be sampled;
/// 50ms is below the threshold where a dismissal reads as late and far above
/// the cost of one comparison per mounted toast.
const TOAST_TICK: Duration = Duration::from_millis(50);

/// How many active toasts are mounted at once. Older toasts stay in the manager
/// and reappear as newer ones leave, so a burst is throttled rather than lost.
const TOAST_VISIBLE_LIMIT: usize = 3;

/// Paint priority of the toast layer.
///
/// Above [`POPUP_PRIORITY`] because a toast reports the outcome of the action
/// the user just took, and an open dropdown or dialog is exactly the situation
/// where that outcome matters most. It is the one layer that is never occluded.
const TOAST_PRIORITY: usize = POPUP_PRIORITY + 1;

/// Width of the toast column. The single geometric constant in this module:
/// toasts are anchored to a viewport corner, so unlike every other overlay here
/// they cannot take their measure from their content or from the viewport.
const TOAST_WIDTH: gpui::Pixels = px(320.);

/// The window-level overlay host: content, one sheet, a dialog stack, toasts.
///
/// The first view of a shell window is always a `ShellRoot`, the same way the
/// first view of a `gpui-component` window is always a `Root`. Scripts reach it
/// through [`ShellRoot::update`], never by constructing overlays themselves.
///
/// # Stacking order
///
/// Painted back to front, each layer above the one before it:
///
/// 1. **Content** — the script's root view.
/// 2. **Sheet** — at most one, anchored to a viewport edge. A sheet is a
///    *place* in the window, so it sits below the dialog stack: a dialog raised
///    from inside a sheet must be readable, and a sheet raised under a dialog
///    must not cover it.
/// 3. **Dialog stack** — in open order, oldest at the bottom. Each dialog is
///    deferred at `10 + index`, so a later dialog always paints over an earlier
///    one regardless of the order the elements were built in.
/// 4. **Toasts** — above the overlay stack, at the root's toast priority.
/// 5. **Tooltip** — one layer, always topmost. Base defers it above every
///    priority the root uses, so it is the one layer whose order is not the
///    root's to choose.
///
/// Only the topmost dialog draws a backdrop. A stack of three dialogs dims the
/// window once, not three times, and the single backdrop is what separates the
/// live dialog from the inert ones behind it.
///
/// # Dismissal order
///
/// Dismissal is always *one* layer, never a cascade:
///
/// - **Escape** closes the topmost dialog only. Lower dialogs render with
///   keyboard handling disabled, so a repeated Escape unwinds the stack one
///   dialog per press and never reaches the sheet while a dialog is open.
///   [`DialogOptions::escape_dismissable`] withdraws the *key binding*, not the
///   underlying cancel action: a close control the script puts inside the
///   dialog still works, which is what makes an undismissable dialog a dialog
///   the user must answer rather than one they cannot leave.
/// - **Backdrop press** closes the topmost dialog, and only if that dialog was
///   opened with [`DialogOptions::backdrop_dismissable`]. A dialog that is
///   asking a question the user must answer keeps its backdrop inert while
///   still dimming what is behind it.
/// - **Enter** does nothing at this layer. Base's dialog host treats Enter as
///   "confirm and close"; that belongs to the dialog's own primary button,
///   which the script owns, so the root vetoes the built-in confirmation
///   instead of guessing which content wants it.
/// - A **sheet** is dismissed by Escape or by its overlay only when no dialog
///   is open, because a dialog above it holds focus.
/// - [`ShellRoot::close_all_dialogs`] is the one operation that unwinds the
///   whole stack, and it leaves the sheet alone.
///
/// # Focus
///
/// Opening an overlay records the currently focused handle and focuses the
/// overlay; closing it restores that handle. Because each open records what was
/// focused at the time, a stack restores focus through its own history: closing
/// the second dialog returns focus to the first, and closing the first returns
/// it to whatever the window was on before either opened.
///
/// Tab and Shift-Tab honour the focus trap that Base's dialog and sheet hosts
/// register, so tabbing inside an overlay cycles within it instead of walking
/// into the content behind it.
///
/// # Presentation
///
/// Backdrop, surface, and spacing, drawn from [`gpui_base::Theme`]'s semantic
/// tokens — nothing else. This type positions and layers; the visual language
/// of what goes *inside* an overlay belongs to the script.
pub struct ShellRoot {
    content: AnyView,
    application: Option<MountedApplication>,
    /// Retains a script application's policy and cancels its work when this
    /// window root is dropped. Ordinary Rust content leaves it empty.
    application_policy: Option<Rc<crate::policy::Policy>>,
    application_generation: Option<Rc<crate::runtime::ApplicationGeneration>>,
    /// Open dialogs, oldest first. The last entry is the topmost and the only
    /// interactive one.
    dialogs: Vec<ActiveDialog>,
    sheet: Option<ActiveSheet>,
    toasts: ToastManager<SharedString, ToastRequest>,
    toast_state: ToastStackState,
    toast_focus_handle: FocusHandle,
    /// Source of ids for toasts pushed without one. Monotonic so that a
    /// replaced id can never collide with a live toast.
    next_toast_ordinal: u64,
    /// The window's one tooltip layer. Base owns the delay, the grace period
    /// between two triggers and the paint priority; the root owns only the
    /// decision to have one, and what an appearing tooltip looks like.
    tooltip_overlay: Entity<TooltipOverlay>,
    /// The performance HUD, while a script has asked for one. See
    /// [`ShellRoot::show_fps_monitor`].
    fps_hud: Option<FpsHudRequest>,
}

/// Where the performance HUD sits over the window and how it behaves.
///
/// What a script's `show_fps_monitor(options)` names, with the same defaults
/// `gpui_fps` gives an overlay a Rust host places by hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FpsHudRequest {
    /// Corner or edge of the window.
    pub anchor: Anchor,
    /// The per-frame budget the HUD grades frame cost against. `None` keeps
    /// the HUD's own default.
    pub frame_budget: Option<Duration>,
}

impl Default for FpsHudRequest {
    fn default() -> Self {
        Self {
            anchor: Anchor::TopRight,
            frame_budget: None,
        }
    }
}

pub(crate) struct MountedApplication {
    pub(crate) view: gpui::Entity<ScriptView>,
    pub(crate) root: PathBuf,
    pub(crate) entry: String,
}

struct ActiveDialog {
    content: AnyView,
    focus_handle: FocusHandle,
    /// What was focused when this dialog opened, to be restored when it closes.
    /// Weak, because the view that held it may be gone by then.
    restore_focus: Option<WeakFocusHandle>,
    options: DialogOptions,
}

struct ActiveSheet {
    content: AnyView,
    placement: Placement,
    focus_handle: FocusHandle,
    restore_focus: Option<WeakFocusHandle>,
}

/// Marks that this `App` already has the root's key bindings.
///
/// The root installs its own bindings rather than relying on `gpui_shell::init`
/// so that adding it to a window is the only step required, and guards on a
/// global so that opening a second window does not append a duplicate binding.
struct KeyBindingsInstalled;

impl Global for KeyBindingsInstalled {}

impl ShellRoot {
    /// Wraps a script view as the content of a window.
    ///
    /// Takes an [`AnyView`] rather than an `Entity<ScriptView>` so the host can
    /// mount a Rust view too — the error screen a failed script load falls back
    /// to is not a `ScriptView`, and the root must be able to carry it.
    pub fn new(content: AnyView, window: &mut Window, cx: &mut Context<Self>) -> Self {
        install_key_bindings(cx);
        Self::spawn_toast_clock(window, cx);

        Self {
            content,
            application: None,
            application_policy: None,
            application_generation: None,
            dialogs: Vec::new(),
            sheet: None,
            toasts: ToastManager::new(ToastMotion::sonner()),
            toast_state: ToastStackState::default(),
            toast_focus_handle: cx.focus_handle().tab_stop(true),
            next_toast_ordinal: 0,
            tooltip_overlay: cx.new(|_| TooltipOverlay::new().render_with(render_tooltip)),
            fps_hud: None,
        }
    }

    pub(crate) fn with_application(
        view: gpui::Entity<ScriptView>,
        root: PathBuf,
        entry: String,
        policy: Rc<crate::policy::Policy>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut shell_root = Self::new(view.clone().into(), window, cx);
        shell_root.application_generation = view.read(cx).application_generation();
        shell_root.application = Some(MountedApplication { view, root, entry });
        shell_root.application_policy = Some(policy);
        shell_root
    }

    /// Reaches the root of the window a call is happening in.
    ///
    /// This is how a script-facing binding gets from the `&mut Window` its call
    /// scope carries to the overlay host, without the script ever holding the
    /// root itself. `None` when the window's first view is not a `ShellRoot`,
    /// which is a host wiring mistake rather than a script error — hence an
    /// `Option` and not a panic.
    pub fn update<R>(
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    ) -> Option<R> {
        let root = window.root::<Self>().flatten()?;
        Some(root.update(cx, |root, cx| f(root, window, cx)))
    }

    /// The tooltip layer of the window a call is happening in.
    ///
    /// The counterpart to [`ShellRoot::update`] for the one overlay a script
    /// never opens by hand: a tooltip trigger is an element, so what reaches it
    /// is a hover listener holding a `&mut Window`, not a call scope. `None`
    /// when the window's first view is not a `ShellRoot` — the same host wiring
    /// mistake `update` reports, and the same reason it is not a panic.
    pub fn tooltip_overlay(window: &mut Window, cx: &mut App) -> Option<Entity<TooltipOverlay>> {
        let root = window.root::<Self>().flatten()?;
        Some(root.read(cx).tooltip_overlay.clone())
    }

    /// The view this window was opened with, below every overlay.
    pub fn content(&self) -> &AnyView {
        &self.content
    }

    pub(crate) fn application(&self) -> Option<&MountedApplication> {
        self.application.as_ref()
    }

    /// How many dialogs are open. The topmost is the only interactive one.
    pub fn dialog_count(&self) -> usize {
        self.dialogs.len()
    }

    /// The dialog that owns focus, keyboard dismissal, and the backdrop.
    pub fn topmost_dialog(&self) -> Option<&AnyView> {
        self.dialogs.last().map(|dialog| &dialog.content)
    }

    /// The open sheet, if any. At most one sheet exists at a time: a sheet is a
    /// region of the window rather than a stack of them.
    pub fn sheet(&self) -> Option<&AnyView> {
        self.sheet.as_ref().map(|sheet| &sheet.content)
    }

    /// How many toasts are mounted, including ones playing their exit.
    pub fn toast_count(&self) -> usize {
        self.toasts.len()
    }

    /// Draws the performance HUD over the window, above every other layer,
    /// until [`hide_fps_monitor`](Self::hide_fps_monitor). Calling it again
    /// moves or reconfigures the HUD that is already up.
    ///
    /// The root owns the HUD rather than the script's tree: the script says
    /// whether and where, and what it renders can neither move the HUD nor
    /// rebuild it. The monitor behind the overlay is `gpui_fps`'s own, one per
    /// window, so a HUD hidden and shown again keeps its history.
    ///
    /// Refused, with `false`, from a render pass; see `overlay_mutation_allowed`.
    pub fn show_fps_monitor(&mut self, request: FpsHudRequest, cx: &mut Context<Self>) -> bool {
        if !overlay_mutation_allowed("show_fps_monitor") {
            return false;
        }
        self.fps_hud = Some(request);
        cx.notify();
        true
    }

    /// Takes the performance HUD down. `true` if one was up.
    pub fn hide_fps_monitor(&mut self, cx: &mut Context<Self>) -> bool {
        if !overlay_mutation_allowed("hide_fps_monitor") {
            return false;
        }
        let was_visible = self.fps_hud.take().is_some();
        if was_visible {
            cx.notify();
        }
        was_visible
    }

    /// Whether the performance HUD is up.
    pub fn fps_monitor_visible(&self) -> bool {
        self.fps_hud.is_some()
    }

    /// The HUD, when a script has asked for one. Above every other layer: it
    /// is a diagnostic, and a dialog over it would hide the reading the
    /// dialog's own frames are producing.
    fn fps_layer(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui_fps::FpsOverlay> {
        let request = self.fps_hud?;
        let mut overlay = gpui_fps::fps_monitor(window, cx).anchor(request.anchor);
        if let Some(budget) = request.frame_budget {
            overlay = overlay.frame_budget(budget);
        }
        Some(overlay)
    }

    /// Opens a dialog on top of the stack, with default dismissal.
    ///
    /// The previously topmost dialog stays mounted and visible but becomes
    /// inert: it keeps its place in the stack so that closing this one returns
    /// the user exactly where they were.
    pub fn open_dialog(&mut self, content: AnyView, window: &mut Window, cx: &mut Context<Self>) {
        self.open_dialog_with(content, DialogOptions::default(), window, cx);
    }

    /// Opens a dialog whose dismissal differs from the default.
    ///
    /// Separate from [`ShellRoot::open_dialog`] because refusing dismissal is a
    /// deliberate, rare choice: a confirmation the user must answer. Making it
    /// the longer call keeps the easy path the dismissable one.
    pub fn open_dialog_with(
        &mut self,
        content: AnyView,
        options: DialogOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !overlay_mutation_allowed("open_dialog") {
            return;
        }

        let focus_handle = cx.focus_handle();
        let restore_focus = window.focused(cx).map(|handle| handle.downgrade());
        focus_handle.focus(window, cx);

        self.dialogs.push(ActiveDialog {
            content,
            focus_handle,
            restore_focus,
            options,
        });
        cx.notify();
    }

    /// Closes the topmost dialog and restores the focus it took.
    ///
    /// Returns whether a dialog was actually closed, so that a caller unwinding
    /// a stack — or a script asking "was there anything to close?" — does not
    /// have to read the stack depth first.
    pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !overlay_mutation_allowed("close_dialog") {
            return false;
        }

        let Some(dialog) = self.dialogs.pop() else {
            return false;
        };
        restore_focus(dialog.restore_focus, window, cx);
        cx.notify();
        true
    }

    /// Closes every dialog at once, restoring focus to where the *first* dialog
    /// took it from.
    ///
    /// Restoring through each dialog in turn would flicker focus across views
    /// that are about to be dropped; the first dialog's record is the only one
    /// that describes the window as it was before the stack existed.
    ///
    /// Leaves an open sheet alone: a sheet is not part of the dialog stack.
    pub fn close_all_dialogs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !overlay_mutation_allowed("close_all_dialogs") {
            return;
        }
        if self.dialogs.is_empty() {
            return;
        }

        let restore = self
            .dialogs
            .first()
            .and_then(|dialog| dialog.restore_focus.clone());
        self.dialogs.clear();
        restore_focus(restore, window, cx);
        cx.notify();
    }

    /// Opens a sheet on the given edge, replacing any sheet already open.
    ///
    /// Replacing rather than stacking keeps the focus record honest: the
    /// incoming sheet inherits the outgoing one's restore target, so closing it
    /// returns focus to the window rather than to a sheet that no longer
    /// exists.
    pub fn open_sheet(
        &mut self,
        placement: Placement,
        content: AnyView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !overlay_mutation_allowed("open_sheet") {
            return;
        }

        let focus_handle = cx.focus_handle();
        let restore_focus = self
            .sheet
            .take()
            .and_then(|sheet| sheet.restore_focus)
            .or_else(|| window.focused(cx).map(|handle| handle.downgrade()));
        focus_handle.focus(window, cx);

        self.sheet = Some(ActiveSheet {
            content,
            placement,
            focus_handle,
            restore_focus,
        });
        cx.notify();
    }

    /// Closes the sheet, returning whether one was open.
    pub fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !overlay_mutation_allowed("close_sheet") {
            return false;
        }

        let Some(sheet) = self.sheet.take() else {
            return false;
        };
        restore_focus(sheet.restore_focus, window, cx);
        cx.notify();
        true
    }

    /// Posts a toast to the stack.
    ///
    /// Toasts never take focus: they report what already happened, so stealing
    /// focus from the work that caused them would be a regression, not a
    /// notification. `_window` is unused but kept in the signature because a
    /// `&mut Window` is what makes this operation legal at all — it can only be
    /// reached from an `Event` or `Task` call scope (`docs/gpui-shell.md`
    /// §16.2).
    pub fn push_toast(
        &mut self,
        toast: ToastRequest,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !overlay_mutation_allowed("push_toast") {
            return;
        }

        let id = toast.id.clone().unwrap_or_else(|| {
            self.next_toast_ordinal += 1;
            SharedString::from(format!("shell-toast-{}", self.next_toast_ordinal))
        });
        let options = ToastOptions {
            timeout: toast.timeout,
        };
        self.toasts
            .push(id, toast, options, cx.background_executor().now());
        cx.notify();
    }

    /// Begins a toast's exit, returning whether it was mounted and active.
    ///
    /// The toast stays mounted until its exit transition finishes, which is why
    /// this reports "started closing" rather than "removed": a caller that
    /// wants to know when it is gone should read [`ShellRoot::toast_count`].
    pub fn remove_toast(&mut self, id: impl Into<SharedString>, cx: &mut Context<Self>) -> bool {
        let dismissed = self
            .toasts
            .dismiss(&id.into(), cx.background_executor().now());
        if dismissed {
            cx.notify();
        }
        dismissed
    }

    /// Begins the exit of every active toast, for a script clearing its own
    /// notifications.
    pub fn clear_toasts(&mut self, cx: &mut Context<Self>) {
        if !self
            .toasts
            .dismiss_all(cx.background_executor().now())
            .is_empty()
        {
            cx.notify();
        }
    }

    /// Samples the toast lifecycle clock forever, until the root is dropped.
    ///
    /// Toast timeouts are the one part of this host that advances without any
    /// input, so nothing else would wake the view to retire them.
    fn spawn_toast_clock(window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(TOAST_TICK).await;
                if this.update(cx, |this, cx| this.advance_toasts(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn advance_toasts(&mut self, cx: &mut Context<Self>) {
        // Timers pause while the user is reading the stack, not while the window
        // is inactive: a visible side-by-side window would hold its toasts forever.
        let paused = self.toast_state.is_expanded();
        if self
            .toasts
            .advance(cx.background_executor().now(), paused)
            .changed
        {
            cx.notify();
        }
    }

    fn on_action_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        cycle_focus(true, window, cx);
    }

    fn on_action_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        cycle_focus(false, window, cx);
    }

    /// Builds the dialog stack, oldest first.
    ///
    /// Each dialog gets Base's dialog host, which supplies the focus trap, the
    /// Escape action, and the backdrop press handling; what this adds is the
    /// stack semantics — layer index, who is topmost, who draws the backdrop,
    /// and closing through the root rather than through a per-dialog handle.
    fn dialog_layer(
        &self,
        colors: &ColorTokens,
        radius: &RadiusTokens,
        spacing: &SpacingTokens,
        cx: &mut Context<Self>,
    ) -> Vec<Dialog> {
        let root = cx.entity();
        let topmost_index = self.dialogs.len().saturating_sub(1);

        self.dialogs
            .iter()
            .enumerate()
            .map(|(index, dialog)| {
                let topmost = index == topmost_index;
                let root = root.clone();
                rebuild_script_overlay(&dialog.content, cx);

                Dialog::new(cx)
                    .focus_handle(dialog.focus_handle.clone())
                    .layer(index, topmost)
                    // Keyboard dismissal belongs to the topmost dialog alone,
                    // stated here rather than left to focus placement.
                    .close_on_escape(topmost && dialog.options.is_escape_dismissable())
                    .close_on_backdrop_press(dialog.options.is_backdrop_dismissable())
                    // Veto Base's "Enter confirms and closes": the dialog's
                    // primary action is script-owned.
                    .on_ok(|_, _, _| false)
                    .request_close(move |_, window, cx| {
                        root.update(cx, |root, cx| {
                            root.close_dialog(window, cx);
                        });
                    })
                    .when(topmost, |this| {
                        this.backdrop(div().absolute().inset_0().bg(backdrop_color()))
                    })
                    .popup(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                v_flex()
                                    .occlude()
                                    .bg(colors.surface)
                                    .text_color(colors.surface_foreground)
                                    .border_1()
                                    .border_color(colors.border)
                                    .rounded(radius.lg)
                                    .p(spacing.lg)
                                    .child(dialog.content.clone()),
                            ),
                    )
            })
            .collect()
    }

    fn sheet_layer(
        &self,
        colors: &ColorTokens,
        spacing: &SpacingTokens,
        cx: &mut Context<Self>,
    ) -> Option<Sheet> {
        let sheet = self.sheet.as_ref()?;
        let root = cx.entity();
        let placement = sheet.placement;
        rebuild_script_overlay(&sheet.content, cx);

        Some(
            Sheet::new(cx)
                .focus_handle(sheet.focus_handle.clone())
                .request_close(move |window, cx| {
                    root.update(cx, |root, cx| {
                        root.close_sheet(window, cx);
                    });
                })
                .overlay(div().absolute().inset_0().bg(backdrop_color()))
                .surface(
                    v_flex()
                        .occlude()
                        .absolute()
                        .map(|this| match placement {
                            Placement::Left => {
                                this.top_0().bottom_0().left_0().w_1_3().border_r_1()
                            }
                            Placement::Right => {
                                this.top_0().bottom_0().right_0().w_1_3().border_l_1()
                            }
                            Placement::Top => this.left_0().right_0().top_0().h_1_3().border_b_1(),
                            Placement::Bottom => {
                                this.left_0().right_0().bottom_0().h_1_3().border_t_1()
                            }
                        })
                        .bg(colors.surface)
                        .text_color(colors.surface_foreground)
                        .border_color(colors.border)
                        .p(spacing.lg)
                        .child(sheet.content.clone()),
                ),
        )
    }

    fn toast_layer(
        &self,
        colors: &ColorTokens,
        radius: &RadiusTokens,
        spacing: &SpacingTokens,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Cloned out of the manager first: building the items needs `cx`
        // mutably for the dismissal listeners.
        let items = self
            .toasts
            .visible(TOAST_VISIBLE_LIMIT)
            .map(|(id, toast, status)| (id.clone(), toast.clone(), status))
            .collect::<Vec<_>>();

        deferred(
            items
                .into_iter()
                .fold(
                    ToastStack::new("shell-toasts", self.toast_state.clone()),
                    |stack, (id, toast, status)| {
                        let dismiss_id = id.clone();
                        stack.item(
                            id.clone(),
                            Toast::new(id)
                                .transition_status(status)
                                .occlude()
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.remove_toast(dismiss_id.clone(), cx);
                                }))
                                .v_flex()
                                .gap(spacing.xxs)
                                .p(spacing.md)
                                .rounded(radius.md)
                                .bg(colors.surface)
                                .text_color(colors.surface_foreground)
                                .border_1()
                                .border_color(level_color(toast.level, colors))
                                .child(toast.title.clone())
                                .children(toast.description.clone().map(|description| {
                                    div().text_color(colors.muted_foreground).child(description)
                                })),
                        )
                    },
                )
                .placement(Anchor::TopRight)
                .focus_handle(self.toast_focus_handle.clone())
                .v_flex()
                .absolute()
                .top(spacing.lg)
                .right(spacing.lg)
                .w(TOAST_WIDTH),
        )
        .with_priority(TOAST_PRIORITY)
    }
}

impl Drop for ShellRoot {
    fn drop(&mut self) {
        if let Some(application) = self.application_generation.take() {
            crate::engine::quickjs::cancel_application_tasks(&application);
        }
        if let Some(policy) = self.application_policy.take() {
            crate::engine::quickjs::cancel_policy_tasks(&policy);
        }
    }
}

impl ShellRoot {
    /// Clears the keyboard when a press lands on nothing that wants it.
    ///
    /// Neither Base nor GPUI owns this: a text field takes focus when it is
    /// pressed and keeps it until something else asks, so clicking the page
    /// beside it leaves a caret blinking in a field the pointer has left. The
    /// window is the only scope that can answer "nothing here wants the
    /// keyboard", and in a shell application this element is the window.
    ///
    /// The test for *did this press land on something focusable* is not a new
    /// one. GPUI transfers focus to a `track_focus` element from a bubble-phase
    /// mouse-down listener and calls `prevent_default` when it does, precisely
    /// so an outer focusable ancestor does not steal it back. The root is the
    /// outermost element, so its own bubble listener runs after every
    /// descendant's, and that flag is already the answer.
    ///
    /// Two things it deliberately does not do. It does not blur when a focus
    /// trap is up: a modal owns the keyboard for as long as it is open, and a
    /// press on its scrim is a dismissal for the modal to interpret, not a
    /// reason to strand the trap with nothing focused. And it does nothing at
    /// all when nothing is focused, because `Window::blur` refreshes the window
    /// whether or not it changed anything, and every background click would
    /// otherwise cost a frame.
    fn blur_on_background_press(
        _: &mut Self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.default_prevented() {
            return;
        }
        if window.focused(cx).is_none() {
            return;
        }
        if active_focus_trap(window, cx).is_some() {
            return;
        }
        window.blur(cx);
    }
}

impl Render for ShellRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = Theme::global(cx).tokens;
        let (colors, radius, spacing) = (tokens.colors, tokens.radius, tokens.spacing);

        div()
            .id("shell-root")
            // The window's base text size, from the theme rather than from
            // GPUI's default rem.
            //
            // Everything this root draws itself -- toasts, the sheet, the
            // dialog scrim's chrome -- states no size of its own and inherits
            // this one. Without it that chrome sat at GPUI's 16px default
            // while the components an application builds set their own sizes,
            // so a dense application drawn at 12px got 16px notifications over
            // it. `md` is the base by the library's own convention: it is what
            // `gpui_component::Theme` already takes its `font_size` from.
            //
            // The default `md` is that same 16px, so a theme that says nothing
            // about type is drawn exactly as it was before this existed.
            .text_size(tokens.typography.md.size)
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_tab))
            .on_action(cx.listener(Self::on_action_tab_prev))
            .on_action(cx.listener(|_, _: &Copy, window, cx| {
                let text = TextSelection::selected_text(window, cx).trim().to_owned();
                if text.is_empty() {
                    cx.propagate();
                } else {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(Self::blur_on_background_press),
            )
            .relative()
            .size_full()
            .bg(colors.background)
            .text_color(colors.foreground)
            // Painted back to front; see the stacking order on `ShellRoot`.
            .child(TextSelectionLayer)
            .child(
                self.content
                    .clone()
                    .cached(StyleRefinement::default().size_full()),
            )
            .children(self.sheet_layer(&colors, &spacing, cx))
            .children(self.dialog_layer(&colors, &radius, &spacing, cx))
            .child(self.toast_layer(&colors, &radius, &spacing, cx))
            .child(self.tooltip_overlay.clone())
            .children(self.fps_layer(window, cx))
    }
}

/// How long a tooltip takes to slide and fade in.
const TOOLTIP_ENTER: Duration = Duration::from_millis(150);

/// How long the box takes to travel when the pointer moves straight from one
/// trigger to the next one beside it.
const TOOLTIP_SLIDE: Duration = Duration::from_millis(200);

/// How far apart two triggers may sit vertically and still count as the same
/// row. Past that the box is somewhere else entirely and sliding it there reads
/// as a stray element rather than as one label following the pointer.
const TOOLTIP_SAME_ROW: gpui::Pixels = px(10.);

/// What an appearing tooltip does on its way in.
///
/// The whole of the root's presentation for the layer: base decides *when* a
/// tooltip is up and where the box goes, and hands back the transition it is
/// in. Enter slides up and fades; a switch between two triggers on one row
/// slides across, because the box that was already up is the same box.
fn render_tooltip(
    content: AnyView,
    transition: TooltipTransition,
    _: &mut Window,
    _: &mut App,
) -> AnyElement {
    div().child(content).map(|element| match transition {
        TooltipTransition::Switch {
            epoch,
            previous,
            current,
        } => {
            if (current.origin.y - previous.origin.y).abs() >= TOOLTIP_SAME_ROW {
                return element.into_any_element();
            }
            let travelled = current.center().x - previous.center().x;
            EffectTransition::new(TOOLTIP_SLIDE)
                .ease(ease_in_out_cubic)
                .slide_x(-travelled, px(0.))
                .apply(
                    element,
                    ElementId::NamedInteger("shell-tooltip-slide".into(), epoch as u64),
                )
                .into_any_element()
        }
        TooltipTransition::Enter { epoch } => EffectTransition::new(TOOLTIP_ENTER)
            .ease(ease_out_cubic)
            .slide_y(px(4.), px(0.))
            .fade(0.0, 1.0)
            .apply(
                element,
                ElementId::NamedInteger("shell-tooltip-enter".into(), epoch as u64),
            )
            .into_any_element(),
    })
}

/// How a dialog may be dismissed.
///
/// Both default to `true`. A dialog that refuses dismissal is asking a question
/// the user has to answer, which is rare enough that it should be spelled out
/// at the call site.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DialogOptions {
    escape_dismissable: bool,
    backdrop_dismissable: bool,
}

impl Default for DialogOptions {
    fn default() -> Self {
        Self {
            escape_dismissable: true,
            backdrop_dismissable: true,
        }
    }
}

impl DialogOptions {
    /// Sets whether Escape closes this dialog while it is topmost.
    pub fn escape_dismissable(mut self, dismissable: bool) -> Self {
        self.escape_dismissable = dismissable;
        self
    }

    /// Sets whether pressing the backdrop closes this dialog.
    pub fn backdrop_dismissable(mut self, dismissable: bool) -> Self {
        self.backdrop_dismissable = dismissable;
        self
    }

    /// Whether Escape closes this dialog.
    pub fn is_escape_dismissable(self) -> bool {
        self.escape_dismissable
    }

    /// Whether a backdrop press closes this dialog.
    pub fn is_backdrop_dismissable(self) -> bool {
        self.backdrop_dismissable
    }
}

/// How urgent a toast is.
///
/// Kept to four levels because that is the set a script can name in
/// `cx.toast({ level: "…" })`, and because the base palette has semantic colors
/// for roughly that many distinctions.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToastLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    /// The name a script uses, in `cx.toast({ level: "success" })`.
    pub fn as_str(self) -> &'static str {
        match self {
            ToastLevel::Info => "info",
            ToastLevel::Success => "success",
            ToastLevel::Warning => "warning",
            ToastLevel::Error => "error",
        }
    }

    /// Parses a script-supplied name; `None` for an unknown one, so the binding
    /// layer reports it instead of quietly downgrading an error to info.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "info" => Some(ToastLevel::Info),
            "success" => Some(ToastLevel::Success),
            "warning" => Some(ToastLevel::Warning),
            "error" => Some(ToastLevel::Error),
            _ => None,
        }
    }
}

/// One toast to post.
///
/// A value rather than a view: a toast is a sentence, not a layout, and keeping
/// it data is what lets the root own the stack's geometry and lifecycle without
/// asking the script to render anything.
#[derive(Clone, Debug)]
pub struct ToastRequest {
    title: SharedString,
    description: Option<SharedString>,
    level: ToastLevel,
    /// `None` means the toast stays until it is dismissed.
    timeout: Option<Duration>,
    /// A caller-chosen identity. Pushing the same id twice replaces the toast
    /// rather than stacking a duplicate, which is what makes a repeated "Saved"
    /// read as one event.
    id: Option<SharedString>,
}

impl ToastRequest {
    /// A toast with the default level and timeout.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            level: ToastLevel::default(),
            timeout: Some(Self::DEFAULT_TIMEOUT),
            id: None,
        }
    }

    /// How long a toast stays before retiring itself, unless overridden. Long
    /// enough to read one sentence, short enough not to accumulate.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

    /// Sets the secondary line shown under the title.
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the urgency level.
    pub fn with_level(mut self, level: ToastLevel) -> Self {
        self.level = level;
        self
    }

    /// Sets how long the toast stays; `None` keeps it until dismissed.
    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the identity used to replace and to dismiss this toast.
    pub fn with_id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn title(&self) -> &SharedString {
        &self.title
    }

    pub fn description(&self) -> Option<&SharedString> {
        self.description.as_ref()
    }

    pub fn level(&self) -> ToastLevel {
        self.level
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn id(&self) -> Option<&SharedString> {
        self.id.as_ref()
    }
}

/// Binds root navigation and selected-text copy once per `App`.
fn install_key_bindings(cx: &mut App) {
    if cx.has_global::<KeyBindingsInstalled>() {
        return;
    }
    cx.bind_keys([
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
    ]);
    cx.set_global(KeyBindingsInstalled);
}

/// Whether an overlay may be opened or closed right now.
///
/// Overlay changes mutate the window, so they are only legal from an `Event` or
/// `Task` call scope (`crate::scope`). Outside any scope — host Rust code, a
/// test — there is nothing to check. A `Render` or `Layout` scope is a script
/// bug: it is reported and ignored rather than panicked on, because a script
/// error must not take the window down (`docs/gpui-shell.md` §5.8).
/// Rebuilds a script overlay's description before it draws.
///
/// An overlay's content is a function, and what it closes over is somebody
/// else's state -- that is the contract `open_dialog` and `open_sheet`
/// document, and the only one they can have: neither answers a view handle, so
/// there is nothing for a script to notify when the state behind the closure
/// moves. Without this, an overlay materializes the description it was built
/// with, once, for as long as it is open: a dialog that looks up what someone
/// typed shows the answer to nothing.
///
/// So the root rebuilds it whenever the root itself draws, which is what
/// `window.refresh()` -- the call whose whole purpose is "there is no view to
/// notify" -- now reaches. Marking it dirty schedules no frame of its own: the
/// overlay is about to render as part of this one, and it renders from the
/// script rather than from the cache.
///
/// A non-script overlay owns its own state and is left alone.
fn rebuild_script_overlay(content: &AnyView, cx: &mut App) {
    if let Ok(view) = content.clone().downcast::<ScriptView>() {
        view.update(cx, |view, _| view.invalidate());
    }
}

fn overlay_mutation_allowed(operation: &str) -> bool {
    match scope::current_phase() {
        None => true,
        Some(phase) if phase.allows_notify() => true,
        Some(phase) => {
            tracing::warn!(
                "`{operation}` is not allowed during the `{}` phase; \
                 overlays may only be opened or closed while handling an event or a task",
                phase.as_str()
            );
            false
        }
    }
}

fn restore_focus(handle: Option<WeakFocusHandle>, window: &mut Window, cx: &mut App) {
    if let Some(handle) = handle.and_then(|handle| handle.upgrade()) {
        window.focus(&handle, cx);
    }
}

/// Moves focus one step, staying inside the active focus trap.
///
/// GPUI's `focus_next` walks the whole window, so inside a trapped overlay it
/// can step past the last focusable child and into the content behind it. When
/// that happens, keep stepping until focus is back inside the trap — that is
/// the wrap-around — and give up if a full cycle finds nothing, so a trap with
/// no focusable child cannot spin.
fn cycle_focus(forward: bool, window: &mut Window, cx: &mut App) {
    let step = |window: &mut Window, cx: &mut App| {
        if forward {
            window.focus_next(cx);
        } else {
            window.focus_prev(cx);
        }
    };

    let Some(trap) = active_focus_trap(window, cx) else {
        step(window, cx);
        return;
    };

    let before = window.focused(cx);
    step(window, cx);

    // A generous bound on the focusable elements in one window; reaching it
    // means the trap has none, not that the window is unusually deep.
    const MAX_STEPS: usize = 100;
    let mut steps = 0;
    while !trap.contains_focused(window, cx) && steps < MAX_STEPS {
        step(window, cx);
        steps += 1;
        if window.focused(cx) == before {
            break;
        }
    }
}

/// The scrim behind a dialog or sheet.
///
/// A fixed translucent black rather than a token: `gpui_base`'s [`ColorTokens`]
/// has no overlay color, and every candidate token inverts with the palette —
/// `foreground` would veil a dark window in white. A scrim is a dimming, not a
/// palette entry.
fn backdrop_color() -> Hsla {
    hsla(0., 0., 0., 0.5)
}

/// The border color that carries a toast's level.
///
/// Border rather than fill: the toast surface stays a surface, and the level is
/// an accent on it. `Warning` borrows the accent pair because the base palette
/// has no warning token — `destructive` is the only semantic status color it
/// defines.
fn level_color(level: ToastLevel, colors: &ColorTokens) -> Hsla {
    match level {
        ToastLevel::Info => colors.border,
        ToastLevel::Success => colors.primary,
        ToastLevel::Warning => colors.accent_foreground,
        ToastLevel::Error => colors.destructive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Entity, TestAppContext, VisualTestContext};

    struct Content;

    impl Render for Content {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn shell_root(cx: &mut TestAppContext) -> (Entity<ShellRoot>, &mut VisualTestContext) {
        cx.update(crate::init);
        cx.add_window_view(|window, cx| {
            let content = cx.new(|_| Content).into();
            ShellRoot::new(content, window, cx)
        })
    }

    fn view(cx: &mut VisualTestContext) -> AnyView {
        cx.update(|_, cx| cx.new(|_| Content).into())
    }

    struct Field {
        handle: FocusHandle,
    }

    impl Render for Field {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            // A focusable box in the top-left corner; everything else in the
            // window is background that wants nothing.
            div().size_full().child(
                div()
                    .id("field")
                    .track_focus(&self.handle)
                    .w(px(100.))
                    .h(px(50.)),
            )
        }
    }

    fn shell_root_with_field(cx: &mut TestAppContext) -> (FocusHandle, &mut VisualTestContext) {
        cx.update(crate::init);
        let handle = std::rc::Rc::new(std::cell::RefCell::new(None));
        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |window, cx| {
                let content = cx
                    .new(|cx| {
                        let focus = cx.focus_handle();
                        *handle.borrow_mut() = Some(focus.clone());
                        Field { handle: focus }
                    })
                    .into();
                ShellRoot::new(content, window, cx)
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let handle = handle.borrow().clone().expect("field focus handle");
        (handle, cx)
    }

    #[gpui::test]
    fn a_press_on_the_background_clears_the_keyboard(cx: &mut TestAppContext) {
        let (field, cx) = shell_root_with_field(cx);

        cx.update(|window, cx| window.focus(&field, cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.update(|window, _| field.is_focused(window)));

        // Well clear of the field, on nothing that tracks focus.
        cx.simulate_click(gpui::point(px(400.), px(400.)), gpui::Modifiers::default());

        assert!(
            !cx.update(|window, _| field.is_focused(window)),
            "a press on the background must not leave a caret blinking in a field the pointer has left"
        );
    }

    #[gpui::test]
    fn a_press_on_a_focusable_element_does_not_clear_it(cx: &mut TestAppContext) {
        let (field, cx) = shell_root_with_field(cx);

        // Pressing the field focuses it, and pressing it again while it is
        // already focused must not blur and refocus it: that churn is what
        // would interrupt an IME composition and fire a spurious blur.
        cx.simulate_click(gpui::point(px(10.), px(10.)), gpui::Modifiers::default());
        assert!(cx.update(|window, _| field.is_focused(window)));

        cx.simulate_click(gpui::point(px(20.), px(20.)), gpui::Modifiers::default());
        assert!(
            cx.update(|window, _| field.is_focused(window)),
            "the root must leave focus alone when the press landed on something that took it"
        );
    }

    #[gpui::test]
    fn closing_the_top_dialog_leaves_the_one_below(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let (first, second) = (view(cx), view(cx));

        root.update_in(cx, |root, window, cx| {
            root.open_dialog(first.clone(), window, cx)
        });
        root.update_in(cx, |root, window, cx| {
            root.open_dialog(second.clone(), window, cx)
        });
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 2);

        assert!(root.update_in(cx, |root, window, cx| root.close_dialog(window, cx)));
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 1);
        assert_eq!(
            root.read_with(cx, |root, _| root.topmost_dialog().map(|v| v.entity_id())),
            Some(first.entity_id())
        );

        // The stack empties one dialog per close, and reports when it is done.
        assert!(root.update_in(cx, |root, window, cx| root.close_dialog(window, cx)));
        assert!(!root.update_in(cx, |root, window, cx| root.close_dialog(window, cx)));
    }

    #[gpui::test]
    fn escape_closes_only_the_topmost_dialog(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let (first, second) = (view(cx), view(cx));

        root.update_in(cx, |root, window, cx| {
            root.open_dialog(first.clone(), window, cx)
        });
        root.update_in(cx, |root, window, cx| root.open_dialog(second, window, cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.simulate_keystrokes("escape");
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 1);
        assert_eq!(
            root.read_with(cx, |root, _| root.topmost_dialog().map(|v| v.entity_id())),
            Some(first.entity_id())
        );
    }

    #[gpui::test]
    fn a_dialog_that_refuses_escape_stays_open(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let content = view(cx);

        root.update_in(cx, |root, window, cx| {
            root.open_dialog_with(
                content,
                DialogOptions::default().escape_dismissable(false),
                window,
                cx,
            )
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.simulate_keystrokes("escape");
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 1);
    }

    #[gpui::test]
    fn closing_a_dialog_restores_the_previous_focus(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let content = view(cx);

        let before = cx.update(|window, cx| {
            let handle = cx.focus_handle();
            handle.focus(window, cx);
            handle
        });

        root.update_in(cx, |root, window, cx| root.open_dialog(content, window, cx));
        assert_ne!(
            cx.update(|window, cx| window.focused(cx)),
            Some(before.clone())
        );

        root.update_in(cx, |root, window, cx| root.close_dialog(window, cx));
        assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(before));
    }

    #[gpui::test]
    fn a_nested_dialog_restores_focus_to_the_one_below(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let (first, second) = (view(cx), view(cx));

        root.update_in(cx, |root, window, cx| root.open_dialog(first, window, cx));
        let outer_focus = cx.update(|window, cx| window.focused(cx));

        root.update_in(cx, |root, window, cx| root.open_dialog(second, window, cx));
        root.update_in(cx, |root, window, cx| root.close_dialog(window, cx));

        assert_eq!(cx.update(|window, cx| window.focused(cx)), outer_focus);
    }

    #[gpui::test]
    fn closing_all_dialogs_restores_the_focus_the_stack_started_from(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let (first, second) = (view(cx), view(cx));

        let before = cx.update(|window, cx| {
            let handle = cx.focus_handle();
            handle.focus(window, cx);
            handle
        });

        root.update_in(cx, |root, window, cx| root.open_dialog(first, window, cx));
        root.update_in(cx, |root, window, cx| root.open_dialog(second, window, cx));
        root.update_in(cx, |root, window, cx| root.close_all_dialogs(window, cx));

        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 0);
        assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(before));
    }

    #[gpui::test]
    fn a_sheet_is_replaced_rather_than_stacked(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let (first, second) = (view(cx), view(cx));

        let before = cx.update(|window, cx| {
            let handle = cx.focus_handle();
            handle.focus(window, cx);
            handle
        });

        root.update_in(cx, |root, window, cx| {
            root.open_sheet(Placement::Right, first, window, cx)
        });
        root.update_in(cx, |root, window, cx| {
            root.open_sheet(Placement::Left, second.clone(), window, cx)
        });
        assert_eq!(
            root.read_with(cx, |root, _| root.sheet().map(|v| v.entity_id())),
            Some(second.entity_id())
        );

        // The replacement inherited the first sheet's restore target, so one
        // close returns to the window rather than to the sheet it replaced.
        assert!(root.update_in(cx, |root, window, cx| root.close_sheet(window, cx)));
        assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(before));
        assert!(!root.update_in(cx, |root, window, cx| root.close_sheet(window, cx)));
    }

    /// Every layer painting at once is the case the stacking order exists for,
    /// and the one where a duplicated element id or a missing hitbox shows up.
    #[gpui::test]
    fn every_layer_draws_together(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        let (sheet, first, second) = (view(cx), view(cx), view(cx));

        root.update_in(cx, |root, window, cx| {
            root.open_sheet(Placement::Right, sheet, window, cx);
            root.open_dialog(first, window, cx);
            root.open_dialog(second, window, cx);
            root.push_toast(ToastRequest::new("Saved"), window, cx);
        });

        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 2);
        assert!(root.read_with(cx, |root, _| root.sheet().is_some()));
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 1);
    }

    #[gpui::test]
    fn a_dismissed_toast_is_unmounted_once_its_exit_completes(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);

        root.update_in(cx, |root, window, cx| {
            root.push_toast(ToastRequest::new("Saved").with_id("saved"), window, cx)
        });
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 1);

        assert!(root.update(cx, |root, cx| root.remove_toast("saved", cx)));
        // Still mounted: dismissal starts the exit transition.
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 1);

        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 0);
    }

    #[gpui::test]
    fn a_toast_retires_itself_when_its_timeout_elapses(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();

        root.update_in(cx, |root, window, cx| {
            root.push_toast(
                ToastRequest::new("Saved").with_timeout(Some(Duration::from_secs(1))),
                window,
                cx,
            )
        });

        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 0);
    }

    #[gpui::test]
    fn a_toast_expires_while_the_window_is_in_the_background(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);

        root.update_in(cx, |root, window, cx| {
            assert!(!window.is_window_active());
            root.push_toast(
                ToastRequest::new("Saved").with_timeout(Some(Duration::from_secs(1))),
                window,
                cx,
            )
        });

        cx.executor().advance_clock(Duration::from_secs(10));
        cx.run_until_parked();
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 0);
    }

    #[gpui::test]
    fn pushing_the_same_toast_id_replaces_it(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);

        for _ in 0..3 {
            root.update_in(cx, |root, window, cx| {
                root.push_toast(ToastRequest::new("Saved").with_id("saved"), window, cx)
            });
        }
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 1);
    }

    #[gpui::test]
    fn toasts_pushed_without_an_id_stack(cx: &mut TestAppContext) {
        let (root, cx) = shell_root(cx);

        for _ in 0..3 {
            root.update_in(cx, |root, window, cx| {
                root.push_toast(ToastRequest::new("Saved"), window, cx)
            });
        }
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 3);
    }

    #[test]
    fn script_facing_names_round_trip() {
        for level in [
            ToastLevel::Info,
            ToastLevel::Success,
            ToastLevel::Warning,
            ToastLevel::Error,
        ] {
            assert_eq!(ToastLevel::from_name(level.as_str()), Some(level));
        }
        assert!(ToastLevel::from_name("fatal").is_none());
    }

    #[test]
    fn toast_request_builder_keeps_every_field() {
        let toast = ToastRequest::new("Saved")
            .with_description("3 files written")
            .with_level(ToastLevel::Success)
            .with_timeout(None)
            .with_id("saved");

        assert_eq!(toast.title(), "Saved");
        assert_eq!(
            toast.description().map(|d| d.as_ref()),
            Some("3 files written")
        );
        assert_eq!(toast.level(), ToastLevel::Success);
        assert_eq!(toast.timeout(), None);
        assert_eq!(toast.id().map(|id| id.as_ref()), Some("saved"));
        assert_eq!(
            ToastRequest::new("Saved").timeout(),
            Some(ToastRequest::DEFAULT_TIMEOUT)
        );
    }

    #[test]
    fn dialog_options_default_to_dismissable() {
        let options = DialogOptions::default();
        assert!(options.is_escape_dismissable());
        assert!(options.is_backdrop_dismissable());

        let options = options
            .escape_dismissable(false)
            .backdrop_dismissable(false);
        assert!(!options.is_escape_dismissable());
        assert!(!options.is_backdrop_dismissable());
    }
}

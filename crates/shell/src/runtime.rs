//! Pieces shared by every scripting engine.
//!
//! Callback storage and the failure surface are the same problem whatever the
//! VM is: handlers belong to exactly one render snapshot, and a script error has
//! to land on screen rather than take the host down. Only the type of the stored
//! handler differs, so it is a type parameter.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Result, anyhow};
use gpui::{
    AnyElement, App, BorderStyle, Bounds, ClipboardItem, Corners, Edges, Element, ElementId,
    Entity, GlobalElementId, Hitbox, Hsla, InspectorElementId, InteractiveElement, IntoElement,
    LayoutId, PaintQuad, ParentElement, Pixels, Point, SharedString, StatefulInteractiveElement,
    Styled, StyledText, WeakEntity, Window, div, px, relative, rems, transparent_black,
};
use gpui_base::{
    Button, ColorTokens, TextSelectionHandle, TextSelectionRegistration, TextSelectionRun,
};

use crate::{spec::CallbackId, view::ScriptView};

/// Where an application's data lives, given who it is.
pub(crate) fn app_data_dir(id: &str) -> Result<PathBuf> {
    validate_identity(id)?;
    Ok(data_home().join("gpui-shell").join("apps").join(id))
}

fn validate_identity(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(anyhow!("an application identity cannot be empty"));
    }
    if id.contains("..") {
        return Err(anyhow!(
            "`{id}` is not a usable application identity: it contains `..`, which would \
             reach outside the data directory"
        ));
    }
    if let Some(character) = id
        .chars()
        .find(|character| !matches!(character, 'a'..='z' | '0'..='9' | '.' | '-' | '_'))
    {
        return Err(anyhow!(
            "`{id}` is not a usable application identity: `{character}` is not allowed. \
             Use lower-case letters, digits, `.`, `-` and `_`"
        ));
    }
    Ok(())
}

/// A bundle id for a host that has only a directory to go on.
///
/// The directory name keeps the folder recognizable and a digest of the full
/// path disambiguates it, so the same directory always reaches the same data and
/// two never collide — including two checkouts of the same source, which are
/// genuinely different installations of something being developed.
///
/// Deliberately *not* what an installed application should use: data keyed by
/// path does not survive the directory moving, which is right while you are
/// editing it and wrong once it is installed.
pub(crate) fn path_identity(root: &Path) -> String {
    let name: String = root
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '-',
        })
        .collect();

    // The digest carries the identity, so a name that sanitizes to nothing — or
    // to something with a leading dot — costs recognizability and not
    // correctness.
    let name = name.replace("..", "-");
    let name = name.trim_matches(['.', '-', '_']);
    let name = if name.is_empty() { "app" } else { name };
    format!("{name}-{:016x}", path_digest(root))
}

/// The platform's per-user data directory, honouring the usual overrides.
pub(crate) fn data_home() -> PathBuf {
    if let Some(explicit) = std::env::var_os("XDG_DATA_HOME").filter(|it| !it.is_empty()) {
        return PathBuf::from(explicit);
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"))
    } else {
        home.join(".local").join("share")
    }
}

/// FNV-1a over the path. Not a security boundary — it only has to keep two
/// directories that have no name of their own from sharing a folder.
fn path_digest(root: &Path) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in root.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Finds the directory an application is rooted at.
///
/// Being pointed at the entry file itself, or at the parent of the real
/// application directory, is the most common way to start — so both are handled
/// here rather than failing with a bare "no such file". The error tells the
/// author what was expected and, when it can tell, where the application
/// actually is.
pub fn resolve_app_root(path: &Path, entry: &str) -> Result<PathBuf> {
    let candidate = if path.is_file() {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        path.to_path_buf()
    };

    if !candidate.exists() {
        return Err(anyhow!("`{}` does not exist", path.display()));
    }

    let root = candidate
        .canonicalize()
        .map_err(|error| anyhow!("cannot read `{}`: {error}", candidate.display()))?;

    if root.join(entry).is_file() {
        return Ok(root);
    }

    Err(anyhow!("{}", missing_entry_message(&root, entry)))
}

fn missing_entry_message(root: &Path, entry: &str) -> String {
    let mut message = format!(
        "no `{entry}` in {}

An application directory must contain {entry},          which default-exports a view class.",
        root.display()
    );

    let nested: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|item| item.path())
        .filter(|path| path.join(entry).is_file())
        .collect();

    match nested.as_slice() {
        [] => {}
        [only] => {
            message.push_str(&format!(
                "

Did you mean `{}`?",
                only.display()
            ));
        }
        several => {
            message.push_str(
                "

Applications found below this directory:",
            );
            for path in several {
                message.push_str(&format!(
                    "
  {}",
                    path.display()
                ));
            }
        }
    }

    message
}

/// One evaluated incarnation of an application.
///
/// Policy answers what code may do; this token answers which incarnation the
/// code belongs to. Reload keeps the policy but replaces this identity. The
/// explicit liveness bit lets every retained entry point reject work from a
/// superseded or rolled-back incarnation without guessing from creation time.
pub(crate) struct ApplicationGeneration {
    id: u64,
    active: Cell<bool>,
}

impl ApplicationGeneration {
    pub(crate) fn new(id: u64) -> Rc<Self> {
        Rc::new(Self {
            id,
            active: Cell::new(true),
        })
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.get()
    }

    pub(crate) fn retire(&self) {
        tracing::trace!(
            application_generation = self.id,
            "retiring shell application generation"
        );
        self.active.set(false);
    }
}

/// A script callback together with the view it was registered from. The view is
/// what a later notify has to reach.
///
/// `pub(crate)` throughout: an engine builds one and the dispatcher reads it,
/// and nothing outside this crate has any use for either half. Narrowing it is
/// what keeps a later field from being a breaking change to somebody else.
pub(crate) struct CallbackEntry<T> {
    pub(crate) value: T,
    pub(crate) view: Option<WeakEntity<ScriptView>>,
    pub(crate) application: Option<Rc<ApplicationGeneration>>,
    /// The host call this callback was registered from, as
    /// [`crate::scope`] numbers them.
    ///
    /// Only the virtualized list reads it, and only to keep the `cx` of that
    /// call usable inside the item renderer it registered — see
    /// [`crate::scope::adopt`]. Every other dispatch opens a scope of its own
    /// and has no use for the one it came from.
    pub(crate) registered_in: Option<u64>,
}

impl<T> CallbackEntry<T> {
    /// Resolves the callback's owner without letting the callback retain it.
    ///
    /// The outer option distinguishes an owner that has gone away from a
    /// callback deliberately registered without a view.
    pub(crate) fn live_view(&self) -> Option<Option<Entity<ScriptView>>> {
        match &self.view {
            Some(view) => view.upgrade().map(Some),
            None => Some(None),
        }
    }
}

impl<T: Clone> Clone for CallbackEntry<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            view: self.view.clone(),
            application: self.application.clone(),
            registered_in: self.registered_in,
        }
    }
}

/// Callbacks live for exactly as long as the snapshot that produced them.
///
/// A script render publishes one [`crate::snapshot::RenderSnapshot`], and that
/// snapshot may be materialized by many GPUI frames. So a handler cannot be
/// retired when a frame ends — only when the snapshot it belongs to is dropped,
/// which is what [`retire`](Self::retire) does.
///
/// Building is staged: [`begin`](Self::begin) opens a generation, handlers
/// accumulate into it, and it becomes reachable only on
/// [`commit`](Self::commit). A script render that fails half-way calls
/// [`abort`](Self::abort) instead, so a failed build leaves no trace — the same
/// transactional rule the snapshot itself follows.
pub(crate) struct CallbackArena<T> {
    next_generation: u64,
    next_callback: CallbackId,
    /// The generation currently being recorded, if a build is open.
    building: Option<(u64, Vec<(CallbackId, CallbackEntry<T>)>)>,
    /// Committed callbacks, indexed by their runtime-unique id. The generation
    /// remains beside each entry so dropping a snapshot can retire its whole
    /// group without making callback lookup linear in the number of handlers.
    live: HashMap<CallbackId, (u64, CallbackEntry<T>)>,
}

impl<T> Default for CallbackArena<T> {
    fn default() -> Self {
        Self {
            next_generation: 0,
            next_callback: 0,
            building: None,
            live: HashMap::new(),
        }
    }
}

impl<T: Clone> CallbackArena<T> {
    pub(crate) fn is_building(&self) -> bool {
        self.building.is_some()
    }

    pub(crate) fn checkpoint(&self) -> usize {
        self.building
            .as_ref()
            .map_or(0, |(_, entries)| entries.len())
    }

    pub(crate) fn rollback_to(&mut self, checkpoint: usize) {
        if let Some((_, entries)) = self.building.as_mut() {
            entries.truncate(checkpoint);
        }
    }

    /// Opens a generation. Any generation left open by an earlier failed build
    /// is discarded rather than committed.
    pub(crate) fn begin(&mut self) -> u64 {
        let generation = self.next_generation;
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .expect("a shell runtime exhausted its callback generations");
        self.building = Some((generation, Vec::new()));
        generation
    }

    /// Publishes the open generation, so its handlers become callable.
    pub(crate) fn commit(&mut self) {
        if let Some((generation, entries)) = self.building.take() {
            self.live.extend(
                entries
                    .into_iter()
                    .map(|(id, entry)| (id, (generation, entry))),
            );
        }
    }

    /// Drops the open generation. A failed script render must not leave
    /// callable handlers behind.
    pub(crate) fn abort(&mut self) {
        self.building = None;
    }

    /// Empties the open generation without closing it.
    ///
    /// The diagnostic retry runs the same render a second time to produce a
    /// better message; the second run must start from an empty index space
    /// rather than stack its handlers on the abandoned ones. The generation
    /// number survives, because the caller is already holding it.
    pub(crate) fn rollback(&mut self) {
        if let Some((_, entries)) = self.building.as_mut() {
            entries.clear();
        }
    }

    /// Releases the handlers of one committed generation, called when the
    /// snapshot that owns them is dropped.
    pub(crate) fn retire(&mut self, generation: u64) {
        self.live
            .retain(|_, (live_generation, _)| *live_generation != generation);
    }

    /// Retains only callbacks accepted by `keep`, including a generation still
    /// being staged. Release uses this to retire an owner's current and
    /// previous generations even when a GPUI frame still retains the entity.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&CallbackEntry<T>) -> bool) {
        if let Some((_, entries)) = self.building.as_mut() {
            entries.retain(|(_, entry)| keep(entry));
        }
        self.live.retain(|_, (_, entry)| keep(entry));
    }

    pub(crate) fn push(&mut self, entry: CallbackEntry<T>) -> CallbackId {
        let Some((_, entries)) = self.building.as_mut() else {
            // Reached only if a handler is registered outside a script render,
            // which is a host bug. An id no lookup can match is the harmless
            // answer.
            tracing::error!("a callback was registered outside a snapshot build");
            return CallbackId::MAX;
        };
        let id = self.next_callback;
        self.next_callback = self
            .next_callback
            .checked_add(1)
            .expect("a shell runtime exhausted its callback ids");
        entries.push((id, entry));
        id
    }

    /// How many committed handlers the arena holds.
    ///
    /// For the regression that says a virtual list does not accumulate them:
    /// its rows are rebuilt every frame, and "the arena is the same size after
    /// a thousand frames of scrolling" is the only way to state that as a fact
    /// rather than as a comment.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.live.len()
    }

    #[cfg(test)]
    pub(crate) fn ids(&self) -> Vec<CallbackId> {
        self.live.keys().copied().collect()
    }

    pub(crate) fn get(&self, id: CallbackId) -> Option<CallbackEntry<T>> {
        self.live.get(&id).map(|(_, entry)| entry.clone())
    }

    /// Releases every stored handler.
    ///
    /// Engines whose values must outlive nothing — QuickJS `Persistent` handles
    /// in particular — call this before tearing the VM down, because a handle
    /// released after its runtime aborts the process.
    pub(crate) fn clear(&mut self) {
        self.building = None;
        self.live.clear();
    }
}

/// What `process.exit(code)` means, decided by the host.
///
/// The script only ever *asks*: one plugin must not be able to take down an
/// application somebody is working in, so the runtime never calls `exit(2)`
/// itself. But a request nobody answers is a lie told in the flattering
/// direction — the script gets a success and nothing happens — so a host that
/// grants the capability installs what it should do, and a host that grants it
/// without installing one is told so at the call rather than never.
///
/// A standalone CLI ends the process. An embedded host might close the plugin's
/// panel, or its window, or refuse.
pub type ExitHandler = Rc<dyn Fn(ExitRequest, &mut Window, &mut App)>;

/// Who asked to exit, and with what code.
///
/// The handler is installed once by the host and may be reached by any script
/// the host is running, so "exit" is not answerable without knowing which one
/// asked: a plugin host closes *that* plugin's panel, and a shell that quit the
/// window instead would let one plugin end another's work.
#[derive(Clone)]
pub struct ExitRequest {
    code: i32,
    view: Option<Entity<ScriptView>>,
}

impl ExitRequest {
    pub(crate) fn new(code: i32, view: Option<Entity<ScriptView>>) -> Self {
        Self { code, view }
    }

    /// The status the script passed, or `0`.
    pub fn code(self: &ExitRequest) -> i32 {
        self.code
    }

    /// The view whose script asked.
    ///
    /// `None` when the request came from outside any view — a module still
    /// loading, or a host call the runtime made itself.
    pub fn view(&self) -> Option<&Entity<ScriptView>> {
        self.view.as_ref()
    }
}

thread_local! {
    static EXIT_HANDLER: RefCell<Option<ExitHandler>> = const { RefCell::new(None) };
}

/// Installs what an exit request does. Replaces any previous handler.
pub fn on_exit_request(handler: impl Fn(ExitRequest, &mut Window, &mut App) + 'static) {
    EXIT_HANDLER.with(|installed| *installed.borrow_mut() = Some(Rc::new(handler)));
}

/// The installed handler, if the host installed one.
///
/// Cloned out rather than borrowed across the call: a handler that reinstalls
/// itself — or that tears down a panel which drops something that does — would
/// otherwise panic on the outstanding borrow.
pub fn exit_handler() -> Option<ExitHandler> {
    EXIT_HANDLER.with(|installed| installed.borrow().clone())
}

/// Forgets the installed handler. A host that goes away should not leave one
/// behind pointing at state it owned.
pub fn clear_exit_handler() {
    EXIT_HANDLER.with(|installed| *installed.borrow_mut() = None);
}

/// A failure reported over an interface that still works.
///
/// A render that throws does not take the last valid description with it — the
/// snapshot is only replaced after a build succeeds — so there is usually still
/// a working interface to show. Blanking it would lose the reader's scroll
/// position, their focus, and whatever they were reading, in exchange for a
/// message that fits in a strip.
///
/// So the strip is what they get: it sits over the interface, says what broke
/// and what to do, and hands over the detail for pasting elsewhere. The
/// interface underneath is one render behind, which the banner says out loud
/// rather than leaving the reader to discover.
pub fn error_banner(message: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let defaults = ColorTokens::default();
    let surface = token("surface", defaults.surface);
    let foreground = token("foreground", defaults.foreground);
    let muted = token("muted_foreground", defaults.muted_foreground);
    let border = token("border", defaults.border);
    let accent = token("destructive", defaults.destructive);

    let copied =
        window.use_keyed_state(SharedString::from("shell-banner-copied"), cx, |_, _| false);
    let is_copied = copied.read(cx).to_owned();
    let payload = format!("This view could not be re-rendered\n\n{message}");

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .flex()
        .flex_col()
        .bg(surface)
        .border_b_1()
        .border_color(border)
        .child(div().h(px(2.)).w_full().bg(accent))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(rems(1.))
                .px(rems(1.))
                .py(rems(0.625))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rems(0.125))
                        .child(
                            div()
                                .text_size(rems(0.75))
                                .line_height(relative(1.4))
                                .text_color(foreground)
                                .child(SharedString::from(
                                    "This view could not be re-rendered; showing the last \
                                     version that worked",
                                )),
                        )
                        .child(
                            div()
                                .text_size(rems(0.6875))
                                .line_height(relative(1.45))
                                .text_color(muted)
                                .child(SharedString::from(first_line(message))),
                        ),
                )
                .child(copy_button(
                    copied, is_copied, payload, foreground, border, muted,
                )),
        )
        .into_any_element()
}

/// A banner has one line for the detail, so it shows the first one and the copy
/// action carries the rest. A stack trace truncated mid-frame reads as noise.
fn first_line(message: &str) -> String {
    relative_error_paths(message.lines().next().unwrap_or(message))
}

/// Shortens paths owned by the process working directory while leaving every
/// external path untouched. This is presentation only: copied diagnostics keep
/// the original path and therefore remain unambiguous.
fn relative_error_paths(message: &str) -> String {
    let Ok(current_dir) = std::env::current_dir() else {
        return message.to_owned();
    };
    let root = current_dir.to_string_lossy();
    message.replace(&format!("{root}/"), "")
}

fn system_monospace_font() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "DejaVu Sans Mono"
    }
}

/// A visible, non-fatal failure surface.
///
/// Used when there is nothing to keep — a view whose very first render failed
/// has no last good interface to put a banner over. The message belongs where
/// the interface was supposed to be.
pub fn error_overlay(message: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    failure_surface(
        "This view could not be rendered",
        message,
        "Fix the script and save; the view re-renders on the next change.",
        window,
        cx,
    )
}

/// The one place a failure becomes an interface.
///
/// Design Guides asks an error to say what happened and what to do next, and to
/// take its colors from semantic roles rather than literals — a failure surface
/// that hardcodes red is unreadable in half the themes it will be seen in. So
/// this is a normal composed surface: one heading, the detail, one recovery
/// line, on the same tokens every other screen uses. `destructive` appears once,
/// as a hairline rule, because emphasis is a budget and the message itself is
/// already the focal point.
///
/// The panel has square corners on purpose: it is not a card floating in the
/// window, it *is* the window's content for as long as the failure lasts.
///
/// A stack trace exists to be pasted somewhere else, so copying it is a first
/// class action rather than something the reader retypes.
pub fn failure_surface(
    heading: &str,
    message: &str,
    recovery: &str,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let defaults = ColorTokens::default();
    let background = token("background", defaults.background);
    let surface = token("surface", defaults.surface);
    let foreground = token("foreground", defaults.foreground);
    let muted = token("muted_foreground", defaults.muted_foreground);
    let border = token("border", defaults.border);
    let accent = token("destructive", defaults.destructive);

    let copied =
        window.use_keyed_state(SharedString::from("shell-failure-copied"), cx, |_, _| false);
    let is_copied = copied.read(cx).to_owned();
    let payload = format!("{heading}\n\n{message}");
    let displayed_message = relative_error_paths(message);
    let selection = window.use_keyed_state(
        SharedString::from("shell-failure-selection"),
        cx,
        |_, cx| TextSelectionHandle::new(displayed_message.clone(), cx),
    );
    let selection = selection.read(cx).clone();

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(background)
        .p(rems(2.))
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .max_w(rems(42.))
                .bg(surface)
                .border_1()
                .border_color(border)
                .child(div().h(px(2.)).w_full().bg(accent))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rems(0.625))
                        .p(rems(1.25))
                        .child(
                            div()
                                .text_size(rems(0.875))
                                .line_height(relative(1.4))
                                .text_color(foreground)
                                .child(SharedString::from(heading.to_owned())),
                        )
                        .child(
                            div()
                                .id("shell-failure-detail")
                                .max_h(rems(18.))
                                .overflow_y_scroll()
                                .font_family(system_monospace_font())
                                .text_size(rems(0.75))
                                .line_height(relative(1.5))
                                .text_color(muted)
                                .child(FailureDetailText::new(selection, displayed_message)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(rems(1.))
                                .pt(rems(0.75))
                                .border_t_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_size(rems(0.75))
                                        .line_height(relative(1.5))
                                        .text_color(muted)
                                        .child(SharedString::from(recovery.to_owned())),
                                )
                                .child(copy_button(
                                    copied, is_copied, payload, foreground, border, muted,
                                )),
                        ),
                ),
        )
        .into_any_element()
}

/// The error detail's `StyledText` adapter for gpui-base selection geometry.
/// Selection state and gestures remain entirely owned by `TextSelection`.
struct FailureDetailText {
    selection: TextSelectionHandle,
    text: SharedString,
    styled_text: StyledText,
}

impl FailureDetailText {
    fn new(selection: TextSelectionHandle, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        Self {
            selection,
            styled_text: StyledText::new(text.clone()),
            text,
        }
    }
}

impl IntoElement for FailureDetailText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FailureDetailText {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.styled_text
            .request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        self.styled_text
            .prepaint(id, inspector_id, bounds, &mut (), window, cx);
        let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);
        let registration = TextSelectionRegistration::new(hitbox.clone(), bounds)
            .with_text_bounds(vec![bounds])
            .with_rendered_element(&self.selection, window, cx);
        self.selection.register(registration, window, cx);
        hitbox
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = self.styled_text.layout().clone();
        let projection = self.selection.update_runs(
            &[TextSelectionRun::new(
                self.text.clone(),
                layout.clone(),
                bounds,
            )],
            cx,
        );
        if let Some(range) = projection.ranges().first().and_then(Clone::clone) {
            paint_selection(&layout, range, window);
        }
        self.styled_text
            .paint(id, inspector_id, bounds, &mut (), &mut (), window, cx);
    }
}

fn paint_selection(layout: &gpui::TextLayout, range: Range<usize>, window: &mut Window) {
    let (Some(start), Some(end)) = (
        layout.position_for_index(range.start),
        layout.position_for_index(range.end),
    ) else {
        return;
    };
    let line_height = layout.line_height();
    let bounds = layout.bounds();
    let mut quads = Vec::new();
    if start.y == end.y {
        quads.push(Bounds::from_corners(
            start,
            Point::new(end.x, end.y + line_height),
        ));
    } else {
        quads.push(Bounds::from_corners(
            start,
            Point::new(bounds.right(), start.y + line_height),
        ));
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
    }
    let color = token("primary", ColorTokens::default().primary).opacity(0.28);
    for bounds in quads {
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

/// Copies the failure, and says so — a copy leaves no visible trace otherwise,
/// which is exactly when confirmation is worth its space.
fn copy_button(
    state: Entity<bool>,
    copied: bool,
    payload: String,
    foreground: Hsla,
    border: Hsla,
    muted: Hsla,
) -> AnyElement {
    Button::new("shell-failure-copy")
        .flex()
        .items_center()
        .justify_center()
        .h(rems(1.625))
        .px(rems(0.75))
        .border_1()
        .border_color(border)
        .text_size(rems(0.75))
        .line_height(relative(1.))
        .text_color(if copied { muted } else { foreground })
        .hover(|style| style.opacity(0.8))
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(payload.clone()));
            state.update(cx, |copied, cx| {
                *copied = true;
                cx.notify();
            });
        })
        .child(SharedString::from(if copied {
            "Copied"
        } else {
            "Copy details"
        }))
        .into_any_element()
}

/// Semantic token with a fallback, because a failure surface must render even
/// when the failure is that the theme never got installed.
fn token(name: &str, fallback: Hsla) -> Hsla {
    crate::theme_tokens::token_color(name).unwrap_or(fallback)
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    fn callback(value: u32) -> CallbackEntry<u32> {
        CallbackEntry {
            value,
            view: None,
            application: None,
            registered_in: None,
        }
    }

    #[test]
    fn callback_generations_do_not_wrap_at_the_old_sixteen_bit_boundary() {
        let mut arena = CallbackArena::default();

        let first_generation = arena.begin();
        let first = arena.push(callback(1));
        arena.commit();

        arena.next_generation = u64::from(u16::MAX) + 1;
        let later_generation = arena.begin();
        let later = arena.push(callback(2));
        arena.commit();

        assert_ne!(first_generation, later_generation);
        assert_ne!(first, later);
        assert_eq!(arena.get(first).map(|entry| entry.value), Some(1));
        assert_eq!(arena.get(later).map(|entry| entry.value), Some(2));

        arena.retire(later_generation);
        assert_eq!(arena.get(first).map(|entry| entry.value), Some(1));
        assert!(arena.get(later).is_none());
    }

    /// The id is joined onto the user's data directory, so an unchecked one
    /// reaches the rest of it. This is the only place that check happens.
    #[test]
    fn an_identity_that_could_escape_the_data_directory_is_refused() {
        for escape in ["../other", "..", "a/b", "a\\b", "a..b"] {
            let error = app_data_dir(escape)
                .err()
                .unwrap_or_else(|| panic!("`{escape}` must be refused"))
                .to_string();
            assert!(
                error.contains(escape),
                "the refusal has to name the id, got: {error}"
            );
        }

        assert!(app_data_dir("").is_err(), "an empty id names nothing");
    }

    #[test]
    fn a_reverse_dns_identity_is_one_path_component() {
        let directory = app_data_dir("com.example.notes").expect("a valid id");
        assert!(directory.ends_with("com.example.notes"));
        assert!(directory.starts_with(data_home()));
    }

    /// Two directories that share a name must not share a folder, and the same
    /// directory must reach the same one every time.
    #[test]
    fn a_path_identity_separates_two_checkouts_of_one_name() {
        let left = path_identity(Path::new("/home/someone/dev/notes"));
        let right = path_identity(Path::new("/home/someone/other/notes"));

        assert_ne!(left, right);
        assert!(left.starts_with("notes-"), "{left}");
        assert_eq!(left, path_identity(Path::new("/home/someone/dev/notes")));
        // And whatever it produces has to survive the same check a host's own
        // id does, or the fallback could reach where a real id cannot.
        assert!(app_data_dir(&left).is_ok());
    }

    /// A directory whose name is entirely unusable still has to land somewhere,
    /// because the digest — not the name — is what keeps two of them apart.
    #[test]
    fn a_path_identity_survives_a_name_with_nothing_usable_in_it() {
        let id = path_identity(Path::new("/tmp/../中文"));
        assert!(app_data_dir(&id).is_ok(), "{id}");
    }

    #[test]
    fn a_path_identity_sanitizes_parent_marker_like_directory_names() {
        let id = path_identity(Path::new("/tmp/my..app"));

        assert!(app_data_dir(&id).is_ok(), "{id}");
        assert!(!id.contains(".."), "{id}");
    }

    #[test]
    fn error_paths_inside_the_working_directory_are_relative() {
        let root = std::env::current_dir().expect("a working directory");
        let message = format!(
            "{}:12: unexpected token",
            root.join("src/main.js").display()
        );
        assert_eq!(
            relative_error_paths(&message),
            "src/main.js:12: unexpected token"
        );

        let external = "/opt/runtime/internal.js:3: host error";
        assert_eq!(relative_error_paths(external), external);
    }
}

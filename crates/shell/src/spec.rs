//! The element description buffer.
//!
//! GPUI elements are values that are consumed when used: `RenderOnce::render`
//! takes `self`, and `ParentElement::child` takes its child by value. A script
//! object therefore cannot *be* an element. Instead a script builder records
//! operations into this arena, and [`crate::materialize`] replays them into real
//! elements inside `Render::render`.
//!
//! One arena is the runtime's scratch space, reset at the start of every script
//! render; a successful render freezes it into a
//! [`crate::snapshot::RenderSnapshot`] and leaves a fresh one behind. Reading it
//! is therefore non-destructive: the same description is replayed by every GPUI
//! frame that materializes the snapshot, which is what keeps repainting off the
//! VM.

use std::{collections::HashSet, rc::Rc};

use gpui::SharedString;
use smallvec::SmallVec;

use crate::HostValue;
use crate::value::Bridged;

/// Index of a node inside a [`SpecArena`].
pub type SpecId = u32;

/// A hash of a description's *shape*, with the values left out of it.
///
/// Two renders of one view that differ only in a price, a label or a handler's
/// identity produce the same fingerprint. One that takes a different branch,
/// grows a node, or calls a different style method does not. That distinction
/// is the whole question a template cache turns on — §20.7 of
/// `docs/gpui-shell.md` — and answering it is the reason this exists: a
/// description that repeats its predecessor's shape is one a template could
/// have filled instead of rebuilt.
///
/// It is accumulated while the description is recorded rather than walked out
/// of the arena afterwards, because a walk that costs `arena.len()` is the cost
/// a template cache would be trying to remove, and instrumentation that
/// distorts the thing it measures is worth less than no instrumentation.
///
/// **Equality here is evidence, not proof.** Two different shapes can collide,
/// and the hash deliberately drops payloads that a stricter definition might
/// keep (see [`Component::shape`]). That is the right trade for a counter. It
/// would not be for a cache that skipped work on the strength of it — §20.7's
/// first problem is that validity has to come from the call site rather than
/// from a comparison after the fact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct StructureFingerprint(u64);

/// One step of the fingerprint's mixer.
///
/// `DefaultHasher` would do the same job, but this sits on the recording path —
/// two or three calls per recorded builder operation — and SipHash's cost would
/// be measured against the ~90 ns floor one recorded call has (§20.6). This is
/// SplitMix64's finalizer with the running state rotated in, which is a handful
/// of instructions and still moves every output bit when one input bit moves.
#[inline]
fn mix(state: u64, value: u64) -> u64 {
    let mut hashed = state.rotate_left(23) ^ value.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    hashed ^= hashed >> 30;
    hashed = hashed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    hashed ^ (hashed >> 27)
}

/// A method name from the reflection table is identified by *where* it is, not
/// by what it says: the same builder method always arrives carrying the same
/// `&'static str`, so its pointer separates it from every other name for the
/// price of one mix. Hashing the bytes instead would put a twenty-byte loop on
/// the recording path to learn nothing more. The length is folded in as well so
/// that two names sharing a prefix inside one interned blob cannot alias.
#[inline]
fn static_name(name: &'static str) -> u64 {
    mix(name.as_ptr() as u64, name.len() as u64)
}

/// Reduces anything `Hash` to one `u64` through [`mix`].
///
/// Used only for values that are not on the hot path — an enum discriminant,
/// and an action's script-defined name.
fn hashed<T: std::hash::Hash>(value: &T) -> u64 {
    #[derive(Default)]
    struct Mixer(u64);

    impl std::hash::Hasher for Mixer {
        fn finish(&self) -> u64 {
            self.0
        }

        fn write(&mut self, bytes: &[u8]) {
            for byte in bytes {
                self.0 = mix(self.0, u64::from(*byte));
            }
        }

        // Integers are written whole rather than byte by byte, which is what
        // keeps a discriminant to a single mix.
        fn write_u8(&mut self, value: u8) {
            self.0 = mix(self.0, u64::from(value));
        }

        fn write_u16(&mut self, value: u16) {
            self.0 = mix(self.0, u64::from(value));
        }

        fn write_u32(&mut self, value: u32) {
            self.0 = mix(self.0, u64::from(value));
        }

        fn write_u64(&mut self, value: u64) {
            self.0 = mix(self.0, value);
        }

        fn write_usize(&mut self, value: usize) {
            self.0 = mix(self.0, value as u64);
        }

        fn write_i64(&mut self, value: i64) {
            self.0 = mix(self.0, value as u64);
        }

        fn write_isize(&mut self, value: isize) {
            self.0 = mix(self.0, value as u64);
        }
    }

    let mut mixer = Mixer::default();
    value.hash(&mut mixer);
    std::hash::Hasher::finish(&mixer)
}

/// Runtime-unique identifier for a script callback.
pub type CallbackId = u64;

/// Retained description of GPUI's reusable `Background` value.
#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundSpec {
    pub kind: BackgroundKind,
    pub opacity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BackgroundKind {
    Solid {
        color: String,
    },
    LinearGradient {
        angle: f32,
        from: (String, f32),
        to: (String, f32),
        color_space: String,
    },
    PatternSlash {
        color: String,
        width: f32,
        interval: f32,
    },
    Checkerboard {
        color: String,
        size: f32,
    },
}

#[derive(Clone)]
pub(crate) struct ModuleComponentSpec {
    pub(crate) module: SharedString,
    pub(crate) component: SharedString,
    pub(crate) id: SharedString,
    pub(crate) props: HostValue,
    pub(crate) policy: Rc<crate::policy::Policy>,
}

impl std::fmt::Debug for ModuleComponentSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ModuleComponentSpec")
            .field("module", &self.module)
            .field("component", &self.component)
            .field("id", &self.id)
            .field("props", &self.props)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ModuleComponentSpec {
    fn eq(&self, other: &Self) -> bool {
        self.module == other.module
            && self.component == other.component
            && self.id == other.id
            && self.props == other.props
            && Rc::ptr_eq(&self.policy, &other.policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextViewFormat {
    Html,
    Markdown,
}

/// Which constructor produced a node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Component {
    Div,
    HFlex,
    VFlex,
    Module(ModuleComponentSpec),
    TextView {
        id: SharedString,
        text: SharedString,
        format: TextViewFormat,
    },
    /// A retained nested script view. The frozen description keeps the entity
    /// itself alive, so releasing the numeric handle cannot invalidate a frame
    /// that was already published.
    ChildView(ChildViewSpec),
    Text(String),
    #[allow(dead_code)] // Constructed by descriptor-driven QuickJS installation in the next layer.
    Registered(RegisteredComponentSpec),
    Button(String),
    Link(String),
    Checkbox(String),
    Switch(String),
    /// A scrollbar the script places itself, driving the scroll area carrying
    /// the same id. Pairing by name is the whole of the wiring — the two share
    /// one `ScrollHandle` in window element state — which is what lets a bar
    /// sit beside a fixed header, span two panes, or scroll a list that paints
    /// no bar of its own.
    Scrollbar(String),
    /// A text input, addressed by its entity handle rather than by an id: the
    /// state is what identifies it, and the state outlives the description.
    Input(crate::entities::EntityHandle),
    /// Multi-line text, addressed by its entity handle for the same reason as
    /// [`Component::Input`]. A separate variant rather than a flag on that one,
    /// because `TextareaState` is a different Rust type: the store cannot hand
    /// out one where the other was asked for, and `Textarea::new` will not take
    /// an `InputState`.
    Textarea(crate::entities::EntityHandle),

    /// A spinbutton frame over the same `InputState` a [`Component::Input`]
    /// holds: there is no numeric state type, only a text state carrying a
    /// step, a range and a numeric mask. So the handle identifies it exactly as
    /// it identifies an input, and the element carries only what a step button
    /// looks like and what happens when one is pressed.
    NumberInput(crate::entities::EntityHandle),
    /// A fixed-length one-time code, addressed by its entity handle exactly as
    /// [`Component::Input`] is. Unlike every other bound component, its cells
    /// are not described by the script: base draws none, and a described cell
    /// would be frozen at the digit the last script render saw. See
    /// `materialize::components::otp_input`.
    OtpInput(crate::entities::EntityHandle),
    /// A vector image, loaded from the application's own directory.
    Svg(String),
    /// An accordion root: a group, and nothing else on screen.
    Accordion(String),
    /// One item: it connects a header with a panel and passes its own `open`
    /// down to both, which is the whole of what it does.
    AccordionItem,
    /// The heading that owns one item's trigger.
    AccordionHeader,
    /// The region an item reveals. Unmounted while shut unless
    /// `keep_mounted(true)` says otherwise.
    AccordionPanel,
    /// The button that asks for the opposite of the item's `open`.
    AccordionTrigger(String),
    /// A pagination root: a navigation landmark carrying the announced label,
    /// and nothing else on screen.
    ///
    /// The page buttons are the script's own elements. What base contributes
    /// that a script could not write for itself is the ellipsis layout, and
    /// that is a calculation rather than an element — it is exported as
    /// `pagination_items(...)`, not as a component.
    Pagination(String),
    /// An avatar root: it renders its `image` slot, or its `fallback` slot when
    /// there is no image, and nothing else. The picture is the script's.
    Avatar,
    /// The image slot of a [`Component::Avatar`], loaded from the
    /// application's own directory exactly as [`Component::Image`] is.
    ///
    /// A component of its own rather than a plain `Image` in the slot, because
    /// base's `Avatar::image` takes an `AvatarImage` and not an element: the
    /// slot has to be resolved into that type, which needs the path back.
    AvatarImage(String),
    /// The fallback slot: an ordinary box holding whatever stands in for the
    /// image — initials, a shape, an svg the script supplies.
    AvatarFallback,
    /// A full-color image, loaded from the application's own directory.
    Image(String),
    /// A native GPUI path. Its geometry is retained in method operations and
    /// resolved against final element bounds during prepaint.
    Path {
        fill: bool,
        background: BackgroundSpec,
        stroke_width: f32,
    },
    /// A tab list. It holds no selection of its own: each `Tab` is told
    /// whether it is selected and reports activation through `on_click`.
    Tabs(String),
    /// One tab in a [`Component::Tabs`].
    Tab(String),
    /// A progress root. It carries the progress role and the announced
    /// `0..=100` value, and draws nothing: the visible bar is a
    /// [`Component::ProgressTrack`] holding a [`Component::ProgressIndicator`],
    /// both styled by the script.
    Progress(String),
    /// The groove of a progress bar. A plain element with no semantics of its
    /// own — the announcement lives on the [`Component::Progress`] around it.
    ProgressTrack,
    /// The filled part of a progress bar, sized by the script from the same
    /// number it gave [`Component::Progress`].
    ProgressIndicator,
    /// The native per-window performance HUD supplied by `gpui-fps`.
    FpsMonitor,
    /// A slider's behavior root, addressed by its entity handle for the same
    /// reason [`Component::Input`] is: the state is what identifies it, and it
    /// outlives the description. It draws nothing at all — the three parts
    /// below are the whole of what is on screen — and announces the value.
    Slider(crate::entities::EntityHandle),
    /// The press and drag surface of a [`Component::Slider`].
    SliderTrack(crate::entities::EntityHandle),
    /// The groove, and the one part that records the box every pointer
    /// position is measured against. A slider without one cannot be moved.
    SliderIndicator(crate::entities::EntityHandle),
    /// The knob. Its position along the axis is read from the state while it
    /// is materialized, never described by the script.
    SliderThumb(crate::entities::EntityHandle),
    /// One option in a radio group. It reports only *becoming* checked: base
    /// drops the handler once the radio is checked or disabled, because a
    /// radio cannot deselect itself.
    Radio(String),
    /// A button that stays down. Controlled through `pressed`.
    Toggle(String),
    /// A set of radios. It holds no selection of its own: each radio is told
    /// whether it is checked and reports the change through `on_change`.
    RadioGroup(String),
    /// A set of toggles, announced as a toolbar. Like [`Component::RadioGroup`]
    /// it holds no state; each toggle carries its own.
    ToggleGroup(String),
    /// A semantic table root. It has no data source and no delegate: the
    /// script nests the groups, rows and cells itself, exactly as HTML does.
    Table(String),
    /// The header row group of a [`Component::Table`].
    TableHeader(String),
    /// The body row group of a [`Component::Table`].
    TableBody(String),
    /// One row, carrying the one-based index it occupies in the whole table so
    /// a screen reader can place it even when only a window of rows is drawn.
    TableRow(String, usize),
    /// One column header, carrying its one-based column index.
    TableHead(String, usize),
    /// One data cell, carrying its one-based column index.
    TableCell(String, usize),
    /// The slot a caption belongs in. It carries no caption role today, so it
    /// says where a caption goes rather than what one means.
    TableCaption(String),
    /// A row or column of panes a user drags apart, carrying the axis its
    /// constructor chose. The axis is part of what the node *is*: base decides
    /// it in `h_resizable` / `v_resizable` and every panel inside reads it, so
    /// there is no builder to change it afterwards.
    Resizable(String, gpui::Axis),
    /// One pane of a [`Component::Resizable`]. It has no id of its own: base
    /// numbers the panels by their position in the group, which is also how the
    /// group's stored sizes are addressed.
    ResizablePanel,
    /// A region whose `content` slot is rendered only while it is open.
    /// Ordinary children are always rendered; the gate applies to the slot
    /// alone.
    Collapsible,
    /// A click-driven anchored surface with a `trigger` slot and a `content`
    /// slot. Controlled: the script holds the open state and is told when the
    /// pointer changed it.
    Popover(String),
    /// A hover-driven anchored surface with the same two slots. It owns its own
    /// open state, so there is nothing to control — only how long the pointer
    /// has to rest before it appears and after it leaves.
    HoverCard(String),
    /// The bare anchored surface underneath [`Component::Popover`]: trigger
    /// measurement, corner arithmetic, deferred paint above the rest of the
    /// window, and window-edge snapping. It holds no open state at all — it
    /// shows whatever is in its `content` slot, so a script opens and closes it
    /// by filling that slot or leaving it empty.
    Popup(String),
    /// A combobox root. It holds no options and no value: it owns the combobox
    /// role, the controlled `open` state, and the transfer of the keyboard
    /// between the trigger and the popup content.
    Select(String),
    /// The same root, announced and keyed as a combobox whose trigger is an
    /// editable field. Base forwards it to `Select` verbatim.
    Combobox(String),
    /// A date-picker root, carrying the focus handle its trigger takes the
    /// keyboard through. The handle is a constructor argument because base's
    /// `DatePicker::new` requires it: a picker without one has no trigger the
    /// keyboard can reach. It holds no date — the calendar does.
    DatePicker(String, crate::entities::EntityHandle),
    /// A dockable layout, addressed by its entity handle for the same reason
    /// [`Component::Input`] is: the layout is the state, it outlives every
    /// description, and the user changes it without a script render.
    ///
    /// Nothing under it is described. Its panels are entities the script handed
    /// it, and its chrome is drawn by handlers this node carries — see
    /// [`crate::dock`] — so the node itself is the whole of the description.
    DockArea(crate::entities::EntityHandle),
    /// Where a dock's own content goes inside the chrome the script drew
    /// around it.
    ///
    /// Base hands a dock's content to the chrome as a finished element and
    /// takes back whatever the chrome returns, so a chrome that wants both has
    /// to place the content itself. An element cannot cross into script, so
    /// this stands in for it: `dock_content()` describes the position, and
    /// materialization puts the real element there.
    DockContent,
    /// A virtualized list: the one component whose description is not the whole
    /// of what it draws. Its rows come from a callback GPUI runs during layout,
    /// so this node carries only the list itself. See [`VirtualListSpec`] and
    /// the exception recorded in [`crate::materialize`].
    VirtualList(Rc<VirtualListSpec>),
    /// GPUI's own lazy lists, driven the same way as [`Component::VirtualList`]:
    /// the items come from a callback run during layout. `list` measures every
    /// item it draws, so rows need not state a height; `uniform_list` measures
    /// one and places the rest by it. See [`ListSpec`].
    List(Rc<ListSpec>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegisteredComponentSpec {
    id: crate::ComponentId,
    name: &'static str,
    payload: crate::ComponentPayload,
}

impl RegisteredComponentSpec {
    #[allow(dead_code)] // Constructed by descriptor-driven QuickJS installation in the next layer.
    pub fn new(
        id: crate::ComponentId,
        name: &'static str,
        payload: crate::ComponentPayload,
    ) -> Self {
        Self { id, name, payload }
    }

    pub fn id(&self) -> crate::ComponentId {
        self.id
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn payload(&self) -> &crate::ComponentPayload {
        &self.payload
    }
}

/// The retained entity mounted by one `child_view(handle)` description.
///
/// Equality and diagnostics use the runtime-unique handle. The entity is the
/// frame lease: materialization never looks the handle up again.
#[derive(Clone)]
pub struct ChildViewSpec {
    handle: crate::entities::EntityHandle,
    view: gpui::Entity<crate::view::ScriptView>,
}

impl ChildViewSpec {
    pub(crate) fn new(
        handle: crate::entities::EntityHandle,
        view: gpui::Entity<crate::view::ScriptView>,
    ) -> Self {
        Self { handle, view }
    }

    pub(crate) fn handle(&self) -> crate::entities::EntityHandle {
        self.handle
    }

    pub(crate) fn view(&self) -> &gpui::Entity<crate::view::ScriptView> {
        &self.view
    }
}

impl std::fmt::Debug for ChildViewSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChildViewSpec")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ChildViewSpec {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

/// What a virtualized list is, beyond its styles.
///
/// Behind an [`Rc`] in [`Component::VirtualList`] because a component is cloned
/// once per node per frame and this is the only variant carrying a vector.
///
/// That vector is the reason the script API is not a literal mirror of
/// `v_virtual_list`. Base wants one `Size` per item, and the *length of that
/// vector is the item count* — so a hundred-thousand-row list would mean a
/// hundred thousand numbers crossing the language boundary on every script
/// render. The script gives a count and either one size or one per item
/// instead, and the vector base wants is built here, once, while the
/// description is being recorded rather than once per frame.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualListSpec {
    id: String,
    axis: gpui::Axis,
    sizes: Rc<Vec<gpui::Size<gpui::Pixels>>>,
    get_key: CallbackId,
    render_items: CallbackId,
}

impl VirtualListSpec {
    pub fn new(
        id: String,
        axis: gpui::Axis,
        sizes: Rc<Vec<gpui::Size<gpui::Pixels>>>,
        get_key: CallbackId,
        render_items: CallbackId,
    ) -> Self {
        Self {
            id,
            axis,
            sizes,
            get_key,
            render_items,
        }
    }

    /// The name that pairs the list with a `Scrollbar`, and its GPUI identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn axis(&self) -> gpui::Axis {
        self.axis
    }

    /// One extent per item. Its length is the item count.
    pub fn sizes(&self) -> &Rc<Vec<gpui::Size<gpui::Pixels>>> {
        &self.sizes
    }

    /// Resolves the stable domain key for one current item index.
    pub fn get_key(&self) -> CallbackId {
        self.get_key
    }

    /// The handler that describes one window of items.
    pub fn render_items(&self) -> CallbackId {
        self.render_items
    }
}

/// Which of GPUI's lazy lists a [`ListSpec`] describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    /// `gpui::list`: every drawn item is measured, so heights may differ.
    Measured,
    /// `gpui::uniform_list`: one item is measured and every row takes its height.
    Uniform,
}

/// The parameters of a `list` or `uniform_list` call, held for the frame.
///
/// The same shape as a [`VirtualListSpec`] without the size table: what
/// distinguishes these lists is that GPUI measures the items itself, so the
/// script says how many there are and nothing about how tall.
#[derive(Clone, Debug, PartialEq)]
pub struct ListSpec {
    id: String,
    kind: ListKind,
    item_count: usize,
    get_key: CallbackId,
    render_items: CallbackId,
}

impl ListSpec {
    pub fn new(
        id: String,
        kind: ListKind,
        item_count: usize,
        get_key: CallbackId,
        render_items: CallbackId,
    ) -> Self {
        Self {
            id,
            kind,
            item_count,
            get_key,
            render_items,
        }
    }

    /// The identity the script gave, also the name a `Scrollbar` pairs with.
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> ListKind {
        self.kind
    }

    /// How many items the collection has, visible or not.
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Resolves the stable domain key for one current item index.
    pub fn get_key(&self) -> CallbackId {
        self.get_key
    }

    /// The handler that describes one window of items.
    pub fn render_items(&self) -> CallbackId {
        self.render_items
    }
}

impl Component {
    /// What this node contributes to a [`StructureFingerprint`]: which
    /// constructor produced it, and nothing it carries.
    ///
    /// A `Text`'s string, a `Button`'s id, a `VirtualList`'s item count and a
    /// `ChildView`'s handle are all *values* as far as a template is concerned.
    /// The slot each would fill is decided by the constructor; what is in it
    /// this time is the thing a template would write, so counting it as
    /// structure would answer the wrong question.
    ///
    /// The discriminant is not stable across builds and does not need to be. A
    /// fingerprint is only ever compared against another taken in the same
    /// process, from the same view, one render apart.
    fn shape(&self) -> u64 {
        match self {
            Component::Registered(component) => mix(59, u64::from(component.id().as_u32())),
            _ => hashed(&std::mem::discriminant(self)),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Component::Div => "div",
            Component::HFlex => "h_flex",
            Component::VFlex => "v_flex",
            Component::Module(_) => "module_component",
            Component::TextView { .. } => "TextView",
            Component::ChildView(_) => "child_view",
            Component::Text(_) => "text",
            Component::Registered(component) => component.name(),
            Component::Button(_) => "Button",
            Component::Link(_) => "Link",
            Component::Checkbox(_) => "Checkbox",
            Component::Switch(_) => "Switch",
            Component::Scrollbar(_) => "Scrollbar",
            Component::Input(_) => "Input",
            Component::Textarea(_) => "Textarea",
            Component::NumberInput(_) => "NumberInput",
            Component::OtpInput(_) => "OtpInput",
            Component::Svg(_) => "svg",
            Component::Image(_) => "image",
            Component::Accordion(_) => "Accordion",
            Component::AccordionItem => "AccordionItem",
            Component::AccordionHeader => "AccordionHeader",
            Component::AccordionPanel => "AccordionPanel",
            Component::AccordionTrigger(_) => "AccordionTrigger",
            Component::Pagination(_) => "Pagination",
            Component::Avatar => "Avatar",
            Component::AvatarImage(_) => "AvatarImage",
            Component::AvatarFallback => "AvatarFallback",
            Component::Path { fill: true, .. } => "path fill",
            Component::Path { fill: false, .. } => "path stroke",
            Component::Tabs(_) => "Tabs",
            Component::Tab(_) => "Tab",
            Component::Progress(_) => "Progress",
            Component::ProgressTrack => "ProgressTrack",
            Component::ProgressIndicator => "ProgressIndicator",
            Component::FpsMonitor => "FpsMonitor",
            Component::Slider(_) => "Slider",
            Component::SliderTrack(_) => "SliderTrack",
            Component::SliderIndicator(_) => "SliderIndicator",
            Component::SliderThumb(_) => "SliderThumb",
            Component::Radio(_) => "Radio",
            Component::Toggle(_) => "Toggle",
            Component::RadioGroup(_) => "RadioGroup",
            Component::ToggleGroup(_) => "ToggleGroup",
            Component::Table(_) => "Table",
            Component::TableHeader(_) => "TableHeader",
            Component::TableBody(_) => "TableBody",
            Component::TableRow(..) => "TableRow",
            Component::TableHead(..) => "TableHead",
            Component::TableCell(..) => "TableCell",
            Component::TableCaption(_) => "TableCaption",
            Component::DockArea(_) => "dock_area",
            Component::DockContent => "dock_content",
            // Named after the constructor rather than the type, because the
            // axis is not a call a reader of the dump could otherwise see.
            Component::Resizable(_, gpui::Axis::Horizontal) => "h_resizable",
            Component::Resizable(_, gpui::Axis::Vertical) => "v_resizable",
            Component::ResizablePanel => "resizable_panel",
            Component::Collapsible => "Collapsible",
            Component::Popover(_) => "Popover",
            Component::HoverCard(_) => "HoverCard",
            Component::Popup(_) => "Popup",
            Component::Select(_) => "Select",
            Component::Combobox(_) => "Combobox",
            Component::DatePicker(..) => "DatePicker",
            // Named after the constructor, as `Resizable` is: the axis is not
            // a call a reader of the dump could otherwise see.
            Component::VirtualList(spec) => match spec.axis() {
                gpui::Axis::Vertical => "v_virtual_list",
                gpui::Axis::Horizontal => "h_virtual_list",
            },
            Component::List(spec) => match spec.kind() {
                ListKind::Measured => "list",
                ListKind::Uniform => "uniform_list",
            },
        }
    }
}

/// One recorded builder call.
#[derive(Clone, Debug, PartialEq)]
pub enum SpecOp {
    /// A no-argument style method, addressed by index into the reflection table.
    NullaryStyle(u16),
    /// A style method that takes arguments.
    ParamStyle(&'static str, SmallVec<[Bridged; 2]>),
    /// A component behavior method.
    Method(&'static str, SmallVec<[Bridged; 2]>),
    /// A method recorded by a registered descriptor. Its erased payload is
    /// interpreted only by the adapter that registered the component.
    RegisteredMethod(crate::RecordedComponentMethod),
    /// An event handler pointing into the callback arena.
    Callback(&'static str, CallbackId),
    /// A handler for one named action.
    ///
    /// Its own op rather than a [`SpecOp::Callback`] because the name it
    /// carries is the script's, discovered at run time, and a `Callback` holds
    /// a `&'static str`. Interning every script id to get it into that slot
    /// would leak one `&'static str` per distinct name a reload ever produced,
    /// to buy a variant that already exists.
    ActionCallback(SharedString, CallbackId),
    /// A state style — hover, active, focus — whose declarations were recorded
    /// into a detached node. Reusing the ordinary style methods there is what
    /// keeps state styling from needing a second value grammar.
    StateStyle(&'static str, SpecId),
    /// A named element slot: an element the component renders in a place of
    /// its own rather than among its children — a `Collapsible`'s content, a
    /// popover's trigger, a number input's buttons.
    ///
    /// The element is detached from the tree when the slot is filled, which is
    /// what stops it from also being drawn as an ordinary child. One `children`
    /// list cannot express any of this: the component has to be able to render
    /// this element somewhere else, or not at all.
    Slot(&'static str, SpecId),
}

impl SpecOp {
    /// What this operation contributes to a [`StructureFingerprint`]: which
    /// call it was, and not what was passed to it.
    ///
    /// The two exclusions are the interesting ones. Arguments are left out
    /// because a colour, a length or a label changing is precisely the case a
    /// template exists to serve. And a [`CallbackId`] is left out because it is
    /// minted per render and retired with the snapshot generation
    /// (`snapshot.rs`), so keeping it would make every description containing a
    /// single handler look like a new shape — which would answer the question
    /// this measurement is asking before it was asked.
    ///
    /// What stays in is the identity of the call: the reflection-table index
    /// for a no-argument style, and the method name for everything else. Two
    /// ops that address different nodes stay distinct because the arena mixes
    /// the tree separately, in [`SpecArena::attach`].
    fn shape(&self) -> u64 {
        match self {
            // The table index *is* which style method was called.
            SpecOp::NullaryStyle(index) => mix(1, u64::from(*index)),
            SpecOp::ParamStyle(name, _) => mix(2, static_name(name)),
            SpecOp::Method(name, _) => mix(3, static_name(name)),
            SpecOp::RegisteredMethod(method) => mix(3, static_name(method.name())),
            SpecOp::Callback(name, _) => mix(4, static_name(name)),
            // The name is the script's own, discovered at run time, so there is
            // no pointer to lean on. Action handlers are rare enough per
            // description that hashing the bytes is not on any hot path.
            SpecOp::ActionCallback(name, _) => mix(5, hashed(&name.as_bytes())),
            // The detached node these point at is part of the shape: a hover
            // style and the declarations inside it are one structure.
            SpecOp::StateStyle(name, id) => mix(mix(6, static_name(name)), u64::from(*id)),
            SpecOp::Slot(name, id) => mix(mix(7, static_name(name)), u64::from(*id)),
        }
    }
}

/// One described element: what constructed it, what was called on it, and what
/// was put inside it.
///
/// The fields are private and read through methods. A `pub` field on a type that
/// crosses a crate boundary makes every later field a breaking change, and this
/// one will grow — a stable key, dependency metadata, a source span are all
/// things a description might eventually carry (see the "Public Data Types
/// Across the Seam" rule in `docs/ARCHITECTURE.md`).
#[derive(Clone, Debug, Default)]
pub struct SpecNode {
    component: Option<Component>,
    ops: SmallVec<[SpecOp; 8]>,
    children: SmallVec<[SpecId; 4]>,
}

impl SpecNode {
    /// What constructed this node. `None` only for a node that was never
    /// pushed, which a reader should treat as absent.
    pub fn component(&self) -> Option<&Component> {
        self.component.as_ref()
    }

    /// The builder calls recorded on it, in the order the script made them.
    pub fn ops(&self) -> &[SpecOp] {
        &self.ops
    }

    /// The nodes attached to it, in order.
    pub fn children(&self) -> &[SpecId] {
        &self.children
    }
}

/// The descriptions one call to a virtualized list's item renderer produced.
///
/// A batch of rows is described into an arena of its own rather than into the
/// runtime's scratch arena, which belongs to whichever script render is in
/// progress and is reset by the next one. This one is materialized and dropped
/// inside the layout pass that asked for it, so nothing a row described
/// outlives the frame that drew it — and two batches cannot see each other's
/// nodes.
pub struct ItemSpecs {
    arena: SpecArena,
    roots: SmallVec<[SpecId; 16]>,
    keys: Vec<String>,
}

impl ItemSpecs {
    pub(crate) fn new(arena: SpecArena, roots: SmallVec<[SpecId; 16]>, keys: Vec<String>) -> Self {
        Self { arena, roots, keys }
    }

    pub fn arena(&self) -> &SpecArena {
        &self.arena
    }

    /// One root per item, in the order the script returned them.
    pub fn roots(&self) -> &[SpecId] {
        &self.roots
    }

    /// One stable domain key per item, in the same order as [`Self::roots`].
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

/// Where inside a node a template writes one of a call's arguments.
///
/// The three positions a value can reach in a recorded description, and the
/// only three [`crate::spec::Template`] fills. A slot is addressed by the
/// operation's index rather than by its name because a node may record the same
/// method twice, and the second one is a different position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotSite {
    /// The string a [`Component::Text`] node carries — what `.child(value)`
    /// records.
    Text,
    /// One argument of a recorded [`SpecOp::ParamStyle`] or [`SpecOp::Method`].
    Argument { op: u16, argument: u8 },
    /// The [`CallbackId`] of a recorded [`SpecOp::Callback`]. A handler is a
    /// slot like any other, but the value written into it is minted per call
    /// rather than carried — which is why a template does not make a handler
    /// free, only the structure around it.
    Handler { op: u16 },
}

/// One position a template fills, and which of the call's arguments fills it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Slot {
    node: SpecId,
    site: SlotSite,
    /// The index of the template parameter whose sentinel came to rest here.
    argument: u16,
}

impl Slot {
    pub(crate) fn new(node: SpecId, site: SlotSite, argument: u16) -> Self {
        Self {
            node,
            site,
            argument,
        }
    }

    pub(crate) fn node(&self) -> SpecId {
        self.node
    }

    pub(crate) fn site(&self) -> SlotSite {
        self.site
    }

    pub(crate) fn argument(&self) -> u16 {
        self.argument
    }
}

/// What a call writes into one slot.
#[derive(Clone, Debug)]
pub(crate) enum SlotValue {
    Text(String),
    Value(Bridged),
    Handler(CallbackId),
}

/// A description recorded once, with the positions its values occupy left open.
///
/// The structure half of §20.7's split. Built by running a script's template
/// body a single time with a sentinel in each parameter position, and used
/// afterwards by grafting it into the live arena and writing that call's
/// arguments into [`Self::slots`] — which is the whole of an instantiation, and
/// runs no script at all.
///
/// It holds no [`CallbackId`] of its own: a handler is a slot, minted per call,
/// because a closure recorded at discovery would capture that first call's
/// values for as long as the template lived.
pub(crate) struct Template {
    arena: SpecArena,
    root: SpecId,
    slots: Vec<Slot>,
    arity: usize,
    /// The application whose script defined it.
    ///
    /// A template outlives every render, which is the point of it, so nothing
    /// else would ever free one: the store would grow by one entry per
    /// `template(...)` call site per hot reload, forever. Holding the
    /// generation lets the same release that retires an application's callbacks
    /// and tasks drop its templates too. `None` only for a runtime that has no
    /// application generation at all, which is a test.
    application: Option<Rc<crate::runtime::ApplicationGeneration>>,
}

impl Template {
    pub(crate) fn new(
        arena: SpecArena,
        root: SpecId,
        slots: Vec<Slot>,
        arity: usize,
        application: Option<Rc<crate::runtime::ApplicationGeneration>>,
    ) -> Self {
        Self {
            arena,
            root,
            slots,
            arity,
            application,
        }
    }

    /// Whether this template belongs to the application generation given.
    pub(crate) fn belongs_to(
        &self,
        application: &Rc<crate::runtime::ApplicationGeneration>,
    ) -> bool {
        self.application
            .as_ref()
            .is_some_and(|owner| Rc::ptr_eq(owner, application))
    }

    pub(crate) fn arena(&self) -> &SpecArena {
        &self.arena
    }

    pub(crate) fn root(&self) -> SpecId {
        self.root
    }

    pub(crate) fn slots(&self) -> &[Slot] {
        &self.slots
    }

    pub(crate) fn arity(&self) -> usize {
        self.arity
    }
}

/// Element descriptions for one script render.
#[derive(Default)]
pub struct SpecArena {
    nodes: Vec<SpecNode>,
    /// Total virtual rows whose native size records this render owns.
    virtual_items: usize,
    /// Nodes already attached to a parent. Re-using one is an error, which is
    /// how Rust's move semantics survive the trip into a garbage-collected
    /// language.
    parented: Vec<bool>,
    /// Nodes consumed by an op rather than by a parent — a state style's
    /// declarations, or the element filling a named slot. They still take ops,
    /// but they can never enter the tree.
    claimed: Vec<bool>,
    /// Retained view handles already described in this snapshot. GPUI cannot
    /// mount one entity at two positions in the same tree.
    mounted_views: HashSet<crate::entities::EntityHandle>,
    /// The shape of everything recorded so far. See [`StructureFingerprint`].
    structure: u64,
}

impl SpecArena {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops every node. Called at the start of each script render, on the
    /// runtime's scratch arena — never on a published snapshot.
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.parented.clear();
        self.claimed.clear();
        self.mounted_views.clear();
        self.virtual_items = 0;
        self.structure = 0;
    }

    /// The shape of what has been recorded, values excluded.
    ///
    /// Read from a published snapshot rather than from the scratch arena: the
    /// scratch one is reset by the next render, and the question is what *this*
    /// description looked like beside the one before it.
    pub(crate) fn structure(&self) -> StructureFingerprint {
        StructureFingerprint(self.structure)
    }

    pub(crate) fn claim_virtual_items(&mut self, count: usize, limit: usize) -> bool {
        let Some(total) = self.virtual_items.checked_add(count) else {
            return false;
        };
        if total > limit {
            return false;
        }
        self.virtual_items = total;
        true
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn push(&mut self, component: Component) -> SpecId {
        self.structure = mix(self.structure, component.shape());
        self.nodes.push(SpecNode {
            component: Some(component),
            ..Default::default()
        });
        self.parented.push(false);
        self.claimed.push(false);
        (self.nodes.len() - 1) as SpecId
    }

    /// Records one retained child entity, rejecting a second mount in this
    /// description before any part of the snapshot can be published.
    pub(crate) fn push_child_view(&mut self, child: ChildViewSpec) -> Result<SpecId, SpecError> {
        if !self.mounted_views.insert(child.handle()) {
            return Err(SpecError::DuplicateChildView);
        }
        Ok(self.push(Component::ChildView(child)))
    }

    /// Records one dock area, rejecting a second `dock_area(...)` naming the
    /// same one.
    ///
    /// The same rule and the same table as a child view's, because it is the
    /// same rule: GPUI cannot mount one entity at two positions in a tree, and
    /// a dock area is an entity. Sharing the table is safe because both handles
    /// come from one store and no id is reused.
    pub(crate) fn push_dock_area(
        &mut self,
        handle: crate::entities::EntityHandle,
    ) -> Result<SpecId, SpecError> {
        if !self.mounted_views.insert(handle) {
            return Err(SpecError::DuplicateChildView);
        }
        Ok(self.push(Component::DockArea(handle)))
    }

    pub fn node(&self, id: SpecId) -> Option<&SpecNode> {
        self.nodes.get(id as usize)
    }

    pub fn push_op(&mut self, id: SpecId, op: SpecOp) -> Result<(), SpecError> {
        self.check_live(id)?;
        // After the check, not before: a rejected call records nothing, so it
        // must not move the shape either.
        self.structure = mix(self.structure, op.shape());
        self.nodes[id as usize].ops.push(op);
        Ok(())
    }

    /// Attaches `child` to `parent`, consuming the child.
    /// Marks a node as consumed by an op rather than by a parent, so it cannot
    /// also be added to the tree.
    pub fn claim(&mut self, id: SpecId) -> Result<(), SpecError> {
        self.ensure_claimable(id)?;
        self.structure = mix(self.structure, mix(8, u64::from(id)));
        self.claimed[id as usize] = true;
        Ok(())
    }

    /// Whether [`Self::claim`] would succeed, without performing it.
    pub(crate) fn ensure_claimable(&self, id: SpecId) -> Result<(), SpecError> {
        self.check_live(id)?;
        if self.claimed[id as usize] {
            return Err(SpecError::Claimed);
        }
        Ok(())
    }

    pub fn attach(&mut self, parent: SpecId, child: SpecId) -> Result<(), SpecError> {
        self.check_live(parent)?;
        self.check_live(child)?;
        if self.claimed[child as usize] {
            return Err(SpecError::Claimed);
        }
        if parent == child {
            return Err(SpecError::SelfParent);
        }
        // The tree is the half of the shape the nodes themselves do not carry:
        // the same components in a different arrangement are a different
        // structure, and a template could not fill one from the other.
        self.structure = mix(
            mix(self.structure, u64::from(parent)),
            u64::from(child) ^ (1 << 32),
        );
        self.parented[child as usize] = true;
        self.nodes[parent as usize].children.push(child);
        Ok(())
    }

    /// Copies a template's nodes into this arena and answers where its root
    /// landed.
    ///
    /// This is an instantiation's whole structural half: no script runs, no
    /// value crosses the bridge, and no builder method is interpreted. A
    /// template arena's ids are dense and start at zero, so remapping is one
    /// addition — every id inside a copied node moves by the same base, and the
    /// nodes keep their order.
    ///
    /// The grafted nodes arrive carrying the `parented` and `claimed` flags
    /// they were recorded with, which is what makes a grafted subtree obey the
    /// same single-use rule as a described one: its interior is already spoken
    /// for, and only its root is free to be attached.
    pub(crate) fn graft(&mut self, template: &Template) -> SpecId {
        let base = self.nodes.len() as SpecId;
        let source = &template.arena;

        self.nodes.reserve(source.nodes.len());
        for node in &source.nodes {
            let mut node = node.clone();
            for child in &mut node.children {
                *child += base;
            }
            for op in &mut node.ops {
                match op {
                    SpecOp::StateStyle(_, id) | SpecOp::Slot(_, id) => *id += base,
                    SpecOp::NullaryStyle(_)
                    | SpecOp::ParamStyle(..)
                    | SpecOp::Method(..)
                    | SpecOp::RegisteredMethod(..)
                    | SpecOp::Callback(..)
                    | SpecOp::ActionCallback(..) => {}
                }
            }
            self.nodes.push(node);
        }
        self.parented.extend_from_slice(&source.parented);
        self.claimed.extend_from_slice(&source.claimed);

        // The root arrives with no parent so the caller can attach it, whatever
        // it was in the template.
        self.parented[(template.root + base) as usize] = false;

        // One mix for the whole graft rather than one per node: the template's
        // own fingerprint already summarizes everything inside it, and a
        // description that instantiates the same template twice must not look
        // like one that instantiated two different ones.
        self.structure = mix(self.structure, mix(9, source.structure));

        template.root + base
    }

    /// Writes one call's value into a grafted slot.
    ///
    /// `base` is what [`Self::graft`] returned less the template's own root, so
    /// that a slot recorded against the template's ids reaches the copy.
    pub(crate) fn write_slot(
        &mut self,
        base: SpecId,
        slot: &Slot,
        value: SlotValue,
    ) -> Result<(), SpecError> {
        let node = self
            .nodes
            .get_mut((slot.node() + base) as usize)
            .ok_or(SpecError::Expired)?;

        match (slot.site(), value) {
            (SlotSite::Text, SlotValue::Text(text)) => {
                node.component = Some(Component::Text(text));
            }
            (SlotSite::Argument { op, argument }, SlotValue::Value(bridged)) => {
                let target = node.ops.get_mut(op as usize).ok_or(SpecError::Expired)?;
                let arguments = match target {
                    SpecOp::ParamStyle(_, arguments) | SpecOp::Method(_, arguments) => arguments,
                    _ => return Err(SpecError::Expired),
                };
                *arguments
                    .get_mut(argument as usize)
                    .ok_or(SpecError::Expired)? = bridged;
            }
            (SlotSite::Handler { op }, SlotValue::Handler(callback)) => {
                match node.ops.get_mut(op as usize).ok_or(SpecError::Expired)? {
                    SpecOp::Callback(_, id) => *id = callback,
                    _ => return Err(SpecError::Expired),
                }
            }
            // Every pairing is decided when the slot is recorded, so a mismatch
            // is the runtime disagreeing with itself rather than a script
            // mistake. `Expired` is the arena's word for "this description is
            // not what you think it is".
            _ => return Err(SpecError::Expired),
        }

        Ok(())
    }

    /// Whether anything in this arena mounts a retained entity.
    ///
    /// A template is grafted many times and GPUI cannot mount one entity at two
    /// positions in a tree, so a body that describes one is refused at
    /// definition rather than at the second call.
    pub(crate) fn mounts_an_entity(&self) -> bool {
        !self.mounted_views.is_empty()
    }

    pub(crate) fn check_live(&self, id: SpecId) -> Result<(), SpecError> {
        let index = id as usize;
        if index >= self.nodes.len() || self.nodes[index].component.is_none() {
            return Err(SpecError::Expired);
        }
        if self.parented[index] {
            return Err(SpecError::AlreadyParented {
                component: self.nodes[index]
                    .component
                    .as_ref()
                    .map(Component::name)
                    .unwrap_or("element"),
            });
        }
        Ok(())
    }

    /// A stable textual dump, used by snapshot tests. Rendering never needs a
    /// GPU to be verified: the description is plain data.
    pub fn debug_tree(&self, root: SpecId) -> String {
        let mut out = String::new();
        self.write_tree(root, 0, &mut out);
        out
    }

    fn write_tree(&self, id: SpecId, depth: usize, out: &mut String) {
        let Some(node) = self.node(id) else {
            return;
        };
        let Some(component) = node.component.as_ref() else {
            return;
        };
        out.push_str(&"  ".repeat(depth));
        out.push_str(component.name());
        match component {
            Component::Module(spec) => out.push_str(&format!(
                " {}.{} {:?} props={:?}",
                spec.module, spec.component, spec.id, spec.props
            )),
            Component::TextView { id, text, format } => out.push_str(&format!(
                " {} {:?} {:?}",
                match format {
                    TextViewFormat::Html => "html",
                    TextViewFormat::Markdown => "markdown",
                },
                id,
                text,
            )),
            Component::Text(value)
            | Component::Button(value)
            | Component::Link(value)
            | Component::Checkbox(value)
            | Component::Switch(value)
            | Component::Svg(value)
            | Component::Image(value)
            | Component::Tabs(value)
            | Component::Tab(value)
            | Component::Progress(value)
            | Component::Radio(value)
            | Component::Toggle(value)
            | Component::RadioGroup(value)
            | Component::ToggleGroup(value)
            | Component::Popover(value)
            | Component::HoverCard(value)
            | Component::Popup(value)
            | Component::Select(value)
            | Component::Combobox(value) => out.push_str(&format!(" {value:?}")),
            // The focus handle is part of what a `DatePicker` *is* rather than
            // something called on it, so the dump carries it beside the id the
            // way a row carries its index.
            Component::DatePicker(value, handle) => out.push_str(&format!(" {value:?} #{handle}")),
            Component::Table(value)
            | Component::TableHeader(value)
            | Component::TableBody(value)
            | Component::TableCaption(value) => out.push_str(&format!(" {value:?}")),
            // The axis is already in the name, so only the id is left to write.
            Component::Resizable(value, _) => out.push_str(&format!(" {value:?}")),
            // The index is part of what the node *is* rather than something
            // called on it — a cell that lost it announces itself in the wrong
            // column — so the dump carries it beside the id, not among the ops.
            Component::TableRow(value, index)
            | Component::TableHead(value, index)
            | Component::TableCell(value, index) => out.push_str(&format!(" {value:?} #{index}")),
            Component::Scrollbar(value) => out.push_str(&format!(" {value:?}")),
            Component::Pagination(value) => out.push_str(&format!(" {value:?}")),
            Component::Accordion(value) | Component::AccordionTrigger(value) => {
                out.push_str(&format!(" {value:?}"))
            }
            // The path is what an avatar image *is*; without it the dump says
            // an image is there but not which one.
            Component::AvatarImage(path) => out.push_str(&format!(" {path:?}")),
            // The item count, not the item sizes: a dump of a hundred thousand
            // extents is not something a test reads, and the count is the part
            // that says what the list is.
            Component::VirtualList(spec) => {
                out.push_str(&format!(" {:?} \u{d7}{}", spec.id(), spec.sizes().len()))
            }
            Component::List(spec) => {
                out.push_str(&format!(" {:?} \u{d7}{}", spec.id(), spec.item_count()))
            }
            Component::ChildView(spec) => out.push_str(&format!(" #{}", spec.handle())),
            Component::Slider(handle)
            | Component::SliderTrack(handle)
            | Component::SliderIndicator(handle)
            | Component::SliderThumb(handle) => out.push_str(&format!(" #{handle}")),
            Component::Input(handle)
            | Component::Textarea(handle)
            | Component::NumberInput(handle)
            | Component::OtpInput(handle) => out.push_str(&format!(" #{handle}")),
            _ => {}
        }
        for op in node.ops() {
            match op {
                SpecOp::NullaryStyle(index) => {
                    out.push_str(&format!(" .{}", crate::style::nullary_name(*index)))
                }
                SpecOp::ParamStyle(name, args) => out.push_str(&format!(" .{name}{args:?}")),
                SpecOp::Method("transition", args) => {
                    if let [
                        Bridged::Str(property),
                        Bridged::Number(duration),
                        Bridged::Number(delay),
                        Bridged::Str(easing),
                    ] = args.as_slice()
                    {
                        out.push_str(&format!(
                            " :transition({property}, {duration}ms, {delay}ms, {easing})"
                        ));
                    } else {
                        out.push_str(" :transition(?)");
                    }
                }
                SpecOp::Method("spring", args) => {
                    if let [
                        Bridged::Str(property),
                        Bridged::Number(response),
                        Bridged::Number(damping),
                        Bridged::Number(epsilon),
                    ] = args.as_slice()
                    {
                        out.push_str(&format!(
                            " :spring({property}, {response}ms, {damping}, {epsilon})"
                        ));
                    } else {
                        out.push_str(" :spring(?)");
                    }
                }
                SpecOp::Method(name, args) => out.push_str(&format!(" :{name}{args:?}")),
                SpecOp::RegisteredMethod(method) => {
                    out.push_str(&format!(" :{}(registered)", method.name()))
                }
                SpecOp::Callback(name, _) => out.push_str(&format!(" :{name}(fn)")),
                SpecOp::ActionCallback(id, _) => out.push_str(&format!(" :on_action({id}, fn)")),
                SpecOp::StateStyle(name, node) => {
                    out.push_str(&format!(" :{name}("));
                    match self.node(*node) {
                        Some(state) => {
                            for op in state.ops() {
                                match op {
                                    SpecOp::NullaryStyle(index) => out.push_str(&format!(
                                        ".{}",
                                        crate::style::nullary_name(*index)
                                    )),
                                    SpecOp::ParamStyle(name, args) => {
                                        out.push_str(&format!(".{name}{args:?}"))
                                    }
                                    _ => {}
                                }
                            }
                        }
                        None => out.push_str("?"),
                    }
                    out.push(')');
                }
                // A slot holds a whole subtree, so it is written under the
                // node instead of on its line of calls.
                SpecOp::Slot(..) => {}
            }
        }
        out.push('\n');
        // A filled slot is detached from `children`, so walking children alone
        // would leave the content out of the dump entirely — and these tests
        // are the only place the description is ever read back.
        for op in node.ops() {
            if let SpecOp::Slot(name, slot) = op {
                out.push_str(&"  ".repeat(depth + 1));
                out.push_str(&format!("@{name}\n"));
                self.write_tree(*slot, depth + 2, out);
            }
        }
        for child in node.children() {
            self.write_tree(*child, depth + 1, out);
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SpecError {
    /// The node was consumed by a method that takes an element — a state
    /// style's declarations, or a named slot — so it cannot also enter the
    /// tree.
    Claimed,
    /// The node belongs to a previous render pass.
    Expired,
    /// The node was already added to a parent.
    AlreadyParented { component: &'static str },
    /// An element was added to itself.
    SelfParent,
    /// One retained entity was described at two positions in one snapshot.
    DuplicateChildView,
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecError::Expired => f.write_str(
                "this element belongs to a previous render pass; elements are single-use values \
                 and must be rebuilt each time render runs",
            ),
            SpecError::AlreadyParented { component } => write!(
                f,
                "element `{component}` was already added to a parent; elements are single-use values"
            ),
            SpecError::Claimed => f.write_str(
                "this element was given to a method that takes one — a state style's \
                 declarations, or a named slot such as content — and cannot also be added \
                 to the tree",
            ),
            SpecError::SelfParent => f.write_str("an element cannot be added to itself"),
            SpecError::DuplicateChildView => f.write_str(
                "a child view handle can be mounted only once in one snapshot; create a second \
                 Entity for a second position",
            ),
        }
    }
}

impl std::error::Error for SpecError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attaching_an_element_twice_is_an_error() {
        let mut arena = SpecArena::new();
        let parent = arena.push(Component::Div);
        let other_parent = arena.push(Component::Div);
        let child = arena.push(Component::Text("hi".into()));

        arena.attach(parent, child).unwrap();
        let error = arena.attach(other_parent, child).unwrap_err();

        assert!(matches!(error, SpecError::AlreadyParented { .. }));
    }

    #[test]
    fn a_parented_element_can_no_longer_take_ops() {
        let mut arena = SpecArena::new();
        let parent = arena.push(Component::Div);
        let child = arena.push(Component::Div);
        arena.attach(parent, child).unwrap();

        assert_eq!(
            arena.push_op(child, SpecOp::NullaryStyle(0)).unwrap_err(),
            SpecError::AlreadyParented { component: "div" }
        );
    }

    #[test]
    fn a_claimed_node_still_takes_styles_but_cannot_be_attached() {
        let mut arena = SpecArena::new();
        let parent = arena.push(Component::Div);
        let state = arena.push(Component::Div);
        arena.claim(state).unwrap();

        assert!(arena.push_op(state, SpecOp::NullaryStyle(0)).is_ok());
        assert_eq!(arena.attach(parent, state).unwrap_err(), SpecError::Claimed);
    }

    #[test]
    fn a_slot_node_can_only_be_claimed_once() {
        let mut arena = SpecArena::new();
        let content = arena.push(Component::Text("body".into()));

        arena.claim(content).unwrap();

        assert_eq!(arena.claim(content).unwrap_err(), SpecError::Claimed);
    }

    #[test]
    fn reset_expires_every_node() {
        let mut arena = SpecArena::new();
        let node = arena.push(Component::Div);
        arena.reset();

        assert_eq!(
            arena.push_op(node, SpecOp::NullaryStyle(0)).unwrap_err(),
            SpecError::Expired
        );
    }

    #[test]
    fn debug_tree_renders_structure_without_a_gpu() {
        let mut arena = SpecArena::new();
        let root = arena.push(Component::VFlex);
        let label = arena.push(Component::Text("Save".into()));
        arena.attach(root, label).unwrap();

        assert_eq!(arena.debug_tree(root), "v_flex\n  text \"Save\"\n");
    }

    #[test]
    fn a_filled_slot_is_dumped_under_the_node_holding_it() {
        let mut arena = SpecArena::new();
        let root = arena.push(Component::Collapsible);
        let content = arena.push(Component::Text("body".into()));
        arena.claim(content).unwrap();
        arena
            .push_op(root, SpecOp::Slot("content", content))
            .unwrap();

        assert_eq!(
            arena.debug_tree(root),
            "Collapsible\n  @content\n    text \"body\"\n"
        );
    }
}

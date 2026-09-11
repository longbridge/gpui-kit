//! Script access to a dockable layout.
//!
//! [`crate::dock`] is the engine-independent half: a [`Panel`] whose body is a
//! script view, a skin that forwards every appearance hook to a
//! [`DockChrome`], and the JSON form of what each hook is handed. This module
//! is the JavaScript face of it — the half the design doc left open — and it is
//! three things.
//!
//! **A retained area.** `DockArea.new(id)` creates a
//! [`gpui_base::dock::DockArea`] in the entity store and hands back a handle,
//! for the reason an input's text is retained rather than described: the layout
//! is what the *user* changed. A drag, a resize, a closed tab and a collapsed
//! dock all happen without a script render, and an area rebuilt from a
//! description would put every one of them back the way the last render
//! described it.
//!
//! **The chrome, requested from layout.** The six handlers a script hangs on
//! `dock_area(...)` are first requested from inside GPUI's layout pass, exactly
//! as a virtual list's item renderer is, and with the same three protections —
//! a [`ScopePhase::Layout`] scope, an arena of its own, and no job drain on the
//! way out. Their descriptions are then cached by callback and resolved native
//! state, so an unchanged frame replays Rust data rather than entering the VM.
//! See [`ShellRuntime::describe_dock_chrome`].
//!
//! **Commands, not callbacks.** A tab's click does not carry a script function.
//! A chrome handler may rerun whenever its native state changes, so a handler
//! registered inside one would outlive the temporary description unpredictably.
//! Instead a chrome element carries a
//! [`DockCommand`](crate::dock::DockCommand) — `select_tab(group, i)`,
//! `close_panel(group, id)`, `toggle_dock(dock)` — which names a container and
//! what to ask it, and is resolved against the contexts the last drawn frame
//! recorded. Nothing about it is a script value, so there is nothing to retire.
//!
//! # Panels come back after a restart
//!
//! `DockArea.register_panel(name, Class)` teaches
//! [`PanelRegistry`](gpui_base::dock::PanelRegistry) to rebuild the panel from
//! a persisted layout: it constructs `Class`, then hands the payload the last
//! save wrote to the instance's own `deserialize(data)`. Its `serialize()` is
//! read on the way out. Both are ordinary methods on the view class, because a
//! panel *is* a view — there is no second object to introduce.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, AppContext as _, Empty, Entity, EntityId, IntoElement as _,
    ParentElement as _, Styled as _, Window, div, px,
};
use gpui_base::dock::{
    DockContext, DockEvent, DockPlacement, DropIndicator, PanelId, TabGroupContext,
};
use rquickjs::{Ctx, Exception, Object, Persistent, Result as JsResult, Value, function::Func};
use serde_json::{Value as Json, json};

use crate::{
    dock::{
        DockChrome, PanelScript, ScriptDockSkin, ScriptPanel, dock_data, drop_indicator_data,
        tab_group_data,
    },
    entities::EntityHandle,
    materialize::dock_placement,
    scope::{self, ScopePhase},
    spec::{CallbackId, Component, SpecArena, SpecId},
    view::ScriptView,
};

use super::{ShellRuntime, ViewType};

/// Installs the host half of the dock API. The prelude wraps these into
/// `DockArea`, `dock_area(...)` and `dock_content()`.
pub fn install(
    ctx: &Ctx<'_>,
    module: &Object<'_>,
    runtime: std::rc::Weak<ShellRuntime>,
) -> JsResult<()> {
    let _ = module;
    let globals = ctx.globals();

    let create = runtime.clone();
    globals.set(
        "__dock_area_new",
        Func::from(
            move |ctx: Ctx<'_>, id: String, version: Option<f64>| -> JsResult<EntityHandle> {
                refuse_in_render(&ctx, "DockArea.new(id)")?;
                let runtime = alive(&ctx, &create)?;
                let chrome = Rc::new(ScriptChrome::new(std::rc::Rc::downgrade(&runtime)));
                let skin = ScriptDockSkin::new(chrome.clone()).with_slots(chrome.slots());
                let version = version
                    .map(|value| non_negative_integer(&ctx, value, "DockArea.new version"))
                    .transpose()?
                    .map(|value| {
                        usize::try_from(value).map_err(|_| {
                            Exception::throw_range(
                                &ctx,
                                "DockArea.new version exceeds this platform's index range",
                            )
                        })
                    })
                    .transpose()?;
                scope::with_current(|window, cx| {
                    runtime
                        .entities()
                        .create_dock(
                            &id,
                            version,
                            skin,
                            scope::current_application_generation(),
                            window,
                            cx,
                        )
                        .map_err(|_| super::entity_api::entity_limit_reached(&ctx))
                })
                .ok_or_else(|| {
                    Exception::throw_type(
                        &ctx,
                        "DockArea.new(id) needs a live host call; call it from init() or an \
                         event handler",
                    )
                })?
            },
        ),
    )?;

    // Every edit to the layout is queued and applied once this call has
    // returned to Rust. Two of the three have to be: `add_panel` is handed the
    // token `cx.new(Class)` returned, and the view behind it is itself still
    // queued; `load` rebuilds every panel through the registry, which
    // constructs views. Removal is queued with them so that the calls take
    // effect in the order they were made.
    let add = runtime.clone();
    globals.set(
        "__dock_add_panel",
        Func::from(
            move |ctx: Ctx<'_>,
                  dock: EntityHandle,
                  view: u32,
                  options: PanelOptions|
                  -> JsResult<()> {
                refuse_in_render(&ctx, "add_panel(view, options)")?;
                let runtime = alive(&ctx, &add)?;
                // Validated here rather than at the boundary, so a bad
                // placement is reported at the line that wrote it.
                placement_of(&ctx, &options.placement)?;
                let live = { runtime.entities().dock(dock).is_some() };
                if !live {
                    return Err(released(&ctx));
                }
                let name = qualified_name(&options.name);
                runtime.queue_dock_edit(
                    &ctx,
                    dock,
                    DockEdit::AddPanel {
                        view,
                        name,
                        options,
                    },
                    "add_panel(view, options)",
                )
            },
        ),
    )?;

    let remove = runtime.clone();
    globals.set(
        "__dock_remove_panel",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, panel: f64| -> JsResult<()> {
                refuse_in_render(&ctx, "remove_panel(id)")?;
                let panel = non_negative_integer(&ctx, panel, "remove_panel(id)")?;
                let runtime = alive(&ctx, &remove)?;
                let live = { runtime.entities().dock(dock).is_some() };
                if !live {
                    return Err(released(&ctx));
                }
                runtime.queue_dock_edit(
                    &ctx,
                    dock,
                    DockEdit::RemovePanel(panel),
                    "remove_panel(id)",
                )
            },
        ),
    )?;

    // Reads. Each answers a JSON document the prelude hands over unchanged,
    // because the shapes here are documents rather than scalars: a layout, a
    // panel list, a dock's four properties.
    let panels = runtime.clone();
    globals.set(
        "__dock_panels",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle| -> JsResult<String> {
                let area = area_of(&ctx, &panels, dock)?;
                let listed = scope::with_current_app(|cx| {
                    let area = area.read(cx);
                    let mut listed = Vec::new();
                    for placement in PLACEMENTS {
                        let Some(tree) = area.layout(placement) else {
                            continue;
                        };
                        // Walked node by node rather than through `tree.panels()`,
                        // because where a panel sits is half of what a script asks
                        // this for: which container holds it, where in that
                        // container it is, and whether it is the one on screen.
                        tree.root().walk(&mut |node| {
                            let (panels, active_ix) = match node.kind() {
                                gpui_base::dock::PaneRef::Tabs { panels, active_ix } => {
                                    (panels.to_vec(), Some(active_ix))
                                }
                                gpui_base::dock::PaneRef::Split { .. } => return,
                            };
                            for (index, id) in panels.into_iter().enumerate() {
                                let Some(panel) = area.panel(id) else {
                                    continue;
                                };
                                listed.push(json!({
                                    "id": id.as_u64(),
                                    "name": panel.panel_name(cx),
                                    "placement": placement_word(placement),
                                    "node": node.id().as_u64(),
                                    "index": index,
                                    "active": active_ix == Some(index),
                                    "visible": panel.visible(cx),
                                    "closable": panel.closable(cx),
                                    "zoomable": panel.zoomable(cx),
                                }));
                            }
                        });
                    }
                    Json::Array(listed)
                })
                .ok_or_else(|| needs_call(&ctx, "panels()"))?;
                Ok(listed.to_string())
            },
        ),
    )?;

    let dump = runtime.clone();
    globals.set(
        "__dock_dump",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle| -> JsResult<String> {
                let area = area_of(&ctx, &dump, dock)?;
                let state = scope::with_current_app(|cx| area.read(cx).dump(cx))
                    .ok_or_else(|| needs_call(&ctx, "dump()"))?;
                serde_json::to_string(&state).map_err(|error| {
                    Exception::throw_type(&ctx, &format!("this layout has no JSON form: {error}"))
                })
            },
        ),
    )?;

    // The one deferred operation in this module. `load` rebuilds every panel
    // through the registry, and rebuilding a script panel constructs a view —
    // which cannot happen while QuickJS holds its runtime lock, because that is
    // where this call is running from. So the request is queued and applied at
    // the same unlocked boundary a `cx.new(Class)` is.
    let load = runtime.clone();
    globals.set(
        "__dock_load",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, state: JsonArgument| -> JsResult<()> {
                refuse_in_render(&ctx, "load(state)")?;
                let runtime = alive(&ctx, &load)?;
                let live = { runtime.entities().dock(dock).is_some() };
                if !live {
                    return Err(released(&ctx));
                }
                let state = serde_json::from_value(state.0).map_err(|error| {
                    Exception::throw_type(
                        &ctx,
                        &format!("this is not a layout written by dump(): {error}"),
                    )
                })?;
                runtime.queue_dock_edit(&ctx, dock, DockEdit::Load(Box::new(state)), "load(state)")
            },
        ),
    )?;

    // The dock-by-dock properties. One entry point per verb rather than one
    // taking a name, because that is how the prelude spells them and a combined
    // call would make every reader pay for the others.
    let is_open = runtime.clone();
    globals.set(
        "__dock_is_open",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, placement: String| -> JsResult<bool> {
                let area = area_of(&ctx, &is_open, dock)?;
                let placement = placement_of(&ctx, &placement)?;
                scope::with_current_app(|cx| area.read(cx).is_dock_open(placement))
                    .ok_or_else(|| needs_call(&ctx, "is_dock_open(placement)"))
            },
        ),
    )?;

    let has_dock = runtime.clone();
    globals.set(
        "__dock_has",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, placement: String| -> JsResult<bool> {
                let area = area_of(&ctx, &has_dock, dock)?;
                let placement = placement_of(&ctx, &placement)?;
                scope::with_current_app(|cx| area.read(cx).has_dock(placement))
                    .ok_or_else(|| needs_call(&ctx, "has_dock(placement)"))
            },
        ),
    )?;

    let toggle = runtime.clone();
    globals.set(
        "__dock_toggle",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, placement: String| -> JsResult<()> {
                refuse_in_render(&ctx, "toggle_dock(placement)")?;
                let area = area_of(&ctx, &toggle, dock)?;
                let placement = placement_of(&ctx, &placement)?;
                scope::with_current(|window, cx| {
                    area.update(cx, |area, cx| area.toggle_dock(placement, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "toggle_dock(placement)"))
            },
        ),
    )?;

    let remove_dock = runtime.clone();
    globals.set(
        "__dock_remove",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, placement: String| -> JsResult<()> {
                refuse_in_render(&ctx, "remove_dock(placement)")?;
                let area = area_of(&ctx, &remove_dock, dock)?;
                let placement = placement_of(&ctx, &placement)?;
                scope::with_current(|window, cx| {
                    area.update(cx, |area, cx| area.remove_dock(placement, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "remove_dock(placement)"))
            },
        ),
    )?;

    let dock_size = runtime.clone();
    globals.set(
        "__dock_size",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, placement: String| -> JsResult<Option<f32>> {
                let area = area_of(&ctx, &dock_size, dock)?;
                let placement = placement_of(&ctx, &placement)?;
                scope::with_current_app(|cx| area.read(cx).dock_size(placement).map(f32::from))
                    .ok_or_else(|| needs_call(&ctx, "dock_size(placement)"))
            },
        ),
    )?;

    let set_dock_size = runtime.clone();
    globals.set(
        "__dock_set_size",
        Func::from(
            move |ctx: Ctx<'_>,
                  dock: EntityHandle,
                  placement: String,
                  value: f32|
                  -> JsResult<()> {
                refuse_in_render(&ctx, "set_dock_size(placement, size)")?;
                finite_non_negative(&ctx, value, "set_dock_size(placement, size)")?;
                let area = area_of(&ctx, &set_dock_size, dock)?;
                let placement = placement_of(&ctx, &placement)?;
                scope::with_current(|window, cx| {
                    area.update(cx, |area, cx| {
                        area.set_dock_size(placement, px(value), window, cx)
                    });
                })
                .ok_or_else(|| needs_call(&ctx, "set_dock_size(placement, size)"))
            },
        ),
    )?;

    let collapsible = runtime.clone();
    globals.set(
        "__dock_set_collapsible",
        Func::from(
            move |ctx: Ctx<'_>,
                  dock: EntityHandle,
                  placement: String,
                  value: bool|
                  -> JsResult<()> {
                refuse_in_render(&ctx, "set_dock_collapsible(placement, collapsible)")?;
                let area = area_of(&ctx, &collapsible, dock)?;
                let placement = placement_of(&ctx, &placement)?;
                scope::with_current(|window, cx| {
                    area.update(cx, |area, cx| {
                        area.set_dock_collapsible(placement, value, window, cx)
                    });
                })
                .ok_or_else(|| needs_call(&ctx, "set_dock_collapsible(placement, collapsible)"))
            },
        ),
    )?;

    let locked = runtime.clone();
    globals.set(
        "__dock_is_locked",
        Func::from(move |ctx: Ctx<'_>, dock: EntityHandle| -> JsResult<bool> {
            let area = area_of(&ctx, &locked, dock)?;
            scope::with_current_app(|cx| area.read(cx).is_locked())
                .ok_or_else(|| needs_call(&ctx, "is_locked()"))
        }),
    )?;

    let set_locked = runtime.clone();
    globals.set(
        "__dock_set_locked",
        Func::from(
            move |ctx: Ctx<'_>, dock: EntityHandle, value: bool| -> JsResult<()> {
                refuse_in_render(&ctx, "set_locked(locked)")?;
                let area = area_of(&ctx, &set_locked, dock)?;
                scope::with_current(|window, cx| {
                    area.update(cx, |area, cx| area.set_locked(value, window, cx));
                })
                .ok_or_else(|| needs_call(&ctx, "set_locked(locked)"))
            },
        ),
    )?;

    let zoomed = runtime.clone();
    globals.set(
        "__dock_is_zoomed",
        Func::from(move |ctx: Ctx<'_>, dock: EntityHandle| -> JsResult<bool> {
            let area = area_of(&ctx, &zoomed, dock)?;
            scope::with_current_app(|cx| area.read(cx).is_zoomed())
                .ok_or_else(|| needs_call(&ctx, "is_zoomed()"))
        }),
    )?;

    let zoom_out = runtime.clone();
    globals.set(
        "__dock_zoom_out",
        Func::from(move |ctx: Ctx<'_>, dock: EntityHandle| -> JsResult<()> {
            refuse_in_render(&ctx, "zoom_out()")?;
            let area = area_of(&ctx, &zoom_out, dock)?;
            scope::with_current(|window, cx| {
                area.update(cx, |area, cx| area.set_zoomed_out(window, cx));
            })
            .ok_or_else(|| needs_call(&ctx, "zoom_out()"))
        }),
    )?;

    let subscribe = runtime.clone();
    globals.set(
        "__dock_on",
        Func::from(
            move |ctx: Ctx<'_>,
                  dock: EntityHandle,
                  name: String,
                  handler: super::entity_api::Handler|
                  -> JsResult<bool> {
                subscribe_dock(&ctx, &subscribe, dock, &name, handler)
            },
        ),
    )?;

    let register = runtime.clone();
    globals.set(
        "__dock_register_panel",
        Func::from(
            move |ctx: Ctx<'_>, name: String, class: PanelClass| -> JsResult<String> {
                let runtime = alive(&ctx, &register)?;
                let view_type = ViewType::from_panel_class(class.0);
                runtime.register_panel_class(&ctx, &name, view_type)
            },
        ),
    )?;

    install_elements(&globals, runtime.clone())?;

    let discard = runtime.clone();
    globals.set(
        "__dock_release",
        Func::from(move |handle: EntityHandle| {
            discard
                .upgrade()
                .is_some_and(|runtime| runtime.entities().release(handle))
        }),
    )?;

    Ok(())
}

/// Every region a panel can live in, in the order `panels()` reports them.
const PLACEMENTS: [DockPlacement; 4] = [
    DockPlacement::Center,
    DockPlacement::Left,
    DockPlacement::Right,
    DockPlacement::Bottom,
];

fn placement_word(placement: DockPlacement) -> &'static str {
    match placement {
        DockPlacement::Center => "center",
        DockPlacement::Left => "left",
        DockPlacement::Right => "right",
        DockPlacement::Bottom => "bottom",
    }
}

/// What `add_panel(view, options)` was told about the panel.
///
/// Plain data, because it waits in the pending queue until a boundary where the
/// view it names can be resolved.
pub(in crate::engine) struct PanelOptions {
    name: String,
    placement: String,
    size: Option<f32>,
    closable: bool,
    zoomable: bool,
    visible: bool,
}

impl<'js> rquickjs::FromJs<'js> for PanelOptions {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let object = value.as_object().ok_or_else(|| {
            Exception::throw_type(ctx, "add_panel(view, options) expects an options object")
        })?;
        let name = object.get::<_, String>("name")?;
        if name.is_empty() {
            return Err(Exception::throw_type(
                ctx,
                "add_panel(view, options) expects a non-empty name",
            ));
        }
        let size = object.get::<_, Option<f32>>("size")?;
        if let Some(size) = size {
            finite_non_negative(ctx, size, "add_panel size")?;
        }
        Ok(Self {
            name,
            placement: object
                .get::<_, Option<String>>("placement")?
                .unwrap_or_else(|| "center".to_owned()),
            size,
            closable: object.get::<_, Option<bool>>("closable")?.unwrap_or(true),
            zoomable: object.get::<_, Option<bool>>("zoomable")?.unwrap_or(true),
            visible: object.get::<_, Option<bool>>("visible")?.unwrap_or(true),
        })
    }
}

/// A view class captured by a host function without leaking the active QuickJS
/// lifetime into the Rust callback type.
///
/// The same shape `NestedViewClass` has, and for the same reason: a closure
/// cannot both take `Ctx<'js>` and touch a `Value<'js>`, because the two elided
/// lifetimes will not unify. Saving inside `FromJs` is where they are still the
/// same one.
struct PanelClass(Persistent<Object<'static>>);

impl<'js> rquickjs::FromJs<'js> for PanelClass {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let class = value.as_object().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "register_panel(name, Class) expects the View subclass the panel is rebuilt from",
            )
        })?;
        Ok(Self(Persistent::save(ctx, class.clone())))
    }
}

/// A plain-data argument converted where the lifetimes still agree.
struct JsonArgument(Json);

impl<'js> rquickjs::FromJs<'js> for JsonArgument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        Ok(Self(super::host::to_json(ctx, &value, 0)?))
    }
}

/// The chrome a script draws, as a [`DockChrome`].
///
/// It holds no handlers of its own: which script function draws each piece is
/// written by [`crate::materialize`] into the slots it shares with the skin,
/// whenever a snapshot is replayed. The skin outlives every snapshot, while
/// callback ids and cached descriptions remain tied to the snapshot that
/// registered them.
pub(super) struct ScriptChrome {
    runtime: std::rc::Weak<ShellRuntime>,
    slots: Rc<crate::dock::DockChromeSlots>,
    cache: RefCell<HashMap<ChromeSlotKey, ChromeSpec>>,
}

const MAX_CHROME_CACHE_ENTRIES: usize = 4_096;

/// Which piece of chrome a cached description belongs to, named so that the
/// same container asking again lands on the same entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ChromeSlotKey {
    TabBar(u64),
    EmptyGroup(u64),
    /// The one hook with no container in its key, because an area never draws
    /// two at once: a group raises its indicator only while its own bounds hold
    /// the pointer, and clears it on the same drag move when they do not, so
    /// the group under the pointer is the only one with an indicator to draw.
    /// [`DropIndicator`] carries no node id to key on either.
    DropIndicator,
    Dock(DockPlacement),
}

struct ChromeSpec {
    callback: CallbackId,
    payload: Json,
    arena: SpecArena,
    root: Option<SpecId>,
}

impl ScriptChrome {
    fn new(runtime: std::rc::Weak<ShellRuntime>) -> Self {
        Self {
            runtime,
            slots: Rc::new(crate::dock::DockChromeSlots::default()),
            cache: RefCell::new(HashMap::new()),
        }
    }

    fn slots(&self) -> Rc<crate::dock::DockChromeSlots> {
        self.slots.clone()
    }

    /// Answers one hook: the description this callback and this state produced,
    /// materialized into an element for the frame being drawn.
    ///
    /// The handler runs only when there is no description for that pair yet. A
    /// description that threw is not one — it is retried on the next frame,
    /// after the entry that stood before it is left alone, because the state it
    /// answers is the state that just failed. A description that answered
    /// `null` is one: the hook is optional, and nothing is a valid answer.
    fn draw(
        &self,
        key: ChromeSlotKey,
        handler: Option<CallbackId>,
        payload: Json,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let handler = handler?;
        let runtime = self.runtime.upgrade()?;
        let stale = self
            .cache
            .borrow()
            .get(&key)
            .is_none_or(|cached| cached.callback != handler || cached.payload != payload);
        if stale {
            let (arena, root) = runtime.describe_dock_chrome(handler, &payload, window, cx)?;
            let mut cache = self.cache.borrow_mut();
            // Past the bound the whole cache goes rather than one entry: which
            // container is worth keeping is not a question this has an answer
            // to, and an area with thousands of live containers is describing
            // each of them again anyway.
            if cache.len() >= MAX_CHROME_CACHE_ENTRIES && !cache.contains_key(&key) {
                cache.clear();
            }
            cache.insert(
                key,
                ChromeSpec {
                    callback: handler,
                    payload,
                    arena,
                    root,
                },
            );
        }

        // Taken out for the length of the materialization rather than borrowed
        // across it: materializing reaches back into the runtime with the same
        // `cx`, and a borrow held over that is a panic waiting for the first
        // description that draws a dock inside a dock.
        let spec = self.cache.borrow_mut().remove(&key)?;
        let element = spec.root.map(|root| {
            crate::materialize::materialize_subtree(&runtime, &spec.arena, root, window, cx)
        });
        self.cache.borrow_mut().insert(key, spec);
        element
    }
}

impl DockChrome for ScriptChrome {
    fn tab_bar(&self, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> AnyElement {
        let hooks = self.slots.get();
        let payload = tab_group_data(group, cx);
        self.draw(
            ChromeSlotKey::TabBar(group.node().as_u64()),
            hooks.tab_bar,
            payload,
            window,
            cx,
        )
        .unwrap_or_else(|| Empty.into_any_element())
    }

    fn empty_group(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let hooks = self.slots.get();
        let payload = tab_group_data(group, cx);
        self.draw(
            ChromeSlotKey::EmptyGroup(group.node().as_u64()),
            hooks.empty_group,
            payload,
            window,
            cx,
        )
    }

    fn drop_indicator(
        &self,
        indicator: DropIndicator,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let hooks = self.slots.get();
        let payload = drop_indicator_data(&indicator);
        self.draw(
            ChromeSlotKey::DropIndicator,
            hooks.drop_indicator,
            payload,
            window,
            cx,
        )
    }

    /// The one hook that is handed an element rather than only state.
    ///
    /// An `AnyElement` cannot cross into script, so the content is installed in
    /// a slot for the length of the call and the script's `dock_content()`
    /// takes it. A chrome that never placed it is not a chrome that wanted the
    /// content gone — it is one that forgot — so whatever is left over is drawn
    /// after what the script returned.
    fn dock(
        &self,
        dock: &DockContext,
        content: AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let hooks = self.slots.get();
        let Some(handler) = hooks.dock else {
            return content;
        };
        let payload = dock_data(dock);
        let slot = crate::dock::ContentSlot::install(content);
        let drawn = self.draw(
            ChromeSlotKey::Dock(dock.placement()),
            Some(handler),
            payload,
            window,
            cx,
        );
        let unplaced = slot.unplaced();
        drop(slot);

        match (drawn, unplaced) {
            (Some(drawn), None) => drawn,
            (Some(drawn), Some(content)) => {
                tracing::warn!(
                    "a dock's chrome handler drew no dock_content(), so the dock's own panels \
                     are drawn after it; place dock_content() where they belong"
                );
                div()
                    .size_full()
                    .child(drawn)
                    .child(content)
                    .into_any_element()
            }
            (None, Some(content)) => content,
            (None, None) => Empty.into_any_element(),
        }
    }
}

/// The script side of one registered panel: a view class, and the two methods
/// on it that carry state across a restart.
pub(super) struct ScriptPanelClass {
    runtime: std::rc::Weak<ShellRuntime>,
    /// Behind an `Option` so it can be dropped while the registration that
    /// holds this is still alive.
    ///
    /// [`register_panel`](gpui_base::dock::register_panel) files the builder in
    /// an `App` global, which outlives the runtime — and the class is a
    /// `Persistent` QuickJS value, which must be released while its context
    /// still exists or QuickJS aborts the process at shutdown. Clearing this
    /// from the runtime's own `Drop` is what makes the two lifetimes agree.
    view_type: RefCell<Option<ViewType>>,
    policy: Rc<crate::policy::Policy>,
    /// The store handles this builder created, keyed by the view they name.
    ///
    /// A panel built from a persisted layout has no script holding it, so its
    /// handle would live until the application unloaded. Recorded here, it is
    /// released when base tells the panel it left the layout.
    built: RefCell<HashMap<EntityId, EntityHandle>>,
}

impl ScriptPanelClass {
    pub(super) fn new(
        runtime: std::rc::Weak<ShellRuntime>,
        view_type: ViewType,
        policy: Rc<crate::policy::Policy>,
    ) -> Self {
        Self {
            runtime,
            view_type: RefCell::new(Some(view_type)),
            policy,
            built: RefCell::new(HashMap::new()),
        }
    }

    /// Releases the class this rebuilds panels from.
    ///
    /// After this the registration is still in place and still answers, with a
    /// draw-nothing placeholder that carries the panel's persisted state
    /// forward — which is exactly what an *unloaded* application's panels get,
    /// and the right answer for a runtime that has gone away.
    pub(super) fn retire(&self) {
        self.view_type.borrow_mut().take();
        self.built.borrow_mut().clear();
    }
}

impl PanelScript for ScriptPanelClass {
    fn build(&self, window: &mut Window, cx: &mut App) -> Option<Entity<ScriptView>> {
        let runtime = self.runtime.upgrade()?;
        let view_type = self.view_type.borrow().clone()?;
        let handle = runtime
            .instantiate_nested_view(&view_type, self.policy.clone(), None, window, cx)
            .map_err(|error| tracing::error!("a dock panel's script could not be built: {error}"))
            .ok()?;
        let view = runtime.entities().view(handle)?;
        self.built.borrow_mut().insert(view.entity_id(), handle);
        Some(view)
    }

    /// The instance's own `serialize()`, if it has one.
    ///
    /// No call scope is opened, and none can be: `Panel::dump` is a read, so
    /// there is no `&mut Window` here. A `serialize()` that calls back into the
    /// host therefore fails the way any host call outside a scope does, which
    /// is the documented contract.
    fn serialize(&self, view: &Entity<ScriptView>, cx: &App) -> Option<Json> {
        let runtime = self.runtime.upgrade()?;
        let object = view.read(cx).object().clone();
        runtime.call_panel_serialize(&object)
    }

    fn deserialize(
        &self,
        view: &Entity<ScriptView>,
        data: &Json,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(runtime) = self.runtime.upgrade() else {
            return;
        };
        runtime.call_panel_deserialize(view, data, window, cx);
    }

    fn release(&self, view: &Entity<ScriptView>, _: &mut Window, cx: &mut App) {
        let handle = self.built.borrow_mut().remove(&view.entity_id());
        if let (Some(handle), Some(runtime)) = (handle, self.runtime.upgrade()) {
            runtime.release_view_handle(handle, cx);
        }
    }
}

/// The element descriptions: the area itself, and where a dock's content goes.
fn install_elements(globals: &Object<'_>, runtime: std::rc::Weak<ShellRuntime>) -> JsResult<()> {
    let area = runtime.clone();
    globals.set(
        "__dock_area_element",
        Func::from(move |ctx: Ctx<'_>, handle: EntityHandle| -> JsResult<u32> {
            let runtime = alive(&ctx, &area)?;
            let live = { runtime.entities().dock(handle).is_some() };
            if !live {
                return Err(Exception::throw_type(
                    &ctx,
                    "this dock area has been released and can no longer be drawn",
                ));
            }
            runtime
                .arena_mut()
                .push_dock_area(handle)
                .map_err(|error| Exception::throw_type(&ctx, &error.to_string()))
        }),
    )?;

    globals.set(
        "__dock_content",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<u32> {
            let runtime = alive(&ctx, &runtime)?;
            Ok(runtime.push_component(Component::DockContent))
        }),
    )?;

    Ok(())
}

fn subscribe_dock(
    ctx: &Ctx<'_>,
    runtime: &std::rc::Weak<ShellRuntime>,
    dock: EntityHandle,
    name: &str,
    handler: super::entity_api::Handler,
) -> JsResult<bool> {
    if name != "layout_changed" {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "unknown dock event `{name}`; the only one is \"layout_changed\", which fires on \
                 every edit, so save on a timer rather than on every one"
            ),
        ));
    }

    let runtime = alive(ctx, runtime)?;
    let owner = runtime.callback_owner();
    let saved = handler.0;
    let dispatcher = std::rc::Rc::downgrade(&runtime);
    let subscribed = scope::with_current(|window, cx| {
        runtime
            .entities()
            .subscribe_dock(dock, window, cx, move |event, window, cx| {
                if !matches!(event, DockEvent::LayoutChanged) {
                    return;
                }
                if let Some(runtime) = dispatcher.upgrade() {
                    runtime.dispatch_dock_event(&saved, &owner, window, cx);
                }
            })
    })
    .ok_or_else(|| needs_call(ctx, "on(\"layout_changed\", handler)"))?;

    if !subscribed {
        return Err(released(ctx));
    }
    Ok(true)
}

/// `shell:<application>/<panel>`, which is what the layout file holds.
fn qualified_name(panel: &str) -> String {
    crate::dock::panel_name(scope::policy().application(), panel).to_owned()
}

fn placement_of(ctx: &Ctx<'_>, name: &str) -> JsResult<DockPlacement> {
    dock_placement(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "`{name}` is not a dock placement; expected \"center\", \"left\", \"right\" or \
                 \"bottom\""
            ),
        )
    })
}

fn area_of(
    ctx: &Ctx<'_>,
    runtime: &std::rc::Weak<ShellRuntime>,
    dock: EntityHandle,
) -> JsResult<Entity<gpui_base::dock::DockArea>> {
    let runtime = alive(ctx, runtime)?;
    let area = { runtime.entities().dock(dock) };
    area.ok_or_else(|| released(ctx))
}

fn alive(ctx: &Ctx<'_>, runtime: &std::rc::Weak<ShellRuntime>) -> JsResult<Rc<ShellRuntime>> {
    runtime
        .upgrade()
        .ok_or_else(|| Exception::throw_type(ctx, "the shell runtime is no longer available"))
}

fn released(ctx: &Ctx<'_>) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        "this dock area has been released; a handle cannot be used after release()",
    )
}

fn needs_call(ctx: &Ctx<'_>, api: &str) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        &format!("{api} needs a live host call; call it from init() or an event handler"),
    )
}

fn finite_number(ctx: &Ctx<'_>, value: f32, api: &str) -> JsResult<()> {
    if !value.is_finite() {
        return Err(Exception::throw_type(
            ctx,
            &format!("{api} expects a finite number"),
        ));
    }
    Ok(())
}

fn finite_non_negative(ctx: &Ctx<'_>, value: f32, api: &str) -> JsResult<()> {
    finite_number(ctx, value, api)?;
    if value < 0.0 {
        return Err(Exception::throw_range(
            ctx,
            &format!("{api} expects a non-negative number"),
        ));
    }
    Ok(())
}

fn non_negative_integer(ctx: &Ctx<'_>, value: f64, api: &str) -> JsResult<u64> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > 9_007_199_254_740_991.0
    {
        return Err(Exception::throw_type(
            ctx,
            &format!("{api} expects a whole, non-negative safe integer"),
        ));
    }
    Ok(value as u64)
}

/// Mutations are refused during a render pass for the reason every other one
/// is: a frame that changed the layout it was describing would describe one
/// layout and draw another. It holds for the queued edits too — the queue only
/// moves *when* the change lands, not whose decision it was.
///
/// Layout is refused during a chrome callback for the same reason twice over:
/// that runs inside GPUI's layout pass, on the frame being laid out.
fn refuse_in_render(ctx: &Ctx<'_>, api: &str) -> JsResult<()> {
    if matches!(
        scope::current_phase(),
        Some(ScopePhase::Render | ScopePhase::Layout)
    ) {
        return Err(Exception::throw_type(
            ctx,
            &format!(
                "{api} changes the layout and cannot be called while one is being described or \
                 laid out; call it from init(), an event handler or a task"
            ),
        ));
    }
    Ok(())
}

/// A JSON document as a JavaScript value.
///
/// Written out rather than routed through `JSON.parse`: a chrome payload crosses
/// on every cache miss, and a string built and reparsed there would be an
/// otherwise unnecessary allocation.
pub(super) fn to_js<'js>(ctx: &Ctx<'js>, value: &Json) -> JsResult<Value<'js>> {
    Ok(match value {
        Json::Null => Value::new_null(ctx.clone()),
        Json::Bool(flag) => Value::new_bool(ctx.clone(), *flag),
        Json::Number(number) => Value::new_number(ctx.clone(), number.as_f64().unwrap_or(f64::NAN)),
        Json::String(text) => rquickjs::String::from_str(ctx.clone(), text)?.into_value(),
        Json::Array(items) => {
            let array = rquickjs::Array::new(ctx.clone())?;
            for (index, item) in items.iter().enumerate() {
                array.set(index, to_js(ctx, item)?)?;
            }
            array.into_value()
        }
        Json::Object(entries) => {
            let object = Object::new(ctx.clone())?;
            for (key, entry) in entries {
                object.set(key.as_str(), to_js(ctx, entry)?)?;
            }
            object.into_value()
        }
    })
}

/// One queued change to a dock area's layout.
pub(in crate::engine) enum DockEdit {
    /// Install a whole layout, rebuilding its panels through the registry.
    Load(Box<gpui_base::dock::DockAreaState>),
    /// Dock the view one `cx.new(Class)` token names, under `name`.
    AddPanel {
        view: u32,
        /// Already namespaced: `shell:<application>/<panel>`.
        name: String,
        options: PanelOptions,
    },
    /// Remove the panel with this id, wherever it sits.
    RemovePanel(u64),
}

/// Applies one queued edit, at a boundary where constructing a view is legal.
pub(super) fn apply_edit(
    runtime: &Rc<ShellRuntime>,
    dock: EntityHandle,
    edit: DockEdit,
    window: &mut Window,
    cx: &mut App,
) -> anyhow::Result<()> {
    let area = { runtime.entities().dock(dock) };
    let Some(area) = area else {
        anyhow::bail!("this dock area was released before an edit to it could be applied");
    };

    match edit {
        DockEdit::Load(state) => area
            .update(cx, |area, cx| area.load(*state, window, cx))
            .map_err(|error| anyhow::anyhow!("{error}")),
        DockEdit::AddPanel {
            view,
            name,
            options,
        } => {
            let handle = runtime.nested_view_for_token(view).ok_or_else(|| {
                anyhow::anyhow!(
                    "add_panel(view, options) was given a view that was released before the \
                     panel could be added"
                )
            })?;
            let body = { runtime.entities().view(handle) }.ok_or_else(|| {
                anyhow::anyhow!(
                    "add_panel(view, options) expects a view from cx.new(Class); this one has \
                     been released"
                )
            })?;
            let placement = dock_placement(&options.placement).ok_or_else(|| {
                anyhow::anyhow!("`{}` is not a dock placement", options.placement)
            })?;
            let script = runtime.panel_script(&name);
            let panel = cx.new(|cx| {
                let mut panel = ScriptPanel::new(&name, body, cx)
                    .with_closable(options.closable)
                    .with_zoomable(options.zoomable)
                    .with_visible(options.visible);
                if let Some(script) = script {
                    panel = panel.with_script(script);
                }
                panel
            });
            area.update(cx, |area, cx| {
                area.add_panel(panel, placement, options.size.map(px), window, cx)
            });
            Ok(())
        }
        DockEdit::RemovePanel(panel) => {
            crate::dock::remove_panel(&area, PanelId::from_u64(panel), window, cx);
            Ok(())
        }
    }
}

//! `Accordion` — five parts, each of which is a chain of concrete types.
//!
//! Base's accordion is composed the way HTML composes a disclosure list: a
//! group holds items, an item connects a heading with a region, and the
//! heading owns the button that asks for the opposite of the item's `open`.
//! None of the five draws anything — no chevron, no border, no animation, no
//! layout. What they carry is the semantics a screen reader reads: the group,
//! the heading and its level, the button and its expanded state, and the
//! region the button controls.
//!
//! # Why the whole subtree is resolved rather than materialized
//!
//! `AccordionItem::header` takes an `AccordionHeader`, `AccordionHeader::new`
//! takes an `AccordionTrigger`, and `AccordionItem::panel` takes an
//! `AccordionPanel`. None of them takes an element. So an already-materialized
//! `AnyElement` is useless at every level, and the item has to read its own
//! subtree back out of the description and rebuild it — three types deep.
//!
//! That is also what makes the controlled state work. `AccordionItem::render`
//! passes its `open` down to both the header and the panel, so a script sets
//! it once on the item rather than three times in agreement with itself: the
//! trigger announces it, the panel mounts on it, and neither can drift from
//! the other.

use std::rc::Rc;

use gpui::{
    AnyElement, App, IntoElement as _, ParentElement as _, Refineable as _, SharedString,
    StyleRefinement, Styled as _, Window, div, prelude::FluentBuilder as _,
};
use gpui_base::{Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger};

use crate::ShellRuntime;
use crate::materialize::{
    Behavior, Children, SlotSpecs, StateStyles, materialize_children, resolve_slot, take_slot_spec,
    warn_unhonoured_a11y,
};
use crate::snapshot::RenderSnapshot;
use crate::spec::{Component, SpecArena, SpecId};

/// The accordion root: a group holding items, and nothing else.
pub(in crate::materialize) fn accordion(
    id: &str,
    refinement: StyleRefinement,
    behavior: Behavior,
    children: Children,
) -> AnyElement {
    warn_unhonoured_a11y(&behavior, "Accordion", &[]);
    let mut element = Accordion::new(SharedString::from(id.to_owned()));
    element.style().refine(&refinement);
    element.extend(children);
    element.into_any_element()
}

/// One item, with its header and panel rebuilt from their descriptions.
#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn accordion_item(
    runtime: &Rc<ShellRuntime>,
    snapshot: Option<&RenderSnapshot>,
    arena: &SpecArena,
    inherited: gpui::Hsla,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    mut slot_specs: SlotSpecs,
    children: Children,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    warn_unhonoured_a11y(&behavior, "AccordionItem", &[]);
    if !states.is_empty() {
        tracing::warn!(
            "state styles on an AccordionItem are ignored; put them on the trigger, which is \
             the interactive part"
        );
    }

    // An item with no `open` is a closed one: base has no uncontrolled mode,
    // and the script owns which item is showing.
    let open = behavior.open.unwrap_or(false);
    let header = take_slot_spec(&mut slot_specs, "header");
    let panel = take_slot_spec(&mut slot_specs, "panel");
    for (name, _) in slot_specs.iter() {
        tracing::warn!(
            "AccordionItem has no `{name}` slot, so the element given to it is not rendered \
             at all: a slot element is not drawn as an ordinary child"
        );
    }
    if header.is_none() {
        tracing::warn!(
            "an AccordionItem with no `header` slot has nothing to open it: the trigger lives \
             in the header, and base draws none of its own"
        );
    }

    let mut element = AccordionItem::new()
        .open(open)
        .disabled(behavior.disabled)
        .when_some(
            header.map(|slot| {
                accordion_header(runtime, snapshot, arena, slot, inherited, window, cx)
            }),
            AccordionItem::header,
        )
        .when_some(
            panel
                .map(|slot| accordion_panel(runtime, snapshot, arena, slot, inherited, window, cx)),
            AccordionItem::panel,
        );
    element.style().refine(&refinement);
    element.extend(children);
    element.into_any_element()
}

/// The heading, rebuilt around the trigger in its own slot.
fn accordion_header(
    runtime: &Rc<ShellRuntime>,
    snapshot: Option<&RenderSnapshot>,
    arena: &SpecArena,
    slot: SpecId,
    inherited: gpui::Hsla,
    window: &mut Window,
    cx: &mut App,
) -> AccordionHeader {
    let Some(node) = arena.node(slot) else {
        return AccordionHeader::new(placeholder_trigger(slot));
    };
    if !matches!(node.component(), Some(Component::AccordionHeader)) {
        tracing::warn!(
            "an AccordionItem's `header` slot must be an AccordionHeader.new(trigger); a {} \
             there loses whatever it draws itself",
            node.component().map(Component::name).unwrap_or("(nothing)")
        );
    }

    let (refinement, behavior, mut slots) =
        resolve_slot(arena, slot, "AccordionHeader", window, cx);
    let trigger = take_slot_spec(&mut slots, "trigger")
        .map(|slot| accordion_trigger(runtime, snapshot, arena, slot, inherited, window, cx))
        // `AccordionHeader::new` takes a trigger and there is no way not to
        // give it one, so a header built without one gets an empty button
        // rather than the whole item disappearing.
        .unwrap_or_else(|| placeholder_trigger(slot));

    let mut header = AccordionHeader::new(trigger);
    // The heading level is announced, not drawn, and base defaults it to 3.
    if let Some(level) = behavior.aria_level {
        header = header.level(level);
    }
    if let Some(key) = behavior.key.clone() {
        header = header.id(key);
    }
    header.style().refine(&refinement);
    header.extend(materialize_children(
        runtime, snapshot, arena, slot, inherited, window, cx,
    ));
    header
}

/// The button, rebuilt with the handler that asks for the other state.
fn accordion_trigger(
    runtime: &Rc<ShellRuntime>,
    snapshot: Option<&RenderSnapshot>,
    arena: &SpecArena,
    slot: SpecId,
    inherited: gpui::Hsla,
    window: &mut Window,
    cx: &mut App,
) -> AccordionTrigger {
    let Some(node) = arena.node(slot) else {
        return placeholder_trigger(slot);
    };
    let id = match node.component() {
        Some(Component::AccordionTrigger(id)) => SharedString::from(id.clone()),
        other => {
            tracing::warn!(
                "an AccordionHeader's `trigger` must be an AccordionTrigger.new(id); a {} there \
                 is not rendered at all",
                other.map(Component::name).unwrap_or("(nothing)")
            );
            placeholder_trigger_id(slot)
        }
    };

    let (refinement, behavior, _) = resolve_slot(arena, slot, "AccordionTrigger", window, cx);
    let mut trigger = AccordionTrigger::new(id);
    // `open` and `disabled` come from the item, which passes its own down over
    // whatever was set here — so they are not read off the trigger at all.
    if let Some(callback) = behavior.on_change {
        let runtime = Rc::downgrade(runtime);
        trigger = trigger.on_change(move |open, _, window, cx| {
            if let Some(runtime) = runtime.upgrade() {
                runtime.dispatch_change(callback, open, window, cx);
            }
        });
    } else {
        tracing::warn!(
            "an AccordionTrigger with no `on_change` cannot open anything: the item's `open` \
             is the script's, and this is what asks it to flip"
        );
    }
    trigger.style().refine(&refinement);
    trigger.extend(materialize_children(
        runtime, snapshot, arena, slot, inherited, window, cx,
    ));
    trigger
}

/// The region, rebuilt from its description.
fn accordion_panel(
    runtime: &Rc<ShellRuntime>,
    snapshot: Option<&RenderSnapshot>,
    arena: &SpecArena,
    slot: SpecId,
    inherited: gpui::Hsla,
    window: &mut Window,
    cx: &mut App,
) -> AccordionPanel {
    let mut panel = AccordionPanel::new();
    let Some(node) = arena.node(slot) else {
        return panel;
    };
    if !matches!(node.component(), Some(Component::AccordionPanel)) {
        tracing::warn!(
            "an AccordionItem's `panel` slot must be an AccordionPanel.new(); a {} there loses \
             whatever it draws itself",
            node.component().map(Component::name).unwrap_or("(nothing)")
        );
    }

    let (refinement, behavior, _) = resolve_slot(arena, slot, "AccordionPanel", window, cx);
    // The item passes its own `open` down, so the only state read here is
    // whether a shut panel stays in the tree — which is how its content keeps
    // its scroll position and its focus across a close and reopen.
    if behavior.keep_mounted {
        panel = panel.keep_mounted(true);
    }
    if let Some(key) = behavior.key.clone() {
        panel = panel.id(key);
    }
    panel.style().refine(&refinement);
    panel.extend(materialize_children(
        runtime, snapshot, arena, slot, inherited, window, cx,
    ));
    panel
}

/// The empty button a malformed header falls back to.
///
/// Keyed by the slot's address rather than by a fixed name, because two items
/// that both went wrong would otherwise be two elements sharing one id — GPUI
/// keys element state by that, so the second would inherit the first's. An
/// error path is where a script is already confused; handing it a second,
/// unrelated symptom is not the moment.
fn placeholder_trigger(slot: SpecId) -> AccordionTrigger {
    AccordionTrigger::new(placeholder_trigger_id(slot))
}

fn placeholder_trigger_id(slot: SpecId) -> SharedString {
    SharedString::from(format!("accordion-trigger-{slot}"))
}

/// A part reached outside the arrangement it belongs to.
pub(in crate::materialize) fn orphan(component: &str, expected: &str) -> AnyElement {
    tracing::warn!("{component} belongs in {expected}. Used as an ordinary child it draws nothing");
    div().into_any_element()
}

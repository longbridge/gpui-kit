//! `Avatar` — the one choice base makes, and none of the picture.
//!
//! Base's `Avatar` renders the element in its `image` slot, or the one in its
//! `fallback` slot when there is no image, and draws nothing itself: no circle,
//! no size, no background. All of that is the script's, written with the style
//! surface onto the three elements here.
//!
//! # Why the image slot is not an ordinary slot
//!
//! Every other slot in the shell hands its component a finished `AnyElement`.
//! `Avatar::image` does not take one — it takes an `AvatarImage`, which is
//! built from an image source. So the slot is resolved rather than
//! materialized: the described node is read back for its path and its styles,
//! and an `AvatarImage` is built from them. The same reasoning as a
//! `NumberInput`'s step buttons, for the same reason: the component needs a
//! concrete type back, and a materialized element is not one.
//!
//! The `fallback` slot has no such constraint — `AvatarFallback` is a box with
//! children — but it is resolved the same way rather than materialized, so
//! that both halves of one component read the same and neither is a special
//! case relative to the other.

use std::rc::Rc;

use gpui::{
    AnyElement, App, IntoElement as _, ParentElement as _, Refineable as _, StyleRefinement,
    Styled as _, Window, div, prelude::FluentBuilder as _,
};
use gpui_base::{Avatar, AvatarFallback, AvatarImage};

use crate::ShellRuntime;
use crate::materialize::{
    Behavior, Children, SlotSpecs, StateStyles, resolve_slot, take_slot_spec, warn_unhonoured_a11y,
};
use crate::snapshot::RenderSnapshot;
use crate::spec::{Component, SpecArena, SpecId};

/// The avatar root.
#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn avatar(
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
    warn_unhonoured_a11y(&behavior, "Avatar", &["accessibility_label"]);

    // `Avatar` is `Styled` and `InteractiveElement` but not stateful, so a
    // hover or active style has no element state to key off. Said rather than
    // dropped, as everywhere else.
    if !states.is_empty() {
        tracing::warn!("state styles on an Avatar are ignored; put them on the element around it");
    }

    let image = take_slot_spec(&mut slot_specs, "image");
    let fallback = take_slot_spec(&mut slot_specs, "fallback");
    for (name, _) in slot_specs.iter() {
        tracing::warn!(
            "Avatar has no `{name}` slot, so the element given to it is not rendered at all: \
             a slot element is not drawn as an ordinary child"
        );
    }
    if image.is_none() && fallback.is_none() {
        tracing::warn!(
            "an Avatar with neither an `image` nor a `fallback` slot draws nothing: base \
             renders one or the other and has no picture of its own"
        );
    }

    let mut element = Avatar::new()
        .when_some(
            image.and_then(|slot| avatar_image(arena, slot, window, cx)),
            Avatar::image,
        )
        .when_some(
            fallback
                .map(|slot| avatar_fallback(runtime, snapshot, arena, slot, inherited, window, cx)),
            Avatar::fallback,
        );
    element.style().refine(&refinement);
    // Ordinary children are drawn beside whichever slot won, which is where a
    // badge or a status dot goes.
    let mut element = element.into_any_element();
    if !children.is_empty() {
        let mut wrapper = div();
        wrapper.extend(std::iter::once(element));
        wrapper.extend(children);
        element = wrapper.into_any_element();
    }
    element
}

/// The image slot, rebuilt as the concrete type `Avatar::image` takes.
///
/// A slot filled with anything but `AvatarImage.new(path)` has no path to build
/// from, so it is reported and dropped rather than silently rendered as a
/// child: a slot element is detached from the tree, and a dropped one would
/// simply vanish.
fn avatar_image(
    arena: &SpecArena,
    slot: SpecId,
    window: &mut Window,
    cx: &mut App,
) -> Option<AvatarImage> {
    let node = arena.node(slot)?;
    let Some(Component::AvatarImage(path)) = node.component() else {
        tracing::warn!(
            "an Avatar's `image` slot must be an AvatarImage.new(path); a {} there is not \
             rendered at all",
            node.component().map(Component::name).unwrap_or("(nothing)")
        );
        return None;
    };
    let (refinement, _, _) = resolve_slot(arena, slot, "AvatarImage", window, cx);
    let mut image = AvatarImage::new(gpui::SharedString::from(path.clone()));
    image.style().refine(&refinement);
    Some(image)
}

/// The fallback slot, rebuilt as the concrete type `Avatar::fallback` takes.
fn avatar_fallback(
    runtime: &Rc<ShellRuntime>,
    snapshot: Option<&RenderSnapshot>,
    arena: &SpecArena,
    slot: SpecId,
    inherited: gpui::Hsla,
    window: &mut Window,
    cx: &mut App,
) -> AvatarFallback {
    let mut fallback = AvatarFallback::new();
    let Some(node) = arena.node(slot) else {
        return fallback;
    };
    if !matches!(node.component(), Some(Component::AvatarFallback)) {
        tracing::warn!(
            "an Avatar's `fallback` slot must be an AvatarFallback.new(); a {} there loses \
             whatever it draws itself",
            node.component().map(Component::name).unwrap_or("(nothing)")
        );
    }
    let (refinement, _, _) = resolve_slot(arena, slot, "AvatarFallback", window, cx);
    fallback.style().refine(&refinement);
    fallback.extend(crate::materialize::materialize_children(
        runtime, snapshot, arena, slot, inherited, window, cx,
    ));
    fallback
}

/// A bare `AvatarImage` or `AvatarFallback` reached outside an `Avatar`.
///
/// Neither is an element on its own — each exists to be resolved by the root —
/// so one used as an ordinary child is a mistake worth naming rather than an
/// empty box worth puzzling over.
pub(in crate::materialize) fn orphan(component: &str) -> AnyElement {
    tracing::warn!(
        "{component} belongs in an Avatar's slot: `Avatar.new().image(...)` or \
         `.fallback(...)`. Used as an ordinary child it draws nothing"
    );
    div().into_any_element()
}

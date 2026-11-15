//! `NumberInput` — the spinbutton frame around retained single-line text state.
//!
//! There is no `NumberInputState`. A number input is an ordinary
//! [`InputState`](gpui_base::input::InputState) with a step, a range and a
//! numeric mask, so the script keeps holding the same handle it would hold for
//! a text field and the entity store needs nothing new. The step, the bounds
//! and the mask are set on that handle rather than on the element — see the
//! `set_step`, `set_min` and `set_max` calls on `InputStateHandle`.
//!
//! # Why the two button slots are not ordinary slots
//!
//! Every other slot in the shell hands its component a finished `AnyElement`.
//! These two cannot. Base builds the step buttons itself and then chains
//! `focusable(false)`, `disabled(...)` and `on_click(...)` onto whatever the
//! decorator returned, so `decrement_button` takes a `FnOnce(Button) -> Button`
//! — it needs the `Button` back, not an element of some other type. A
//! materialized `AnyElement` is not a `Button` and never can be.
//!
//! So these two slots are resolved rather than materialized: the described
//! node's ops are turned into a [`Decoration`] — a refinement, its state
//! styles, its accessibility label and its already-materialized children — and
//! replayed onto the `Button` base hands the decorator.
//!
//! That is not a nicety. Base's `Button` is unstyled: zero content and zero
//! size. A number input whose buttons were left undecorated has two step
//! controls that cannot be seen and cannot be hit, and one whose `input` slot
//! is empty has nowhere to type. All three slots are load-bearing, which is why
//! the `input` one falls back to the bare editor for the state the control
//! already holds rather than to nothing.
//!
//! # The keyboard is free here
//!
//! Base's `NumberInput` sets `key_context("NumberInput")` on its own frame and
//! binds Up and Down to its two actions in `gpui_base::init`, which the shell
//! calls. So arrow-key stepping works in a script without the shell wiring
//! anything — unlike `DatePicker`, which sets no key context and therefore
//! loses its keyboard once it is driven from a script.

use std::rc::Rc;

use gpui::{
    AnyElement, App, IntoElement as _, ParentElement as _, Refineable as _, SharedString,
    StyleRefinement, Styled as _, Window, div, prelude::FluentBuilder as _,
};
use gpui_base::{Button, NumberInput, StepAction, StyledExt as _, input::Input};

use crate::{
    engine::ShellRuntime,
    entities::EntityHandle,
    materialize::{
        Behavior, Children, SlotSpecs, StateStyles, apply_motion, element_id, finish,
        materialize_node, resolve_ops, warn_ignored_key, warn_unhonoured_a11y,
        with_active_and_focus, with_hover,
    },
    snapshot::RenderSnapshot,
    spec::{Component, SpecArena, SpecId},
};

// Eleven arguments, because this is the one component that reads its slots
// unmaterialized: the arena, the ambient text color and the two GPUI contexts
// are what `materialize_node` would otherwise have already spent on its behalf.
#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn number_input(
    runtime: &Rc<ShellRuntime>,
    snapshot: Option<&RenderSnapshot>,
    arena: &SpecArena,
    handle: EntityHandle,
    inherited: gpui::Hsla,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    mut slot_specs: SlotSpecs,
    children: Children,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    // The identity is the state, exactly as it is for `Input`: the handle
    // outlives the description, and a second name would only compete with it.
    warn_ignored_key(&behavior, "NumberInput");
    // The keyboard belongs to the `InputState`, and base announces the frame as
    // a spinbutton itself.
    warn_unhonoured_a11y(&behavior, "NumberInput", &[]);

    let Some(state) = runtime.entities().input(handle) else {
        tracing::error!("input handle {handle} is no longer live");
        return div().into_any_element();
    };

    // `NumberInput` is `Styled` and `ParentElement` but not interactive: it
    // renders an `InputBase` and hangs its actions on that, so a state style on
    // the root has no `Interactivity` to land on. The `Switch` precedent — say
    // so rather than drop it in silence.
    if !states.is_empty() {
        tracing::warn!(
            "state styles on a NumberInput are ignored; put them on the step buttons, which are \
             interactive, or on the row around the control"
        );
    }

    let editor = take_slot(&mut slot_specs, "input");
    let decrement = take_slot(&mut slot_specs, "decrement_button");
    let increment = take_slot(&mut slot_specs, "increment_button");
    for (name, _) in slot_specs.iter() {
        tracing::warn!(
            "NumberInput has no `{name}` slot, so the element given to it is not rendered at \
             all: a slot element is not drawn as an ordinary child"
        );
    }

    let editor = match editor {
        Some(slot) => materialize_node(runtime, snapshot, arena, slot, inherited, window, cx),
        // Base supplies no editor of its own, so an unfilled slot would leave a
        // frame with nothing to type into. The bare editor for the state the
        // control already holds is the only thing it could mean — and it is the
        // bare one on purpose: `Input.new(state)` is the *framed* editor, and a
        // frame inside this frame draws two borders.
        None => Input::new(&state).into_any_element(),
    };
    let decrement = decrement.map(|slot| {
        Decoration::resolve(
            runtime,
            snapshot,
            arena,
            slot,
            "decrement_button",
            inherited,
            window,
            cx,
        )
    });
    let increment = increment.map(|slot| {
        Decoration::resolve(
            runtime,
            snapshot,
            arena,
            slot,
            "increment_button",
            inherited,
            window,
            cx,
        )
    });
    if decrement.is_none() || increment.is_none() {
        tracing::warn!(
            "a NumberInput whose `decrement_button` or `increment_button` slot is empty draws \
             that step button with no size and no content: base's Button is unstyled, so an \
             undecorated one cannot be seen and cannot be pressed"
        );
    }

    let element = NumberInput::new(&state)
        .disabled(behavior.disabled)
        .input(editor)
        .when(behavior.controls_right, NumberInput::controls_right)
        .when_some(decrement, |input, decoration| {
            input.decrement_button(move |button| decoration.apply(button))
        })
        .when_some(increment, |input, decoration| {
            input.increment_button(move |button| decoration.apply(button))
        })
        .when_some(behavior.on_step, |input, callback| {
            let runtime = Rc::downgrade(runtime);
            input.on_step(move |action, window, cx| {
                let Some(runtime) = runtime.upgrade() else {
                    return;
                };
                runtime.dispatch_step(
                    callback,
                    match action {
                        StepAction::Increment => "increment",
                        StepAction::Decrement => "decrement",
                    },
                    window,
                    cx,
                );
            })
        });

    finish(element, refinement, children)
}

/// Takes the node filling `name`, leaving any other slot for its own reader.
///
/// The unmaterialized twin of [`take_slot`](crate::materialize::take_slot):
/// what the two button slots need is the description, not an element built
/// from it.
fn take_slot(slots: &mut SlotSpecs, name: &str) -> Option<SpecId> {
    slots
        .iter()
        .position(|(slot, _)| *slot == name)
        .map(|index| slots.remove(index).1)
}

/// A described element, resolved into the parts that can be moved onto a
/// `Button` somebody else constructed.
#[derive(Default)]
struct Decoration {
    /// Set when the slot was filled with `h_flex()` or `v_flex()`. Those two
    /// carry their layout in the constructor rather than in a recorded op, so
    /// it has to be reapplied before the script's own styles refine over it.
    flex: Option<gpui::Axis>,
    refinement: StyleRefinement,
    states: StateStyles,
    accessibility_label: Option<SharedString>,
    children: Children,
}

impl Decoration {
    #[allow(clippy::too_many_arguments)]
    fn resolve(
        runtime: &Rc<ShellRuntime>,
        snapshot: Option<&RenderSnapshot>,
        arena: &SpecArena,
        slot: SpecId,
        name: &str,
        inherited: gpui::Hsla,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let Some(node) = arena.node(slot) else {
            return Self::default();
        };
        let Some(component) = node.component() else {
            return Self::default();
        };

        let (mut refinement, behavior, states, motions, inner) = resolve_ops(arena, node);
        // Motion is sampled against an identity the same way every other node's
        // is; the script's own name for the slot wins over its address for the
        // same reason it does there.
        apply_motion(
            element_id(slot, behavior.key.clone()),
            &motions,
            &mut refinement,
            window,
            cx,
        );

        let flex = match component {
            Component::Div => None,
            Component::HFlex => Some(gpui::Axis::Horizontal),
            Component::VFlex => Some(gpui::Axis::Vertical),
            Component::Button(id) => {
                tracing::warn!(
                    "the id in Button.new(\"{id}\") is dropped in a NumberInput's `{name}` slot: \
                     base builds the step button and identifies it, and only the styles and the \
                     children written here are carried onto it. An h_flex() says the same thing \
                     without an id nobody reads"
                );
                None
            }
            other => {
                tracing::warn!(
                    "a NumberInput's `{name}` slot supplies the styles and the children of the \
                     step button base builds, so a {} put there loses whatever it draws itself. \
                     Wrap it in an h_flex()",
                    other.name()
                );
                None
            }
        };

        for (method, called) in [
            ("disabled", behavior.disabled),
            ("on_click", behavior.on_click.is_some()),
        ] {
            if called {
                tracing::warn!(
                    "`{method}` on a NumberInput's `{name}` slot is overwritten: base chains its \
                     own onto the decorated button, because the control owns whether stepping is \
                     allowed and what a press does"
                );
            }
        }
        for (inner, _) in inner.iter() {
            tracing::warn!(
                "a step button has no `{inner}` slot, so the element given to it is not rendered \
                 at all"
            );
        }

        let inherited = refinement.text.color.unwrap_or(inherited);
        Self {
            flex,
            refinement,
            states,
            accessibility_label: behavior.accessibility_label,
            children: node
                .children()
                .iter()
                .map(|child| {
                    materialize_node(runtime, snapshot, arena, *child, inherited, window, cx)
                })
                .collect(),
        }
    }

    /// Replays the description onto the button base built.
    fn apply(self, button: Button) -> Button {
        let button = with_hover(button, &self.states);
        let mut button = with_active_and_focus(button, &self.states);
        if let Some(axis) = self.flex {
            button = match axis {
                gpui::Axis::Horizontal => button.h_flex(),
                gpui::Axis::Vertical => button.v_flex(),
            };
        }
        // Honoured rather than warned about, unlike `disabled`: base sets no
        // label on the step buttons, and an icon-only one announces nothing
        // without it.
        if let Some(label) = self.accessibility_label {
            button = button.accessibility_label(label);
        }
        button.style().refine(&self.refinement);
        button.extend(self.children);
        button
    }
}

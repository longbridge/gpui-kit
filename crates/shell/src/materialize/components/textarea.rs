//! `Textarea` — the frame around retained multi-line text state.
//!
//! Almost everything here is what [`Component::Input`](crate::spec::Component)
//! does, and for the same reasons: the state is the identity, `InputBase`
//! carries the semantics a bare `div` would not, and a click anywhere in the
//! frame focuses the text rather than only a click on the glyphs.
//!
//! Two things differ. The text is laid out from the top, because a paragraph
//! that grows downward from a vertically centred first line jumps as it is
//! typed. And the height is the script's: the layout default is one row even
//! for a textarea — being multi-line is carried by the mode, not by the layout
//! — so `rows(...)` on the state or `.h(...)` on the frame is what makes it
//! look like a textarea.

use std::rc::Rc;

use gpui::{
    AnyElement, InteractiveElement as _, IntoElement as _, MouseButton, ParentElement as _,
    Refineable as _, StyleRefinement, Styled as _, div,
};
use gpui_base::input::{InputBase, Textarea};

use crate::{
    engine::ShellRuntime,
    entities::EntityHandle,
    materialize::{
        Behavior, Children, StateStyles, warn_unhonoured_a11y, with_active_and_focus, with_hover,
    },
};

pub(in crate::materialize) fn textarea(
    runtime: &Rc<ShellRuntime>,
    handle: EntityHandle,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    // The keyboard belongs to the `TextareaState`, which is what the mouse
    // handler below hands it. A focus handle on the frame would be a target the
    // caret never follows.
    warn_unhonoured_a11y(&behavior, "Textarea", &[]);
    let Some(state) = runtime.entities().textarea(handle) else {
        tracing::error!("textarea handle {handle} is no longer live");
        return div().into_any_element();
    };

    let focus_target = state.clone();
    let mut frame = InputBase::new(("gpui-shell-textarea", handle))
        .flex()
        .items_start()
        .w_full()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            focus_target.update(cx, |state, cx| state.focus(window, cx));
        });

    frame.style().refine(&refinement);
    frame.extend(children);
    let frame = with_hover(frame, &states);
    let frame = with_active_and_focus(frame, &states);
    let callbacks = crate::InlineTokenCallbacks::new(
        &state,
        behavior
            .token
            .map(|id| crate::ComponentElementCallback::from_runtime(runtime, id)),
        behavior
            .on_token_click
            .map(|id| crate::ComponentCallback::from_runtime(runtime, id)),
    );
    let textarea = callbacks.apply(
        Textarea::new(&state),
        |input, render| input.token(move |token, window, cx| render(token, window, cx)),
        |input, listen| input.on_token_click(move |event, window, cx| listen(event, window, cx)),
    );
    frame.child(textarea).into_any_element()
}

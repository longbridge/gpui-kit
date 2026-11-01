//! `OtpInput` — a fixed-length code whose cells are drawn by the shell.
//!
//! Base's `OtpInput` is the keyboard and the focus and nothing else: "unstyled
//! OTP interaction root. Applications provide the visual cells as children." A
//! `gpui-component` application does exactly that, building one box per digit
//! from `value()`, `len()`, `is_masked()` and `cursor_visible()` on every
//! frame.
//!
//! # Why the cells are not the script's
//!
//! A script renders once; what it produced is frozen into a `RenderSnapshot`
//! and replayed by every frame after, without the VM. That is what makes an
//! `Input` work — the glyphs are drawn by the `InputState` entity, so typing
//! repaints without anyone asking the script for a new description.
//!
//! Cells described by the script are the opposite arrangement, and two separate
//! things break:
//!
//! * **Digits appear all at once, or not at all.** For the screen to change as
//!   a digit lands, the script would have to render again, which needs a
//!   callback. `OtpState` now emits `Change` for each keyboard edit, but a
//!   script render still does not own the native cell repaint or caret blink.
//!   Typing the first five digits of a six-digit code would leave the screen
//!   exactly as it was, and the sixth would fill every cell at once.
//! * **The caret never blinks.** `BlinkCursor` notifies `OtpState` twice a
//!   second, but `OtpState::render` returns `Empty`: the notification repaints
//!   an empty entity and never marks the script view dirty. There is no event
//!   at all for a script to hang the blink on.
//!
//! So the cells are built here, from the state, on every frame — and what they
//! look like comes from style templates the script declares once:
//!
//! ```js
//! OtpInput.new(this.code)
//!   .flex().gap(8)
//!   .cell_style((cell) => cell.size(40).border_1().border_color(`#d1d5db`).rounded("md")
//!                             .flex().items_center().justify_center())
//!   .cell_active_style((cell) => cell.border_color(`#2563eb`))
//!   .caret_style((caret) => caret.w(2).h(18).bg(`#111111`))
//! ```
//!
//! This is the arrangement `Slider` already uses for its fill: the script says
//! how a box *looks* and the shell says *when and where* there is one. It costs
//! the script the ability to put arbitrary elements in a cell — a badge on the
//! third digit, say — which is the same price [`Component::SliderIndicator`]'s
//! `range_style` pays, and for the same reason.
//!
//! # Why three templates rather than one with a state argument
//!
//! The shell already has one method per named template — `hover`, `active`,
//! `focus`, `range_style` — all built on the same detached-node op, and all
//! *layered* onto the style underneath rather than replacing it. A
//! `cell_style(state, declare)` taking the state as an argument would be a
//! second grammar for the thing the first grammar already spells, and its
//! declarations would have to be complete rather than layered: every state
//! repeating the width, the height and the border. So `cell_active_style` is
//! declared and applied exactly as `hover` is, on top of `cell_style`.
//!
//! # What masking means here
//!
//! Base stores the flag and draws nothing, so the glyph is the shell's to
//! choose. It is [`MASK_CHAR`], the same bullet base's own masked text editors
//! use — not `gpui-component`'s asterisk icon, because the shell has no icon
//! set it can count on.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Focusable as _, InteractiveElement as _, IntoElement as _, MouseButton,
    ParentElement as _, Refineable as _, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_base::OtpInput;

use crate::{
    engine::ShellRuntime,
    entities::EntityHandle,
    materialize::{
        Behavior, Children, StateStyles, finish, warn_ignored_key, warn_unhonoured_a11y,
        warn_unsupported,
    },
};

/// What a masked cell shows in place of its digit.
///
/// The bullet rather than an asterisk: it is what `gpui_base`'s own masked text
/// editors draw, and unlike `*` it sits on the centre line, which is where a
/// box the size of a digit expects it. `gpui-component` uses an asterisk icon,
/// but an icon is an asset, and the shell cannot assume a script has one.
const MASK_CHAR: char = '\u{2022}';

/// The interaction root, plus one cell per digit.
///
/// Its identity is the state's — base builds it from `state.entity_id()` — so
/// `id()` has nowhere to go.
///
// Eight arguments: the window and the app on top of the usual six, because
// unlike every other component this one reads the state while it is being
// built — the digits from the app, and whether the code holds the keyboard
// from the window.
#[allow(clippy::too_many_arguments)]
pub(in crate::materialize) fn otp_input(
    runtime: &Rc<ShellRuntime>,
    handle: EntityHandle,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
    window: &Window,
    cx: &App,
) -> AnyElement {
    warn_ignored_key(&behavior, "OtpInput");
    // The keyboard belongs to the `OtpState`: base's root tracks that handle
    // itself and has no builder to override any of it.
    warn_unhonoured_a11y(&behavior, "OtpInput", &[]);
    warn_unsupported(
        "OtpInput",
        &[
            ("on_click", behavior.on_click.is_some()),
            (
                "accessibility_label",
                behavior.accessibility_label.is_some(),
            ),
        ],
    );
    // Named separately from the two above because the call has somewhere to go,
    // just not here: the code is state, so what changed is the state's to
    // report.
    if behavior.on_change.is_some() {
        tracing::warn!(
            "`on_change` is not an OtpInput method: the code is reported by the state it lives \
             in. Subscribe with `state.on(\"change\", ...)` for edits or \
             `state.on(\"complete\", ...)` for a full code"
        );
    }

    let Some(state) = runtime.entities().otp(handle) else {
        tracing::error!("otp handle {handle} is no longer live");
        return div().into_any_element();
    };

    // `OtpInput` is a `RenderOnce` over a `Div` and implements neither
    // `InteractiveElement` nor `StatefulInteractiveElement`, so a state style
    // here has nowhere to land. As with `Switch` and `Slider`, saying so beats
    // dropping it without a word.
    if states.hover.is_some() || states.active.is_some() || states.focus.is_some() {
        tracing::warn!(
            "state styles on an OtpInput are ignored: it is not an interactive element, and \
             its cells are the shell's rather than the script's. Put a hover on the element \
             around it, and use `cell_active_style` for the cell taking the next digit"
        );
    }
    // Without it every cell is a zero-sized box: the control renders, reports
    // nothing wrong, and is invisible.
    if behavior.cell_style.is_none() {
        tracing::warn!(
            "this OtpInput has no `cell_style`, so its cells have no size, no border and no \
             background: it draws nothing at all. Declare one with \
             `cell_style((cell) => cell.size(40)...)`"
        );
    }

    let read = state.read(cx);
    let value = read.value().clone();
    let length = read.len();
    let masked = read.is_masked();
    let caret_visible = read.cursor_visible(cx);
    // Only while the control holds the keyboard, and only while it can take a
    // digit: a highlighted cell on a blurred or disabled code says the next
    // keystroke goes there, and it does not.
    let active = (!behavior.disabled && read.focus_handle(cx).is_focused(window))
        .then(|| cursor_index(value.chars().count(), length));

    let mut all = Children::with_capacity(length + children.len());
    all.extend((0..length).map(|index| {
        let focus_target = state.clone();
        div()
            .map(|cell| refine(cell, behavior.cell_style.as_ref()))
            .when(active == Some(index), |cell| {
                refine(cell, behavior.cell_active_style.as_ref())
            })
            // Base focuses on Tab, but a code is something a pointer user
            // clicks *at* rather than tabs to, and the root's own box is the
            // gaps between the cells.
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus_target.update(cx, |state, cx| state.focus(window, cx));
            })
            .map(
                |cell| match cell_content(&value, index, masked, active, caret_visible) {
                    CellContent::Digit(digit) => cell.child(SharedString::from(digit.to_string())),
                    CellContent::Caret => cell
                        .when_some(behavior.caret_style.as_ref(), |cell, style| {
                            cell.child(refine(div(), Some(style)))
                        }),
                    CellContent::Empty => cell,
                },
            )
            .into_any_element()
    }));
    // After the cells, because the cells are what an `OtpInput` is. A script
    // that adds a child gets it beside them rather than instead of them.
    all.extend(children);
    // The state, mounted as a view so that its notifications reach the screen.
    // The cells above read it while they are built, but it is a plain entity:
    // a keystroke or a caret blink notifies it and dirties no view, and the
    // frame replays `ShellRoot`'s cached content with the old cells until the
    // pointer moves. As a view node under the script view, its notification
    // dirties every view above it. Its render is `Empty`, and the absolute
    // holder keeps it out of the flow, so a script `gap` adds no space.
    all.push(div().absolute().child(state.clone()).into_any_element());

    let otp = OtpInput::new(&state).disabled(behavior.disabled);
    finish(otp, refinement, all)
}

/// Applies a declared template, if the script declared one.
fn refine<E: Styled>(mut element: E, style: Option<&StyleRefinement>) -> E {
    if let Some(style) = style {
        element.style().refine(style);
    }
    element
}

/// Which cell the next digit lands in.
///
/// Clamped to the last cell rather than running off the end: a complete code
/// still has to say where a backspace would take effect, and base accepts a
/// programmatic value longer than the code — `set_value` is deliberately
/// unfiltered — so `filled` is not bounded by `length`.
fn cursor_index(filled: usize, length: usize) -> usize {
    filled.min(length.saturating_sub(1))
}

/// What one cell shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CellContent {
    /// A digit, or [`MASK_CHAR`] standing in for one.
    Digit(char),
    /// The blinking mark on the cell taking the next digit.
    Caret,
    Empty,
}

/// Resolves one cell against the state as it is this frame.
///
/// Masking is decided here because base decides nothing about it: `OtpState`
/// holds the flag and draws no cells, so what a masked cell shows is the
/// shell's answer to give.
fn cell_content(
    value: &str,
    index: usize,
    masked: bool,
    active: Option<usize>,
    caret_visible: bool,
) -> CellContent {
    match value.chars().nth(index) {
        Some(_) if masked => CellContent::Digit(MASK_CHAR),
        Some(digit) => CellContent::Digit(digit),
        None if active == Some(index) && caret_visible => CellContent::Caret,
        None => CellContent::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The caret sits where the next digit goes — except on a complete code,
    /// which has no such cell and would otherwise point past the last one.
    #[test]
    fn the_cursor_stops_at_the_last_cell() {
        assert_eq!(cursor_index(0, 6), 0);
        assert_eq!(cursor_index(3, 6), 3);
        assert_eq!(cursor_index(6, 6), 5);
        // `set_value` is unfiltered by design, so an over-length code reaches
        // here rather than being refused on the way in.
        assert_eq!(cursor_index(9, 6), 5);
        assert_eq!(cursor_index(0, 0), 0);
    }

    /// The three things a cell can be, and the one the script cannot reach:
    /// masking replaces the glyph rather than the value, so the code stays
    /// readable through `value()` while the screen does not show it.
    #[test]
    fn a_cell_shows_its_digit_masked_or_the_caret() {
        let active = Some(2);
        assert_eq!(
            cell_content("12", 0, false, active, true),
            CellContent::Digit('1')
        );
        assert_eq!(
            cell_content("12", 0, true, active, true),
            CellContent::Digit(MASK_CHAR)
        );
        assert_eq!(
            cell_content("12", 2, false, active, true),
            CellContent::Caret
        );
        // Half of every blink, and every cell that is not the active one.
        assert_eq!(
            cell_content("12", 2, false, active, false),
            CellContent::Empty
        );
        assert_eq!(
            cell_content("12", 3, false, active, true),
            CellContent::Empty
        );
        assert_eq!(cell_content("12", 2, false, None, true), CellContent::Empty);
    }
}

//! The dock panel wrapper around a terminal view.
//!
//! A dock panel is two traits: behavior in [`BasePanel`] (re-exported from
//! `gpui-base`) and presentation in [`Panel`]. The terminal itself stays in
//! `crate::terminal`; this type only bridges it into the dock — a tab title,
//! the dock's focus contract, and a full-size host for the view.

use std::path::PathBuf;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    dock::{BasePanel, Panel, PanelEvent},
    h_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::terminal::{TerminalEvent, TerminalState};

/// Side of the warning dot shown in the tab while a bell went unread.
const ATTENTION_DOT: Pixels = px(6.);

/// One terminal tab in the center dock area.
///
/// Each instance owns its own [`TerminalState`], and therefore its own PTY
/// running the login shell; "new terminal tab" means a new instance.
///
/// Needs-attention state machine: a BEL that arrives while the terminal has
/// no focus flips [`Self::needs_attention`], which paints a warning dot in
/// the tab title; the user switching back to the tab (or focus landing on
/// the terminal by any route) clears it.
pub struct TerminalPanel {
    terminal: Entity<TerminalState>,
    /// A bell arrived while nobody was looking at this terminal.
    needs_attention: bool,
    /// Holds the terminal event subscription for this panel's lifetime;
    /// dropping it would unsubscribe.
    _events: Subscription,
}

impl TerminalPanel {
    /// Create a panel backed by a fresh terminal running the login shell.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let terminal = cx.new(TerminalState::new);
        Self::with_terminal(terminal, cx)
    }

    /// Create a panel whose login shell starts in `cwd` (the shell's own
    /// default when `None`).
    pub fn new_in(cx: &mut Context<Self>, cwd: Option<PathBuf>) -> Self {
        let terminal = cx.new(|cx| TerminalState::new_in(cx, cwd));
        Self::with_terminal(terminal, cx)
    }

    /// Bridge a terminal view into the dock: subscribe to its bell/focus
    /// events for the tab-title attention dot.
    fn with_terminal(terminal: Entity<TerminalState>, cx: &mut Context<Self>) -> Self {
        // The terminal gates bells on its own focus, so every `Bell` here is
        // one the user has not seen yet.
        let _events = cx.subscribe(
            &terminal,
            |this, _, event: &TerminalEvent, cx| match event {
                TerminalEvent::Bell => {
                    this.needs_attention = true;
                    cx.notify();
                }
                TerminalEvent::Focused => {
                    if this.needs_attention {
                        this.needs_attention = false;
                        cx.notify();
                    }
                }
            },
        );
        Self {
            terminal,
            needs_attention: false,
            _events,
        }
    }
}

impl EventEmitter<PanelEvent> for TerminalPanel {}

impl Focusable for TerminalPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.terminal.read(cx).focus_handle.clone()
    }
}

impl BasePanel for TerminalPanel {
    fn panel_name(&self) -> &'static str {
        "Terminal"
    }

    fn set_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        // The terminal owns keyboard input, so becoming the displayed tab
        // means becoming the focus target.
        if active {
            // Switching back counts as reading the bell. Focusing the
            // terminal below would clear it too, but focus may already sit
            // on this terminal, in which case no Focused event re-fires.
            if self.needs_attention {
                self.needs_attention = false;
                cx.notify();
            }
            let focus_handle = self.terminal.read(cx).focus_handle.clone();
            window.focus(&focus_handle, cx);
        }
    }
}

impl Panel for TerminalPanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .items_center()
            .child(Icon::new(IconName::SquareTerminal).small())
            .child("Terminal")
            // Warning dot while a bell went unread. The tab strip rebuilds
            // this element from the panel on every frame it redraws, so the
            // `cx.notify()` calls on state changes are what refresh it.
            .when(self.needs_attention, |title| {
                title.child(
                    div()
                        .size(ATTENTION_DOT)
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(cx.theme().warning),
                )
            })
    }

    // `tab_name` is deliberately left unimplemented: the dock prefers it
    // over `title()` when drawing tabs, and it is plain text, so keeping it
    // would hide the warning dot. The icon + label + dot element above
    // serves every tab shape instead.

    fn inner_padding(&self, _: &App) -> bool {
        // The terminal paints its own full-bleed surface; padding would show
        // as a ring of theme background around it.
        false
    }
}

impl Render for TerminalPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.terminal.clone())
    }
}

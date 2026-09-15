//! Haptic feedback, for hosts that have it.
//!
//! GPUI has no haptics of its own. A mobile host that does — iOS's
//! `UISelectionFeedbackGenerator`, Android's `HapticFeedbackConstants` —
//! installs a provider once, and Base behaviors ask for feedback at the
//! moments the platform would: a long press taking hold of a selection, for
//! one. Without a provider the requests go nowhere, which is what a desktop
//! wants.

use std::rc::Rc;

use gpui::{App, Global};

/// The kinds of feedback a behavior may ask for, named after what happened
/// rather than after a motor pattern, so a host maps each to its own idiom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HapticFeedback {
    /// A selection was taken or changed under a finger: the tick iOS plays
    /// through `UISelectionFeedbackGenerator`.
    Selection,
    /// Something snapped or locked in: a light `UIImpactFeedbackGenerator`.
    Impact,
}

type HapticProvider = Rc<dyn Fn(HapticFeedback, &mut App)>;

struct HapticsGlobal(HapticProvider);

impl Global for HapticsGlobal {}

/// The haptics seam between Base behaviors and the host.
pub struct Haptics;

impl Haptics {
    /// Installs the host's feedback provider, replacing any earlier one.
    pub fn set_provider(provider: impl Fn(HapticFeedback, &mut App) + 'static, cx: &mut App) {
        cx.set_global(HapticsGlobal(Rc::new(provider)));
    }

    /// Asks the host for feedback. Nothing happens without a provider.
    pub fn play(feedback: HapticFeedback, cx: &mut App) {
        let Some(provider) = cx
            .try_global::<HapticsGlobal>()
            .map(|haptics| haptics.0.clone())
        else {
            return;
        };
        provider(feedback, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[gpui::test]
    fn plays_through_the_installed_provider_only(cx: &mut gpui::TestAppContext) {
        let played = Rc::new(RefCell::new(Vec::new()));
        cx.update(|cx| Haptics::play(HapticFeedback::Selection, cx));
        assert!(played.borrow().is_empty());

        let sink = played.clone();
        cx.update(|cx| {
            Haptics::set_provider(move |feedback, _| sink.borrow_mut().push(feedback), cx);
            Haptics::play(HapticFeedback::Selection, cx);
            Haptics::play(HapticFeedback::Impact, cx);
        });
        assert_eq!(
            played.borrow().as_slice(),
            [HapticFeedback::Selection, HapticFeedback::Impact]
        );
    }
}

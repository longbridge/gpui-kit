//! Honors the operating system's reduced-motion preference.
//!
//! Every Base transition, spring, presence and reveal consults
//! [`App::reduce_motion`], but GPUI never reads the platform's setting into
//! that flag: it stays `false` until something sets it. [`init`] reads the
//! setting when Base initializes and writes it into the flag, so an
//! application inherits the user's choice by calling `gpui_base::init` or
//! `gpui_component::init`.
//!
//! The setting is read on the platforms below; everywhere else the flag is
//! left alone.
//!
//! - macOS: `NSWorkspace.accessibilityDisplayShouldReduceMotion`, the
//!   "Reduce motion" switch under Accessibility → Display.
//! - Windows: `SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION)`, the
//!   "Animation effects" switch under Accessibility → Visual effects, whose
//!   off state asks for reduced motion.
//! - Linux: the `reduced-motion` key of the `org.freedesktop.appearance`
//!   namespace of the XDG desktop portal's Settings interface, which GNOME
//!   and KDE back with their own animation switches. The portal answers over
//!   D-Bus, so the reading lands a moment after `init` returns, and Base keeps
//!   following the portal's change signal for the life of the application.
//!
//! Use [`set_reduce_motion_preference`] for an explicit application choice,
//! including a choice equal to the current flag, or to resume the system
//! preference. A direct [`App::set_reduce_motion`] call retains the legacy
//! value-comparison behavior: Base leaves a value different from what it last
//! wrote alone, but cannot recognize a same-value override.
//!
//! Under GPUI's test scheduler the platform is never consulted, whichever
//! crate's tests are running: a probe that answers from another thread would
//! break the scheduler's determinism, and a test wanting reduced motion sets
//! the flag itself.

use gpui::{App, Global};

/// The last system reading, flag ownership and platform-listener state.
#[derive(Default)]
struct SystemReduceMotion {
    applied: Option<bool>,
    system: Option<bool>,
    overridden: bool,
    following: bool,
}

impl Global for SystemReduceMotion {}

/// Applies the system's reduced-motion preference during `gpui_base::init`.
pub(crate) fn init(cx: &mut App) {
    apply_system_reduce_motion(cx);
}

/// Reads the operating system's reduced-motion preference into
/// [`App::set_reduce_motion`].
///
/// `gpui_base::init` calls this once. Call it again to re-read the preference
/// on a platform Base does not follow live. The current macOS and Windows
/// implementations read the preference only when this function is called.
///
/// Explicit overrides are preserved. For direct [`App::set_reduce_motion`]
/// calls, a flag different from what Base last wrote is also left alone; a
/// same-value application choice cannot be detected. This function does not
/// clear either override: use [`set_reduce_motion_preference`] with
/// [`ReduceMotionPreference::System`].
/// Where the platform cannot say (wasm, an unsupported desktop, a Linux
/// session without the portal) the flag is left as it is, and under GPUI's
/// test scheduler the platform is not asked at all.
pub fn apply_system_reduce_motion(cx: &mut App) {
    if is_test_scheduler(cx) {
        return;
    }
    apply_preference(platform::read(), cx);
    let state = cx.default_global::<SystemReduceMotion>();
    if !state.following {
        state.following = true;
        platform::follow(cx);
    }
}

/// The source and value of the application's reduced-motion preference.
///
/// This differs from [`crate::ReducedMotion`], which selects how an individual
/// animation behaves while reduced motion is active.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReduceMotionPreference {
    /// Use the last known system preference and existing platform updates.
    #[default]
    System,
    /// Reduce motion regardless of the system preference.
    Reduce,
    /// Allow motion regardless of the system preference.
    Allow,
}

/// Selects the application's reduced-motion preference.
///
/// [`ReduceMotionPreference::Reduce`] and [`ReduceMotionPreference::Allow`] set
/// [`App::reduce_motion`] and prevent Base's system readings from changing it,
/// even when the value already matches. The application can still change the
/// flag directly; system readings continue to be cached.
///
/// [`ReduceMotionPreference::System`] releases application ownership, resumes
/// the last known system value and calls [`apply_system_reduce_motion`] through
/// the existing platform mechanism. If no system value is known, the current
/// flag is kept until a reading arrives. On Linux, an existing portal listener
/// supplies updates; selecting System does not restart it or force a new read.
///
/// System also releases ownership taken through [`App::set_reduce_motion`].
/// Under the test scheduler, explicit choices and cached readings still apply,
/// but the platform is never queried.
pub fn set_reduce_motion_preference(preference: ReduceMotionPreference, cx: &mut App) {
    match preference {
        ReduceMotionPreference::Reduce | ReduceMotionPreference::Allow => {
            cx.default_global::<SystemReduceMotion>().overridden = true;
            cx.set_reduce_motion(preference == ReduceMotionPreference::Reduce);
        }
        ReduceMotionPreference::System => {
            let current = cx.reduce_motion();
            let state = cx.default_global::<SystemReduceMotion>();
            state.overridden = false;
            // Release both explicit ownership and the legacy value-comparison guard.
            state.applied = Some(current);
            let system = state.system;
            apply_preference(system, cx);
            apply_system_reduce_motion(cx);
        }
    }
}

/// Whether the application runs on GPUI's deterministic test scheduler,
/// which must not be woken by a platform answering from its own thread.
fn is_test_scheduler(cx: &App) -> bool {
    cx.background_executor()
        .scheduler_executor()
        .scheduler()
        .as_test()
        .is_some()
}

/// Writes one reading of the system preference into the flag, unless the
/// application has taken the flag over since the reading Base last applied.
fn apply_preference(preference: Option<bool>, cx: &mut App) {
    let Some(reduce) = preference else {
        return;
    };
    let state = cx.default_global::<SystemReduceMotion>();
    state.system = Some(reduce);
    let applied = state.applied;
    if state.overridden || cx.reduce_motion() != applied.unwrap_or(false) {
        return;
    }
    cx.set_reduce_motion(reduce);
    cx.default_global::<SystemReduceMotion>().applied = Some(reduce);
}

#[cfg(target_os = "macos")]
mod platform {
    use gpui::App;
    use objc2_app_kit::NSWorkspace;

    pub(super) fn read() -> Option<bool> {
        Some(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion())
    }

    pub(super) fn follow(_cx: &mut App) {}
}

#[cfg(target_os = "windows")]
mod platform {
    use gpui::App;
    use windows::{
        Win32::UI::WindowsAndMessaging::{
            SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
        },
        core::BOOL,
    };

    pub(super) fn read() -> Option<bool> {
        let mut animations_enabled = BOOL(0);
        // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL through `pvParam`
        // and ignores `uiParam`; the BOOL outlives the call.
        unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                Some((&mut animations_enabled as *mut BOOL).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        }
        .ok()?;
        Some(!animations_enabled.as_bool())
    }

    pub(super) fn follow(_cx: &mut App) {}
}

#[cfg(target_os = "linux")]
mod platform {
    use ashpd::desktop::settings::{ReducedMotion, Settings};
    use futures::StreamExt as _;
    use gpui::App;

    /// The portal answers asynchronously; [`follow`] applies the first reading.
    pub(super) fn read() -> Option<bool> {
        None
    }

    pub(super) fn follow(cx: &mut App) {
        cx.spawn(async move |cx| {
            let settings = Settings::new().await.ok()?;
            let current = settings.reduced_motion().await.ok()?;
            cx.update(|cx| super::apply_preference(Some(reduces(current)), cx));
            let mut changes = settings.receive_reduced_motion_changed().await.ok()?;
            while let Some(preference) = changes.next().await {
                cx.update(|cx| super::apply_preference(Some(reduces(preference)), cx));
            }
            Some(())
        })
        .detach();
    }

    fn reduces(preference: ReducedMotion) -> bool {
        matches!(preference, ReducedMotion::ReducedMotion)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod platform {
    use gpui::App;

    pub(super) fn read() -> Option<bool> {
        None
    }

    pub(super) fn follow(_cx: &mut App) {}
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::{
        ReduceMotionPreference, apply_preference, apply_system_reduce_motion,
        set_reduce_motion_preference,
    };

    #[gpui::test]
    fn an_explicit_same_value_override_survives_system_changes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            for (preference, value) in [
                (ReduceMotionPreference::Allow, false),
                (ReduceMotionPreference::Reduce, true),
            ] {
                set_reduce_motion_preference(ReduceMotionPreference::System, cx);
                apply_preference(Some(value), cx);
                set_reduce_motion_preference(preference, cx);
                apply_preference(Some(!value), cx);
                assert_eq!(cx.reduce_motion(), value);
                apply_preference(Some(value), cx);
                apply_preference(Some(!value), cx);
                assert_eq!(cx.reduce_motion(), value);
            }
        });
    }

    #[gpui::test]
    fn clearing_an_override_restores_the_latest_system_reading(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_preference(Some(false), cx);
            set_reduce_motion_preference(ReduceMotionPreference::Reduce, cx);
            apply_preference(Some(true), cx);
            apply_preference(Some(false), cx);
            assert!(cx.reduce_motion());
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(!cx.reduce_motion());
            apply_preference(Some(true), cx);
            assert!(cx.reduce_motion());

            set_reduce_motion_preference(ReduceMotionPreference::Allow, cx);
            apply_preference(Some(false), cx);
            apply_preference(Some(true), cx);
            assert!(!cx.reduce_motion());
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(cx.reduce_motion());
            apply_preference(Some(false), cx);
            assert!(!cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn an_override_before_the_first_reading_is_explicit_even_when_false(cx: &mut TestAppContext) {
        cx.update(|cx| {
            set_reduce_motion_preference(ReduceMotionPreference::Allow, cx);
            apply_preference(Some(true), cx);
            assert!(!cx.reduce_motion());
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn clearing_without_a_system_reading_keeps_the_flag_until_one_arrives(cx: &mut TestAppContext) {
        cx.update(|cx| {
            set_reduce_motion_preference(ReduceMotionPreference::Reduce, cx);
            apply_preference(None, cx);
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(cx.reduce_motion());
            apply_preference(Some(false), cx);
            assert!(!cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn clearing_can_resume_after_a_legacy_direct_override(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_preference(Some(false), cx);
            cx.set_reduce_motion(true);
            apply_preference(Some(false), cx);
            assert!(cx.reduce_motion());
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(!cx.reduce_motion());
            // Direct setters retain their previous value-comparison contract.
            cx.set_reduce_motion(true);
            apply_preference(Some(false), cx);
            assert!(cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn unknown_readings_preserve_the_last_known_system_preference(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_preference(Some(true), cx);
            set_reduce_motion_preference(ReduceMotionPreference::Allow, cx);
            apply_preference(None, cx);
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn the_application_can_change_its_explicit_override_without_yielding_ownership(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            apply_preference(Some(false), cx);
            set_reduce_motion_preference(ReduceMotionPreference::Reduce, cx);
            cx.set_reduce_motion(false);
            apply_preference(Some(true), cx);
            assert!(!cx.reduce_motion());
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn repeated_clears_and_later_readings_respect_the_current_owner(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_preference(Some(false), cx);
            set_reduce_motion_preference(ReduceMotionPreference::Reduce, cx);
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(!cx.reduce_motion());
            set_reduce_motion_preference(ReduceMotionPreference::Allow, cx);
            // A platform result arriving after the new choice must only cache.
            apply_preference(Some(true), cx);
            assert!(!cx.reduce_motion());
            set_reduce_motion_preference(ReduceMotionPreference::System, cx);
            assert!(cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn a_system_preference_for_reduced_motion_sets_the_flag(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_preference(Some(true), cx);
            assert!(cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn an_unknown_system_preference_leaves_the_flag_alone(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_preference(None, cx);
            assert!(!cx.reduce_motion());

            cx.set_reduce_motion(true);
            apply_preference(None, cx);
            assert!(cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn the_system_drives_the_flag_until_the_application_sets_it(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_preference(Some(true), cx);
            apply_preference(Some(false), cx);
            assert!(!cx.reduce_motion());
            apply_preference(Some(true), cx);
            assert!(cx.reduce_motion());

            cx.set_reduce_motion(false);
            apply_preference(Some(true), cx);
            assert!(!cx.reduce_motion());
        });
    }

    #[gpui::test]
    fn the_test_scheduler_is_never_asked_for_the_platform_preference(cx: &mut TestAppContext) {
        cx.update(|cx| {
            apply_system_reduce_motion(cx);
            assert!(!cx.reduce_motion());
        });
        cx.run_until_parked();
        cx.update(|cx| assert!(!cx.reduce_motion()));
    }

    #[gpui::test]
    fn a_flag_the_application_set_before_the_first_reading_is_kept(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_reduce_motion(true);
            apply_preference(Some(false), cx);
            assert!(cx.reduce_motion());
        });
    }
}

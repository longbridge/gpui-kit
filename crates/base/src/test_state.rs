//! Fluent, feature-independent configuration for headless UI observation.
use gpui::{Element, FocusHandle, Hitbox, InteractiveElement, SharedString};

/// Facts reported by a control to UI tests. Configure through `.test_state(...)`.
/// Reporting a fact does not alter the control's behavior.
#[cfg(feature = "test-support")]
#[derive(Default)]
pub struct TestState {
    pub(crate) checked: Option<bool>,
    pub(crate) indeterminate: Option<bool>,
    pub(crate) selected: Option<bool>,
    pub(crate) expanded: Option<bool>,
    pub(crate) value: Option<SharedString>,
    pub(crate) focus: Option<FocusHandle>,
    pub(crate) disabled: bool,
    pub(crate) text: Option<SharedString>,
}

/// Zero-sized configuration when UI testing is disabled.
#[cfg(not(feature = "test-support"))]
#[derive(Default)]
pub struct TestState {
    _private: (),
}

#[cfg_attr(not(feature = "test-support"), allow(unused_variables, unused_mut))]
impl TestState {
    pub fn checked(mut self, checked: bool) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.checked = Some(checked);
        }
        self
    }
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.indeterminate = Some(indeterminate);
        }
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.selected = Some(selected);
        }
        self
    }
    pub fn expanded(mut self, expanded: bool) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.expanded = Some(expanded);
        }
        self
    }
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.value = Some(value.into());
        }
        self
    }
    pub fn focus(mut self, focus: &FocusHandle) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.focus = Some(focus.clone());
        }
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.disabled = disabled;
        }
        self
    }
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        #[cfg(feature = "test-support")]
        {
            self.text = Some(text.into());
        }
        self
    }
    pub(crate) fn configure(self, configure: impl FnOnce(Self) -> Self) -> Self {
        #[cfg(feature = "test-support")]
        {
            configure(self)
        }
        #[cfg(not(feature = "test-support"))]
        {
            let _ = configure;
            self
        }
    }
}
impl gpui::prelude::FluentBuilder for TestState {}

#[doc(hidden)]
#[cfg(feature = "test-support")]
pub type TestStateElement<E> = crate::test_support::Observed<E>;
#[doc(hidden)]
#[cfg(not(feature = "test-support"))]
pub type TestStateElement<E> = E;

/// Adds test facts inline without splitting a native element's builder chain.
pub trait TestStateExt:
    Element<PrepaintState = Option<Hitbox>> + InteractiveElement + Sized
{
    /// Runs `configure` only with `test-support`. Otherwise returns the original
    /// element, without calling the closure or creating an observation wrapper.
    fn test_state(self, configure: impl FnOnce(TestState) -> TestState) -> TestStateElement<Self> {
        #[cfg(feature = "test-support")]
        {
            crate::test_support::Observed::new(self, TestState::default().configure(configure))
        }
        #[cfg(not(feature = "test-support"))]
        {
            let _ = configure;
            self
        }
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> TestStateExt for E {}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::div;
    use std::cell::Cell;

    #[test]
    fn configuration_runs_only_with_test_support() {
        let calls = Cell::new(0);
        let element = div().id("target").test_state(|test| {
            calls.set(calls.get() + 1);
            test.text("Target").checked(true)
        });
        assert_eq!(calls.get(), usize::from(cfg!(feature = "test-support")));
        #[cfg(not(feature = "test-support"))]
        {
            // Normal builds preserve the exact native type, not a no-op wrapper.
            let _: gpui::Stateful<gpui::Div> = element;
            assert_eq!(std::mem::size_of::<TestState>(), 0);
        }
        #[cfg(feature = "test-support")]
        assert_eq!(Element::id(&element), Some("target".into()));
    }

    #[test]
    fn component_configuration_skips_normal_builds_too() {
        let calls = Cell::new(0);
        let _ = crate::Button::new("button").test_state(|test| {
            calls.set(calls.get() + 1);
            test.text("Button")
        });
        let _ = crate::input::InputBase::new("input").test_state(|test| {
            calls.set(calls.get() + 1);
            test.value("Input")
        });
        let _ = crate::Select::new("select").test_state(|test| {
            calls.set(calls.get() + 1);
            test.value("Select")
        });
        assert_eq!(
            calls.get(),
            if cfg!(feature = "test-support") { 3 } else { 0 }
        );
    }
}

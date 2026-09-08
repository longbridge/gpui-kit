//! Registers native elements without accepting a second description of their state.
use gpui::{Element, Hitbox, InteractiveElement};

#[doc(hidden)]
#[cfg(feature = "test-support")]
pub type ObservedElement<E> = crate::test_support::Observed<E>;
#[doc(hidden)]
#[cfg(not(feature = "test-support"))]
pub type ObservedElement<E> = E;

/// Registers an existing element for UI queries. Its native accessibility
/// properties are read automatically; there are no test-only property setters.
///
/// Test-only values cannot override the native control:
/// ```compile_fail
/// use gpui::{div, prelude::*};
/// use gpui_base::TestSupportExt;
/// div().id("input").test_support().test_props(|props| props.value("invented"));
/// ```
pub trait TestSupportExt:
    Element<PrepaintState = Option<Hitbox>> + InteractiveElement + Sized
{
    /// With `test-support`, observes the element without adding a layout node.
    /// Otherwise returns the original element with its exact native type.
    /// Call before `track_focus` so the actual focus binding can be observed.
    /// Querying focus on a focus-capable element with a missed binding panics.
    fn test_support(self) -> ObservedElement<Self> {
        #[cfg(feature = "test-support")]
        {
            crate::test_support::Observed::new(self)
        }
        #[cfg(not(feature = "test-support"))]
        {
            self
        }
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> TestSupportExt for E {}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::div;

    #[test]
    fn observation_preserves_identity_and_native_type_in_normal_builds() {
        let element = div().id("target").test_support().test_support();
        assert_eq!(Element::id(&element), Some("target".into()));
        #[cfg(not(feature = "test-support"))]
        let _: gpui::Stateful<gpui::Div> = element;
    }
}

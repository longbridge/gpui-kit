//! Handing a typed native value from a child materializer to its parent.
//!
//! The shell materializes a child into an erased [`gpui::AnyElement`], but a
//! native parent such as `Menu`, `Settings`, or `Table` needs the concrete Rust
//! value its builder takes, not an element. [`Carrier`] is the one element type
//! that carries such a value through that erasure, and [`take`] is the only way
//! back out. Every family that needs typed children shares this, so the
//! contract — and the message a mismatch produces — is written once.

use gpui_shell::{
    anyhow, gpui,
    gpui::{
        Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement as _,
        LayoutId, Pixels, Window,
    },
};

/// An element that renders nothing and exists to carry `T` to its parent.
pub(super) struct Carrier<T: 'static>(Option<T>);

impl<T: 'static> Carrier<T> {
    pub(super) fn new(value: T) -> Self {
        Self(Some(value))
    }
}

impl<T: 'static> gpui::IntoElement for Carrier<T> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<T: 'static> Element for Carrier<T> {
    type RequestLayoutState = gpui::AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (LayoutId, gpui::AnyElement) {
        let mut element = gpui::div().into_any_element();
        let id = element.request_layout(window, cx);
        (id, element)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut gpui::AnyElement,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut gpui::AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        element.paint(window, cx);
    }
}

/// Takes the typed value a child carried, exactly once.
///
/// `name` names the child the parent expected, so a script that nests the
/// wrong component reads which one was wrong rather than a downcast failure.
pub(super) fn take<T: 'static>(element: &mut gpui::AnyElement, name: &str) -> anyhow::Result<T> {
    element
        .downcast_mut::<Carrier<T>>()
        .ok_or_else(|| anyhow::anyhow!("{name} materialized an incompatible child"))?
        .0
        .take()
        .ok_or_else(|| anyhow::anyhow!("{name} child was already consumed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_carrier_yields_its_value_once_and_rejects_the_wrong_type() {
        let mut element = Carrier::new(String::from("value")).into_any_element();
        assert_eq!(take::<String>(&mut element, "Part").unwrap(), "value");
        assert!(
            take::<String>(&mut element, "Part").is_err(),
            "a typed child belongs to exactly one parent"
        );

        let mut wrong = Carrier::new(42_u32).into_any_element();
        assert!(take::<String>(&mut wrong, "Part").is_err());
    }
}

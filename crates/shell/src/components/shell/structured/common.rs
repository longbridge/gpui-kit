pub(super) fn positive_usize(value: f64, label: &str) -> Result<usize, String> {
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 || value >= usize::MAX as f64 {
        return Err(format!(
            "{label} expects an exactly representable positive integer"
        ));
    }
    Ok(value as usize)
}

pub(super) fn positive_u16(value: f64, label: &str) -> Result<u16, String> {
    let value = positive_usize(value, label)?;
    u16::try_from(value)
        .map_err(|_| format!("{label} expects an integer no greater than {}", u16::MAX))
}

pub(super) fn nonnegative_f32(value: f64, label: &str) -> Result<f32, String> {
    if !value.is_finite() || value < 0.0 || value > f32::MAX as f64 {
        return Err(format!(
            "{label} expects a nonnegative finite number representable as f32"
        ));
    }
    Ok(value as f32)
}
use gpui::{
    Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, Pixels, Window,
};
use gpui_shell::{anyhow, gpui};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_conversions_reject_rounded_overflow_and_u16_overflow() {
        assert_eq!(positive_usize(1.0, "value").unwrap(), 1);
        assert!(positive_usize(usize::MAX as f64, "value").is_err());
        assert_eq!(positive_u16(u16::MAX as f64, "span").unwrap(), u16::MAX);
        assert!(positive_u16(u16::MAX as f64 + 1.0, "span").is_err());
    }

    #[test]
    fn f32_conversion_rejects_values_that_overflow_to_infinity() {
        assert_eq!(nonnegative_f32(42.5, "width").unwrap(), 42.5_f32);
        assert!(nonnegative_f32((f32::MAX as f64) * 2.0, "width").is_err());
        assert!(nonnegative_f32(-1.0, "width").is_err());
        assert!(nonnegative_f32(f64::NAN, "width").is_err());
    }
}

pub(super) fn take_element<T: gpui::IntoElement + 'static>(
    element: &mut gpui::AnyElement,
    name: &str,
) -> anyhow::Result<T> {
    element
        .downcast_mut::<TypedChildElement<T>>()
        .ok_or_else(|| anyhow::anyhow!("registered {name} materialized an incompatible element"))?
        .take()
        .ok_or_else(|| anyhow::anyhow!("registered {name} child was already consumed"))
}

pub(super) struct TypedChildElement<T: gpui::IntoElement + 'static> {
    value: Option<T>,
}

impl<T: gpui::IntoElement + 'static> TypedChildElement<T> {
    pub(super) fn new(value: T) -> Self {
        Self { value: Some(value) }
    }
    fn take(&mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T: gpui::IntoElement + 'static> gpui::IntoElement for TypedChildElement<T> {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl<T: gpui::IntoElement + 'static> Element for TypedChildElement<T> {
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
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut element = self
            .take()
            .expect("typed child can request layout only once")
            .into_any_element();
        let layout = element.request_layout(window, cx);
        (layout, element)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
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
        element: &mut Self::RequestLayoutState,
        _: &mut (),
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        element.paint(window, cx);
    }
}

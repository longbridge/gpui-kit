mod band;
mod linear;
mod ordinal;
mod point;
mod sealed;

pub use band::ScaleBand;
pub use linear::ScaleLinear;
pub use ordinal::ScaleOrdinal;
pub use point::ScalePoint;
/// The value types a linear scale and the charts built on it accept: `f64`,
/// and `rust_decimal::Decimal` with the `decimal` feature. Not meant to be
/// implemented outside GPUI Kit.
#[doc(hidden)]
pub use sealed::Sealed;

pub trait Scale<T> {
    /// Get the tick of the scale.
    fn tick(&self, value: &T) -> Option<f32>;

    /// Get the least index of the scale.
    fn least_index(&self, _tick: f32) -> usize {
        0
    }

    /// Get the least index of the scale with the domain.
    fn least_index_with_domain(&self, _tick: f32, _domain: &[T]) -> (usize, f32) {
        (0, 0.)
    }
}

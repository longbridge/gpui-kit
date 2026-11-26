use std::rc::Rc;

/// How a slider maps its position to a value.
///
/// [`ValueMapping::Linear`] and [`ValueMapping::Logarithmic`] are the built-in
/// mappings. [`ValueMapping::Custom`] takes a pair of inverse functions when a
/// parameter follows its own curve.
///
/// The functions must be monotonic and inverse to each other. Inputs are
/// clamped to `0.0..=1.0` and `min..=max`, and a non-finite result falls back
/// to the low end.
#[derive(Clone, Default)]
pub enum ValueMapping {
    /// Values change uniformly across the slider range.
    #[default]
    Linear,
    /// The distance between values grows exponentially; `min` must be greater
    /// than zero.
    ///
    /// This is useful for parameters with a large range where smaller changes
    /// are more significant at lower values, such as volume, frequency, and
    /// zoom.
    ///
    /// Moving the slider 1/3 of the way yields ~10, 2/3 yields ~100, and the
    /// full range covers three orders of magnitude evenly.
    Logarithmic,
    /// A caller-supplied pair of inverse mappings between a value and its
    /// fraction of the range.
    Custom {
        /// Maps a value inside `min..=max` to `0.0..=1.0`.
        to_normalized: Rc<dyn Fn(f32) -> f32>,
        /// Maps `0.0..=1.0` back to a value inside `min..=max`.
        from_normalized: Rc<dyn Fn(f32) -> f32>,
    },
}

impl ValueMapping {
    /// Creates a custom mapping from a pair of inverse functions.
    ///
    /// `to_normalized` maps a value to its fraction of the range, and
    /// `from_normalized` maps that fraction back to a value.
    pub fn custom(
        to_normalized: impl Fn(f32) -> f32 + 'static,
        from_normalized: impl Fn(f32) -> f32 + 'static,
    ) -> Self {
        Self::Custom {
            to_normalized: Rc::new(to_normalized),
            from_normalized: Rc::new(from_normalized),
        }
    }

    /// Whether this is the built-in linear mapping.
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::Linear)
    }

    /// Whether this is the built-in logarithmic mapping.
    #[inline]
    pub fn is_logarithmic(&self) -> bool {
        matches!(self, Self::Logarithmic)
    }
}

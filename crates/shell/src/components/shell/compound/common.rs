pub(super) fn nonempty_id(id: &str, component: &str) -> Result<String, String> {
    if id.trim().is_empty() {
        return Err(format!("{component}(id) expects a nonempty string id"));
    }
    Ok(id.to_owned())
}

pub(super) fn nonnegative_usize(value: f64, label: &str) -> Result<usize, String> {
    // On 64-bit targets `usize::MAX as f64` rounds to 2^64, hence the
    // deliberately exclusive upper bound.
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= usize::MAX as f64 {
        return Err(format!(
            "{label} expects an exactly representable nonnegative integer"
        ));
    }
    Ok(value as usize)
}

pub(super) fn finite_f32(value: f64, label: &str) -> Result<f32, String> {
    if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
        return Err(format!(
            "{label} expects a finite number representable as f32"
        ));
    }
    Ok(value as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usize_conversion_rejects_fractional_negative_and_overflow_values() {
        assert_eq!(nonnegative_usize(0.0, "value").unwrap(), 0);
        assert_eq!(nonnegative_usize(42.0, "value").unwrap(), 42);
        assert!(nonnegative_usize(-1.0, "value").is_err());
        assert!(nonnegative_usize(1.5, "value").is_err());
        assert!(nonnegative_usize(f64::INFINITY, "value").is_err());
        assert!(nonnegative_usize(usize::MAX as f64, "value").is_err());
    }

    #[test]
    fn f32_conversion_rejects_values_that_would_become_infinite() {
        assert_eq!(finite_f32(42.5, "value").unwrap(), 42.5_f32);
        assert!(finite_f32((f32::MAX as f64) * 2.0, "value").is_err());
        assert!(finite_f32((f32::MIN as f64) * 2.0, "value").is_err());
        assert!(finite_f32(f64::NAN, "value").is_err());
    }

    #[test]
    fn ids_must_contain_non_whitespace_text() {
        assert!(nonempty_id("", "Widget").is_err());
        assert!(nonempty_id("  \t", "Widget").is_err());
        assert_eq!(nonempty_id("widget-1", "Widget").unwrap(), "widget-1");
    }
}

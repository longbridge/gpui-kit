pub trait Sealed {}

impl Sealed for f64 {}

#[cfg(feature = "decimal")]
impl Sealed for rust_decimal::Decimal {}

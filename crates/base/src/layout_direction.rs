/// The horizontal direction used to interpret layout and navigation order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LayoutDirection {
    /// Left-to-right order.
    #[default]
    Ltr,
    /// Right-to-left order.
    Rtl,
}

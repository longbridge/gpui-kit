//! One module per bound `gpui-base` component.
//!
//! A submodule rather than more arms in [`materialize_node`](super::materialize_node)
//! because the arms are the part that grows: every component the shell binds
//! adds one, and a match that reaches a few hundred lines stops being readable
//! at exactly the point it matters most. Each file here holds one component's
//! whole translation — the builder calls, the callbacks it wires, and the
//! states it cannot honour — so a reader looking for `Tabs` opens `tabs.rs`
//! rather than scrolling.
//!
//! Being a child of `materialize` is what makes that free: a descendant module
//! reaches its ancestors' private items, so [`Behavior`](super::Behavior),
//! [`finish`](super::finish) and the rest stay private to the render path while
//! still being usable from here. Nothing in this directory is published, and
//! nothing outside `materialize` may name it.
//!
//! Each module exposes exactly one `pub(super) fn materialize` taking the same
//! shape `materialize_node` already resolved — the node's id, its refinement,
//! its behavior, its state styles and its children — and returning the finished
//! element. The dispatch stays in `materialize_node`, so the description of
//! *which* component this is never leaves the one place that knows.

pub(super) mod accordion;
pub(super) mod avatar;
pub(super) mod collapsible;
pub(super) mod dock;
pub(super) mod fps;
pub(super) mod group;
pub(super) mod list;
pub(super) mod number_input;
pub(super) mod otp_input;
pub(super) mod pagination;
pub(super) mod popover;
pub(super) mod progress;
pub(super) mod radio;
pub(super) mod resizable;
pub(super) mod scrollbar;
pub(super) mod select;
pub(super) mod slider;
pub(super) mod table;
pub(super) mod tabs;
pub(super) mod textarea;
pub(super) mod toggle;
pub(super) mod tooltip;
pub(super) mod virtual_list;

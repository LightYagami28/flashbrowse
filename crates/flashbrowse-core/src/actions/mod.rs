//! Action executors (`click`, `fill`, `scroll`).
//! `select` (combobox/dropdown) sets the option value via
//! `DOM.resolveNode` + `Runtime.callFunctionOn` (no native popup).

pub mod click;
pub mod fill;
pub mod keys;
pub mod scroll;
pub mod select;

pub use click::box_center;
pub use scroll::ScrollDirection;

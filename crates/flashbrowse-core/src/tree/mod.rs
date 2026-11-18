//! AXTree compression pipeline (`compress` → `render`).

pub mod compressor;
pub mod formatter;
pub mod node;

pub use compressor::compress;
pub use formatter::render;
pub use node::{ActionMap, SemanticNode};

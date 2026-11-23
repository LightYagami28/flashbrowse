//! AXTree compression pipeline (`compress` → `render`).

pub mod compressor;
pub mod formatter;
pub mod node;

pub use compressor::{compress, compress_with_options, CompressOptions};
pub use formatter::{render, render_nested};
pub use node::{ActionMap, SemanticNode};

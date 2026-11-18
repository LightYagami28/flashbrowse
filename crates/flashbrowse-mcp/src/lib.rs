//! `flashbrowse-mcp`: MCP JSON-RPC 2.0 stdio server (2025-06-18, fallback
//! 2025-03-26 → 2024-11-05).
//!
//! ```text
//! serve_stdio() ──▶ stdin lines ──▶ dispatch (initialize / tools/*)
//!                                        └──▶ flashbrowse_core::BrowserSession
//! stdout ◀── one JSON-RPC frame per line, pure JSON (never logs)
//! stderr ◀── tracing (stderr-only subscriber)
//! ```

pub mod protocol;
pub mod server;
pub mod tools;

pub use protocol::{McpError, ToolResult};
pub use server::{serve, serve_stdio};
pub use tools::{call_tool, tool_definitions, SessionHandle};

/// Crate name (kept for scaffold parity with pre-Phase-3 `lib.rs`).
pub const CRATE_NAME: &str = "flashbrowse-mcp";

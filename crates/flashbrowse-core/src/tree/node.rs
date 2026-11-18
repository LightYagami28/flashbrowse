//! Actionable semantic node representation.
//!
//! The compressor converts Chromium `AXNode`s into `SemanticNode`s - one
//! flat output line per node. `action_id` is `Some` only for elements that
//! can be clicked/filled (they carry a `backend_node_id` in the `ActionMap`).

use std::collections::HashMap;

/// Action handle: interactive element ID → CDP `BackendNodeId`.
pub type ActionMap = HashMap<u32, i64>;

/// One line of the compressed action graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticNode {
    /// Sequential `[n]` ID when `actionable`, else `None`.
    pub action_id: Option<u32>,
    /// Normalized role, e.g. `link`, `button`, `textbox`, `heading`.
    pub role: String,
    /// Accessible name (may be empty for decorative content).
    pub name: String,
    /// Extra annotations: `disabled`, `checked`, `value=…` (sorted, deduped).
    pub extras: Vec<String>,
    /// Heading level (`Some(1..=6)`) when `role == "heading"`.
    pub level: Option<u32>,
    /// Original AXTree `nodeId` (diagnostics).
    pub node_id: String,
}

impl SemanticNode {
    /// `true` when this node consumes an action ID.
    pub fn actionable(&self) -> bool {
        self.action_id.is_some()
    }
}

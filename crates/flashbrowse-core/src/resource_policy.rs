//! Network resource load policy for a browser session.
//!
//! The compact AX tree is unchanged across policies; only what Chrome fetches
//! differs (see [`crate::interceptor::blocked_patterns`]).

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// How aggressively to block decorative or heavy resources while navigating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourcePolicy {
    /// Fonts, images, and trackers blocked (legacy default).
    #[default]
    AgentDefault,
    /// Agent default plus audio/video.
    TextOnly,
    /// Trackers only: faithful paint for screenshots without a second tool.
    FullRender,
}

impl ResourcePolicy {
    /// CLI / MCP string form.
    pub fn as_str(self) -> &'static str {
        match self {
            ResourcePolicy::AgentDefault => "agent-default",
            ResourcePolicy::TextOnly => "text-only",
            ResourcePolicy::FullRender => "full-render",
        }
    }

    /// Backward compat for `--text-only` and persisted `text_only: true`.
    pub fn from_text_only(text_only: bool) -> Self {
        if text_only {
            ResourcePolicy::TextOnly
        } else {
            ResourcePolicy::AgentDefault
        }
    }
}

impl FromStr for ResourcePolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "agent-default" | "agent_default" | "default" => Ok(ResourcePolicy::AgentDefault),
            "text-only" | "text_only" => Ok(ResourcePolicy::TextOnly),
            "full-render" | "full_render" | "full" => Ok(ResourcePolicy::FullRender),
            other => Err(format!(
                "unknown resource policy '{other}' (agent-default|text-only|full-render)"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResourcePolicy;

    #[test]
    fn parses_aliases() {
        assert_eq!(
            "full_render".parse::<ResourcePolicy>().unwrap(),
            ResourcePolicy::FullRender
        );
    }
}

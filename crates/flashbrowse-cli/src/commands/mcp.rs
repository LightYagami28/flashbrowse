//! `flashbrowse mcp` - MCP stdio server for AI agents.

use std::path::PathBuf;

use flashbrowse_core::{ResourcePolicy, SessionConfig};

/// Run the MCP server on stdio until stdin closes.
pub async fn run(
    chrome_path: Option<PathBuf>,
    resource_policy: ResourcePolicy,
) -> anyhow::Result<()> {
    let config = SessionConfig {
        chrome_path,
        resource_policy,
        ..Default::default()
    };
    flashbrowse_mcp::serve_stdio(config)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

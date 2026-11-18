//! `Network` domain types (enable, setBlockedURLs, setExtraHTTPHeaders).

use serde::Serialize;

use crate::error::CdpError;
use crate::transport::CdpClient;

/// Params for `Network.setBlockedURLs`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBlockedURLsParams {
    pub urls: Vec<String>,
}

/// `Network.enable`.
pub async fn enable(client: &CdpClient, session_id: &str) -> Result<(), CdpError> {
    let _: serde_json::Value = client
        .send(Some(session_id), "Network.enable", serde_json::json!({}))
        .await?;
    Ok(())
}

/// `Network.domain` enable without `sessionId` (browser-level)? Not used -
/// session enable is sufficient for flattened tabs.
pub async fn set_blocked_urls(
    client: &CdpClient,
    session_id: &str,
    urls: Vec<String>,
) -> Result<(), CdpError> {
    let _: serde_json::Value = client
        .send_typed(
            Some(session_id),
            "Network.setBlockedURLs",
            &SetBlockedURLsParams { urls },
        )
        .await?;
    Ok(())
}

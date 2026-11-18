//! `Browser` domain types.

use serde::{Deserialize, Serialize};

use crate::error::CdpError;
use crate::transport::CdpClient;

/// Empty params for `Browser.getVersion`.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionParams {}

/// Response of `Browser.getVersion`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionResponse {
    pub protocol_version: String,
    pub product: String,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub js_version: Option<String>,
}

/// `Browser.getVersion` - verifies handshake against a fresh Chromium.
pub async fn get_version(client: &CdpClient) -> Result<GetVersionResponse, CdpError> {
    client
        .send_typed(None, "Browser.getVersion", &GetVersionParams::default())
        .await
}

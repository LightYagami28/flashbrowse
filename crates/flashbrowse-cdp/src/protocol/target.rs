//! `Target` domain types (createTarget, attachToTarget).

use serde::{Deserialize, Serialize};

use crate::error::CdpError;
use crate::transport::CdpClient;

/// Params for `Target.createTarget`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetParams {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,
}

/// Response of `Target.createTarget`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetResponse {
    pub target_id: String,
}

/// Params for `Target.attachToTarget`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetParams {
    pub target_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten: Option<bool>,
}

/// Response of `Target.attachToTarget`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetResponse {
    pub session_id: String,
}

/// Create a new target (tab) at `url`.
pub async fn create_target(
    client: &CdpClient,
    url: &str,
) -> Result<CreateTargetResponse, CdpError> {
    client
        .send_typed(
            None,
            "Target.createTarget",
            &CreateTargetParams {
                url: url.to_string(),
                headless: None,
            },
        )
        .await
}

/// Attach to a target in flatten mode. Returns `sessionId` for all page-level calls.
pub async fn attach_to_target(
    client: &CdpClient,
    target_id: &str,
) -> Result<AttachToTargetResponse, CdpError> {
    client
        .send_typed(
            None,
            "Target.attachToTarget",
            &AttachToTargetParams {
                target_id: target_id.to_string(),
                flatten: Some(true),
            },
        )
        .await
}

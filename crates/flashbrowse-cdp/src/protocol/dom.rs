//! `DOM` domain types (getBoxModel, scrollIntoViewIfNeeded, resolveNode).

use serde::{Deserialize, Serialize};

use crate::error::CdpError;
use crate::transport::CdpClient;

/// Params for `DOM.getBoxModel`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelParams {
    pub backend_node_id: i64,
}

/// Box geometry returned by `DOM.getBoxModel`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    pub content: Vec<f64>,
    pub border: Vec<f64>,
    pub margin: Vec<f64>,
    pub padding: Vec<f64>,
    pub width: f64,
    pub height: f64,
}

/// Response of `DOM.getBoxModel`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelResponse {
    pub model: BoxModel,
}

/// Params for `DOM.scrollIntoViewIfNeeded`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollIntoViewParams {
    pub backend_node_id: i64,
}

/// Params for `DOM.resolveNode` (backend node id -> JS remote object id).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeParams {
    pub backend_node_id: i64,
}

/// Response of `DOM.resolveNode`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeResponse {
    pub object: RemoteObject,
}

/// Remote object handle.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    #[serde(default)]
    pub object_id: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
}

/// `DOM.getBoxModel` - bounding-box coordinates for synthetic input.
pub async fn get_box_model(
    client: &CdpClient,
    session_id: &str,
    backend_node_id: i64,
) -> Result<GetBoxModelResponse, CdpError> {
    client
        .send_typed(
            Some(session_id),
            "DOM.getBoxModel",
            &GetBoxModelParams { backend_node_id },
        )
        .await
}

/// `DOM.scrollIntoViewIfNeeded`.
///
/// Propagates CDP errors to the caller. Callers that treat scroll as
/// best-effort (e.g. before `getBoxModel`) should log and proceed.
pub async fn scroll_into_view_if_needed(
    client: &CdpClient,
    session_id: &str,
    backend_node_id: i64,
) -> Result<(), CdpError> {
    let _: serde_json::Value = client
        .send_typed(
            Some(session_id),
            "DOM.scrollIntoViewIfNeeded",
            &ScrollIntoViewParams { backend_node_id },
        )
        .await?;
    Ok(())
}

/// `DOM.resolveNode` - turn a backend node id into a JS object handle.
pub async fn resolve_node(
    client: &CdpClient,
    session_id: &str,
    backend_node_id: i64,
) -> Result<ResolveNodeResponse, CdpError> {
    client
        .send_typed(
            Some(session_id),
            "DOM.resolveNode",
            &ResolveNodeParams { backend_node_id },
        )
        .await
}

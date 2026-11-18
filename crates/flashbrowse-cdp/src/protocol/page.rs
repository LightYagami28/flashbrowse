//! `Page` domain types (navigate, addScriptToEvaluateOnNewDocument, lifecycle).

use serde::{Deserialize, Serialize};

use crate::error::CdpError;
use crate::transport::CdpClient;

/// Params for `Page.navigate`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateParams {
    pub url: String,
}

/// Response of `Page.navigate`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateResponse {
    pub frame_id: String,
    #[serde(default)]
    pub loader_id: Option<String>,
}

/// Params for `Page.addScriptToEvaluateOnNewDocument`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptParams {
    pub source: String,
}

/// Response of `Page.addScriptToEvaluateOnNewDocument`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptResponse {
    #[serde(default)]
    pub identifier: Option<String>,
}

/// Params for `Page.captureScreenshot`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_beyond_viewport: Option<bool>,
}

/// Response of `Page.captureScreenshot` (base64-encoded PNG when format=png).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotResponse {
    pub data: String,
}

/// `Page.navigate` within a flattened session.
pub async fn navigate(
    client: &CdpClient,
    session_id: &str,
    url: &str,
) -> Result<NavigateResponse, CdpError> {
    client
        .send_typed(
            Some(session_id),
            "Page.navigate",
            &NavigateParams {
                url: url.to_string(),
            },
        )
        .await
}

/// `Page.addScriptToEvaluateOnNewDocument` (stealth injection before page scripts).
pub async fn add_script_on_new_document(
    client: &CdpClient,
    session_id: &str,
    source: &str,
) -> Result<AddScriptResponse, CdpError> {
    client
        .send_typed(
            Some(session_id),
            "Page.addScriptToEvaluateOnNewDocument",
            &AddScriptParams {
                source: source.to_string(),
            },
        )
        .await
}

/// `Page.enable`.
pub async fn enable(client: &CdpClient, session_id: &str) -> Result<(), CdpError> {
    let _: serde_json::Value = client
        .send(Some(session_id), "Page.enable", serde_json::json!({}))
        .await?;
    // Lifecycle events are opt-in: without this, `Page.lifecycleEvent` never
    // fires and every navigation would wait for the full timeout.
    let _: serde_json::Value = client
        .send(
            Some(session_id),
            "Page.setLifecycleEventsEnabled",
            serde_json::json!({ "enabled": true }),
        )
        .await?;
    Ok(())
}

/// `Page.captureScreenshot`.
pub async fn capture_screenshot(
    client: &CdpClient,
    session_id: &str,
    full_page: bool,
) -> Result<CaptureScreenshotResponse, CdpError> {
    client
        .send_typed(
            Some(session_id),
            "Page.captureScreenshot",
            &CaptureScreenshotParams {
                format: Some("png".to_string()),
                capture_beyond_viewport: Some(full_page),
            },
        )
        .await
}

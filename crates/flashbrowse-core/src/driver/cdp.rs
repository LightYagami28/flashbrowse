//! Chromium CDP implementation of browser primitives used by [`BrowserSession`](crate::BrowserSession).

use std::sync::Arc;

use flashbrowse_cdp::protocol::{accessibility, network, page, runtime};
use flashbrowse_cdp::CdpClient;
use serde_json::Value;

use crate::interceptor;
use crate::resource_policy::ResourcePolicy;
use crate::CoreError;

/// CDP-backed browser operations for one flattened page target.
#[derive(Clone)]
pub struct CdpDriver {
    client: Arc<CdpClient>,
    session_id: String,
}

impl CdpDriver {
    pub fn new(client: Arc<CdpClient>, session_id: String) -> Self {
        Self { client, session_id }
    }

    pub fn client(&self) -> Arc<CdpClient> {
        self.client.clone()
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub async fn set_resource_policy(&self, policy: ResourcePolicy) -> Result<(), CoreError> {
        network::set_blocked_urls(
            &self.client,
            &self.session_id,
            interceptor::blocked_patterns(policy),
        )
        .await?;
        Ok(())
    }

    pub async fn set_blocklist(&self, patterns: Vec<String>) -> Result<(), CoreError> {
        network::set_blocked_urls(&self.client, &self.session_id, patterns).await?;
        Ok(())
    }

    pub async fn evaluate(&self, expression: &str) -> Result<Value, CoreError> {
        let r = runtime::evaluate(&self.client, &self.session_id, expression).await?;
        Ok(r.result.value.unwrap_or(Value::Null))
    }

    pub async fn get_full_ax_tree(&self) -> Result<Vec<accessibility::AXNode>, CoreError> {
        let resp = accessibility::get_full_ax_tree(&self.client, &self.session_id).await?;
        Ok(resp.nodes)
    }

    pub async fn capture_screenshot(&self, full_page: bool) -> Result<String, CoreError> {
        let resp = page::capture_screenshot(&self.client, &self.session_id, full_page).await?;
        Ok(resp.data)
    }
}

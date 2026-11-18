//! Verifies raw WebSocket connection to headless Chrome.
//! Requires a system Chromium binary - skipped if none is found.

use flashbrowse_cdp::{
    protocol::{browser, page, target},
    CdpClient, LaunchOptions,
};

#[tokio::test]
async fn test_cdp_connection_and_get_version() {
    // Skip gracefully when Chrome is absent (CI without browser).
    if flashbrowse_cdp::find_chrome(None).is_err() {
        eprintln!("SKIP: no Chromium binary found");
        return;
    }
    let chrome = flashbrowse_cdp::launch_chrome(LaunchOptions::default())
        .await
        .expect("launch chrome");

    let ws = chrome.browser_ws_url().to_string();
    assert!(ws.starts_with("ws://127.0.0.1:"), "ws url: {ws}");
    assert!(ws.contains("/devtools/browser/"));

    let client = CdpClient::connect(&ws).await.expect("connect");

    // Browser.getVersion - proves handshake + protocolVersion + product.
    let ver = browser::get_version(&client).await.expect("getVersion");
    assert!(!ver.protocol_version.is_empty(), "protocolVersion empty");
    assert!(!ver.product.is_empty(), "product empty");

    // Flatten-mode session: createTarget → attachToTarget → Page.navigate.
    let t = target::create_target(&client, "about:blank")
        .await
        .expect("createTarget");
    let s = target::attach_to_target(&client, &t.target_id)
        .await
        .expect("attachToTarget");
    assert!(!s.session_id.is_empty());

    page::enable(&client, &s.session_id)
        .await
        .expect("Page.enable");
    let nav = page::navigate(&client, &s.session_id, "about:blank")
        .await
        .expect("Page.navigate");
    assert!(!nav.frame_id.is_empty());
}

//! Validates stealth evasions against a live headless instance.
//! Requires system Chromium - skipped when absent.

use flashbrowse_cdp::protocol::runtime;
use flashbrowse_core::{BrowserSession, SessionConfig};

#[tokio::test]
async fn test_stealth_fingerprint() {
    if flashbrowse_cdp::find_chrome(None).is_err() {
        eprintln!("SKIP: no Chromium binary found");
        return;
    }

    let mut session = BrowserSession::launch(SessionConfig::default())
        .await
        .expect("launch session");
    session
        .open("data:text/html,<html><body><h1>stealth</h1></body></html>")
        .await
        .expect("open data url");

    let sid = session.session_id().to_string();
    let client = session.client();

    // navigator.webdriver === undefined
    let webdriver = runtime::evaluate(&client, &sid, "(navigator.webdriver === undefined)")
        .await
        .expect("evaluate")
        .result
        .value;
    assert_eq!(
        webdriver,
        Some(serde_json::Value::Bool(true)),
        "navigator.webdriver should be undefined"
    );

    // window.chrome present with stubs
    let chrome = runtime::evaluate(
        &client,
        &sid,
        "window.chrome && typeof window.chrome === 'object'",
    )
    .await
    .expect("evaluate")
    .result
    .value;
    assert_eq!(
        chrome,
        Some(serde_json::Value::Bool(true)),
        "window.chrome stub missing"
    );

    // plugins.length > 0
    let plugins = runtime::evaluate(&client, &sid, "(navigator.plugins.length > 0)")
        .await
        .expect("evaluate")
        .result
        .value;
    assert_eq!(
        plugins,
        Some(serde_json::Value::Bool(true)),
        "navigator.plugins should be mocked non-empty"
    );

    session.shutdown().await;
}

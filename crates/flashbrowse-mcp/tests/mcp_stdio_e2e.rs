//! End-to-end MCP JSON-RPC validation over duplex pipes.
//! Drives `serve()` in-process (no binary needed until Phase 4 CLI wiring).
//! Requires system Chromium - skipped when absent.

use flashbrowse_core::SessionConfig;
use flashbrowse_mcp::server::serve;
use serde_json::Value;
use tokio::io::{duplex, AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Send one JSON-RPC line and await the next response line as JSON.
async fn roundtrip<W, R>(client_out: &mut W, client_lines: &mut R, frame: Value) -> Value
where
    W: AsyncWriteExt + Unpin,
    R: AsyncBufReadExt + Unpin,
{
    client_out
        .write_all(frame.to_string().as_bytes())
        .await
        .unwrap();
    client_out.write_all(b"\n").await.unwrap();
    client_out.flush().await.unwrap();
    let mut line = String::new();
    client_lines.read_line(&mut line).await.unwrap();
    assert!(!line.is_empty(), "empty response (EOF?)");
    // Stdout purity: every line must be valid JSON.
    let v: Value = serde_json::from_str(line.trim()).expect("stdout line must be JSON");
    assert_eq!(v.get("jsonrpc").and_then(|x| x.as_str()), Some("2.0"));
    v
}

#[tokio::test]
async fn test_mcp_stdio_e2e() {
    if flashbrowse_cdp::find_chrome(None).is_err() {
        eprintln!("SKIP: no Chromium binary found");
        return;
    }

    let (server_in, mut client_out) = duplex(64 * 1024);
    let (server_out, client_in) = duplex(64 * 1024);
    let mut client_lines = BufReader::new(client_in);

    let server = tokio::spawn(async move {
        serve(server_in, server_out, SessionConfig::default())
            .await
            .expect("serve")
    });

    // initialize
    let resp = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "e2e"}} }),
    )
    .await;
    assert!(resp.get("error").is_none(), "init error: {resp}");
    let result = resp.get("result").expect("init result");
    assert!(
        result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .is_some(),
        "missing protocolVersion: {result}"
    );
    assert_eq!(
        result
            .get("serverInfo")
            .and_then(|s| s.get("name"))
            .and_then(|v| v.as_str()),
        Some("flashbrowse")
    );

    // tools/list
    let resp = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    )
    .await;
    let tools = resp
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools list");
    assert_eq!(tools.len(), 4);
    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    for want in [
        "browser_open",
        "browser_act",
        "browser_extract_markdown",
        "browser_screenshot",
    ] {
        assert!(names.contains(&want.to_string()), "missing tool {want}");
    }

    // browser_open → numbered tree contains [1] Button: "Submit"
    let url = "data:text/html,<html><body><button>Submit</button></body></html>";
    let resp = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "browser_open", "arguments": {"url": url}}}),
    )
    .await;
    let content = tool_text(&resp);
    assert!(
        content.contains("[1] Button: \"Submit\""),
        "browser_open tree missing [1] Button: {content}"
    );

    // browser_act click [1] → success
    let resp = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "browser_act", "arguments": {"action_id": 1, "action": "click"}}}),
    )
    .await;
    assert_tool_ok(&resp);

    // browser_act hover + scroll + press_key should also succeed (no error).
    for (id, action, value) in [
        (5, "hover", serde_json::json!(null)),
        (6, "scroll", serde_json::json!("down")),
        (7, "press_key", serde_json::json!("Enter")),
    ] {
        let args = serde_json::json!({"action_id": 1, "action": action, "value": value});
        let resp = roundtrip(
            &mut client_out,
            &mut client_lines,
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": "browser_act", "arguments": args}}),
        )
        .await;
        assert_tool_ok(&resp);
    }

    // browser_extract_markdown → text
    let resp = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": {"name": "browser_extract_markdown", "arguments": {}}}),
    )
    .await;
    assert_tool_ok(&resp);

    // browser_screenshot → image content block
    let resp = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": "browser_screenshot", "arguments": {"full_page": false}}}),
    )
    .await;
    let first = resp
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .expect("image content");
    assert_eq!(
        first.get("type").and_then(|v| v.as_str()),
        Some("image"),
        "screenshot should be image content: {first}"
    );

    // ping → result
    let resp = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc": "2.0", "id": 10, "method": "ping", "params": {}}),
    )
    .await;
    assert!(resp.get("error").is_none());

    // Drop client → server loop EOF → clean teardown.
    drop(client_out);
    drop(client_lines);
    server.await.expect("server task");
}

fn tool_text(resp: &Value) -> String {
    resp.get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn assert_tool_ok(resp: &Value) {
    assert!(resp.get("error").is_none(), "JSON-RPC error: {resp}");
    let is_error = resp
        .get("result")
        .and_then(|r| r.get("isError"))
        .and_then(|v| v.as_bool());
    assert_ne!(
        is_error,
        Some(true),
        "tool returned isError: {}",
        tool_text(resp)
    );
}

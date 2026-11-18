//! MCP throughput stress: 50 sequential tools/call over one server loop.
//! Run: cargo test -p flashbrowse-mcp --test stress_mcp -- --nocapture

use flashbrowse_core::SessionConfig;
use flashbrowse_mcp::server::serve;
use serde_json::Value;
use std::time::Instant;
use tokio::io::{duplex, AsyncBufReadExt, AsyncWriteExt, BufReader};

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
    serde_json::from_str(line.trim()).expect("pure JSON stdout")
}

#[tokio::test]
async fn stress_mcp_x50() {
    if flashbrowse_cdp::find_chrome(None).is_err() {
        println!("SKIP: no Chrome");
        return;
    }
    let (server_in, mut client_out) = duplex(256 * 1024);
    let (server_out, client_in) = duplex(256 * 1024);
    let mut client_lines = BufReader::new(client_in);
    let server = tokio::spawn(async move {
        serve(server_in, server_out, SessionConfig::default())
            .await
            .expect("serve")
    });

    let t0 = Instant::now();
    let _ = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"stress"}}}),
    )
    .await;

    let url = "data:text/html,<html><body><button>A</button><input aria-label='x'/><button>B</button></body></html>";
    let open = roundtrip(&mut client_out, &mut client_lines,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"browser_open","arguments":{"url":url}}})).await;
    assert!(
        serde_json::to_string(&open).unwrap().contains("[1]"),
        "open ok"
    );

    // 40x browser_act (click/fill/scroll/press_key cycle) + markdown + screenshot.
    for i in 0..40 {
        let (action, value) = match i % 4 {
            0 => ("click", serde_json::json!(null)),
            1 => ("fill", serde_json::json!("hello")),
            2 => ("scroll", serde_json::json!("down")),
            _ => ("press_key", serde_json::json!("Enter")),
        };
        let resp = roundtrip(&mut client_out, &mut client_lines,
            serde_json::json!({"jsonrpc":"2.0","id": 100+i,"method":"tools/call","params":{"name":"browser_act","arguments":{"action_id":1,"action":action,"value":value}}})).await;
        assert!(resp.get("error").is_none(), "act {i} failed: {resp}");
        let is_err = resp
            .get("result")
            .and_then(|r| r.get("isError"))
            .and_then(|v| v.as_bool());
        assert_ne!(is_err, Some(true), "act {i} isError");
    }
    let md = roundtrip(&mut client_out, &mut client_lines,
        serde_json::json!({"jsonrpc":"2.0","id":900,"method":"tools/call","params":{"name":"browser_extract_markdown","arguments":{}}})).await;
    assert!(md.get("error").is_none());
    let shot = roundtrip(&mut client_out, &mut client_lines,
        serde_json::json!({"jsonrpc":"2.0","id":901,"method":"tools/call","params":{"name":"browser_screenshot","arguments":{"full_page":false}}})).await;
    assert!(
        serde_json::to_string(&shot).unwrap().contains("\"image\""),
        "screenshot image"
    );

    // Unknown tool + bad id must be tool-errors, not server deaths.
    let bad = roundtrip(&mut client_out, &mut client_lines,
        serde_json::json!({"jsonrpc":"2.0","id":902,"method":"tools/call","params":{"name":"nope_nope","arguments":{}}})).await;
    assert!(
        bad.get("error").is_some(),
        "unknown tool should be JSON-RPC error"
    );
    let bad_id = roundtrip(&mut client_out, &mut client_lines,
        serde_json::json!({"jsonrpc":"2.0","id":903,"method":"tools/call","params":{"name":"browser_act","arguments":{"action_id":999999,"action":"click"}}})).await;
    assert_eq!(
        bad_id
            .get("result")
            .and_then(|r| r.get("isError"))
            .and_then(|v| v.as_bool()),
        Some(true),
        "bad id -> isError"
    );

    // Still alive after error storm.
    let ok = roundtrip(
        &mut client_out,
        &mut client_lines,
        serde_json::json!({"jsonrpc":"2.0","id":904,"method":"ping","params":{}}),
    )
    .await;
    assert!(ok.get("error").is_none());

    let elapsed = t0.elapsed();
    println!("STRESS-MCP x50: 50 tool calls in {:?}", elapsed);
    drop(client_out);
    drop(client_lines);
    server.await.expect("server");
    assert!(elapsed.as_secs() < 60, "MCP 50 too slow");
}

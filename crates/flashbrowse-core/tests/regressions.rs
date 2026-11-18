//! Regression tests for the audit findings (MCP crash, protocol fields,
//! compressor quality, key mapping, URL allowlist).

use flashbrowse_cdp::protocol::accessibility::{AXNode, AXProperty, AXValue};
use flashbrowse_core::tree::compress;
use serde_json::Value;

// ---------------------------------------------------------------- protocol

#[test]
fn ax_node_deserializes_cdp_field_names() {
    // CDP sends `nodeId` and `backendDOMNodeId` (not camelCase of the Rust
    // field names) - a mismatch silently dropped every action ID.
    let raw = r#"{
        "nodeId": "12",
        "ignored": false,
        "role": {"type": "internalRole", "value": "button"},
        "name": {"type": "computedString", "value": "Submit"},
        "backendDOMNodeId": 4242,
        "childIds": ["13"],
        "properties": [
            {"name": "checked", "value": {"type": "booleanOrUndefined", "value": true}},
            {"name": "level", "value": {"type": "integer", "value": 3}}
        ]
    }"#;
    let node: AXNode = serde_json::from_str(raw).expect("AXNode must parse");
    assert_eq!(node.node_id, "12");
    assert_eq!(node.backend_node_id, Some(4242));
    assert!(node.prop_bool("checked"));
    assert_eq!(node.prop_i64("level"), Some(3));
}

#[test]
fn ax_node_tolerates_missing_optional_fields() {
    let raw = r#"{"nodeId": "1"}"#;
    let node: AXNode = serde_json::from_str(raw).expect("minimal node must parse");
    assert_eq!(node.node_id, "1");
    assert!(node.backend_node_id.is_none());
    assert!(node.properties.is_none());
}

#[test]
fn ax_value_exposes_text_bool_and_int() {
    let s: AXValue = serde_json::from_str(r#"{"type":"string","value":"hi"}"#).unwrap();
    assert_eq!(s.as_text(), "hi");
    let b: AXValue = serde_json::from_str(r#"{"type":"boolean","value":true}"#).unwrap();
    assert!(b.as_bool());
    let i: AXValue = serde_json::from_str(r#"{"type":"integer","value":7}"#).unwrap();
    assert_eq!(i.as_i64(), Some(7));
}

// -------------------------------------------------------------- compressor

fn node(id: &str, role: &str, name: &str, backend: Option<i64>) -> AXNode {
    AXNode {
        node_id: id.to_string(),
        ignored: false,
        role: Some(AXValue {
            type_: Some("RoleValue".to_string()),
            value: Some(serde_json::Value::String(role.to_string())),
        }),
        name: Some(AXValue {
            type_: Some("ComputedString".to_string()),
            value: Some(serde_json::Value::String(name.to_string())),
        }),
        value: None,
        backend_node_id: backend,
        child_ids: vec![],
        ignored_reasons: None,
        properties: None,
    }
}

fn with_children(mut n: AXNode, children: &[&str]) -> AXNode {
    n.child_ids = children.iter().map(|s| s.to_string()).collect();
    n
}

#[test]
fn unnamed_links_are_dropped_unnamed_buttons_are_kept() {
    let nodes = vec![
        node("1", "link", "", Some(1)),
        node("2", "button", "", Some(2)),
        node("3", "link", "Named", Some(3)),
    ];
    let (out, map) = compress(&nodes);
    // 3 nodes -> the unnamed link is unusable for an agent -> dropped, so
    // only 2 actions remain (the button and the named link).
    assert_eq!(map.len(), 2, "unnamed link must be dropped");
    // The unnamed button stays, but is marked.
    let unlabeled = out
        .iter()
        .find(|n| n.action_id.is_some() && n.name == "(unlabeled)");
    assert!(
        unlabeled.is_some(),
        "unnamed button must be kept and marked"
    );
    assert!(unlabeled.map(|n| n.role == "button").unwrap_or(false));
    assert!(out
        .iter()
        .any(|n| n.action_id.is_some() && n.name == "Named"));
    // No action may ever carry an empty name.
    assert!(
        out.iter()
            .filter(|n| n.action_id.is_some())
            .all(|n| !n.name.is_empty()),
        "actionable nodes must always have a name"
    );
}

#[test]
fn empty_link_name_is_derived_from_children() {
    let nodes = vec![
        with_children(node("1", "link", "", Some(1)), &["2"]),
        node("2", "statictext", "Story Title", Some(2)),
    ];
    let (out, _) = compress(&nodes);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "Story Title");
    assert_eq!(out[0].action_id, Some(1));
}

#[test]
fn layout_containers_do_not_duplicate_their_children() {
    let nodes = vec![
        with_children(
            node("1", "layouttablecell", "a | b | c", Some(1)),
            &["2", "3"],
        ),
        node("2", "link", "a", Some(2)),
        node("3", "link", "b", Some(3)),
    ];
    let (out, _) = compress(&nodes);
    // The container name (a | b | c) is the concatenation of its children.
    assert!(!out.iter().any(|n| n.name.contains("a | b | c")));
    assert_eq!(out.iter().filter(|n| n.action_id.is_some()).count(), 2);
}

#[test]
fn punctuation_only_text_is_dropped() {
    let nodes = vec![node("1", "statictext", " | ", None)];
    let (out, _) = compress(&nodes);
    assert!(out.is_empty(), "separator punctuation is noise");
}

#[test]
fn heading_levels_come_from_properties() {
    let mut h = node("1", "heading", "Title", None);
    h.properties = Some(vec![AXProperty {
        name: "level".to_string(),
        value: Some(AXValue {
            type_: Some("integer".to_string()),
            value: Some(serde_json::Value::from(2)),
        }),
    }]);
    let (out, _) = compress(&[h]);
    assert_eq!(out[0].level, Some(2));
}

// -------------------------------------------------------------------- keys

#[test]
fn key_mapping_uses_dom_codes() {
    use flashbrowse_core::actions::keys::resolve;
    assert_eq!(resolve("a").unwrap().code, "KeyA");
    assert_eq!(resolve("Z").unwrap().code, "KeyZ");
    assert_eq!(resolve("7").unwrap().code, "Digit7");
    assert_eq!(resolve("Enter").unwrap().vk, Some(13));
    assert!(resolve("").is_none());
    assert!(resolve("Ctrl+C").is_none());
}

// -------------------------------------------------------------- mcp server

#[tokio::test]
async fn malformed_tools_call_does_not_kill_the_server() {
    use flashbrowse_mcp::server::serve;
    use tokio::io::{duplex, AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (server_in, mut client_out) = duplex(64 * 1024);
    let (server_out, client_in) = duplex(64 * 1024);
    let mut client_lines = BufReader::new(client_in);

    let server = tokio::spawn(async move {
        serve(
            server_in,
            server_out,
            flashbrowse_core::SessionConfig::default(),
        )
        .await
    });

    let mut send = async |frame: Value| -> Value {
        client_out
            .write_all(frame.to_string().as_bytes())
            .await
            .unwrap();
        client_out.write_all(b"\n").await.unwrap();
        client_out.flush().await.unwrap();
        let mut line = String::new();
        client_lines.read_line(&mut line).await.unwrap();
        serde_json::from_str(line.trim()).expect("valid JSON frame")
    };

    // initialize
    let r = send(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2025-06-18", "capabilities": {}}
    }))
    .await;
    assert!(r.get("result").is_some(), "init must succeed");

    // malformed tools/call (no params) -> JSON-RPC error, server survives
    let r = send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call"})).await;
    assert_eq!(
        r.get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64()),
        Some(-32602),
        "malformed tools/call must return -32602"
    );

    // malformed tools/call (no name) -> JSON-RPC error
    let r = send(serde_json::json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"arguments": {}}
    }))
    .await;
    assert_eq!(
        r.get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64()),
        Some(-32602)
    );

    // server is still alive
    let r = send(serde_json::json!({"jsonrpc": "2.0", "id": 4, "method": "ping"})).await;
    assert!(
        r.get("error").is_none(),
        "server must survive malformed frames"
    );

    drop(client_out);
    drop(client_lines);
    let _ = server.await.expect("server task");
}

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    use flashbrowse_mcp::server::serve;
    use tokio::io::{duplex, AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (server_in, mut client_out) = duplex(64 * 1024);
    let (server_out, client_in) = duplex(64 * 1024);
    let mut client_lines = BufReader::new(client_in);
    let server = tokio::spawn(async move {
        serve(
            server_in,
            server_out,
            flashbrowse_core::SessionConfig::default(),
        )
        .await
    });

    client_out
        .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"does/not/exist","params":{}}"#)
        .await
        .unwrap();
    client_out.write_all(b"\n").await.unwrap();
    client_out.flush().await.unwrap();
    let mut line = String::new();
    client_lines.read_line(&mut line).await.unwrap();
    let v: Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64()),
        Some(-32601)
    );

    drop(client_out);
    drop(client_lines);
    let _ = server.await.expect("server task");
}

// --------------------------------------------------------- stale action map

/// After a failed navigation the action map must be cleared.
///
/// The old map points at backend node ids from the *previous* page, which no
/// longer exist. Acting on them makes CDP calls hang until they time out
/// (15 s per call) instead of failing fast.
#[tokio::test]
async fn failed_navigation_clears_the_action_map() {
    if flashbrowse_cdp::find_chrome(None).is_err() {
        eprintln!("SKIP: no Chromium binary found");
        return;
    }
    use flashbrowse_core::{BrowserSession, SessionConfig};

    let mut session = BrowserSession::launch(SessionConfig::default())
        .await
        .expect("launch");

    // A page that loads fine.
    let tree = session
        .open("data:text/html,<html><body><button>Go</button></body></html>")
        .await
        .expect("open");
    assert!(tree.contains("[1]"), "expected an action:\n{tree}");
    assert!(!session.action_map().is_empty());

    // Now a navigation that cannot produce a tree.
    let err = session
        .open("http://127.0.0.1:1/definitely-not-listening")
        .await
        .expect_err("navigation to a dead port must fail");
    assert!(
        matches!(err, flashbrowse_core::CoreError::NavigationFailed(_)),
        "unexpected error: {err:?}"
    );

    // The stale map must be gone, so acting on the old IDs fails immediately
    // with "action not found" instead of hanging on a dead CDP call.
    assert!(
        session.action_map().is_empty(),
        "stale action map survived a failed navigation: {:?}",
        session.action_map()
    );
    let acted = session.click(1).await;
    assert!(
        acted.is_err(),
        "acting on a stale id must fail, got: {acted:?}"
    );
}

// --------------------------------------------------------- resource policy

/// An invalid `resource_policy` must be rejected, not silently ignored.
///
/// The agent asked for a specific network policy; falling back to the
/// default would give it a different policy with no indication.
#[tokio::test]
async fn invalid_resource_policy_is_rejected() {
    use flashbrowse_mcp::server::serve;
    use tokio::io::{duplex, AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (server_in, mut client_out) = duplex(64 * 1024);
    let (server_out, client_in) = duplex(64 * 1024);
    let mut client_lines = BufReader::new(client_in);
    let server = tokio::spawn(async move {
        serve(
            server_in,
            server_out,
            flashbrowse_core::SessionConfig::default(),
        )
        .await
    });

    // Handshake first - the server rejects tools/call before initialize.
    client_out
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        )
        .await
        .unwrap();
    client_out.write_all(b"\n").await.unwrap();
    client_out.flush().await.unwrap();
    let mut line = String::new();
    client_lines.read_line(&mut line).await.unwrap();
    let _: Value = serde_json::from_str(line.trim()).unwrap();

    client_out
        .write_all(
            br#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"browser_open","arguments":{"url":"https://example.com","resource_policy":"bogus"}}}"#,
        )
        .await
        .unwrap();
    client_out.write_all(b"\n").await.unwrap();
    client_out.flush().await.unwrap();

    let mut line = String::new();
    client_lines.read_line(&mut line).await.unwrap();
    let v: Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_i64()),
        Some(-32602),
        "an invalid resource_policy must be an InvalidParams error: {v}"
    );

    drop(client_out);
    drop(client_lines);
    let _ = server.await.expect("server task");
}

// ------------------------------------------------------------- text capture

/// Plain text in a `<div>` and a `<span>` must survive compression.
///
/// Chromium gives EVERY `StaticText` an `InlineTextBox` child. Treating
/// that as "the StaticText owns children, so it is a concatenation" made
/// the compressor drop every piece of text on the page.
#[tokio::test]
async fn plain_text_in_generic_containers_survives() {
    use flashbrowse_core::tree::compress;

    // div > statictext, span > statictext - with the InlineTextBox child
    // Chromium always attaches.
    let nodes = vec![
        AXNode {
            node_id: "1".to_string(),
            ignored: false,
            role: Some(AXValue {
                type_: Some("RoleValue".to_string()),
                value: Some(serde_json::Value::String("generic".to_string())),
            }),
            name: Some(AXValue {
                type_: Some("ComputedString".to_string()),
                value: Some(serde_json::Value::String(String::new())),
            }),
            value: None,
            backend_node_id: None,
            child_ids: vec!["2".to_string()],
            ignored_reasons: None,
            properties: None,
        },
        AXNode {
            node_id: "2".to_string(),
            ignored: false,
            role: Some(AXValue {
                type_: Some("RoleValue".to_string()),
                value: Some(serde_json::Value::String("statictext".to_string())),
            }),
            name: Some(AXValue {
                type_: Some("ComputedString".to_string()),
                value: Some(serde_json::Value::String("div text".to_string())),
            }),
            value: None,
            backend_node_id: None,
            child_ids: vec!["3".to_string()],
            ignored_reasons: None,
            properties: None,
        },
        AXNode {
            node_id: "3".to_string(),
            ignored: false,
            role: Some(AXValue {
                type_: Some("RoleValue".to_string()),
                value: Some(serde_json::Value::String("inlinetextbox".to_string())),
            }),
            name: Some(AXValue {
                type_: Some("ComputedString".to_string()),
                value: Some(serde_json::Value::String("div text".to_string())),
            }),
            value: None,
            backend_node_id: None,
            child_ids: vec![],
            ignored_reasons: None,
            properties: None,
        },
    ];

    let (out, _) = compress(&nodes);
    let text: Vec<&str> = out.iter().map(|n| n.name.as_str()).collect();
    assert!(
        text.contains(&"div text"),
        "plain text was dropped: {text:?}"
    );
    // The InlineTextBox duplicate must not be emitted.
    assert_eq!(
        text.iter().filter(|t| **t == "div text").count(),
        1,
        "text duplicated: {text:?}"
    );
}

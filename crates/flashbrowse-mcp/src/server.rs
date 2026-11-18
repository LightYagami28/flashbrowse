//! Stdio transport loop (newline-delimited JSON-RPC 2.0).
//!
//! Strict stdout discipline: stdout is pure JSON-RPC, one line per frame.
//! All `tracing` output goes to stderr (initialized by `serve_stdio`, not
//! here, so in-process tests can install their own subscriber).

use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use flashbrowse_core::SessionConfig;

use crate::protocol::{
    negotiate, JsonRpcError, JsonRpcErrorResponse, JsonRpcRequest, JsonRpcResult, McpError,
};
use crate::tools;

/// Generic serve loop over any `AsyncRead` stdin / `AsyncWrite` stdout.
///
/// This shape lets integration tests drive the server over in-memory pipes
/// without spawning a binary (see `tests/mcp_stdio_e2e.rs`).
pub async fn serve<R, W>(reader: R, writer: W, config: SessionConfig) -> Result<(), McpError>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut writer = writer;
    let handle: tools::SessionHandle = Arc::new(Mutex::new(None));

    // Track whether `initialize` was exchanged (best-effort lifecycle).
    let mut initialized = false;

    // 1 MiB max line (M5): protects against a runaway client OOMing the server.
    const MAX_LINE: usize = 1024 * 1024;
    while let Some(line) = lines.next_line().await? {
        // Cap before trim/parse (length check is pre-parse).
        if line.len() > MAX_LINE {
            let resp = JsonRpcErrorResponse {
                jsonrpc: "2.0".to_string(),
                id: Value::Null,
                error: JsonRpcError::new(-32700, "Parse error: line too large".to_string()),
            };
            write_frame(&mut writer, &resp).await?;
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => {
                // Parse error → error response with id null (per JSON-RPC §5.1).
                let resp = JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    error: JsonRpcError::new(-32700, "Parse error".to_string()),
                };
                write_frame(&mut writer, &resp).await?;
                continue;
            }
        };
        // Strict JSON-RPC 2.0 tag (M5): reject non-2.0 frames.
        if let Some(tag) = req.jsonrpc.as_deref() {
            if tag != "2.0" {
                if !req.is_notification() {
                    let resp = JsonRpcErrorResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id.clone().unwrap_or(Value::Null),
                        error: JsonRpcError::new(
                            -32600,
                            "Invalid Request: jsonrpc must be \"2.0\"".to_string(),
                        ),
                    };
                    write_frame(&mut writer, &resp).await?;
                }
                continue;
            }
        }
        // Lifecycle enforcement (M5): tools/* before initialize is an error.
        if (req.method == "tools/list" || req.method == "tools/call") && !initialized {
            let resp = JsonRpcErrorResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone().unwrap_or(Value::Null),
                error: JsonRpcError::new(-32002, "Server not initialized".to_string()),
            };
            write_frame(&mut writer, &resp).await?;
            continue;
        }
        // Per-request deadline (M4): browser ops inside tools/call are
        // bounded by the session's 15s CDP timeouts + a 30s outer budget
        // applied at dispatch below.

        match req.method.as_str() {
            "initialize" => {
                let client_version = req
                    .params
                    .as_ref()
                    .and_then(|p| p.get("protocolVersion"))
                    .and_then(|v| v.as_str());
                let version = negotiate(client_version);
                if !req.is_notification() {
                    initialized = true;
                    let result = serde_json::json!({
                        "protocolVersion": version,
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "flashbrowse", "version": env!("CARGO_PKG_VERSION")}
                    });
                    let resp = JsonRpcResult {
                        jsonrpc: "2.0".to_string(),
                        id: req.id.clone().unwrap_or(Value::Null),
                        result,
                    };
                    write_frame(&mut writer, &resp).await?;
                }
            }
            "notifications/initialized" => {
                // Notification: acknowledge silently, never respond.
                initialized = true;
            }
            "ping" => {
                if !req.is_notification() {
                    let resp = JsonRpcResult {
                        jsonrpc: "2.0".to_string(),
                        id: req.id.clone().unwrap_or(Value::Null),
                        result: Value::Null,
                    };
                    write_frame(&mut writer, &resp).await?;
                }
            }
            "tools/list" => {
                if req.is_notification() {
                    continue;
                }
                let result = serde_json::json!({"tools": tools::tool_definitions()});
                let resp = JsonRpcResult {
                    jsonrpc: "2.0".to_string(),
                    id: req.id.clone().unwrap_or(Value::Null),
                    result,
                };
                write_frame(&mut writer, &resp).await?;
            }
            "tools/call" => {
                if req.is_notification() {
                    continue;
                }
                // NEVER let a malformed frame terminate the server: reply
                // with a JSON-RPC error and keep serving.
                let Some(params) = req.params.clone() else {
                    let resp = JsonRpcErrorResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id.clone().unwrap_or(Value::Null),
                        error: JsonRpcError::new(-32602, "tools/call missing params".to_string()),
                    };
                    write_frame(&mut writer, &resp).await?;
                    continue;
                };
                let Some(name) = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                else {
                    let resp = JsonRpcErrorResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id.clone().unwrap_or(Value::Null),
                        error: JsonRpcError::new(-32602, "tools/call missing 'name'".to_string()),
                    };
                    write_frame(&mut writer, &resp).await?;
                    continue;
                };
                let name = name.as_str();
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                // Outer budget (M4/3.6): must exceed the worst inner
                // latency (navigate 15s + load wait 15s + tree fetches),
                // otherwise the browser navigates while the agent gets a
                // tree for the previous page.
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(120),
                    tools::call_tool(name, &arguments, &handle, &config),
                )
                .await;
                let result = match result {
                    Err(_) => {
                        let resp = JsonRpcErrorResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id.clone().unwrap_or(Value::Null),
                            error: JsonRpcError::new(-32603, "tools/call timed out".to_string()),
                        };
                        write_frame(&mut writer, &resp).await?;
                        continue;
                    }
                    Ok(r) => r,
                };
                let id = req.id.clone().unwrap_or(Value::Null);
                match result {
                    Ok(tr) => {
                        let resp = JsonRpcResult {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: serde_json::to_value(tr)?,
                        };
                        write_frame(&mut writer, &resp).await?;
                    }
                    Err(McpError::InvalidParams(msg)) => {
                        let resp = JsonRpcErrorResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            error: JsonRpcError::new(-32602, msg),
                        };
                        write_frame(&mut writer, &resp).await?;
                    }
                    Err(e) => {
                        let resp = JsonRpcErrorResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            error: JsonRpcError::new(-32603, e.to_string()),
                        };
                        write_frame(&mut writer, &resp).await?;
                    }
                }
            }
            "notifications/cancelled" => {
                // Best-effort: browser ops are short; ignore.
            }
            other => {
                if req.is_notification() {
                    // Unknown notification: ignore silently.
                    continue;
                }
                let resp = JsonRpcErrorResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id.clone().unwrap_or(Value::Null),
                    error: JsonRpcError::new(-32601, format!("Method not found: {other}")),
                };
                write_frame(&mut writer, &resp).await?;
            }
        }
    }

    // Clean teardown: kill browser if it was launched.
    if let Some(session) = handle.lock().await.take() {
        session.shutdown().await;
    }
    Ok(())
}

/// Entry point wiring real stdin/stdout + stderr-only logging.
///
/// Call this from the `flashbrowse mcp` CLI command (Phase 4).
pub async fn serve_stdio(config: SessionConfig) -> Result<(), McpError> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();
    serve(tokio::io::stdin(), tokio::io::stdout(), config).await
}

async fn write_frame<W>(writer: &mut W, frame: &impl serde::Serialize) -> Result<(), McpError>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut line = serde_json::to_string(frame)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

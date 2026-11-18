//! MCP tool definitions and execution.

use std::sync::Arc;

use flashbrowse_core::actions::ScrollDirection;
use flashbrowse_core::{
    BrowserSession, ResourcePolicy, ScreenshotOptions, SessionConfig,
    SCREENSHOT_PREPARE_MS_DEFAULT, SCREENSHOT_PREPARE_MS_MAX,
};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::protocol::{McpError, ToolResult};

/// The single browser session shared by all `tools/call` invocations.
pub type SessionHandle = Arc<Mutex<Option<BrowserSession>>>;

/// Parameter schema for tools that need no arguments except an explicit
/// `{}` object is still valid per MCP's `inputSchema` requirement.
fn empty_schema() -> Value {
    serde_json::json!({"type": "object", "properties": {}})
}

/// All four tool definitions exposed via `tools/list`.
pub fn tool_definitions() -> Vec<Value> {
    vec![
        serde_json::json!({
            "name": "browser_open",
            "description": "Navigates to a URL, applies stealth and network filtering, and returns the token-compressed, numbered semantic action tree. Use browser_screenshot on this same session for visuals (no second browser tool).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "Target HTTP/HTTPS or file URL"},
                    "text_only": {"type": "boolean", "description": "Legacy: selects text-only resource policy", "default": false},
                    "resource_policy": {"type": "string", "enum": ["agent-default", "text-only", "full-render"], "description": "Network load policy (tree size unchanged); full-render loads images/fonts for faithful screenshots"}
                },
                "required": ["url"]
            }
        }),
        serde_json::json!({
            "name": "browser_act",
            "description": "Executes an interaction (click, fill, hover, press_key, scroll) against a numbered action element ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action_id": {"type": "integer", "description": "Element ID number from the action tree"},
                    "action": {"type": "string", "enum": ["click", "fill", "hover", "press_key", "scroll", "select"], "description": "Interaction to perform"},
                    "value": {"type": "string", "description": "Text payload for 'fill', key name for 'press_key', direction for 'scroll', or option label for 'select'"},
                    "amount": {"type": "number", "description": "Pixel distance for 'scroll' (default 300)"}
                },
                "required": ["action_id", "action"]
            }
        }),
        serde_json::json!({
            "name": "browser_extract_markdown",
            "description": "Extracts readable article and document text formatted as clean Markdown, stripping all boilerplate.",
            "inputSchema": empty_schema()
        }),
        serde_json::json!({
            "name": "browser_screenshot",
            "description": "Captures a viewport or full-page PNG of the current session (after browser_open/browser_act). Waits for images/network to settle. Use this tool instead of a separate browser MCP.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "full_page": {"type": "boolean", "default": false},
                    "prepare_lazy_content": {"type": "boolean", "description": "Scroll the page to surface lazy-loaded images", "default": true},
                    "timeout_ms": {"type": "integer", "description": "Max ms to wait before best-effort capture (500-60000)", "default": 8000}
                }
            }
        }),
    ]
}

/// Parse `text_only` from args (default `false`).
fn arg_bool(args: &Value, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Parse required string arg, or `InvalidParams`.
fn arg_string(args: &Value, key: &str) -> Result<String, McpError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| McpError::InvalidParams(format!("missing required '{key}'")))
}

/// Parse optional string arg.
fn arg_opt_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

/// Parse required integer arg (`action_id`), rejecting overflow.
fn arg_u32(args: &Value, key: &str) -> Result<u32, McpError> {
    let raw = args
        .get(key)
        .ok_or_else(|| McpError::InvalidParams(format!("missing required '{key}'")))?;
    let n = raw
        .as_u64()
        .ok_or_else(|| McpError::InvalidParams(format!("'{key}' must be an integer")))?;
    u32::try_from(n).map_err(|_| McpError::InvalidParams(format!("'{key}' out of range (u32 max)")))
}

/// Resolve the resource policy for a `browser_open` call.
///
/// An unparseable `resource_policy` is an error, not a silent fallback: the
/// agent asked for a specific network policy and would otherwise get a
/// different one with no indication.
fn arg_resource_policy(
    arguments: &Value,
    default_config: &SessionConfig,
) -> Result<ResourcePolicy, McpError> {
    if let Some(s) = arguments.get("resource_policy").and_then(|v| v.as_str()) {
        return s.parse::<ResourcePolicy>().map_err(McpError::InvalidParams);
    }
    if arguments
        .get("text_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Ok(ResourcePolicy::TextOnly);
    }
    Ok(default_config.resource_policy)
}

async fn ensure_session<'a>(
    handle: &'a SessionHandle,
    default_config: &SessionConfig,
    want_policy: ResourcePolicy,
) -> Result<tokio::sync::MutexGuard<'a, Option<BrowserSession>>, McpError> {
    let mut guard = handle.lock().await;
    let needs_relaunch = match guard.as_ref() {
        Some(s) => s.resource_policy() != want_policy,
        None => true,
    };
    if needs_relaunch {
        if let Some(old) = guard.take() {
            old.shutdown().await;
        }
        let mut cfg = default_config.clone();
        cfg.resource_policy = want_policy;
        let fresh = BrowserSession::launch(cfg)
            .await
            .map_err(|e| McpError::Internal(e.to_string()))?;
        *guard = Some(fresh);
    }
    Ok(guard)
}

fn require_session(guard: &mut Option<BrowserSession>) -> Result<&mut BrowserSession, McpError> {
    guard.as_mut().ok_or_else(|| {
        McpError::Internal("no browser session: call browser_open first".to_string())
    })
}

/// Execute one `tools/call`. Returns a `ToolResult` (success or `isError`).
///
/// Every call is timed and appended to the metrics trail so operators can
/// see latency, token cost, and error rate per tool.
pub async fn call_tool(
    name: &str,
    arguments: &Value,
    handle: &SessionHandle,
    default_config: &SessionConfig,
) -> Result<ToolResult, McpError> {
    let started = std::time::Instant::now();
    let result = match name {
        "browser_open" => call_open(arguments, handle, default_config).await,
        "browser_act" => call_act(arguments, handle, default_config).await,
        "browser_extract_markdown" => call_markdown(handle).await,
        "browser_screenshot" => call_screenshot(arguments, handle).await,
        other => Err(McpError::InvalidParams(format!("unknown tool '{other}'"))),
    };
    record_tool_metric(name, arguments, &result, started);
    result
}

/// Payload size (chars) and actionable-element count of a tool result.
fn payload_of(result: &Result<ToolResult, McpError>) -> (usize, usize) {
    let Ok(tr) = result else {
        return (0, 0);
    };
    let mut chars = 0usize;
    let mut actions = 0usize;
    for item in &tr.content {
        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
            chars += text.len();
            actions += flashbrowse_core::observability::count_actions(text);
        }
    }
    (chars, actions)
}

/// Append one MCP tool call to the metrics trail.
fn record_tool_metric(
    name: &str,
    arguments: &Value,
    result: &Result<ToolResult, McpError>,
    started: std::time::Instant,
) {
    use flashbrowse_core::observability;

    let target = arguments
        .get("url")
        .and_then(|u| u.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            arguments
                .get("action")
                .and_then(|a| a.as_str())
                .map(|a| format!("mcp:{a}"))
                .unwrap_or_else(|| format!("mcp:{name}"))
        });
    let (chars, actions) = payload_of(result);
    let (ok, error) = match result {
        Ok(tr) => (tr.is_error != Some(true), None),
        Err(e) => (false, Some(e.to_string())),
    };
    let r = observability::record_op(
        &format!("mcp:{name}"),
        &target,
        started,
        Some((chars, actions)),
        ok,
        error,
    );
    tracing::debug!(
        tool = name,
        ms = r.ms,
        tokens = r.tokens,
        actions = r.actions,
        ok = r.ok,
        "mcp tool call"
    );
}

/// MCP URL allowlist (H4 fix): agent boundary must not read local files.
/// Only `http(s)://`, `data:`, and `about:blank` are navigable via MCP.
/// CLI commands keep full `file://` support (local-user trust boundary).
fn validate_mcp_url(url: &str) -> Result<(), McpError> {
    let t = url.trim();
    if t.is_empty() {
        return Err(McpError::InvalidParams("url must not be empty".to_string()));
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower == "about:blank"
    {
        // Reject file/chrome/view-source smuggling via whitespace/control.
        if t.chars().any(|c| c.is_control()) {
            return Err(McpError::InvalidParams(
                "url contains control characters".to_string(),
            ));
        }
        return Ok(());
    }
    Err(McpError::InvalidParams(format!(
        "url scheme not allowed via MCP (allowed: http, https, data, about:blank): {t}"
    )))
}

async fn call_open(
    arguments: &Value,
    handle: &SessionHandle,
    default_config: &SessionConfig,
) -> Result<ToolResult, McpError> {
    let url = arg_string(arguments, "url")?;
    validate_mcp_url(&url)?;
    let policy = arg_resource_policy(arguments, default_config)?;
    let mut guard = ensure_session(handle, default_config, policy).await?;
    let session = require_session(&mut guard)?;
    match session.open(&url).await {
        Ok(tree) => Ok(ToolResult::text(tree)),
        Err(e) => Ok(ToolResult::error(e.to_string())),
    }
}

async fn call_act(
    arguments: &Value,
    handle: &SessionHandle,
    default_config: &SessionConfig,
) -> Result<ToolResult, McpError> {
    let action_id = arg_u32(arguments, "action_id")?;
    let action = arg_string(arguments, "action")?;
    let value = arg_opt_string(arguments, "value").unwrap_or_default();

    let mut guard = handle.lock().await;
    // Reuse existing session; if none, this is an error (must open first).
    if guard.is_none() {
        // Try launching with defaults so a fresh agent can act after open-less flow?
        // Spec ordering is open→act; surface a clear error instead of silent launch.
        return Ok(ToolResult::error(
            "no browser session: call browser_open first".to_string(),
        ));
    }
    // Touch default_config so the launch path stays explicit for future use.
    let _ = default_config;
    let session = guard.as_mut().expect("session present");

    let result = match action.as_str() {
        "click" => session.click(action_id).await,
        "fill" => {
            // Allow explicit empty string (clear-field); reject missing key.
            if arguments.get("value").is_none() {
                return Ok(ToolResult::error(
                    "fill requires 'value' (text payload, may be empty to clear)".to_string(),
                ));
            }
            session.fill(action_id, &value).await
        }
        "hover" => session.hover(action_id).await,
        "press_key" => {
            if value.is_empty() {
                return Ok(ToolResult::error(
                    "press_key requires 'value' (key name, e.g. Enter, ArrowDown, a)".to_string(),
                ));
            }
            session.press_key(&value).await
        }
        "select" => {
            if arguments.get("value").is_none() {
                return Ok(ToolResult::error(
                    "select requires 'value' (option text or value)".to_string(),
                ));
            }
            session.select(action_id, &value).await
        }
        "scroll" => {
            let direction = if value.is_empty() {
                ScrollDirection::Down
            } else {
                match ScrollDirection::parse(&value) {
                    Some(d) => d,
                    None => {
                        return Ok(ToolResult::error(format!(
                            "invalid scroll direction '{value}' (expected up|down|left|right)"
                        )))
                    }
                }
            };
            let amount = arguments
                .get("amount")
                .and_then(|v| v.as_f64())
                .unwrap_or(300.0);
            session.scroll(direction, amount).await
        }
        other => {
            return Ok(ToolResult::error(format!(
                "unknown action '{other}' (expected click|fill|hover|press_key|scroll|select)"
            )))
        }
    };
    match result {
        Ok(tree) => Ok(ToolResult::text(tree)),
        Err(e) => Ok(ToolResult::error(e.to_string())),
    }
}

async fn call_markdown(handle: &SessionHandle) -> Result<ToolResult, McpError> {
    let guard = handle.lock().await;
    match guard.as_ref() {
        None => Ok(ToolResult::error(
            "no browser session: call browser_open first".to_string(),
        )),
        Some(s) => match s.extract_markdown().await {
            Ok(md) => Ok(ToolResult::text(md)),
            Err(e) => Ok(ToolResult::error(e.to_string())),
        },
    }
}

fn arg_u64(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key)
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
        .clamp(500, SCREENSHOT_PREPARE_MS_MAX)
}

async fn call_screenshot(
    arguments: &Value,
    handle: &SessionHandle,
) -> Result<ToolResult, McpError> {
    let opts = ScreenshotOptions {
        full_page: arg_bool(arguments, "full_page"),
        prepare_lazy_content: arguments
            .get("prepare_lazy_content")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        timeout_ms: arg_u64(arguments, "timeout_ms", SCREENSHOT_PREPARE_MS_DEFAULT),
    };
    let guard = handle.lock().await;
    match guard.as_ref() {
        None => Ok(ToolResult::error(
            "no browser session: call browser_open first".to_string(),
        )),
        Some(s) => match s.screenshot(opts).await {
            Ok(data) => Ok(ToolResult::image(data)),
            Err(e) => Ok(ToolResult::error(e.to_string())),
        },
    }
}

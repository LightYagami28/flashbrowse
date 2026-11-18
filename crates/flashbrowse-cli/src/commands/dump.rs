//! `flashbrowse dump <url> --format markdown|axtree`.

use std::path::PathBuf;

use flashbrowse_core::{BrowserSession, ResourcePolicy, SessionConfig};

use flashbrowse_core::observability;

/// Emit Markdown (default) or the formatted AXTree for `url`.
/// Format is validated BEFORE launch so a typo costs zero browsers (H7).
pub async fn run(
    url: String,
    format: String,
    chrome_path: Option<PathBuf>,
    resource_policy: ResourcePolicy,
) -> anyhow::Result<()> {
    let started = std::time::Instant::now();
    match format.as_str() {
        "markdown" | "axtree" => {}
        other => {
            return Err(anyhow::anyhow!(
                "unknown --format '{other}' (expected markdown|axtree)"
            ))
        }
    }
    let config = SessionConfig {
        chrome_path,
        resource_policy,
        ..Default::default()
    };
    let mut session = BrowserSession::launch(config).await?;
    let result = async {
        let _ = session.open(&url).await?;
        let out = match format.as_str() {
            "markdown" => session.extract_markdown().await?,
            "axtree" => session.last_tree().to_string(),
            _ => unreachable!("validated above"),
        };
        Ok::<String, flashbrowse_core::CoreError>(out)
    }
    .await;
    session.shutdown().await;
    let out = match result {
        Ok(o) => o,
        Err(e) => {
            let msg = e.to_string();
            observability::record_op("dump", &url, started, None, false, Some(msg));
            return Err(e.into());
        }
    };
    println!("{out}");
    let actions = if format == "axtree" {
        observability::count_actions(&out)
    } else {
        0
    };
    observability::record_op(
        "dump",
        &url,
        started,
        Some((out.len(), actions)),
        true,
        None,
    );
    Ok(())
}

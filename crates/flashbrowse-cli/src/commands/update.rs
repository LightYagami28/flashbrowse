//! `flashbrowse update` - check for a newer release (update channel aware).
//!
//! Channels (env `FLASHBROWSE_UPDATE_CHANNEL`, default `stable`):
//!   * `stable`  - latest `v*` release
//!   * `beta`    - latest `v*-beta*` release
//!   * `nightly` - latest `v*-nightly*` release
//!   * `off`     - never check
//!
//! The check is advisory: it prints what is available and how to get it.
//! It never modifies the installation without `--install`.

use serde_json::Value;

/// Update channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
    Nightly,
    Off,
}

impl Channel {
    /// Parse a channel name.
    pub fn parse(s: &str) -> Channel {
        match s.trim().to_ascii_lowercase().as_str() {
            "beta" | "preview" => Channel::Beta,
            "nightly" | "dev" => Channel::Nightly,
            "off" | "none" | "disabled" => Channel::Off,
            _ => Channel::Stable,
        }
    }

    /// Read the channel from the environment (default `stable`).
    pub fn from_env() -> Channel {
        Channel::parse(
            &std::env::var("FLASHBROWSE_UPDATE_CHANNEL").unwrap_or_else(|_| "stable".into()),
        )
    }

    /// Does a release tag match this channel?
    pub fn matches(&self, tag: &str) -> bool {
        match self {
            Channel::Stable => {
                !tag.contains("beta") && !tag.contains("nightly") && !tag.contains("rc")
            }
            Channel::Beta => tag.contains("beta") || tag.contains("rc"),
            Channel::Nightly => tag.contains("nightly"),
            Channel::Off => false,
        }
    }

    /// Name of the channel.
    pub fn name(&self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Nightly => "nightly",
            Channel::Off => "off",
        }
    }
}

/// Latest release info.
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// Release tag, e.g. `v1.2.3`.
    pub tag: String,
    /// Release URL.
    pub url: String,
}

/// Fetch the latest release matching `channel` from the GitHub API.
pub async fn latest_release(channel: Channel) -> Result<ReleaseInfo, String> {
    if channel == Channel::Off {
        return Err("update channel is 'off'".to_string());
    }
    let client = reqwest::Client::builder()
        .user_agent("flashbrowse-update/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    let url = "https://api.github.com/repos/nhrbuilds/flashbrowse/releases";
    let list: Vec<Value> = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    for rel in list {
        let Some(tag) = rel.get("tag_name").and_then(|t| t.as_str()) else {
            continue;
        };
        if rel.get("draft").and_then(|d| d.as_bool()).unwrap_or(false) {
            continue;
        }
        if !channel.matches(tag) {
            continue;
        }
        let html = rel
            .get("html_url")
            .and_then(|u| u.as_str())
            .unwrap_or("https://github.com/nhrbuilds/flashbrowse/releases")
            .to_string();
        return Ok(ReleaseInfo {
            tag: tag.to_string(),
            url: html,
        });
    }
    Err(format!("no {} release found", channel.name()))
}

/// Compare semver-ish tags. Returns `true` when `candidate` is newer.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim()
            .trim_start_matches('v')
            .split(['.', '-'])
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let a = parse(candidate);
    let b = parse(current);
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// `flashbrowse update [--check]`.
pub async fn run(check_only: bool) -> anyhow::Result<()> {
    let channel = Channel::from_env();
    let current = env!("CARGO_PKG_VERSION");
    if channel == Channel::Off {
        println!("update channel is 'off' (FLASHBROWSE_UPDATE_CHANNEL=off)");
        return Ok(());
    }
    match latest_release(channel).await {
        Ok(info) => {
            if is_newer(&info.tag, current) {
                println!("update available: {current} -> {}", info.tag);
                println!("  {}", info.url);
                if !check_only {
                    println!("  install: curl -fsSL https://raw.githubusercontent.com/nhrbuilds/flashbrowse/main/install.sh | sh -s -- --version {}",
                        info.tag);
                }
            } else {
                println!(
                    "flashbrowse {current} is up to date (channel: {})",
                    channel.name()
                );
            }
        }
        Err(e) => {
            // Never fail a command because the update check failed.
            eprintln!("update check skipped: {e}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_newer, Channel};

    #[test]
    fn channel_matching() {
        let stable = Channel::Stable;
        assert!(stable.matches("v1.2.3"));
        assert!(!stable.matches("v1.2.3-beta.1"));
        assert!(!stable.matches("v1.2.3-nightly"));
        assert!(Channel::Beta.matches("v1.2.3-beta.1"));
        assert!(Channel::Nightly.matches("v1.2.3-nightly"));
        assert!(!Channel::Off.matches("v1.2.3"));
    }

    #[test]
    fn version_comparison() {
        assert!(is_newer("v1.0.1", "1.0.0"));
        assert!(is_newer("v2.0.0", "v1.9.9"));
        assert!(!is_newer("v1.0.0", "v1.0.0"));
        assert!(!is_newer("v0.9.0", "v1.0.0"));
    }
}

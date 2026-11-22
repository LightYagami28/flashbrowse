//! Discover a Firefox binary for future BiDi sessions.

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

/// Firefox is not installed or not on PATH.
#[derive(Debug, Error)]
#[error("no Firefox binary found (searched: {searched:?})")]
pub struct FirefoxNotAvailable {
    pub searched: Vec<String>,
}

/// Options for a future `launch_firefox` implementation.
#[derive(Debug, Clone, Default)]
pub struct FirefoxLaunchOptions {
    pub firefox_path: Option<PathBuf>,
    pub headless: bool,
    pub remote_debugging_port: u16,
}

const CANDIDATES: &[&str] = &[
    "firefox",
    "firefox-esr",
    "mozilla-firefox",
    "Mozilla Firefox",
];

/// Resolve Firefox using explicit path, `FIREFOX_PATH`, or PATH.
pub fn find_firefox(explicit: Option<&PathBuf>) -> Result<PathBuf, FirefoxNotAvailable> {
    let mut searched = Vec::new();
    if let Some(p) = explicit {
        searched.push(p.display().to_string());
        if p.is_file() {
            return Ok(p.clone());
        }
        // An explicit path is authoritative. Falling back to PATH here would
        // silently run a different Firefox than the caller asked for, and it
        // made the result depend on whether the host happens to have Firefox
        // installed (it does on CI runners, not on a dev box).
        return Err(FirefoxNotAvailable { searched });
    }
    if let Ok(env) = std::env::var("FIREFOX_PATH") {
        let p = PathBuf::from(&env);
        searched.push(env);
        if p.is_file() {
            return Ok(p);
        }
    }
    for name in CANDIDATES {
        searched.push((*name).to_string());
        if let Ok(p) = which::which(name) {
            return Ok(p);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let app = PathBuf::from("/Applications/Firefox.app/Contents/MacOS/firefox");
        searched.push(app.display().to_string());
        if app.is_file() {
            return Ok(app);
        }
    }
    Err(FirefoxNotAvailable { searched })
}

/// Best-effort `--version` probe for doctor/setup (not wired in CLI yet).
pub fn firefox_version(path: &Path) -> Option<String> {
    let out = Command::new(path)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let first = line.lines().next()?.trim();
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::find_firefox;

    #[test]
    fn find_firefox_err_when_missing() {
        let err = find_firefox(Some(&PathBuf::from("/nonexistent/firefox"))).unwrap_err();
        // Only the explicit path may be searched: a fallback to PATH would
        // return the host's Firefox (present on CI, absent on many dev
        // machines) and make this test depend on the machine.
        assert_eq!(err.searched, vec!["/nonexistent/firefox".to_string()]);
    }
}

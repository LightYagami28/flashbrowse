//! Platform-idiomatic cache locations for flashbrowse state.
//!
//! One definition, shared by the CLI and the MCP server so that metrics,
//! sessions, and the managed browser always land in the same place.

use std::path::PathBuf;

/// Root cache dir for flashbrowse state.
/// - Linux/macOS: `$XDG_CACHE_HOME/flashbrowse` or `~/.cache/flashbrowse`
/// - Windows: `%LOCALAPPDATA%\flashbrowse`
pub fn cache_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            if !local.is_empty() {
                return PathBuf::from(local).join("flashbrowse");
            }
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("flashbrowse");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".cache").join("flashbrowse")
}

/// Directory holding persistent session descriptors.
pub fn sessions_dir() -> PathBuf {
    cache_dir().join("sessions")
}

#[cfg(test)]
mod tests {
    use super::cache_dir;

    #[test]
    fn cache_dir_is_absolute_and_named() {
        let d = cache_dir();
        assert!(
            d.is_absolute(),
            "cache dir must be absolute: {}",
            d.display()
        );
        assert_eq!(
            d.file_name().and_then(|s| s.to_str()),
            Some("flashbrowse"),
            "cache dir must end in 'flashbrowse'"
        );
    }
}

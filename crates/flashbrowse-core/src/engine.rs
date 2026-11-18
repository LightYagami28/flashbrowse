//! Browser engine selection (Chrome today; Firefox planned).

use serde::{Deserialize, Serialize};

/// Which browser binary/protocol stack backs a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserEngine {
    #[default]
    Chrome,
    /// WebDriver BiDi (see `flashbrowse-bidi` crate); not yet user-facing.
    Firefox,
}

impl BrowserEngine {
    pub fn as_str(self) -> &'static str {
        match self {
            BrowserEngine::Chrome => "chrome",
            BrowserEngine::Firefox => "firefox",
        }
    }
}

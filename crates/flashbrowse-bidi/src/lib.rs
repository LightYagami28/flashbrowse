//! Firefox WebDriver BiDi integration (milestone 3 groundwork).
//!
//! Chromium sessions use `flashbrowse-cdp`. Firefox will attach over BiDi
//! (`browsingContext.captureScreenshot`, navigation, input) once a full
//! accessibility-tree snapshot exists or a DOM adapter feeds the compressor.
//!
//! This crate currently provides binary discovery and launch option types only.

mod launcher;

pub use launcher::{find_firefox, firefox_version, FirefoxLaunchOptions, FirefoxNotAvailable};

/// Experimental flag: tree parity on Firefox is not production-ready.
pub const FIREFOX_EXPERIMENTAL: bool = true;

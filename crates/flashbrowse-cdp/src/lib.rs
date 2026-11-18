//! `flashbrowse-cdp`: strongly-typed CDP client & WebSocket frame engine.
//!
//! # Architecture
//!
//! ```text
//! ChromeProcess (launcher) ──browser_ws_url──▶ CdpClient (transport)
//!                                                 │  sessionId mux
//!                                                 ├─► Page.* / DOM.* / Input.*
//!                                                 └─► EventEmitter (Page.loadEventFired, ...)
//! ```
//!
//! All calls use **browser-level flatten mode**:
//! `Browser.getVersion` → `Target.createTarget` → `Target.attachToTarget {flatten:true}`.
//!
//! # Example
//!
//! ```ignore
//! use flashbrowse_cdp::{launch_chrome, CdpClient, protocol::{browser, target, page}};
//!
//! let chrome = launch_chrome(Default::default()).await?;
//! let client = CdpClient::connect(chrome.browser_ws_url()).await?;
//! let ver = browser::get_version(&client).await?;
//! assert!(!ver.protocol_version.is_empty());
//! let target = target::create_target(&client, "about:blank").await?;
//! let session = target::attach_to_target(&client, &target.target_id).await?;
//! page::navigate(&client, &session.session_id, "about:blank").await?;
//! ```

pub mod error;
pub mod events;
pub mod launcher;
pub mod protocol;
pub mod transport;

pub use error::CdpError;
pub use events::EventEmitter;
pub use launcher::{find_chrome, launch_chrome, ChromeProcess, LaunchOptions};
pub use transport::CdpClient;

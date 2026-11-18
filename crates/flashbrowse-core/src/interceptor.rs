//! Network interceptor URL patterns (`Network.setBlockedURLs`).
//!
//! Blocking is *free for the AX tree*: a blocked image still exposes its
//! `alt` text as the accessible name (verified against Chromium 147), so we
//! can skip raster media entirely and cut page weight by 50-90%.
//!
//! Fonts + trackers are always blocked in agent-default mode. Images are
//! blocked by default (raster/vector media is useless to an LLM agent);
//! `screenshot()` temporarily lifts font/image blocks so pixels are still
//! captured (unless the session already uses [`ResourcePolicy::FullRender`]).
//!
//! CSS is deliberately NOT blocked:
//!   * measured share of transferred bytes: 1-3% on modern sites (18% on
//!     Hacker News) - CSS is not the bottleneck (JS is 9-55%, and JS must
//!     stay because it hydrates the page we read);
//!   * blocking it breaks visibility semantics: `display:none` /
//!     `visibility:hidden` elements are excluded from the AX tree only
//!     because CSS says so. Measured with a local page: with CSS blocked,
//!     `SECRET HIDDEN CONTENT` (a `display:none` div) appears in the tree -
//!     content the user cannot see would be exposed to the agent.

use crate::resource_policy::ResourcePolicy;

/// Analytics/telemetry domains (blocked in every policy).
const TRACKER_BLOCKED: &[&str] = &[
    "*google-analytics.com*",
    "*googletagmanager.com*",
    "*segment.io*",
    "*datadoghq.com*",
    "*hotjar.com*",
    "*sentry.io*",
    "*mixpanel.com*",
    "*doubleclick.net*",
    "*facebook.net*",
    "*amazon-adsystem.com*",
    "*adsbygoogle*",
    "*googlesyndication.com*",
];

/// Font URL patterns (blocked in agent-default and text-only).
const FONT_BLOCKED: &[&str] = &["*.woff*", "*.woff2*", "*.ttf*", "*.otf*", "*.eot*"];

/// Raster/vector media: blocked in agent-default and text-only.
const IMAGE_BLOCKED: &[&str] = &[
    "*.png*", "*.jpg*", "*.jpeg*", "*.gif*", "*.webp*", "*.avif*", "*.svg*", "*.ico*",
];

/// Additional media blocked only in text-only mode.
const TEXT_ONLY_BLOCKED: &[&str] = &["*.mp4*", "*.webm*", "*.m4v*", "*.mp3*", "*.wav*"];

fn push_unique(out: &mut Vec<String>, patterns: &[&str]) {
    for p in patterns {
        let s = (*p).to_string();
        if !out.contains(&s) {
            out.push(s);
        }
    }
}

/// Patterns for `Network.setBlockedURLs` for the active session policy.
pub fn blocked_patterns(policy: ResourcePolicy) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    push_unique(&mut out, TRACKER_BLOCKED);
    match policy {
        ResourcePolicy::FullRender => out,
        ResourcePolicy::AgentDefault => {
            push_unique(&mut out, FONT_BLOCKED);
            push_unique(&mut out, IMAGE_BLOCKED);
            out
        }
        ResourcePolicy::TextOnly => {
            push_unique(&mut out, FONT_BLOCKED);
            push_unique(&mut out, IMAGE_BLOCKED);
            push_unique(&mut out, TEXT_ONLY_BLOCKED);
            out
        }
    }
}

/// Blocklist used while preparing a screenshot in agent-default / text-only:
/// trackers only so images and fonts can load for the capture.
pub fn blocked_patterns_for_screenshot(policy: ResourcePolicy) -> Vec<String> {
    match policy {
        ResourcePolicy::FullRender => blocked_patterns(policy),
        ResourcePolicy::AgentDefault | ResourcePolicy::TextOnly => {
            let mut out: Vec<String> = Vec::new();
            push_unique(&mut out, TRACKER_BLOCKED);
            if policy == ResourcePolicy::TextOnly {
                push_unique(&mut out, TEXT_ONLY_BLOCKED);
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{blocked_patterns, blocked_patterns_for_screenshot};
    use crate::resource_policy::ResourcePolicy;

    #[test]
    fn text_only_gates_media() {
        let normal = blocked_patterns(ResourcePolicy::AgentDefault);
        let text_only = blocked_patterns(ResourcePolicy::TextOnly);
        assert!(normal.iter().any(|p| p == "*.woff2*"));
        assert!(!normal.iter().any(|p| p == "*.mp4*"));
        assert!(text_only.iter().any(|p| p == "*.mp4*"));
        assert!(text_only.len() > normal.len());
    }

    #[test]
    fn full_render_only_blocks_trackers() {
        let full = blocked_patterns(ResourcePolicy::FullRender);
        assert!(!full.iter().any(|p| p == "*.png*"));
        assert!(!full.iter().any(|p| p == "*.woff2*"));
        assert!(full.iter().any(|p| p == "*sentry.io*"));
    }

    #[test]
    fn screenshot_lifts_fonts_and_images() {
        let normal = blocked_patterns(ResourcePolicy::AgentDefault);
        assert!(normal.iter().any(|p| p == "*.png*"));
        let for_cap = blocked_patterns_for_screenshot(ResourcePolicy::AgentDefault);
        assert!(!for_cap.iter().any(|p| p == "*.png*"));
        assert!(!for_cap.iter().any(|p| p == "*.woff2*"));
        assert!(for_cap.iter().any(|p| p == "*sentry.io*"));
    }

    #[test]
    fn full_render_screenshot_unchanged() {
        let a = blocked_patterns(ResourcePolicy::FullRender);
        let b = blocked_patterns_for_screenshot(ResourcePolicy::FullRender);
        assert_eq!(a, b);
    }
}

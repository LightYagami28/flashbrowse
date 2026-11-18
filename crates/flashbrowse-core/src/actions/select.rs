//! Combobox/dropdown option selection.
//!
//! `BrowserSession::select(id, text)` clicks the `<select>`/combobox, then
//! picks the option whose `text` or `value` equals `text` (JSON-encoded
//! needle for injection-safe evaluation). This module hosts shared
//! option-matching helpers for tests.

/// Match `want` against an option's `text`/`value` (text first).
pub fn option_matches(text: &str, value: &str, want: &str) -> bool {
    text.trim() == want.trim() || value == want
}

#[cfg(test)]
mod tests {
    use super::option_matches;

    #[test]
    fn matches_by_text_then_value() {
        assert!(option_matches("United States", "US", "United States"));
        assert!(option_matches("United States", "US", "US"));
        assert!(!option_matches("Canada", "CA", "US"));
    }
}

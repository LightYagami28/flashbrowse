//! LLM-optimized textual representation renderer.
//!
//! Output format:
//! ```text
//! Title: Hacker News
//! URL: https://news.ycombinator.com
//! ---
//! [1] Link: "Hacker News"
//! Heading 1: Top Stories
//! [10] Link: "Rust 1.85 Released" (value="http://...")
//! ```

use super::node::SemanticNode;

/// Max characters per name before elision (token guard).
const MAX_NAME_CHARS: usize = 120;

/// Longest useful `extras` payload per line (token guard).
const MAX_EXTRAS_CHARS: usize = 80;

fn truncate(s: &str) -> String {
    // Char-count truncation, never mid-UTF8.
    if s.chars().count() <= MAX_NAME_CHARS {
        return s.to_string();
    }
    let mut out: String = s.chars().take(MAX_NAME_CHARS - 1).collect();
    out.push('…');
    out
}

fn render_extras(extras: &[String]) -> String {
    if extras.is_empty() {
        return String::new();
    }
    let mut joined = extras.join(", ");
    if joined.chars().count() > MAX_EXTRAS_CHARS {
        joined = joined
            .chars()
            .take(MAX_EXTRAS_CHARS - 1)
            .collect::<String>()
            + "…";
    }
    format!(" ({joined})")
}

/// Render compressor output to the spec's LLM text format.
pub fn render(title: &str, url: &str, nodes: &[SemanticNode]) -> String {
    let mut out = String::new();
    out.push_str(&format!("Title: {}\n", title.trim()));
    out.push_str(&format!("URL: {}\n", url.trim()));
    out.push_str("---\n");

    for n in nodes {
        let name = truncate(n.name.trim());
        // Capitalize role for readability per spec: `[1] Link:`, `Heading 1:`.
        let role_title = match n.role.as_str() {
            "heading" => "Heading".to_string(),
            "text" => "Text".to_string(),
            other => {
                let mut c = other.chars();
                match c.next() {
                    None => String::new(),
                    Some(first) => first.to_ascii_uppercase().to_string() + c.as_str(),
                }
            }
        };
        match n.action_id {
            Some(id) => {
                out.push_str(&format!("[{}] {}: \"{}\"", id, role_title, name));
                out.push_str(&render_extras(&n.extras));
                out.push('\n');
            }
            None if n.role == "heading" => {
                let lvl = n.level.unwrap_or(1);
                if name.is_empty() {
                    continue;
                }
                out.push_str(&format!("{} {}: {}\n", role_title, lvl, name));
            }
            None => {
                if name.is_empty() {
                    // Structural label only (landmark container).
                    out.push_str(&format!("{role_title}\n"));
                    continue;
                }
                // Plain content (link text without ID, list items, paragraphs).
                out.push_str(&format!("{}: {}\n", role_title, name));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::tree::node::SemanticNode;

    fn sn(id: Option<u32>, role: &str, name: &str) -> SemanticNode {
        SemanticNode {
            action_id: id,
            role: role.to_string(),
            name: name.to_string(),
            extras: vec![],
            level: if role == "heading" { Some(1) } else { None },
            node_id: "1".to_string(),
        }
    }

    #[test]
    fn renders_numbered_lines() {
        let out = render(
            "HN",
            "https://news.ycombinator.com",
            &[
                sn(Some(1), "link", "Hacker News"),
                sn(None, "heading", "Top Stories"),
            ],
        );
        assert!(out.contains("Title: HN"));
        assert!(out.contains("URL: https://news.ycombinator.com"));
        assert!(out.contains("[1] Link: \"Hacker News\""));
        assert!(out.contains("Heading 1: Top Stories"));
    }
}

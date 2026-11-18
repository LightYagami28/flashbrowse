//! Benchmarks raw HTML/AXTree vs token-compressed action graph.
//!
//! Pure compressor benchmark (no Chrome) - generates a synthetic 5k-node
//! tree, compresses, and asserts ≥80% token reduction while retaining every
//! interactive element. Token estimate: 1 token ≈ 4 characters.

use flashbrowse_cdp::protocol::accessibility::{AXNode, AXValue};
use flashbrowse_core::tree::{compress, render};

fn axval(s: &str) -> AXValue {
    AXValue {
        type_: Some("ComputedString".to_string()),
        value: Some(serde_json::Value::String(s.to_string())),
    }
}

fn roleval(s: &str) -> AXValue {
    AXValue {
        type_: Some("RoleValue".to_string()),
        value: Some(serde_json::Value::String(s.to_string())),
    }
}

fn node(id: &str, role: &str, name: &str, backend: Option<i64>) -> AXNode {
    AXNode {
        node_id: id.to_string(),
        ignored: false,
        role: Some(roleval(role)),
        name: Some(axval(name)),
        value: None,
        backend_node_id: backend,
        child_ids: vec![],
        ignored_reasons: None,
        properties: None,
    }
}

#[test]
fn compression_ratio_80_pct_and_retains_interactive() {
    // Dense synthetic page: 4,800 structural nodes + 200 actionable elements.
    let mut nodes: Vec<AXNode> = Vec::with_capacity(5_000);
    // Empty generic wrappers (should all be pruned).
    for i in 0..4_800 {
        nodes.push(node(&format!("gen-{i}"), "generic", "", None));
    }
    // Actionable links/buttons with backend ids.
    for i in 0..200 {
        let role = if i % 2 == 0 { "link" } else { "button" };
        nodes.push(node(
            &format!("act-{i}"),
            role,
            &format!("Item {i}"),
            Some((i + 1) as i64),
        ));
    }
    // A handful of headings (content, unnumbered).
    for i in 0..20 {
        nodes.push(node(
            &format!("h-{i}"),
            "heading",
            &format!("Section {i}"),
            None,
        ));
    }

    let (semantic, map) = compress(&nodes);
    assert_eq!(map.len(), 200, "every actionable element retained");

    // Token estimates: naive = unpruned AXTree debug render (stands in for raw
    // DOM/HTML size before compression), compressed = rendered text.
    let naive_chars = format!("{nodes:#?}").len();
    let rendered = render("Dense", "https://example.com", &semantic);
    let compressed_chars = rendered.len();
    let naive_tokens = naive_chars.div_ceil(4);
    let compressed_tokens = compressed_chars.div_ceil(4);

    let reduction = 1.0 - (compressed_tokens as f64 / naive_tokens as f64);
    assert!(
        reduction >= 0.80,
        "reduction {:.1}% < 80% (naive {naive_tokens} -> {compressed_tokens} tokens)",
        reduction * 100.0
    );

    // All 200 actionable lines survive in the rendered output.
    for i in 1..=200u32 {
        assert!(
            rendered.contains(&format!("[{i}] ")),
            "missing action [{i}] in rendered output"
        );
    }
}

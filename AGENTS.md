# flashbrowse - agent conventions

- **Decision lock:** session-file hybrid, `--chrome-path`/`CHROME_PATH` support, `ResourcePolicy` (`agent-default` | `text-only` | `full-render`, legacy `--text-only`), MCP latest (2025-06-18).
- **Screenshots:** same MCP session (`browser_screenshot`); blocking lift must not change AX tree output. Screenshot prepare uses network idle + image readiness (timeout capped), not a fixed 400 ms sleep. Lifting the blocklist is not enough on its own: Chrome never retries a blocked request, so blocked images must be re-requested for the capture.
- **Firefox:** `flashbrowse-bidi` crate = discovery/launch groundwork only; production sessions remain Chrome until BiDi AX tree lands.
- **Phase gates:** `cargo clippy --workspace --all-targets -- -D warnings` must be clean before advancing phases.
- **Tracer test:** `cargo test --workspace` (needs system Chrome for `cdp_connection`; skips gracefully when absent).
- **Non-goal (MVP):** no daemon socket - `close` + session files only.

## Accuracy rules (do not break these)

1. **Never drop visible text.** Every pruning rule in the compressor must
   remove a *duplicate*, never the only copy of the content. A table cell, a
   list item and a paragraph must all survive compression.
2. **Document order.** Chromium returns the AX tree breadth-first; the
   compressor rebuilds it and walks depth-first (`document_order`). Do not
   replace that with the raw array order.
3. **Never trade accuracy for speed.** No truncating waits, no early
   snapshots, no "probably good enough" tree. If the tree is not final,
   wait for it.
4. **No em dashes or en dashes** anywhere in the repo (source, docs, commits).
5. **Benchmarks are sequential.** One tool at a time, one page at a time, and
   the machine must be idle. A parallel or loaded benchmark is a lie.
6. **Settle exit conditions.** The open/settle loop must accept a tree of ANY
   size (`!nodes.is_empty()`), never `nodes.len() >= 3`; a minimal page would
   otherwise wait out the whole deadline. `InlineTextBox` is layout noise,
   never content: it exists on every `StaticText` and must not count as a
   meaningful child.

## Observability

- Every operation records a `MetricRecord` in
  `~/.cache/flashbrowse/metrics.jsonl` via
  `flashbrowse_core::observability::record_op`.
- Logs go to **stderr only** (stdout must stay pure JSON-RPC for MCP).
- `--log-format json` emits one JSON object per line.

## Release gates

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `node scripts/soak.mjs --minutes 5 --url <url>` (smoke version of the
  soak; the full 1-2 h run is done before a release)

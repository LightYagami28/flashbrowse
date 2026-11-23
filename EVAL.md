# flashbrowse evaluation results

Measured on 2026-11-23 (UTC), machine: Chromium `google-chrome-stable`, release
binary built from this tree (`target/release/flashbrowse`).

## Release gates

| Gate | Result |
|------|--------|
| `cargo fmt --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | pass (all browser tests; `fidelity_across_fixtures` included) |
| `flashbrowse doctor --json` | pass |
| `scripts/soak.mjs` 5 min (re-run post-loadshed) | pass (0% errors, drift 0.996x). See `eval/results/soak-smoke.json` |

## WP2: Fidelity oracle (zero LLM)

`scripts/fidelity.mjs` on all fixture pages (`http://127.0.0.1:8099/`). **100% text
recall** (0 missing strings). Raw vs compressed:

| Page | Static texts checked | Raw | Tree | Reduction |
|------|---------------------|-----|------|-----------|
| index.html | 35 | 75.3 KB | 4.6 KB | 93.9% |
| forms.html | 7 | 18.1 KB | 0.6 KB | 96.4% |
| table.html | 1 | 121.1 KB | 1.5 KB | 98.7% |
| a11y.html | 0 | 13.5 KB | 0.4 KB | 96.7% |
| unicode.html | 3 | 6.3 KB | 0.6 KB | 90.6% |
| shadow.html | 3 | 4.8 KB | 0.3 KB | 94.1% |
| frames.html | 2 | 3.3 KB | 0.2 KB | 93.1% |
| spatial.html | 6 | 6.7 KB | 0.3 KB | 95.3% |

Full JSON: [`eval/results/fidelity-report.json`](eval/results/fidelity-report.json).

Rust integration test `fidelity_across_fixtures` in CI encodes the same rules.

## WP3: Observation arm sizes (chars, fixture subset)

Arms A/B/C/D via `dump`; arm E via Playwright `ariaSnapshot()`. Body
chars after the `---` header where applicable:

| Fixture | A flat | B nested | C full nested | D full flat | E Playwright |
|---------|--------|----------|---------------|-------------|--------------|
| table.html | 1573 | 2135 | 4983 | 2135 | 4983 |
| index.html | 4709 | 5269 | (see JSON) | (see JSON) | 5850 |
| forms.html | 661 | 859 | 1855 | 1335 | 554 |

Nested formatting adds ~15 to 40% on small pages; full tree (C/D) is
3 to 10x arm A on dense pages. Full table:
[`eval/results/arm-sizes.json`](eval/results/arm-sizes.json).

## WP4/WP5: MCP pilot (read-only oracle, not LLM)

`eval/run.mjs --dry-run` opens each pilot task via MCP `browser_open` and
scores with `browser_evaluate` on the **live session**. This is the success
predicate harness, not an agent:

| Task | Success (dry-run) | Note |
|------|-------------------|------|
| forms-country-japan | pass | initial state |
| forms-name-ada | pass | |
| forms-subscribe-checked | pass | |
| table-row-25 | pass | |
| unicode-cafe | pass | |
| frames-parent-text | pass | |
| spatial-slider-value | pass | |
| a11y-read-more-count | pass | |
| forms-select-france | fail (expected) | requires `select` action |
| shadow-button | fail (expected) | shadow text not in `body.innerText` for DOM check |

**8/10** oracle checks pass. JSON: [`eval/results/mcp-pilot-dry-run.json`](eval/results/mcp-pilot-dry-run.json).

## WP5: LLM pilot (fixtures only, OpenRouter)

Provider: OpenRouter (`eval/lib/llm.js`). Temperature 0, `repeats=1`, MCP
drives actions; arms B/C/E swap observation text only. Keep fixture server up
(`eval/with-fixtures.mjs`).

| Model | Arm A (flat) | Arm B (nested) | Arm C (full nested) | Arm E (Playwright YAML) |
|-------|-------------|----------------|---------------------|-------------------------|
| `nvidia/nemotron-3.5-super-vl:free` | 5/10 | **8/10** | **8/10** | 3/10 |
| `inclusionai/ling-3.2-flash:free` | 0/10 | (not run) | (not run) | (not run) |

Arm E uses Playwright `ariaSnapshot()` for the initial user message but still
expects flashbrowse numeric `browser_act` IDs, so it is a weak baseline for
agent success until the harness maps YAML to actions or uses Playwright to act.

Shared failures on nemotron: `forms-select-france` (native `<select>`), `shadow-button`
(shadow DOM). Nested/full arms beat flat on the same model (8/10 vs 5/10), so
format and extra nodes help this free model more than compression hurts.

Ling hit tool-calling limits (0/10, mostly 1 turn) and OpenRouter 429 on burst
smoke tests.

```sh
cd eval
export OPENROUTER_API_KEY=...   # never commit keys
node with-fixtures.mjs -- node run.mjs --arm b \
  --model 'nvidia/nemotron-3.5-super-vl:free' --tasks tasks/pilot --repeats 1 \
  --binary ../target/release/flashbrowse --out results/pilot-nemotron-arm-b.json
```

## WP6 / WP7

Set-of-marks screenshot: not started. Release version bump: after you review
these numbers and say commit.

## Artifacts

- [`eval/results/fidelity-report.json`](eval/results/fidelity-report.json)
- [`eval/results/arm-sizes.json`](eval/results/arm-sizes.json)
- [`eval/results/mcp-pilot-dry-run.json`](eval/results/mcp-pilot-dry-run.json)
- [`eval/results/soak-smoke.json`](eval/results/soak-smoke.json)
- [`eval/results/pilot-nemotron-arm-a.json`](eval/results/pilot-nemotron-arm-a.json)
- [`eval/results/pilot-nemotron-arm-b.json`](eval/results/pilot-nemotron-arm-b.json)
- [`eval/results/pilot-nemotron-arm-c.json`](eval/results/pilot-nemotron-arm-c.json)
- [`eval/results/pilot-nemotron-arm-e.json`](eval/results/pilot-nemotron-arm-e.json)
- [`eval/results/pilot-ling-arm-a.json`](eval/results/pilot-ling-arm-a.json)
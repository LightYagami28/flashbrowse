# Breaking-point findings (work in progress, uncommitted)

Rolling log from `break-suite.mjs`, fidelity sweeps, and probes. Generated during
the 2026-11-23 stress day. Do not treat as release-quality until reviewed.

## Infrastructure lessons

| Issue | Impact | Mitigation |
|-------|--------|------------|
| Fixture server exits after shell/loadshed | All MCP opens → `chrome-error://` | `with-fixtures.mjs` / `break-suite` wrapper |
| Playwright browsers lost after reboot | Arm E / `ariaSnapshot` fails | `npx playwright install chromium` in `eval/` |
| `evaluate` success checks | Dry-run looked broken (`evaluate undefined`) | `success-mcp.js` accepts `true`/`1`, better error text |

## Real-site fidelity (`fidelity-real-public.json`)

19 public URLs, sequential. Summary: **3 dump failures**, **43 missing StaticText strings** (not all are product bugs).

### Hard failures (navigation / dump)

| URL | Symptom |
|-----|---------|
| `https://httpstat.us/200` / `500` | Dump fails (site incompatible or blocks Chrome) |

MDN (`developer.mozilla.org`) dumps successfully on retry; torture re-run shows **15 / 81**
StaticText strings missing (`fidelity-torture.json`).

### Information loss candidates (review in compressor)

| URL | Missing count | Notes |
|-----|---------------|-------|
| `doc.rust-lang.org/std/` | **30 / 455** | Dense docs; many sentence *fragments* missing (not just long paragraphs) |
| `github.com/torvalds/linux` | 7 / 63 | Long README markdown blocks (`Linux kernel ============`, maintainer docs). Likely pruning/layout folding, not random |
| `the-internet.../infinite_scroll` | 2 | Below-fold lorem; scroll not in fidelity script |
| `jsonplaceholder.typicode.com` | 1 | Footer with emoji (`❤️`); encoding or named-control parent rule |
| `www.w3.org/WAI/demos/bad/` | 1 | Opaque hash string `a5ed074c6d27b875` |
| `www.rust-lang.org` | 5 | See JSON for strings |
| `www.iana.org/domains/example` | 1 | Long paragraph truncated at 120 chars in oracle |

Still **100% recall on all 14 local fixtures** including new stress pages
(`fidelity-local-extended.json`): 200-row table, 60 identical buttons, modal,
80-deep custom tags.

### Open probe (`probe-real-public.json`)

| URL | Latency | Actions | Note |
|-----|---------|---------|------|
| `doc.rust-lang.org/std/` | 1.1s | **345** | Huge action map |
| `github.com/torvalds/linux` | 3.6s | **165** | Heavy but usable |
| `news.ycombinator.com` | 2.4s | **2** | Compression extreme (88.7% in Rust stress) |
| `nested-depth.html` | **15.3s** | 0 | Custom tag chain; settle cost |
| `heavy-table.html` | 0.9s | 0 | 448 KB raw → 4.8 KB tree; text recall OK |

## Task competence (nemotron free, arm B local / arm A real)

### New local stress tasks (`llm-stress-local-arm-b.json`)

| Task | Result |
|------|--------|
| heavy-row-150 | **pass** (1 turn) |
| many-buttons-37 | **pass** (6 turns) |
| lazy-tab-two | fail (6 turns) |
| modal-fill | fail (10 turns) |
| contenteditable-type | fail (3 turns) |

### Real web (`llm-real-web-arm-a.json`, **5/6**)

| Task | Result |
|------|--------|
| the-internet checkboxes | pass |
| the-internet dropdown (`select`) | pass (8 turns) |
| httpbin form | pass |
| HN front page | pass |
| Wikipedia portal | pass |
| example.com title check | fail (`evaluate` false; page content vs predicate) |

Dry-run (no LLM): real tasks that need interaction correctly **fail** without an agent; smoke tasks (HN, Wikipedia) **pass**.

## Product breakpoints to fix or document

1. **GitHub README / long markdown in repos** - fidelity missing multi-paragraph static text.
2. **Infinite scroll / below-fold text** - fidelity oracle does not scroll; tree may be correct for viewport only.
3. **Modal + contenteditable** - agents struggle; may need tree state (`expanded`) or better action names.
4. **`<dialog>`** - modal-fill failure; confirm tree exposes Open/Save with stable IDs.
5. **httpstat.us** - unusable with flashbrowse today.
6. **example.com** - align success predicate with actual DOM (or allow redirect).

## Artifacts (under `eval/results/`)

- `probe-fixtures-local.json`, `probe-real-public.json`
- `fidelity-local-extended.json`, `fidelity-real-public.json`
- `dry-stress-local.json`, `dry-real-web.json`
- `llm-stress-local-arm-b.json`, `llm-real-web-arm-a.json`
- `break-suite-log.jsonl`
- `soak-15min-mixed.json` - **15 min done**: 0% errors, 2079 tool calls, p50 254ms, drift **1.0x**
- `soak-90min-mixed.json` - **90 min done**: 0% errors, **13,152** tool calls, drift **0.985x**, ~25M tokens
- `mcp-marathon-3x.json` - 51/57 opens OK (6 fails: httpstat ×3 cycles)
- `fidelity-real-public-rerun.json` - same gaps as prior (rustc/GitHub/MDN)

## Long queue (`long-runs.mjs`)

After the current **15 min soak** + **`run-batch.mjs`** finish, automatically:

1. **90 min soak** → `soak-90min-mixed.json` (HN, MDN, rustc std, the-internet, fixtures)
2. **MCP marathon** 3× all `real-public` URLs → `mcp-marathon-3x.json`
3. **Fidelity** real-public rerun → `fidelity-real-public-rerun.json`
4. **`agent-sweep-long.json`**: pilot A/B/C **×3 repeats**, arm D ×2, real-web ×3, stress B ×2, Ling pilot ×2

Log: `/tmp/long-runs.log`. No commits.

## Agent batch sweep (`agent-sweep.json`, completed)

All **12 jobs OK** (~16 min wall). Highlights (see `agent-summary.json`):

| Run | Pass rate |
|-----|-----------|
| nemotron pilot A ×2 (20 runs) | **16/20 (80%)** |
| nemotron pilot B ×2 (20 runs) | **2/20 (10%)** - high variance vs earlier 8/10 single |
| nemotron pilot C rerun | 8/10 |
| nemotron real A ×2 | **15/20 (75%)** |
| nemotron real B | **6/10 (60%)** |
| nemotron stress A / C | 3/5, 2/5 |
| Ling pilot B, real A | **0/10 each** |

Rolling table: `eval/results/agent-summary.json` via `node summarize-runs.mjs`.

## Commands to continue

```sh
cd eval
node with-fixtures.mjs -- node break-suite.mjs --phase llm-stress
node break-suite.mjs --phase fidelity-real
node break-suite.mjs --phase soak-15
node run-batch.mjs
node summarize-runs.mjs
node mcp-marathon.mjs --cycles 2
```
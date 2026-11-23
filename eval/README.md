# flashbrowse task eval (dev-only)

Sequential harness for comparing observation arms (compressed flat, nested,
full tree, Playwright `ariaSnapshot`). Not part of release artifacts.

## Quick start

```sh
# Install Node deps once
cd eval && npm install && npx playwright install chromium
```

Pilot tasks load fixtures from `http://127.0.0.1:8099`. The server must stay
up for the whole run (a one-shot background `python3 -m http.server` often dies
after loadshedding or shell exit). Prefer the wrapper:

```sh
cd eval
node with-fixtures.mjs -- npm run run -- --dry-run --base http://127.0.0.1:8099 \
  --binary ../target/release/flashbrowse --tasks tasks/pilot \
  --out results/mcp-pilot-dry-run.json
```

Or start the server manually in another terminal:

```sh
python3 -m http.server 8099 --bind 127.0.0.1 \
  --directory crates/flashbrowse-core/tests/fixtures
```

```sh
cd eval

# Check success predicates without an LLM
npm run verify -- --base http://127.0.0.1:8099

# Pilot agent run (arm A). Set OPENROUTER_API_KEY or OPENAI_API_KEY.
export OPENROUTER_API_KEY=...   # optional: export OPENROUTER_MODEL=nvidia/nemotron-3.5-super-vl:free
npm run run -- \
  --base http://127.0.0.1:8099 \
  --arm a \
  --model 'nvidia/nemotron-3.5-super-vl:free' \
  --tasks tasks/pilot \
  --binary ../target/release/flashbrowse \
  --repeats 1 \
  --out results/pilot-nemotron-arm-a.json

# Markdown summary
npm run report -- --in results/pilot.json --out EVAL-pilot.md
```

## Arms

| Arm | Observation source |
|-----|-------------------|
| A | MCP `browser_open` / `browser_act` tree (compressed, flat) |
| B | `dump --format axtree-nested` on the same URL after each step |
| C | `dump --format axtree-nested --no-prune` |
| D | `dump --format axtree --no-prune` |
| E | Playwright `locator.ariaSnapshot()` (nested YAML) |

Arms B-D re-open the page in a throwaway dump for observation only (same
fixture URL and policy). The live MCP session still drives actions.

## Outputs

`run.mjs` writes JSON with per-run success, turns, token estimates, wall
time, and failure categories. `report.mjs` turns that into tables for
`EVAL.md`.

Pilot: 10 tasks, arms A/B/C/E, `repeats=1`. Full sweep: all tasks, all arms,
`repeats=3`, idle machine.
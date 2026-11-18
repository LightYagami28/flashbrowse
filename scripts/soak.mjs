#!/usr/bin/env node
/**
 * flashbrowse soak test.
 *
 * Drives a long-running MCP stdio session against a mix of local and real
 * pages, then reports:
 *   - throughput (tools/second) over time
 *   - latency drift (p50 of the first tenth vs the last tenth)
 *   - error rate per tool
 *   - process health: RSS growth of the browser and the flashbrowse process,
 *     plus a count of orphaned Chrome processes
 *
 * Usage:
 *   node scripts/soak.mjs --minutes 90 --url http://localhost:8099/ --url https://example.com
 *
 * Flags:
 *   --minutes <n>     how long to soak (default 60)
 *   --url <u>         a URL to include; repeatable
 *   --interval <ms>   pause between iterations (default 250)
 *   --screenshot      include a screenshot tool call every N iterations
 *   --out <file>      write the JSON report here (default: soak-report.json)
 *   --bin <path>      flashbrowse binary (default: target/release/flashbrowse)
 *
 * The soak is intentionally SEQUENTIAL: one tool call at a time, one page at
 * a time. Running tools in parallel would measure the machine, not the tool.
 *
 * Exit codes:
 *   - non-zero if error rate > 5%
 *   - non-zero if latency drift (last-tenth p50 / first-tenth p50) > 1.2x
 *   - non-zero if flashbrowse RSS grew > 50% during the run
 */

import { spawn } from "node:child_process";
import { existsSync, writeFileSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { execSync } from "node:child_process";

// ---------------------------------------------------------------- arguments

function parseArgs(argv) {
  const args = {
    minutes: 60,
    urls: [],
    interval: 250,
    screenshotEvery: 20,
    out: "soak-report.json",
    bin: "target/release/flashbrowse",
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    switch (a) {
      case "--minutes":
        args.minutes = Number(argv[++i]);
        break;
      case "--url":
        args.urls.push(argv[++i]);
        break;
      case "--interval":
        args.interval = Number(argv[++i]);
        break;
      case "--screenshot":
        args.screenshotEvery = Number(argv[++i]);
        break;
      case "--out":
        args.out = argv[++i];
        break;
      case "--bin":
        args.bin = argv[++i];
        break;
      case "--help":
        console.log(readFileSync(new URL(import.meta.url)).toString().split("*/")[0]);
        process.exit(0);
        break;
      default:
        console.error(`unknown flag: ${a}`);
        process.exit(2);
    }
  }
  if (args.urls.length === 0) args.urls.push("https://example.com");
  return args;
}

const args = parseArgs(process.argv);
const BIN = resolve(args.bin);
if (!existsSync(BIN)) {
  console.error(`flashbrowse binary not found: ${BIN}`);
  console.error("build it first: cargo build --release");
  process.exit(2);
}

// ---------------------------------------------------------------- MCP client

class McpClient {
  constructor(child) {
    this.child = child;
    this.nextId = 1;
    this.pending = new Map();
    this.buffer = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => this.#onData(chunk));
    child.stderr.on("data", (d) => {
      if (process.env.SOAK_VERBOSE) process.stderr.write(`[fb] ${d}`);
    });
  }

  #onData(chunk) {
    this.buffer += chunk;
    let idx;
    while ((idx = this.buffer.indexOf("\n")) >= 0) {
      const line = this.buffer.slice(0, idx).trim();
      this.buffer = this.buffer.slice(idx + 1);
      if (!line) continue;
      let msg;
      try {
        msg = JSON.parse(line);
      } catch {
        continue;
      }
      if (msg.id !== undefined && this.pending.has(msg.id)) {
        const { resolve: res, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error.message || "mcp error"));
        else res(msg.result);
      }
    }
  }

  request(method, params) {
    const id = this.nextId++;
    const payload = { jsonrpc: "2.0", id, method, params };
    return new Promise((res, rej) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        rej(new Error(`timeout after 180s: ${method}`));
      }, 180_000);
      this.pending.set(id, {
        resolve: (v) => {
          clearTimeout(timer);
          res(v);
        },
        reject: (e) => {
          clearTimeout(timer);
          rej(e);
        },
      });
      this.child.stdin.write(JSON.stringify(payload) + "\n");
    });
  }

  notify(method, params) {
    this.child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n");
  }
}

function textOf(result) {
  if (!result || !Array.isArray(result.content)) return "";
  return result.content
    .filter((c) => c.type === "text")
    .map((c) => c.text)
    .join("\n");
}

// ---------------------------------------------------------------- monitoring

function rssKb(pid) {
  try {
    const status = readFileSync(`/proc/${pid}/status`, "utf8");
    const m = status.match(/VmRSS:\s+(\d+)\s+kB/);
    return m ? Number(m[1]) : 0;
  } catch {
    return 0;
  }
}

function chromeCount() {
  try {
    const out = execSync("ps -eo args 2>/dev/null || true", { encoding: "utf8" });
    return out.split("\n").filter((l) => l.includes("chrome") && l.includes("flashbrowse-")).length;
  } catch {
    return 0;
  }
}

function chromeRssKb() {
  try {
    const out = execSync(
      "ps -eo pid,rss,args 2>/dev/null || true",
      { encoding: "utf8" }
    );
    let total = 0;
    for (const line of out.split("\n")) {
      if (line.includes("chrome") && line.includes("flashbrowse-")) {
        const parts = line.trim().split(/\s+/);
        total += Number(parts[1]) || 0;
      }
    }
    return total;
  } catch {
    return 0;
  }
}

// ---------------------------------------------------------------- helpers

function percentile(sorted, p) {
  if (!sorted.length) return 0;
  const i = Math.min(sorted.length - 1, Math.floor(sorted.length * p));
  return sorted[i];
}

function summarize(samples) {
  if (!samples.length) return { n: 0 };
  const ms = samples.map((s) => s.ms).sort((a, b) => a - b);
  const tokens = samples.map((s) => s.tokens).sort((a, b) => a - b);
  return {
    n: samples.length,
    errors: samples.filter((s) => !s.ok).length,
    p50_ms: percentile(ms, 0.5),
    p95_ms: percentile(ms, 0.95),
    max_ms: ms[ms.length - 1] ?? 0,
    p50_tokens: percentile(tokens, 0.5),
    total_tokens: tokens.reduce((a, b) => a + b, 0),
  };
}

// ---------------------------------------------------------------- soak run

const child = spawn(BIN, ["mcp", "--log-format", "json"], {
  stdio: ["pipe", "pipe", "pipe"],
});
const client = new McpClient(child);

const samples = [];
const perTool = new Map();
const timeline = [];
const errors = [];

let iteration = 0;
const startedAt = Date.now();
const deadline = startedAt + args.minutes * 60_000;

// Progress printer (one line every 30 s).
let lastPrint = 0;
function progress() {
  const now = Date.now();
  if (now - lastPrint < 30_000) return;
  lastPrint = now;
  const elapsedMin = ((now - startedAt) / 60_000).toFixed(1);
  const s = summarize(samples);
  const chrome = chromeCount();
  console.log(
    `[soak ${elapsedMin}m] iters=${iteration} tools=${samples.length} ` +
      `err=${s.errors ?? 0} p50=${s.p50_ms ?? 0}ms p95=${s.p95_ms ?? 0}ms ` +
      `tokens=${s.total_tokens ?? 0} chrome_procs=${chrome} ` +
      `rss_fb=${rssKb(child.pid)}kB rss_chrome=${chromeRssKb()}kB`
  );
}
const progressTimer = setInterval(progress, 5_000);

// Wait for the server to be ready.
const init = await client.request("initialize", {
  protocolVersion: "2025-06-18",
  capabilities: {},
  clientInfo: { name: "soak", version: "1.0.0" },
});
client.notify("notifications/initialized", {});

console.log(
  `soak starting: ${args.minutes} min, ${args.urls.length} url(s), ` +
    `interval ${args.interval}ms, pid ${child.pid}`
);
console.log(`initialize -> ${JSON.stringify(init?.serverInfo ?? {})}`);

// Action inventory for the current tree.
function actionIds(tree) {
  const ids = [];
  for (const line of tree.split("\n")) {
    const m = line.match(/^\s*\[(\d+)\]/);
    if (m) ids.push(Number(m[1]));
  }
  return ids;
}

async function timed(tool, argsObj) {
  const t0 = Date.now();
  let ok = true;
  let tokens = 0;
  let err = null;
  let text = "";
  try {
    const res = await client.request("tools/call", { name: tool, arguments: argsObj });
    text = textOf(res);
    tokens = Math.ceil(text.length / 4);
    if (res && res.isError) {
      ok = false;
      err = text.slice(0, 200);
    }
  } catch (e) {
    ok = false;
    err = String(e).slice(0, 200);
  }
  const sample = {
    tool,
    ms: Date.now() - t0,
    tokens,
    ok,
    err,
    at: Date.now(),
  };
  samples.push(sample);
  const bucket = perTool.get(tool) ?? [];
  bucket.push(sample);
  perTool.set(tool, bucket);
  if (!ok) {
    errors.push({ iteration, tool, err });
    console.error(`[soak] iteration ${iteration} ${tool} FAILED: ${err}`);
  }
  // The payload text is returned but NOT kept in the sample: the report
  // stays small and the timeline stays readable.
  return { ...sample, text };
}

let currentUrlIndex = 0;

while (Date.now() < deadline) {
  iteration++;
  const url = args.urls[currentUrlIndex % args.urls.length];
  currentUrlIndex++;

  const open = await timed("browser_open", { url });
  progress();

  const tree = open.text ?? "";
  const ids = actionIds(tree);

  // Exercise a rotating subset of tools against whatever the page offers.
  await timed("browser_act", {
    action_id: ids.length ? ids[0] : 1,
    action: "scroll",
    value: "down",
  });

  if (ids.length > 1) {
    await timed("browser_act", {
      action_id: ids[1],
      action: "hover",
    });
  }

  await timed("browser_extract_markdown", {});

  if (iteration % args.screenshotEvery === 0) {
    await timed("browser_screenshot", { full_page: false });
  }

  progress();

  if (args.interval > 0) {
    await new Promise((r) => setTimeout(r, args.interval));
  }

  const now = Date.now();
  timeline.push({
    t: Math.round((now - startedAt) / 1000),
    iteration,
    samples: samples.length,
    chrome: chromeCount(),
    rssFb: rssKb(child.pid),
    rssChrome: chromeRssKb(),
  });
}

clearInterval(progressTimer);
console.log("soak window elapsed; shutting the MCP server down...");
child.stdin.end();
await new Promise((r) => {
  child.on("exit", r);
  setTimeout(() => {
    child.kill("SIGTERM");
    r();
  }, 5000);
});

// ---------------------------------------------------------------- report

const firstTenth = summarize(samples.slice(0, Math.max(1, Math.floor(samples.length / 10))));
const lastTenth = summarize(samples.slice(-Math.max(1, Math.floor(samples.length / 10))));
const durationSec = (Date.now() - startedAt) / 1000;

const report = {
  startedAt: new Date(startedAt).toISOString(),
  durationSec: Math.round(durationSec),
  minutes: Number(args.minutes),
  urls: args.urls,
  iterations: iteration,
  toolCalls: samples.length,
  throughput_per_sec: Number((samples.length / durationSec).toFixed(3)),
  overall: summarize(samples),
  drift: {
    first_tenth_p50_ms: firstTenth.p50_ms,
    last_tenth_p50_ms: lastTenth.p50_ms,
    drift_ratio:
      firstTenth.p50_ms > 0
        ? Number((lastTenth.p50_ms / firstTenth.p50_ms).toFixed(3))
        : null,
    first_tenth_p95_ms: firstTenth.p95_ms,
    last_tenth_p95_ms: lastTenth.p95_ms,
  },
  perTool: Object.fromEntries(
    [...perTool.entries()].map(([k, v]) => [k, summarize(v)])
  ),
  memory: {
    rss_flashbrowse_start_kb: timeline[0]?.rssFb ?? 0,
    rss_flashbrowse_end_kb: timeline.at(-1)?.rssFb ?? 0,
    rss_chrome_start_kb: timeline[0]?.rssChrome ?? 0,
    rss_chrome_end_kb: timeline.at(-1)?.rssChrome ?? 0,
    chrome_processes_end: chromeCount(),
  },
  errors: errors.slice(0, 100),
  errorCount: errors.length,
  timeline,
};

writeFileSync(resolve(args.out), JSON.stringify(report, null, 2));

console.log("\n===== SOAK REPORT =====");
console.log(`duration          ${report.durationSec}s (${report.iterations} iterations)`);
console.log(`tool calls        ${report.toolCalls} (${report.throughput_per_sec}/s)`);
console.log(`error rate        ${report.errorCount} (${((report.errorCount / Math.max(1, report.toolCalls)) * 100).toFixed(2)}%)`);
console.log(`latency p50/p95   ${report.overall.p50_ms}ms / ${report.overall.p95_ms}ms  max ${report.overall.max_ms}ms`);
console.log(
  `latency drift     first-tenth p50 ${report.drift.first_tenth_p50_ms}ms -> last-tenth p50 ${report.drift.last_tenth_p50_ms}ms (${report.drift.drift_ratio}x)`
);
console.log(`tokens total      ${report.overall.total_tokens}`);
console.log(
  `memory            fb ${report.memory.rss_flashbrowse_start_kb}kB -> ${report.memory.rss_flashbrowse_end_kb}kB | chrome ${report.memory.rss_chrome_start_kb}kB -> ${report.memory.rss_chrome_end_kb}kB`
);
console.log(`chrome procs      ${report.memory.chrome_processes_end} at exit`);
console.log(`report written    ${resolve(args.out)}`);

const errorFail = report.errorCount > report.toolCalls * 0.05;
const driftFail =
  report.drift.drift_ratio != null && report.drift.drift_ratio > 1.2;
const rssStart = report.memory.rss_flashbrowse_start_kb || 1;
const rssEnd = report.memory.rss_flashbrowse_end_kb || rssStart;
const rssFail = rssEnd > rssStart * 1.5;
if (errorFail) console.error("soak FAILED: error rate above 5%");
if (driftFail) {
  console.error(
    `soak FAILED: latency drift ${report.drift.drift_ratio}x exceeds 1.2x`
  );
}
if (rssFail) {
  console.error(
    `soak FAILED: flashbrowse RSS grew from ${rssStart}kB to ${rssEnd}kB (> 1.5x)`
  );
}
process.exit(errorFail || driftFail || rssFail ? 1 : 0);

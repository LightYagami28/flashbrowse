#!/usr/bin/env node
/**
 * Waits for competing jobs, then runs long sequential workloads.
 */

import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const evalDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(evalDir, "..");

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function run(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      stdio: "inherit",
      cwd: opts.cwd ?? root,
      env: process.env,
    });
    child.on("exit", (code) => (code === 0 ? resolve() : reject(new Error(`${cmd} exit ${code}`))));
  });
}

async function withFixtureServer(fn) {
  const fixtures = path.join(root, "crates/flashbrowse-core/tests/fixtures");
  const server = spawn("python3", ["-m", "http.server", "8099", "--bind", "127.0.0.1", "--directory", fixtures], {
    stdio: "ignore",
  });
  await sleep(500);
  try {
    await fn();
  } finally {
    server.kill("SIGTERM");
  }
}

async function waitForPgrep(pattern, label) {
  for (;;) {
    const ok = await new Promise((resolve) => {
      const c = spawn("pgrep", ["-f", pattern], { stdio: ["ignore", "pipe", "ignore"] });
      let out = "";
      c.stdout.on("data", (d) => (out += d));
      c.on("close", () => {
        const lines = out.trim().split("\n").filter(Boolean);
        const hit = lines.some((l) => !l.includes("long-runs.mjs"));
        resolve(hit);
      });
    });
    if (!ok) {
      console.log(`${label}: idle`);
      return;
    }
    console.log(`${label}: still running, wait 60s...`);
    await sleep(60_000);
  }
}

console.log("long-runs: waiting for soak.mjs and run-batch.mjs to finish...");
await waitForPgrep("soak.mjs", "soak");
await waitForPgrep("run-batch.mjs", "run-batch");

console.log("\n========== soak 90 min (mixed real + fixtures) ==========");
await withFixtureServer(() =>
  run("node", [
    path.join(root, "scripts/soak.mjs"),
    "--minutes",
    "90",
  "--url",
  "http://127.0.0.1:8099/table.html",
  "--url",
  "http://127.0.0.1:8099/many-buttons.html",
  "--url",
  "https://news.ycombinator.com",
  "--url",
  "https://the-internet.herokuapp.com/tables",
  "--url",
  "https://developer.mozilla.org/en-US/docs/Web/HTML",
  "--url",
  "https://doc.rust-lang.org/std/",
  "--url",
  "https://example.com",
  "--out",
    path.join(evalDir, "results/soak-90min-mixed.json"),
  ]),
);

console.log("\n========== MCP marathon (3 cycles x real-public URLs) ==========");
await run(
  "node",
  [
    path.join(evalDir, "mcp-marathon.mjs"),
    "--cycles",
    "3",
    "--urls",
    path.join(evalDir, "urls/real-public.txt"),
    "--out",
    path.join(evalDir, "results/mcp-marathon-3x.json"),
  ],
  { cwd: evalDir },
);

console.log("\n========== fidelity real-public (re-run) ==========");
const urls = await import("node:fs/promises").then((fs) => fs.readFile(path.join(evalDir, "urls/real-public.txt"), "utf8"));
const urlArgs = urls
  .split("\n")
  .map((l) => l.replace(/#.*$/, "").trim())
  .filter(Boolean)
  .flatMap((u) => ["--url", u]);
await run(
  "node",
  [
    path.join(root, "scripts/fidelity.mjs"),
    "--binary",
    path.join(root, "target/release/flashbrowse"),
    "--out",
    path.join(evalDir, "results/fidelity-real-public-rerun.json"),
    "--timeout",
    "240000",
    ...urlArgs,
  ],
  { cwd: root },
);

console.log("\n========== agent batch extended (repeats=3 pilot A/B) ==========");
await run("node", [path.join(evalDir, "run-batch.mjs"), "--manifest", path.join(evalDir, "batches/agent-sweep-long.json")], {
  cwd: evalDir,
});

console.log("\n========== summarize ==========");
await run("node", [path.join(evalDir, "summarize-runs.mjs")], { cwd: evalDir });

console.log("long-runs: done");
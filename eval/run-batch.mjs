#!/usr/bin/env node
/**
 * Sequential agent batch runs for more data. One browser per task (via run.mjs).
 *
 *   node run-batch.mjs --manifest batches/agent-sweep.json
 */

import { readFile, writeFile, mkdir } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const evalDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(evalDir, "..");
const binary = path.join(root, "target/release/flashbrowse");

function parseArgs(argv) {
  let manifest = path.join(evalDir, "batches/agent-sweep.json");
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === "--manifest") {
      const m = argv[++i];
      manifest = path.isAbsolute(m) ? m : path.join(evalDir, m);
    }
  }
  return { manifest };
}

function runNode(args, { cwd = evalDir, fixtures = false } = {}) {
  const cmd = fixtures
    ? ["with-fixtures.mjs", "--", "node", "run.mjs", ...args]
    : ["run.mjs", ...args];
  return new Promise((resolve, reject) => {
    const child = spawn("node", cmd, { stdio: "inherit", cwd, env: process.env });
    child.on("exit", (code) => (code === 0 ? resolve() : reject(new Error(`exit ${code}`))));
  });
}

const { manifest } = parseArgs(process.argv);
const batch = JSON.parse(await readFile(manifest, "utf8"));
const log = [];

await mkdir(path.join(evalDir, "results"), { recursive: true });

for (const job of batch.jobs) {
  const out = path.isAbsolute(job.out)
    ? job.out
    : path.join(evalDir, "results", job.out);
  const tasksDir = path.isAbsolute(job.tasks)
    ? job.tasks
    : path.join(evalDir, job.tasks);
  const args = [
    "--binary",
    binary,
    "--tasks",
    tasksDir,
    "--arm",
    job.arm ?? "a",
    "--repeats",
    String(job.repeats ?? 1),
    "--out",
    out,
    "--model",
    job.model ?? "nvidia/nemotron-3.5-super-vl:free",
    "--max-turns",
    String(job.max_turns ?? 8),
  ];
  if (job.base) args.push("--base", job.base);
  else if (job.fixtures) args.push("--base", `http://127.0.0.1:${process.env.FLASHBROWSE_FIXTURE_PORT ?? "8099"}`);
  if (job.dry_run) args.push("--dry-run");

  console.log(`\n=== batch: ${job.id} ===`);
  const t0 = Date.now();
  try {
    await runNode(args, { fixtures: job.fixtures === true });
    log.push({ id: job.id, ok: true, ms: Date.now() - t0, out });
  } catch (e) {
    log.push({ id: job.id, ok: false, ms: Date.now() - t0, error: e.message, out });
    console.error(job.id, e.message);
  }
}

const summaryPath = path.join(evalDir, "results/agent-batch-summary.json");
await writeFile(summaryPath, JSON.stringify({ manifest, at: new Date().toISOString(), log }, null, 2));
console.log(`wrote ${summaryPath}`);
#!/usr/bin/env node
/**
 * Sequential open probe: dump axtree, record size, latency, and errors per URL.
 * stdin or --urls file, one URL per line (# comments allowed).
 */

import { execFile } from "node:child_process";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { promisify } from "node:util";
import path from "node:path";

const run = promisify(execFile);

function parseArgs(argv) {
  const opts = {
    urlsFile: null,
    binary: "./target/release/flashbrowse",
    out: "eval/results/probe-urls.json",
    timeout: 180_000,
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--urls") opts.urlsFile = argv[++i];
    else if (a === "--binary") opts.binary = argv[++i];
    else if (a === "--out") opts.out = argv[++i];
    else if (a === "--timeout") opts.timeout = Number(argv[++i]);
  }
  if (!opts.urlsFile) {
    console.error("usage: node probe-urls.mjs --urls <file> [--binary ...] [--out ...]");
    process.exit(2);
  }
  return opts;
}

async function loadUrls(file) {
  const text = await readFile(file, "utf8");
  return text
    .split("\n")
    .map((l) => l.replace(/#.*$/, "").trim())
    .filter(Boolean);
}

async function probeOne(binary, url, timeout) {
  const started = Date.now();
  const row = { url, ok: false, ms: 0, treeChars: 0, actionCount: 0, error: null };
  try {
    const { stdout, stderr } = await run(
      binary,
      ["dump", url, "--format", "axtree"],
      { maxBuffer: 64 * 1024 * 1024, timeout },
    );
    row.ms = Date.now() - started;
    row.treeChars = stdout.length;
    row.actionCount = (stdout.match(/^\[\d+\]/gm) ?? []).length;
    row.ok = !stderr?.includes("Error:");
    if (stdout.includes("chrome-error://") || /error/i.test(stdout.slice(0, 200))) {
      row.ok = false;
      row.error = "tree suggests error page";
    }
  } catch (e) {
    row.ms = Date.now() - started;
    row.error = e.killed ? "timeout" : (e.stderr ?? e.message ?? String(e)).slice(0, 500);
  }
  return row;
}

const opts = parseArgs(process.argv);
const urls = await loadUrls(opts.urlsFile);
await mkdir(path.dirname(opts.out), { recursive: true });

const results = [];
for (const url of urls) {
  const row = await probeOne(opts.binary, url, opts.timeout);
  results.push(row);
  console.log(`${row.ok ? "OK" : "FAIL"}\t${row.ms}ms\t${row.actionCount} actions\t${url}`);
  if (row.error) console.log(`  ${row.error.split("\n")[0]}`);
}

const summary = {
  generatedAt: new Date().toISOString(),
  n: results.length,
  failed: results.filter((r) => !r.ok).length,
  p50_ms: results.map((r) => r.ms).sort((a, b) => a - b)[Math.floor(results.length / 2)],
  max_actions: Math.max(...results.map((r) => r.actionCount)),
};
await writeFile(opts.out, JSON.stringify({ summary, results }, null, 2));
console.log(`wrote ${opts.out}`);
#!/usr/bin/env node
/** Measure observation payload sizes per eval arm on each fixture (sequential). */

import { execFile } from "node:child_process";
import { readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { chromium } from "playwright";

const run = promisify(execFile);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

function parseArgs(argv) {
  let base = "http://127.0.0.1:8099";
  let binary = path.join(__dirname, "..", "target", "release", "flashbrowse");
  let out = path.join(__dirname, "results", "arm-sizes.json");
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === "--base") base = argv[++i];
    else if (argv[i] === "--binary") binary = argv[++i];
    else if (argv[i] === "--out") out = argv[++i];
  }
  return { base, binary, out };
}

const fixturesDir = path.join(__dirname, "..", "crates/flashbrowse-core/tests/fixtures");
const fixtures = (await readdir(fixturesDir)).filter((f) => f.endsWith(".html"));
const { base, binary, out } = parseArgs(process.argv);
const rows = [];

for (const fixture of fixtures) {
  const url = `${base.replace(/\/$/, "")}/${fixture}`;
  const entry = { fixture, url, arms: {} };
  for (const [arm, args] of [
    ["a", ["dump", url, "--format", "axtree"]],
    ["b", ["dump", url, "--format", "axtree-nested"]],
    ["c", ["dump", url, "--format", "axtree-nested", "--no-prune"]],
    ["d", ["dump", url, "--format", "axtree", "--no-prune"]],
  ]) {
    const { stdout } = await run(binary, args, { maxBuffer: 64 * 1024 * 1024 });
    const body = stdout.split("---\n").slice(1).join("---\n");
    const actions = (body.match(/^\[\d+\]/gm) ?? []).length;
    entry.arms[arm] = { chars: stdout.length, body_chars: body.length, actions };
  }
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  await page.goto(url, { waitUntil: "domcontentloaded" });
  const snap = await page.locator("body").ariaSnapshot();
  await browser.close();
  entry.arms.e = { chars: snap.length, body_chars: snap.length, actions: 0 };
  rows.push(entry);
  console.log(fixture, entry.arms.a.chars, entry.arms.b.chars, entry.arms.e.chars);
}

await writeFile(out, JSON.stringify({ measured_at: new Date().toISOString(), rows }, null, 2));
console.log(`wrote ${out}`);
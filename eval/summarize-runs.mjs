#!/usr/bin/env node
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const resultsDir = path.join(path.dirname(fileURLToPath(import.meta.url)), "results");

const files = (await readdir(resultsDir)).filter(
  (f) => f.endsWith(".json") && !f.includes("fidelity") && !f.includes("probe") && !f.includes("soak"),
);

const rows = [];
for (const f of files) {
  const j = JSON.parse(await readFile(path.join(resultsDir, f), "utf8"));
  if (!j.runs) continue;
  const pass = j.runs.filter((r) => r.success).length;
  const model = j.opts?.dryRun ? "dry-run" : (j.opts?.model ?? "?");
  const arm = j.opts?.arm ?? "?";
  rows.push({
    file: f,
    model,
    arm,
    pass,
    total: j.runs.length,
    rate: Number((pass / j.runs.length).toFixed(3)),
    repeats: j.opts?.repeats ?? 1,
  });
}

rows.sort((a, b) => a.file.localeCompare(b.file));
const out = path.join(resultsDir, "agent-summary.json");
await writeFile(out, JSON.stringify({ at: new Date().toISOString(), rows }, null, 2));
console.table(rows);
console.log(`wrote ${out}`);
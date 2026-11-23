#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";

function parseArgs(argv) {
  let input = "results/run.json";
  let output = "EVAL-pilot.md";
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === "--in") input = argv[++i];
    else if (argv[i] === "--out") output = argv[++i];
  }
  return { input, output };
}

const { input, output } = parseArgs(process.argv);
const data = JSON.parse(await readFile(input, "utf8"));
const runs = data.runs ?? [];

const byArm = new Map();
for (const r of runs) {
  const k = r.arm;
  if (!byArm.has(k)) byArm.set(k, { ok: 0, n: 0 });
  const row = byArm.get(k);
  row.n++;
  if (r.success) row.ok++;
}

let md = "# flashbrowse eval (generated)\n\n";
md += "| Arm | Pass | Total | Rate |\n|-----|------|-------|------|\n";
for (const [arm, row] of byArm) {
  const rate = row.n ? ((100 * row.ok) / row.n).toFixed(1) : "0";
  md += `| ${arm} | ${row.ok} | ${row.n} | ${rate}% |\n`;
}
md += "\n## Per run\n\n";
for (const r of runs) {
  md += `- ${r.task_id} arm=${r.arm} rep=${r.repeat} success=${r.success} turns=${r.turns} ${r.failure_category ?? ""}\n`;
}

await writeFile(output, md);
console.log(`wrote ${output}`);
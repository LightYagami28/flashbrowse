#!/usr/bin/env node
/**
 * MCP session marathon: N x browser_open across a URL list on one serial server.
 */

import { readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { McpClient } from "./lib/mcp.js";

const evalDir = path.dirname(fileURLToPath(import.meta.url));

function parseArgs(argv) {
  const opts = {
    urlsFile: path.join(evalDir, "urls/real-public.txt"),
    binary: path.join(evalDir, "..", "target/release/flashbrowse"),
    cycles: 3,
    out: path.join(evalDir, "results/mcp-marathon.json"),
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--urls") opts.urlsFile = argv[++i];
    else if (a === "--binary") opts.binary = argv[++i];
    else if (a === "--cycles") opts.cycles = Number(argv[++i]);
    else if (a === "--out") opts.out = argv[++i];
  }
  return opts;
}

const opts = parseArgs(process.argv);
const urls = (await readFile(opts.urlsFile, "utf8"))
  .split("\n")
  .map((l) => l.replace(/#.*$/, "").trim())
  .filter(Boolean);

const mcp = new McpClient(opts.binary);
await mcp.initialize();

const runs = [];
const started = Date.now();
let errors = 0;

for (let c = 0; c < opts.cycles; c++) {
  for (const url of urls) {
    const t0 = Date.now();
    let ok = true;
    let err = null;
    let chars = 0;
    try {
      const tree = await mcp.callTool("browser_open", { url });
      chars = tree.length;
    } catch (e) {
      ok = false;
      err = e.message ?? String(e);
      errors++;
    }
    runs.push({ cycle: c, url, ok, ms: Date.now() - t0, chars, error: err });
    console.log(`${ok ? "OK" : "FAIL"}\t${Date.now() - t0}ms\t${url}`);
  }
}

mcp.close();
await mkdir(path.dirname(opts.out), { recursive: true });
await writeFile(
  opts.out,
  JSON.stringify(
    {
      cycles: opts.cycles,
      urls: urls.length,
      total: runs.length,
      errors,
      wall_ms: Date.now() - started,
      runs,
    },
    null,
    2,
  ),
);
console.log(`wrote ${opts.out}, errors=${errors}`);
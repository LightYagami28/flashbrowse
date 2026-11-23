#!/usr/bin/env node
/**
 * Long-running breaking-point sweep (sequential). Writes under eval/results/.
 *
 *   node break-suite.mjs --phase fidelity-real
 *   node break-suite.mjs --phase all
 */

import { spawn } from "node:child_process";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const evalDir = path.join(root, "eval");
const binary = path.join(root, "target/release/flashbrowse");

function parseArgs(argv) {
  let phase = "all";
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === "--phase") phase = argv[++i];
  }
  return { phase };
}

function run(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      stdio: "inherit",
      cwd: opts.cwd ?? root,
      env: { ...process.env, ...opts.env },
    });
    child.on("exit", (code) => (code === 0 ? resolve() : reject(new Error(`${cmd} ${args.join(" ")} => ${code}`))));
  });
}

async function loadUrlFile(name) {
  const p = path.join(evalDir, "urls", name);
  return (await readFile(p, "utf8"))
    .split("\n")
    .map((l) => l.replace(/#.*$/, "").trim())
    .filter(Boolean);
}

async function fidelityBatch(urls, out) {
  const args = ["scripts/fidelity.mjs", "--binary", binary, "--out", out, "--timeout", "180000"];
  for (const u of urls) args.push("--url", u);
  await run("node", args);
}

async function phaseFidelityLocal() {
  await withFixtures(async () => {
    const urls = await loadUrlFile("fixtures-local.txt");
    await fidelityBatch(urls, path.join(evalDir, "results/fidelity-local-extended.json"));
  });
}

async function phaseFidelityReal() {
  const urls = await loadUrlFile("real-public.txt");
  await fidelityBatch(urls, path.join(evalDir, "results/fidelity-real-public.json"));
}

async function phaseProbeReal() {
  await run("node", [
    path.join(evalDir, "probe-urls.mjs"),
    "--urls",
    path.join(evalDir, "urls/real-public.txt"),
    "--binary",
    binary,
    "--out",
    path.join(evalDir, "results/probe-real-public.json"),
    "--timeout",
    "180000",
  ]);
}

async function phaseProbeLocal() {
  await run("node", [
    path.join(evalDir, "probe-urls.mjs"),
    "--urls",
    path.join(evalDir, "urls/fixtures-local.txt"),
    "--binary",
    binary,
    "--out",
    path.join(evalDir, "results/probe-fixtures-local.json"),
  ]);
}

async function dryRunTasks(tasksDir, out) {
  await run(
    "node",
    [
      path.join(evalDir, "run.mjs"),
      "--dry-run",
      "--base",
      "http://127.0.0.1:8099",
      "--binary",
      binary,
      "--tasks",
      tasksDir,
      "--out",
      out,
    ],
    { cwd: evalDir },
  );
}

async function llmTasks(tasksDir, arm, model, out) {
  if (!process.env.OPENROUTER_API_KEY && !process.env.OPENAI_API_KEY) {
    console.log("skip LLM: no API key");
    return;
  }
  await run(
    "node",
    [
      path.join(evalDir, "run.mjs"),
      "--base",
      "http://127.0.0.1:8099",
      "--binary",
      binary,
      "--tasks",
      tasksDir,
      "--arm",
      arm,
      "--model",
      model,
      "--repeats",
      "1",
      "--out",
      out,
    ],
    { cwd: evalDir },
  );
}

async function withFixtures(fn) {
  const fixtures = path.join(root, "crates/flashbrowse-core/tests/fixtures");
  const server = spawn("python3", ["-m", "http.server", "8099", "--bind", "127.0.0.1", "--directory", fixtures], {
    stdio: "ignore",
  });
  await new Promise((r) => setTimeout(r, 500));
  try {
    await fn();
  } finally {
    server.kill("SIGTERM");
  }
}

const log = [];
async function record(phase, status, note = "") {
  log.push({ at: new Date().toISOString(), phase, status, note });
  await mkdir(path.join(evalDir, "results"), { recursive: true });
  await writeFile(path.join(evalDir, "results/break-suite-log.jsonl"), log.map((x) => JSON.stringify(x)).join("\n") + "\n");
}

const { phase } = parseArgs(process.argv);

const phases = {
  "fidelity-local": phaseFidelityLocal,
  "fidelity-real": phaseFidelityReal,
  "probe-local": () => withFixtures(phaseProbeLocal),
  "probe-real": phaseProbeReal,
  "dry-stress": () =>
    withFixtures(() => dryRunTasks(path.join(evalDir, "tasks/stress-local"), path.join(evalDir, "results/dry-stress-local.json"))),
  "dry-real": () =>
    dryRunTasks(path.join(evalDir, "tasks/real-web"), path.join(evalDir, "results/dry-real-web.json")),
  "llm-stress": () =>
    withFixtures(() =>
      llmTasks(
        path.join(evalDir, "tasks/stress-local"),
        "b",
        "nvidia/nemotron-3.5-super-vl:free",
        path.join(evalDir, "results/llm-stress-local-arm-b.json"),
      ),
    ),
  "llm-real": () =>
    llmTasks(
      path.join(evalDir, "tasks/real-web"),
      "a",
      "nvidia/nemotron-3.5-super-vl:free",
      path.join(evalDir, "results/llm-real-web-arm-a.json"),
    ),
  "stress-rust": async () => {
    await run("cargo", ["test", "-p", "flashbrowse-core", "--test", "stress", "--", "--nocapture"], { cwd: root });
  },
  "soak-90": async () => {
    await withFixtures(async () => {
      await run(
        "node",
        [
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
        ],
        { cwd: root },
      );
    });
  },
  "soak-15": async () => {
    await withFixtures(async () => {
      await run(
        "node",
        [
          path.join(root, "scripts/soak.mjs"),
          "--minutes",
          "15",
          "--url",
          "http://127.0.0.1:8099/table.html",
          "--url",
          "https://news.ycombinator.com",
          "--url",
          "https://the-internet.herokuapp.com/tables",
          "--url",
          "https://example.com",
          "--out",
          path.join(evalDir, "results/soak-15min-mixed.json"),
        ],
        { cwd: root },
      );
    });
  },
};

const order = [
  "probe-local",
  "fidelity-local",
  "dry-stress",
  "llm-stress",
  "probe-real",
  "fidelity-real",
  "dry-real",
  "llm-real",
  "stress-rust",
  "soak-15",
];

async function main() {
  const todo = phase === "all" ? order : [phase];
  for (const name of todo) {
    const fn = phases[name];
    if (!fn) {
      console.error(`unknown phase ${name}`);
      process.exit(2);
    }
    console.log(`\n========== ${name} ==========`);
    try {
      await fn();
      await record(name, "pass");
    } catch (e) {
      await record(name, "fail", e.message);
      console.error(name, e.message);
      if (phase !== "all") process.exit(1);
    }
  }
}

main();
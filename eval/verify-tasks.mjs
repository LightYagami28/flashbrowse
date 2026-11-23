#!/usr/bin/env node
/**
 * Load every task JSON, open the fixture in Playwright, and verify that
 * success criteria are structurally valid. Does not run an agent.
 */

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { checkSuccess } from "./lib/success.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function parseArgs(argv) {
  let base = "http://127.0.0.1:8099";
  let tasksDir = path.join(__dirname, "tasks");
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === "--base") base = argv[++i];
    else if (argv[i] === "--tasks") tasksDir = argv[++i];
  }
  return { base, tasksDir };
}

async function loadTasks(dir) {
  const names = (await readdir(dir)).filter((n) => n.endsWith(".json"));
  const tasks = [];
  for (const n of names) {
    const raw = await readFile(path.join(dir, n), "utf8");
    tasks.push(JSON.parse(raw));
  }
  return tasks;
}

const { base, tasksDir } = parseArgs(process.argv);

const tasks = await loadTasks(tasksDir);
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
let failures = 0;

for (const task of tasks) {
  const url = `${base.replace(/\/$/, "")}/${task.fixture}`;
  await page.goto(url, { waitUntil: "domcontentloaded" });
  for (const c of task.success ?? []) {
    const count = await page.locator(c.selector).count();
    if (count === 0) {
      console.error(`FAIL ${task.id}: missing selector ${c.selector}`);
      failures++;
      continue;
    }
  }
  if (!(task.success ?? []).length) {
    console.error(`FAIL ${task.id}: no success criteria`);
    failures++;
    continue;
  }
  console.log(`OK   ${task.id} (selectors present)`);
}

await browser.close();
process.exit(failures > 0 ? 1 : 0);
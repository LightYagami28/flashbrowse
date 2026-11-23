#!/usr/bin/env node
/**
 * MCP agent loop for task eval. Arm A uses the live MCP tree; other arms can
 * swap the observation payload via CLI dump (same URL, fresh browser).
 *
 * Agent runs need OPENROUTER_API_KEY or OPENAI_API_KEY (OpenRouter supports
 * free models via --model). Use --dry-run to exercise MCP + scoring only.
 */

import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { McpClient } from "./lib/mcp.js";
import { observeDump } from "./lib/observe.js";
import { checkSuccessMcp } from "./lib/success-mcp.js";
import { chat, defaultModel } from "./lib/llm.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const SYSTEM = `You control a browser via flashbrowse MCP tools.
Call browser_open once, then browser_act with numeric action ids from the tree.
When the goal is done, reply with exactly: DONE
Temperature 0. Use only the tools provided.`;

function parseArgs(argv) {
  const opts = {
    base: process.env.FLASHBROWSE_FIXTURE_BASE ?? "http://127.0.0.1:8099",
    binary: path.join(__dirname, "..", "target", "release", "flashbrowse"),
    tasksDir: path.join(__dirname, "tasks", "pilot"),
    arm: "a",
    repeats: 1,
    out: path.join(__dirname, "results", "run.json"),
    dryRun: false,
    model: defaultModel(),
    maxTurns: 8,
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--base") opts.base = argv[++i];
    else if (a === "--binary") opts.binary = argv[++i];
    else if (a === "--tasks") opts.tasksDir = argv[++i];
    else if (a === "--arm") opts.arm = argv[++i];
    else if (a === "--repeats") opts.repeats = Number(argv[++i]);
    else if (a === "--out") opts.out = argv[++i];
    else if (a === "--dry-run") opts.dryRun = true;
    else if (a === "--model") opts.model = argv[++i];
    else if (a === "--max-turns") opts.maxTurns = Number(argv[++i]);
  }
  return opts;
}

async function loadTasks(dir) {
  const names = (await readdir(dir)).filter((n) => n.endsWith(".json"));
  const out = [];
  for (const n of names) {
    out.push(JSON.parse(await readFile(path.join(dir, n), "utf8")));
  }
  return out;
}

const MCP_TOOLS = [
  {
    type: "function",
    function: {
      name: "browser_open",
      description: "Open a URL and return the accessibility tree",
      parameters: {
        type: "object",
        properties: { url: { type: "string" } },
        required: ["url"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "browser_act",
      description: "click, fill, select, hover, scroll, press_key",
      parameters: {
        type: "object",
        properties: {
          action_id: { type: "number" },
          action: { type: "string" },
          value: { type: "string" },
        },
        required: ["action_id", "action"],
      },
    },
  },
];

async function armObservation({ arm, binary, url, mcp, treeFromMcp }) {
  if (arm === "e") {
    const browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(url, { waitUntil: "domcontentloaded" });
    const snap = await page.locator("body").ariaSnapshot();
    await browser.close();
    return snap;
  }
  if (arm === "a") return treeFromMcp;
  return observeDump({ binary, url, arm });
}

const opts = parseArgs(process.argv);
const tasks = await loadTasks(opts.tasksDir);
await mkdir(path.dirname(opts.out), { recursive: true });

const runs = [];

function resolveTaskUrl(task, opts) {
  if (task.url) return task.url;
  if (!task.fixture) throw new Error(`task ${task.id} needs url or fixture`);
  return `${opts.base.replace(/\/$/, "")}/${task.fixture}`;
}

for (const task of tasks) {
  const url = resolveTaskUrl(task, opts);
  for (let rep = 0; rep < opts.repeats; rep++) {
    const started = Date.now();
    const mcp = new McpClient(opts.binary);
    await mcp.initialize();
    let turns = 0;
    let success = false;
    let failure = null;
    let inputTokens = 0;
    let outputTokens = 0;

    try {
      const openTree = await mcp.callTool("browser_open", { url });
      if (opts.dryRun) {
        const score = await checkSuccessMcp(mcp, task.success);
        success = score.ok;
        failure = success ? null : score.failures.join("; ");
      } else {
        const messages = [
          { role: "system", content: SYSTEM },
          {
            role: "user",
            content: `Goal: ${task.goal}\n\nPage observation:\n${await armObservation({
              arm: opts.arm,
              binary: opts.binary,
              url,
              mcp,
              treeFromMcp: openTree,
            })}`,
          },
        ];
        while (turns < (task.max_turns ?? opts.maxTurns)) {
          turns++;
          const msg = await chat(opts.model, messages, MCP_TOOLS);
          inputTokens += msg.usage?.prompt_tokens ?? 0;
          outputTokens += msg.usage?.completion_tokens ?? 0;
          if (/^done\b/i.test(msg.content?.trim() ?? "")) break;
          if (msg.tool_calls?.length) {
            for (const call of msg.tool_calls) {
              const args = JSON.parse(call.function.arguments);
              const text = await mcp.callTool(call.function.name, args);
              messages.push(msg);
              messages.push({
                role: "tool",
                tool_call_id: call.id,
                content:
                  opts.arm === "a"
                    ? text
                    : await armObservation({
                        arm: opts.arm,
                        binary: opts.binary,
                        url,
                        mcp,
                        treeFromMcp: text,
                      }),
              });
            }
          } else {
            messages.push(msg);
          }
        }
        const score = await checkSuccessMcp(mcp, task.success);
        success = score.ok;
        failure = success ? null : score.failures.join("; ");
      }
    } catch (e) {
      failure = e.message ?? String(e);
    } finally {
      mcp.close();
    }

    runs.push({
      task_id: task.id,
      arm: opts.arm,
      repeat: rep,
      success,
      turns,
      input_tokens: inputTokens,
      output_tokens: outputTokens,
      wall_ms: Date.now() - started,
      failure_category: failure,
    });
    console.log(
      `${task.id} rep=${rep} arm=${opts.arm} success=${success} turns=${turns}`,
    );
  }
}

await writeFile(opts.out, JSON.stringify({ runs, opts }, null, 2));
console.log(`wrote ${opts.out}`);
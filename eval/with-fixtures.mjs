#!/usr/bin/env node
/**
 * Run a command with the fixture HTTP server up for the whole child lifetime.
 *
 *   node with-fixtures.mjs -- node run.mjs --dry-run ...
 */

import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixtures = path.join(
  __dirname,
  "..",
  "crates/flashbrowse-core/tests/fixtures",
);

const argv = process.argv.slice(2);
const dash = argv.indexOf("--");
if (dash < 0 || dash === argv.length - 1) {
  console.error("usage: node with-fixtures.mjs -- <command> [args...]");
  process.exit(2);
}
const cmd = argv.slice(dash + 1);

let port = Number(process.env.FLASHBROWSE_FIXTURE_PORT ?? "8099");
if (process.env.FLASHBROWSE_FIXTURE_PORT == null) {
  for (const p of [8099, 8098, 8097]) {
    try {
      const { createServer } = await import("node:net");
      await new Promise((resolve, reject) => {
        const s = createServer();
        s.once("error", reject);
        s.listen(p, "127.0.0.1", () => {
          s.close(resolve);
        });
      });
      port = p;
      break;
    } catch {
      /* try next */
    }
  }
}
process.stdout.write(`fixture server on http://127.0.0.1:${port}\n`);
const server = spawn(
  "python3",
  ["-m", "http.server", String(port), "--bind", "127.0.0.1", "--directory", fixtures],
  { stdio: "inherit" },
);

function shutdown(code) {
  server.kill("SIGTERM");
  process.exit(code ?? 0);
}

process.on("SIGINT", () => shutdown(130));
process.on("SIGTERM", () => shutdown(143));

await new Promise((resolve, reject) => {
  server.on("error", reject);
  setTimeout(resolve, 400);
});

const child = spawn(cmd[0], cmd.slice(1), {
  stdio: "inherit",
  env: {
    ...process.env,
    FLASHBROWSE_FIXTURE_PORT: String(port),
    FLASHBROWSE_FIXTURE_BASE: `http://127.0.0.1:${port}`,
  },
  cwd: process.cwd(),
});

child.on("exit", (code) => shutdown(code ?? 0));
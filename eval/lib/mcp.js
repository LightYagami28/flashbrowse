import { spawn } from "node:child_process";
import readline from "node:readline";

/** Minimal MCP stdio client (JSON-RPC, one response per id). */
export class McpClient {
  #proc;
  #rl;
  #pending = new Map();
  #nextId = 1;

  constructor(binary, env = {}) {
    this.#proc = spawn(binary, ["mcp"], {
      stdio: ["pipe", "pipe", "inherit"],
      env: { ...process.env, ...env },
    });
    this.#rl = readline.createInterface({ input: this.#proc.stdout });
    this.#rl.on("line", (line) => this.#onLine(line));
  }

  #onLine(line) {
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      return;
    }
    if (msg.id != null && this.#pending.has(msg.id)) {
      const { resolve, reject } = this.#pending.get(msg.id);
      this.#pending.delete(msg.id);
      if (msg.error) reject(new Error(msg.error.message ?? JSON.stringify(msg.error)));
      else resolve(msg.result);
    }
  }

  request(method, params) {
    const id = this.#nextId++;
    const frame = JSON.stringify({ jsonrpc: "2.0", id, method, params });
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      this.#proc.stdin.write(`${frame}\n`);
    });
  }

  notify(method, params = {}) {
    const frame = JSON.stringify({ jsonrpc: "2.0", method, params });
    this.#proc.stdin.write(`${frame}\n`);
  }

  async initialize() {
    await this.request("initialize", {
      protocolVersion: "2025-06-18",
      capabilities: {},
      clientInfo: { name: "flashbrowse-eval", version: "0.1.0" },
    });
    // JSON-RPC notification: no id, no response.
    this.notify("notifications/initialized", {});
  }

  async callTool(name, arguments_ = {}) {
    const result = await this.request("tools/call", {
      name,
      arguments: arguments_,
    });
    const text =
      result?.content?.find((c) => c.type === "text")?.text ??
      result?.content?.[0]?.text ??
      "";
  if (result?.isError) throw new Error(text || "tool error");
    return text;
  }

  close() {
    this.#proc.stdin.end();
    this.#proc.kill("SIGTERM");
  }
}
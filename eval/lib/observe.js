import { execFile } from "node:child_process";
import { promisify } from "node:util";

const run = promisify(execFile);

/**
 * Fetch an observation string for eval arms that use CLI dump.
 *
 * @param {object} opts
 * @param {string} opts.binary
 * @param {string} opts.url
 * @param {"a"|"b"|"c"|"d"} opts.arm
 */
export async function observeDump({ binary, url, arm }) {
  const args = ["dump", url];
  if (arm === "b") args.push("--format", "axtree-nested");
  else if (arm === "c") args.push("--format", "axtree-nested", "--no-prune");
  else if (arm === "d") args.push("--format", "axtree", "--no-prune");
  else args.push("--format", "axtree");

  const { stdout } = await run(binary, args, {
    maxBuffer: 32 * 1024 * 1024,
    timeout: 120_000,
  });
  return stdout;
}
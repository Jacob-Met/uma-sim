import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

export const entry = process.env.UMA_TUI_UNDER_TEST ?? fileURLToPath(new URL("../tui.js", import.meta.url));

export async function until(check, describe, timeout = 10000) {
  const deadline = Date.now() + timeout;
  while (!check()) {
    if (Date.now() >= deadline) throw new Error("Timed out: " + describe());
    await delay(5);
  }
}

export function launch(t, url, args) {
  const child = spawn(process.execPath, [entry, ...args], {
    env: { ...process.env, UMA_SIM_API: url }, stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "", stderr = "", ended = false;
  child.stdout.on("data", (data) => { stdout += data; });
  child.stderr.on("data", (data) => { stderr += data; });
  const completion = once(child, "close").then(([code, signal]) => {
    ended = true;
    return { code, signal, stdout, stderr };
  });
  t.after(async () => {
    if (!ended) child.kill("SIGKILL");
    await completion;
  });
  const describe = () => JSON.stringify({ stdout, stderr, ended });
  return {
    child,
    get stdout() { return stdout; },
    get stderr() { return stderr; },
    async prompt(count = 1) {
      await until(() => ended || (stdout.match(/\n> /g) ?? []).length >= count, describe);
      assert.equal(ended, false, describe());
    },
    send(command) { child.stdin.write(command + "\n"); },
    async finish() { await until(() => ended, describe); return completion; },
    async quit() { this.send("quit"); return this.finish(); },
  };
}

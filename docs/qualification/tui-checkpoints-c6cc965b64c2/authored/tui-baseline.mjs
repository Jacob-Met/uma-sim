#!/usr/bin/env node
/** Terminal client for one explicitly addressed uma-sim career session. */
import readline from "node:readline";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const API = (process.env.UMA_SIM_API ?? "http://127.0.0.1:8765").replace(/\/+$/, "");
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const usage = `Usage: node packages/uma-sim-cli/tui.js [seed] [scenario] [--new]
       node packages/uma-sim-cli/tui.js --session=<id>
       node packages/uma-sim-cli/tui.js --list-sessions

Without --session, start a fresh, uniquely named terminal career (seed 42, URA).
--new makes this intent explicit; it cannot be combined with --session.
--session resumes an existing named career without restarting or activating it.
--list-sessions lists the server's live careers without changing them.
Commands: [choice] | auto | fast | state | quit
`;

function options(args) {
  const positional = [];
  let session;
  let fresh = false;
  let list = false;
  let help = false;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--help" || arg === "-h") help = true;
    else if (arg === "--new") fresh = true;
    else if (arg === "--list-sessions") list = true;
    else if (arg === "--session" || arg.startsWith("--session=")) {
      if (session !== undefined) throw new Error("Specify --session only once.");
      session = arg === "--session" ? args[++i] : arg.slice("--session=".length);
      // Empty means the mutable active session in older APIs. Never send it.
      if (!session || (arg === "--session" && session.startsWith("--"))
        || !/^[A-Za-z0-9_.-]{1,64}$/.test(session) || session === "." || session === "..") {
        throw new Error("--session needs a named session id: 1–64 letters, digits, '.', '_' or '-'. Use --list-sessions to find one.");
      }
    } else if (arg.startsWith("--")) throw new Error(`Unknown option '${arg}'. Use --help.`);
    else positional.push(arg);
  }
  if (help) return { help: true };
  if (positional.length > 2) throw new Error("Expected at most a seed and scenario. Use --help.");
  if (session !== undefined && (fresh || list || positional.length)) {
    throw new Error("--session resumes a career; do not combine it with --new, --list-sessions, a seed or a scenario.");
  }
  if (list && (fresh || positional.length)) throw new Error("--list-sessions cannot be combined with career creation options.");
  const seed = positional[0] ?? "42";
  if (!/^[+-]?\d+$/.test(seed) || BigInt(seed) < -(1n << 63n) || BigInt(seed) >= (1n << 63n)) {
    throw new Error("Seed must be a signed 64-bit integer.");
  }
  return { session, list, seed, scenario: positional[1] ?? "ura" };
}

class ApiError extends Error {
  constructor(message, { cause, uncertain = false } = {}) {
    super(message, { cause });
    this.uncertain = uncertain;
  }
}

async function api(method, route, body, signal) {
  let res;
  let raw;
  try {
    res = await fetch(`${API}${route}`, {
      method,
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal,
    });
    raw = await res.text();
  } catch (cause) {
    throw new ApiError(`${method} ${route}: ${cause.message}`, { cause, uncertain: method === "POST" });
  }
  let value;
  try {
    value = JSON.parse(raw);
  } catch (cause) {
    throw new ApiError(`${method} ${route}: HTTP ${res.status}, invalid JSON response`, {
      cause, uncertain: method === "POST",
    });
  }
  if (!res.ok) {
    const detail = typeof value?.error === "string" ? `: ${value.error}` : "";
    throw new ApiError(`${method} ${route}: HTTP ${res.status}${detail}`);
  }
  return value;
}

async function health(signal) {
  const result = await api("GET", "/v1/health", undefined, AbortSignal.any([signal, AbortSignal.timeout(2000)]));
  if (result?.ok !== true) throw new ApiError("The configured API did not return a valid uma-sim health response.");
}

function refused(error) {
  return error?.code === "ECONNREFUSED" || error?.errors?.some(refused) || (error?.cause && refused(error.cause));
}

async function connect(canStart, signal) {
  try {
    await health(signal);
    return;
  } catch (error) {
    const url = new URL(API);
    const local = url.protocol === "http:" && ["127.0.0.1", "localhost"].includes(url.hostname)
      && url.pathname === "/" && !url.search && !url.hash && !url.username && !url.password;
    if (!canStart || !local || !refused(error) || signal.aborted) throw error;
  }
  const exe = process.platform === "win32" ? "uma-sim-api.exe" : "uma-sim-api";
  const candidates = ["target/release", "target/debug", "uma-sim-core/target/release", "uma-sim-core/target/debug"]
    .map((dir) => path.join(repoRoot, dir, exe));
  const binary = candidates.find((candidate) => fs.existsSync(candidate));
  if (!binary) {
    throw new Error("uma-sim-api binary not found; run `cargo build --release -p uma-sim-core` from the repo root, "
      + `or start \`uma-sim serve\` yourself (API: ${API}).`);
  }
  console.log("Starting REST API…");
  const child = spawn(binary, [new URL(API).port || "80"], { cwd: repoRoot, detached: true, stdio: "ignore" });
  await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("spawn", resolve);
  });
  child.unref();
  for (let i = 0; i < 30; i++) {
    await delay(1000, undefined, { signal });
    try {
      await health(signal);
      return;
    } catch (error) {
      if (signal.aborted || !refused(error)) throw error;
    }
  }
  throw new Error("REST API did not start");
}

function sidebar(snapshot) {
  const s = snapshot?.state ?? snapshot;
  if (!s?.stats) return "(no state)";
  const st = s.stats;
  const resources = s.scenarioResources?.values ?? {};
  const resLine = Object.keys(resources).length ? `\n  Scenario: ${JSON.stringify(resources)}` : "";
  return [
    `Turn ${s.turn}  Y${s.date?.year} M${s.date?.month}`,
    `SPD ${st.speed} STA ${st.stamina} POW ${st.power}`,
    `GUT ${st.guts} WIT ${st.wit}`,
    `E ${s.energy}  Mood ${s.mood}  Fans ${s.fans}  SP ${s.skillPoints}`,
    `Phase: ${s.phase}${resLine}`,
  ].join("\n");
}

function report(error, session) {
  console.error(error.message);
  if (error.uncertain) {
    console.error(`The request may have reached the server. Resume with --session=${session} and inspect state before retrying; no action was retried automatically.`);
  }
}

async function main() {
  const config = options(process.argv.slice(2));
  if (config.help) {
    console.log(usage);
    return;
  }
  const controller = new AbortController();
  const signal = controller.signal;
  const session = config.session ?? `tui-${randomUUID()}`;
  // Install the iterator before doing I/O so early input and EOF are observed.
  const rl = config.list ? undefined : readline.createInterface({ input: process.stdin, output: process.stdout });
  const lines = rl?.[Symbol.asyncIterator]();
  const interrupt = () => {
    process.exitCode = 130;
    controller.abort();
    rl?.close();
  };
  process.on("SIGINT", interrupt);
  rl?.on("SIGINT", interrupt);
  rl?.on("close", () => controller.abort());
  const read = (route) => api("GET", `${route}?session=${encodeURIComponent(session)}`, undefined, signal);
  const act = (route, body) => api("POST", route, { ...body, session }, signal);
  try {
    await connect(!config.session && !config.list, signal);
    if (config.list) {
      const result = await api("GET", "/v1/sessions", undefined, signal);
      if (!Array.isArray(result?.sessions)) throw new Error("Invalid session list from API.");
      if (!result.sessions.length) console.log("No live career sessions.");
      for (const item of result.sessions) {
        const marker = item.id === result.active ? " *" : "";
        console.log(`${item.id || "(main)"}${marker}\t${item.label}\tturn ${item.turn}\t${item.scenarioId}\t${item.traineeName}`);
      }
      return;
    }
    if (config.session) {
      await read("/v1/run/state"); // A missing session is fatal; never fall back or start over.
      console.log(`Resuming session: ${session}`);
    } else {
      await act("/v1/run/start", {
        seed: config.seed, scenario: config.scenario, trainee: "Special Week", speed: "1", traceTelemetry: "true",
      });
      console.log(`New session: ${session}`);
    }
    console.log(`Resume this career with --session=${session}`);
    console.log("uma-sim TUI — [choice] | auto | fast | state | quit\n");
    while (!signal.aborted) {
      let snapshot;
      try {
        snapshot = await read("/v1/run/state");
        const text = await read("/v1/run/text");
        const choices = await read("/v1/run/choices");
        console.log("\n┌─ State ─────────────────────");
        console.log(sidebar(snapshot));
        console.log("└─────────────────────────────");
        console.log((text.text ?? "").split("\n").slice(-8).join("\n"));
        if (choices.choices?.length) {
          console.log("\nChoices:", choices.choices.map((c) => `${c.id} (${String(c.label ?? c.id).slice(0, 30)})`).join(" | "));
        }
      } catch (error) {
        if (signal.aborted) break;
        snapshot = undefined;
        report(error, session);
      }
      if (signal.aborted) break;
      process.stdout.write("\n> ");
      const next = await lines.next();
      if (next.done || signal.aborted) break;
      const command = next.value.trim();
      if (command === "quit" || command === "q") break;
      try {
        if (command === "auto") await act("/v1/run/auto", { policy: "bot" });
        else if (command === "fast") await act("/v1/run/fast", { multiplier: "100" });
        else if (command === "state") {
          console.log(snapshot ? JSON.stringify(snapshot, null, 2) : "State unavailable; refreshing the selected session.");
        } else if (command) await act("/v1/run/action", { action: command });
      } catch (error) {
        report(error, session);
      }
    }
  } catch (error) {
    if (!signal.aborted || error.uncertain) report(error, session);
    if (!signal.aborted) process.exitCode = 1;
  } finally {
    rl?.close();
    process.removeListener("SIGINT", interrupt);
  }
}

main().catch((error) => {
  console.error(error.message);
  process.exitCode = 1;
});

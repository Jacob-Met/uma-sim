#!/usr/bin/env node
/**
 * Polished TUI for uma-sim REST API — sidebar stats + event log.
 */
import readline from "readline";
import { spawn } from "child_process";
import path from "path";
import { fileURLToPath } from "url";

const API = process.env.UMA_SIM_API ?? "http://127.0.0.1:8765";
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../..");

async function api(method, route, body) {
  const res = await fetch(`${API}${route}`, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  return res.json();
}

function sidebar(state) {
  const s = state?.state ?? state;
  if (!s?.stats) return "(no state)";
  const st = s.stats;
  const res = s.scenarioResources?.values ?? {};
  const resLine = Object.keys(res).length ? `\n  Scenario: ${JSON.stringify(res)}` : "";
  return [
    `Turn ${s.turn}  Y${s.date?.year} M${s.date?.month}`,
    `SPD ${st.speed} STA ${st.stamina} POW ${st.power}`,
    `GUT ${st.guts} WIT ${st.wit}`,
    `E ${s.energy}  Mood ${s.mood}  Fans ${s.fans}  SP ${s.skillPoints}`,
    `Phase: ${s.phase}${resLine}`,
  ].join("\n");
}

async function ensureApi() {
  const fs = await import("fs");
  const rustCandidates = [
    path.join(repoRoot, "uma-sim-core", "target", "release", process.platform === "win32" ? "uma-sim-api.exe" : "uma-sim-api"),
    path.join(repoRoot, "uma-sim-core", "target", "debug", process.platform === "win32" ? "uma-sim-api.exe" : "uma-sim-api"),
  ];
  const rustBin = rustCandidates.find((p) => fs.existsSync(p));
  if (rustBin) {
    spawn(rustBin, [], { cwd: repoRoot, detached: true, stdio: "ignore" }).unref();
  } else {
    const legacy = path.join(repoRoot, "legacy", "uma-sim-kotlin");
    const gradlew = path.join(legacy, process.platform === "win32" ? "gradlew.bat" : "gradlew");
    spawn(gradlew, [":sim-engine:runSimApi"], {
      cwd: legacy,
      shell: true,
      detached: true,
      stdio: "ignore",
    }).unref();
  }
  for (let i = 0; i < 30; i++) {
    await new Promise((r) => setTimeout(r, 1000));
    try {
      await fetch(`${API}/v1/run/state`);
      return;
    } catch {}
  }
  throw new Error("REST API did not start");
}

async function main() {
  try {
    await fetch(`${API}/v1/run/state`);
  } catch {
    console.log("Starting REST API…");
    await ensureApi();
  }

  const seed = process.argv[2] ?? "42";
  const scenario = process.argv[3] ?? "ura";
  await api("POST", "/v1/run/start", {
    seed,
    scenario,
    trainee: "Special Week",
    speed: "1",
    traceTelemetry: "true",
  });

  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  console.log("uma-sim TUI — [choice] | auto | fast | state | quit\n");

  const loop = async () => {
    const snap = await api("GET", "/v1/run/state");
    const text = await api("GET", "/v1/run/text");
    const choices = await api("GET", "/v1/run/choices");
    console.log("\n┌─ State ─────────────────────");
    console.log(sidebar(snap));
    console.log("└─────────────────────────────");
    console.log((text.text ?? "").split("\n").slice(-8).join("\n"));
    if (choices.choices?.length) {
      console.log("\nChoices:", choices.choices.map((c) => `${c.id} (${c.label.slice(0, 30)})`).join(" | "));
    }
    rl.question("\n> ", async (line) => {
      const cmd = line.trim();
      if (cmd === "quit" || cmd === "q") {
        rl.close();
        return;
      }
      if (cmd === "auto") await api("POST", "/v1/run/auto", { policy: "bot" });
      else if (cmd === "fast") await api("POST", "/v1/run/fast", { multiplier: "100" });
      else if (cmd === "state") console.log(JSON.stringify(snap, null, 2));
      else if (cmd) await api("POST", "/v1/run/action", { action: cmd });
      loop();
    });
  };
  loop();
}

main().catch((e) => {
  console.error(e.message);
  process.exit(1);
});

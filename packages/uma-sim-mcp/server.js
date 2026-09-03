/**
 * MCP-style HTTP bridge to uma-sim REST API (desktop).
 * Start the Rust API: cargo run --manifest-path uma-sim-core/Cargo.toml --bin uma-sim-api
 * Or legacy JVM: cd legacy/uma-sim-kotlin && gradlew :sim-engine:runSimApi
 */
const API = process.env.UMA_SIM_API ?? "http://127.0.0.1:8765";

async function post(path, body = {}) {
  const res = await fetch(`${API}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return res.json();
}

async function get(path) {
  const res = await fetch(`${API}${path}`);
  return res.json();
}

const tools = {
  sim_start: (seed = 42, scenario = "ura", trainee = "Special Week", speed = 1, legacyFactors = "") =>
    post("/v1/run/start", {
      seed: String(seed),
      scenario,
      trainee,
      speed: String(speed),
      legacyFactors,
      traceTelemetry: "true",
    }),
  sim_state: () => get("/v1/run/state"),
  sim_text: () => get("/v1/run/text"),
  sim_choices: () => get("/v1/run/choices"),
  sim_act: (action) => post("/v1/run/action", { action }),
  sim_auto: (policy = "bot") => post("/v1/run/auto", { policy }),
  sim_fast_forward: (multiplier = 100) => post("/v1/run/fast", { multiplier: String(multiplier) }),
  sim_export_telemetry: () => get("/v1/run/telemetry"),
};

const cmd = process.argv[2];
const arg = process.argv[3];
(async () => {
  if (!cmd || cmd === "help") {
    console.log("Usage: node server.js <command> [arg]");
    console.log("Commands:", Object.keys(tools).join(", "));
    console.log("Requires: uma-sim-api (or legacy runSimApi) on", API);
    process.exit(0);
  }
  let out;
  switch (cmd) {
    case "sim_start":
      out = await tools.sim_start(arg ? Number(arg) : 42);
      break;
    case "sim_state":
      out = await tools.sim_state();
      break;
    case "sim_text":
      out = await tools.sim_text();
      break;
    case "sim_choices":
      out = await tools.sim_choices();
      break;
    case "sim_act":
      out = await tools.sim_act(arg ?? "train_speed");
      break;
    case "sim_auto":
      out = await tools.sim_auto(arg ?? "bot");
      break;
    case "sim_fast":
    case "sim_fast_forward":
      out = await tools.sim_fast_forward(arg ? Number(arg) : 100);
      break;
    case "sim_export_telemetry":
      out = await tools.sim_export_telemetry();
      break;
    default:
      console.error("Unknown command", cmd);
      process.exit(1);
  }
  console.log(JSON.stringify(out, null, 2));
})();

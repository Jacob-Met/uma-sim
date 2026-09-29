#!/usr/bin/env node
/**
 * Runs the Rust `uma-sim` CLI from a source checkout.
 * Build it first: `cargo build --release -p uma-sim-core` (repo root).
 * Usage: node run.js start --seed=42 --speed=20
 */
import { spawnSync } from "child_process";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../..");
const args = process.argv.slice(2);
const exe = process.platform === "win32" ? "uma-sim.exe" : "uma-sim";

// Cargo workspace builds land in <repo>/target; per-crate builds in uma-sim-core/target.
const rustCandidates = [
  path.join(repoRoot, "target", "release", exe),
  path.join(repoRoot, "target", "debug", exe),
  path.join(repoRoot, "uma-sim-core", "target", "release", exe),
  path.join(repoRoot, "uma-sim-core", "target", "debug", exe),
];

const rustBin = rustCandidates.find((p) => fs.existsSync(p));
if (!rustBin) {
  console.error(
    "uma-sim binary not found. From the repo root run:\n" +
      "  cargo build --release -p uma-sim-core\n" +
      "Looked in:\n  " +
      rustCandidates.join("\n  "),
  );
  process.exit(1);
}

const result = spawnSync(rustBin, args, { cwd: repoRoot, stdio: "inherit", shell: false });
process.exit(result.status ?? 1);

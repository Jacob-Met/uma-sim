#!/usr/bin/env bash
# Smoke-test a staged release folder the way a user runs it: the source tree's
# data folders are hidden, and the binary is started from an unrelated working
# directory. Catches data paths baked in at compile time.
#
# Usage: scripts/smoke_release_layout.sh <stage-dir> <binary-name>
#   e.g. scripts/smoke_release_layout.sh dist/uma-sim-linux-x64 uma-sim
set -euo pipefail

STAGE=$(cd "$1" && pwd)
BIN="$STAGE/$2"
REPO=$(cd "$(dirname "$0")/.." && pwd)
WORK=$(mktemp -d)

restore() {
  for d in research knowledge; do
    if [ -d "$REPO/$d.smoke-hidden" ]; then mv "$REPO/$d.smoke-hidden" "$REPO/$d"; fi
  done
  rm -rf "$WORK"
}
trap restore EXIT

for d in research knowledge; do mv "$REPO/$d" "$REPO/$d.smoke-hidden"; done

cd "$WORK"
for model in stub physics; do
  echo "== uma-sim fast --seed=42 --race-model=$model (cwd: $WORK)"
  out=$("$BIN" fast --seed=42 --race-model="$model" 2>&1) || { echo "$out"; exit 1; }
  echo "$out" | tail -n 2
  echo "$out" | grep -q "career=true turn=72" || { echo "career did not finish"; exit 1; }
done
echo "release layout OK"

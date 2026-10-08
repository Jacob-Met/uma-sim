#!/bin/sh
# Stub external policy server for uma-sim REST regression test T6
# (`policy_external_positive_path_with_stub`).
#
# Protocol (uma-sim-core/src/policy_external.rs): NDJSON on stdin/stdout.
#   {"cmd":"ping"} -> reply must contain "ok":true
#   choose request -> {"kind":"<race|rest|train|choose|lesson>","payload":"..."}
#   {"cmd":"quit"} -> exit (lets the Drop impl reap the child cleanly)
#
# The engine refuses non-race actions on mandatory-race turns and non-choose
# actions on event turns, so the stub answers those phases with a valid
# action and trains GUTS on free turns. The built-in default heuristic trains
# SPEED (policy.rs: `default_auto_policy`), so a GUTS step proves the REST
# layer honored the external policy instead of silently downgrading.
while IFS= read -r line; do
  case "$line" in
    *ping*) printf '%s\n' '{"ok":true}' ;;
    *quit*) exit 0 ;;
    *)
      lower=$(printf '%s' "$line" | tr '[:upper:]' '[:lower:]')
      case "$lower" in
        *mandatory*) printf '%s\n' '{"kind":"race"}' ;;
        *event_*) printf '%s\n' '{"kind":"choose","payload":"0"}' ;;
        *) printf '%s\n' '{"kind":"train","payload":"guts"}' ;;
      esac
      ;;
  esac
done

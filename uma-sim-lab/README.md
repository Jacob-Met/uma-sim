# uma-sim-lab — saved-career branch-and-compare laboratory

A career library for uma-sim: save named, durable checkpoints of a career,
fork any checkpoint into independent branches, play each branch forward with
its own RNG stream, and compare branches side by side.

## Why

"Save before a meaningful choice, play two independent continuations, resume
either and compare actual decision/turn/outcome differences." The engine
already had `RunSnapshot` export/restore; the lab adds the durable named
library, branch isolation, per-branch RNG binding, and comparison on top —
without touching game mechanics.

## Layout

- `checkpoints/<name>.json` — full `RunSnapshot` + visible seed / scenario /
  trainee / turn / RNG-call context.
- `branches/<name>.json` — branch head snapshot: a deep copy of the
  checkpoint at fork time. A branch's play steps only ever rewrite its own
  head; the checkpoint and sibling branches are never mutated.
- `timelines/<branch>.json` — recorded per-action timeline of the last play.

Library dir: `$UMA_LAB_DIR`, else `~/.uma-sim/lab`.

## CLI (`uma-lab`)

```
start <checkpoint> [--seed=N] [--scenario=ura] [--trainee=NAME] [--turns=N] [--note=...]
list [--branches]
show <checkpoint>
status <branch>
fork <checkpoint> <branch> [--note=...]
play <branch> [--policy=default|speed|stamina|restful|racer] [--turns=N]
step <branch> <action-id>        # restore-next-choice single step
compare <branch-a> <branch-b> [--out=FILE]
export <checkpoint> <file>      # portable checkpoint JSON
import <file>
export-branch <branch> <file>
delete [--branch] <name>
```

## Example

```sh
uma-lab start choice-point --seed=20261003 --turns=10 --note="speed vs stamina?"
uma-lab fork choice-point speedster
uma-lab fork choice-point marathoner
uma-lab play speedster --policy=speed --turns=25
uma-lab play marathoner --policy=restful --turns=25
uma-lab compare speedster marathoner
```

## Guarantees (tested)

- **Checkpoint fidelity**: restore reproduces state and RNG position
  (seed + call count); identical checkpoint + identical actions reproduce
  state and RNG evolution bit-for-bit (timeline content hash).
- **Branch isolation**: playing a branch cannot mutate its checkpoint of
  origin or any sibling (byte-compared in tests).
- **Per-branch RNG**: each fork starts from the checkpoint's exact RNG
  position and evolves an independent stream per branch.
- **Comparison**: first divergence (step + field) plus a side-by-side
  per-turn table and outcome summaries; reproducible.

Run: `cargo test -p uma-sim-lab` (6 acceptance tests).

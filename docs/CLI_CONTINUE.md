# Continue a saved career

Run `uma-sim continue` from the directory containing your existing
`.uma-sim/session.json`. It loads that career and finishes it with the local
default policy. Use `uma-sim continue --policy=bot` for the local scoring policy.

For example:

```sh
uma-sim start --seed=73 --scenario=unity --race-model=stub --deck=10001
uma-sim step rest
uma-sim continue --policy=bot
uma-sim state
```

The command resumes the stored seed, scenario, trainee, deck, legacy factors,
turn, prior log, settings and RNG checkpoint. It does not call `start`, replace
saved settings with CLI defaults, or select an external policy. Options such as
`--seed`, `--scenario`, `--speed` and `--policy=external` are rejected before
loading or writing a save. Exactly zero or one `--policy=default|bot` option is
accepted. `continue --help` displays help without reading a career.
`UMA_POLICY_CMD` does not override either local continuation policy, and
`UMA_RACE_MODEL` does not replace the race model already saved in the career.

| Command | Saved-career behavior |
| --- | --- |
| `start` | Starts a new career and saves it. |
| `fast` | Starts a new career, plays it, and replaces the saved career. |
| `continue` | Loads the existing career and saves its completed continuation. |

A completed save is an exact byte-preserving no-op. Missing or undecodable
saves, inconsistent saved metadata and unknown saved scenarios return exit 1
without replacing the file. An invalid command option returns exit 2.
Successful completion and an already completed career return exit 0.

Continuation retains the existing playback bound: a 500-action limit checked
between batches, using the saved speed multiplier clamped to 1–100. A final
batch can cross that limit. If the engine does not reach completion, the
command reports failure and keeps the previous on-disk save.
A successfully completed run is published through the existing `RunSession`
writer; ordinary success output appears only after publication succeeds. The
writer preserves the previous save after handled write, sync or rename errors.
Concurrent commands remain unsynchronized, and this is not a power-loss
durability guarantee.

The command retains the simulator's existing replay limits. Current snapshots
carry the engine's RNG state words; older snapshots without those words use
the existing best-effort seed/call-count restoration. The engine also has
pre-existing factor-selection nondeterminism across fresh processes.
Preserving the saved seed does not promise identical full-career outcomes
across separate runs.

Native source qualification is recorded in
`docs/qualification/cli-continue-3dc74ef3b04d/`. This additive command consumes
the qualified session writer from PR #118 without changing its implementation.

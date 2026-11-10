# Native local bot stat targets: independent held-out evaluation

**Qualified acceptance:** assigning effective stat caps as the local bot's default targets fixes zero-valued stat gains and improves the measured terminal objective. This is **not evidence of stronger racing**.

## Frozen comparison

- Baseline source: `fda3c78f1673a038bf38e12a581340d06b4a572a`; binary SHA-256 `8a118340573ea7e09a09bf33790fc2423affb3aa5f4268053025869a9089c218`.
- Candidate source: `e0a720aae76ef6cce13a0e36c5e0a00d9330f1d9`; tree `02db4f31a4837e779cc231a223167643c24e8d49`; binary SHA-256 `79d663a1f8c452f16e9d67ec44a273cf6892ffe719812b3f6b05ddf1b1ef9c8a`.
- Only production change: `config.stat_targets = config.stat_caps.clone()` in the native local bot adapter. The evaluator and catalogs were unchanged.
- Both binaries used the same baseline runtime catalogs, physics races and local `--policy=bot`. Child environments removed `UMA_POLICY_CMD`, `UMA_REPO_ROOT` and `UMA_RACE_MODEL`.
- Producer and independent evaluator were separate workers. Candidate was frozen before any candidate hold-out execution or disclosure. The original baseline was not rerun.

Primary cohort: 64 previously untouched random seeds, Special Week, empty deck, all four scenarios (256 careers). Secondary cohort: the first 32 of those seeds, Sakura Bakushin O and six fixed catalog cards (128 careers). Contexts share seeds and are **not 384 independent random samples**. Full seed lists and deck IDs are now disclosed in outcomes.json and must not be reused as unseen validation for future policy tuning.

Primary seed-file commitment: `8a1761ba793336947784bfb191ac3cdb8be88f24e211ace8367fecc2caecc37e`. Secondary: `ef43ac859ec4637cafb6e300b0f930ae977e25e580e4ac5291b270e3d7084b3e`.

## Results

| Context | Scenario | N | Mean score delta | Improved / regressed / tied | Mean score before shop delta |
|---|---|---:|---:|---:|---:|
| Special Week, empty | URA | 64 | +259.656 | 51 / 13 / 0 | +343.391 |
| Special Week, empty | Grand Live | 64 | +94.375 | 50 / 13 / 1 | +114.281 |
| Special Week, empty | Unity | 64 | +532.281 | 64 / 0 / 0 | +532.281 |
| Special Week, empty | Trackblazer | 64 | +127.766 | 62 / 2 / 0 | +127.766 |
| Sakura, six cards | URA | 32 | +632.438 | 31 / 1 / 0 | +643.125 |
| Sakura, six cards | Grand Live | 32 | +139.719 | 21 / 11 / 0 | +221.250 |
| Sakura, six cards | Unity | 32 | +454.406 | 30 / 2 / 0 | +448.031 |
| Sakura, six cards | Trackblazer | 32 | +170.531 | 29 / 3 / 0 | +170.063 |

Across the predefined mixture, 338 improved, 45 regressed and one tied; mean score delta +285.4375. Mean native inheritance utility U was nonnegative in every cell, unchanged in both Trackblazer cells. Detailed U, blue/grade utility, 600/1100 brackets, skill points and per-stat deltas are retained.

The scorer converts remaining skill points into score at a simplified flat gold-skill rate and uses fixed all-A rank aptitudes. Score is not a race-win measure. Empty-deck Unity mean Speed decreased 574.656 while Stamina increased 692.734. The policy still concentrates almost exclusively on one facility.

## Actual trajectory check and failures

After pairing exact (seed, scenario, trainee) records, the native `compare-batches` command was run for all eight cells. Worst, median and best score-delta pairs per cell were replayed through the actual native `fast` route: **48 complete 72-turn careers**. Every replay's final stats and terminal score matched its batch record.

All parsed race results in those selected replays were ninth place: **696 baseline races and 695 candidate races, zero wins on both sides**. This selected sample is diagnostic, not an unbiased race-rate estimate.

The worst primary URA pair lost **1,305 score**. Candidate recorded nine training failures and 16 successful training actions; baseline recorded two failures and 22 successes. The full logs are retained, including the loss and all ninth-place results.

Read-only causal inspection found optional racing occurs before recovery in `choose_action_local`. The scheduler checks free phase, turns 8–55 and fans below the scenario target; it does not compare energy, readiness or expected value. Repeated low-fan finishes can therefore keep the bot racing while useful training is delayed. Recovery and race scheduling remain substantive open gaps.

## Cost and scope

Measured entire native batch subprocesses, including startup, policy decisions, races, terminal evaluation and output. Sequential runs on the same Renpō host; no GitHub Actions.

| 384-career evaluation | Wall seconds | User + system CPU seconds |
|---|---:|---:|
| Baseline | 22.0186 | 22.0071 |
| Candidate | 20.4092 | 20.3997 |

These are observed full costs, not an isolated throughput benchmark. The Grand Live action-path change accounts for much of the difference. Per-cell timings and the additional replay costs are included. No rollout/search budget is hidden.

## Reproduce and inspect

`outcomes.json` contains both raw terminal records, fixed contexts/seeds, binary/cohort hashes, timing measurements, paired summaries and all 48 replay logs. `trajectory-audit.json` contains compact race/training/failure counts. `SHA256SUMS` verifies the evidence files.

Build each pinned source with the same toolchain/features and embedded UI; place each binary beside the **baseline** research/, knowledge/ and content_packs/ catalogs. The published baseline preview supplies an already runnable baseline. Then:

```sh
python3 reproduce.py /absolute/baseline/uma-sim /absolute/candidate/uma-sim /absolute/new-output-directory
```

The script calls the real batch consumer with identical seeds/context, verifies expected row identities, and runs native paired comparisons. The output directory must not exist. Rebuilding may change binary bytes; recorded hashes identify the actual evaluated artifacts. The now-public seeds support reproduction, not a fresh held-out claim.

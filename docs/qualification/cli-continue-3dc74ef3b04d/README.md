# Native CLI continuation qualification

The new `uma-sim continue [--policy=default|bot]` command finishes an existing local saved career. The command uses RunSession load/restore and the qualified PR #118 writer unchanged. It never calls start, alters stored settings, or selects an external policy.

## Source authority and scope

- Repository: Jacob-Met/uma-sim.
- Qualified dependency: PR #118, commit `248c62564bb816630d6983ec5f733b965a8e1e43`, tree `7cab649fb15bb8e7071d66eab329aba5f5cd68e9`; its real main parent is `38291c1aaf915da17229eca0afbbd58d9b3ce453`.
- Native baseline `80a1aaa420dbb1b4683aad3b25732ba0897d8d28` is a 302-leaf exact component projection of that dependency. It does not establish whole-repository ancestry. Every input Git blob, mode and SHA256 is recorded in component-source-manifest.json.
- Product freeze `6f6c325445cb0f52b193dbddf7c379ad0c3ffa43` changes four paths: the new module, five CLI hook/help lines, actual-process tests and the user guide. Writer, engine, RNG, API, manifests and lock are unchanged.
- Current main was last observed as `ac4a4c2d4b4159bea1012e66cb79e76b6bc25d56`. Its CLI and writer differ from this dependency. Full current-main compatibility is pending; this packet is not a safe blind overlay onto current main.
- Consumer scope notification: https://github.com/Jacob-Met/hamon/issues/140#issuecomment-6069947046. This is a coordination notification, not peer approval.
- Native-only handoff: Actions-triggering publication is held under https://github.com/Jacob-Met/hamon/issues/143#issuecomment-6067592767. GitHub access also stopped on the shared 403 rate limit; no alternate transport was used.

## Observed native results

The unmodified baseline built offline with the canonical lock in 84 seconds using a private target and copied public package index/archive inputs. Four authored actual-process continuation regressions failed on that baseline as expected. A separate authored-save control shows unsupported continue emits general help with status 0 and keeps the save unchanged, while fast replaces its seed/scenario/deck identity. Its receipt's literal word-Usage probe is false because old help begins “uma-sim CLI v0.4”; the retained output shows the actual help.

Candidate Cargo testing was not started because its disk admission check found 218,374,144 available bytes below a 471,859,200-byte threshold. A smaller native method compiled only the CLI and two integration test binaries against exact unchanged baseline dependency libraries. The first method omitted the existing CLI's serde extern and failed to compile; its invocation, errors and receipt are preserved. Correcting that invocation alone produced:
- 14/14 new continuation tests passing.
- 10/10 unchanged PR #118 persistence tests passing.
- All three scoped native compilations exit 0, with empty compiler stderr.
- rustfmt --check on the three changed Rust files exits 0.
- 301 existing component inputs unchanged; only the CLI is an edited existing input.
- Candidate executable SHA256 `db18c2878f4627ed14d247ed26e1e94327a3d14b6aadc600d407c3292f67ab4a`.

New cases cover noninitial default and bot continuation, metadata/settings/deck/log/RNG progression, exact terminal bytes, missing/corrupt saves, unknown and duplicate options, invalid saved identity/scenario, bounded nonprogress, old snapshots, handled write failure without success output and existing symlink behavior. No real player's save is used.

## Reproduction and limitations

With sufficient local disk and the exact dependency inputs, run:
```sh
cargo test --offline --locked -j1 -p uma-sim-core --test cli_continue --test cli_session_persistence -- --test-threads=1
```
This Cargo command is the portable intended gate, not the command claimed to have completed for this candidate. The completed native rustc commands, library hashes, environment inputs, output hashes and executable pin are in scoped-v2/candidate-scoped-native-receipt.json and its unchanged method file. Use a private target and temporary directory.

Qualification covers the Linux native CLI and the exact PR118 component composition. It does not claim current-main integration, Windows behavior, full-workspace coverage, native Clippy, hosted CI or deployment. Legacy RNG fallback, factor-selection nondeterminism across fresh processes, unsynchronized concurrent commands and writer durability limits remain explicit in CLI_CONTINUE.md. An action-limit failure leaves the previous disk save intact. The 500 limit is the existing engine parameter and uses its saved speed behavior.

Independent receiving, when complete, is recorded separately and does not rewrite author evidence.

# Durable terminal checkpoint receiving — c6cc965b64c2

## Delivered behavior

The Node terminal can save its selected live career with `save NAME`, inspect the API's durable library with `checkpoints` or `--list-checkpoints`, and reopen a saved career with `--checkpoint=NAME`.

Saving sends an explicit named session and `overwrite:false`. Reopening calls the existing checkpoint fork endpoint with a fresh UUID id; it never calls the replacing library-load endpoint. Existing live-session resume, new-career isolation, request targeting, uncertainty handling and EOF/SIGINT behavior from PR #72 remain intact.

The source boundary is seven files: terminal code, its package guide/script, three additive test/helper files, and thirteen lines in the existing Rust CI job. Native library/API, browser, MCP, simulation/catalogs, dependency manifests/locks and all other source remain unchanged. Issue #63 retains browser/session resolution; #66/#69 own MCP; #74 owns checkpoint replacement/storage. This contribution exposes no checkpoint overwrite operation.

## Exact source and receiver

- Receiving base: `b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c` (tree `619519ef197ff8bc75bd76273bf48fd1531f2377`).
- Frozen implementation: `3077775807c1eb5c2af6937038bd26cf5f00c75b`, direct child of that base.
- Original terminal blob: `e3fad690acbe62f43cf9eeab421da3c7b3b5818d`.
- API source blob: `4ff7034bc5701d793cd93d645d89d40d73fc3273`.
- Current library source blob: `4360cd3dbca832a9a22be138ca13a5deb4dfeb0f`, including the already integrated branch-publication preservation.
- The native executable was freshly built from that pinned source with installed Cargo/Rust 1.99.0, offline and locked, one build job, development profile with debug symbols disabled. No previous Linux binary was substituted.
- Native execution uses installed Node 26.3.0 on macOS. The required hosted gate uses Node 20 and its freshly built release API; that hosted result is separate from this authored receiving packet.

Exact source blobs, SHA-256 file records, full source-tree preservation counts, and executable identity are retained in `manifest.json`. The working directory is private for each API process, and `UMA_REPO_ROOT` points at the pinned checkout's real research and canonical catalogs. Fault proxies forward to that actual native API before deliberately dropping, corrupting or holding selected responses.

## Meaningful before and after

The receiver prepares a progressed seed-42 career at **turn 7, RNG calls 34**. On the unchanged PR #72 terminal, the proposed `save before-race` command is still an unrecognized game-choice string: the existing native unknown-action fallback applies a rest, producing **turn 8, RNG calls 38**, and no checkpoint. After a real API process stop/restart, the live id returns **404**, the library remains empty, and the new list option is unsupported. This is evidence of the missing terminal capability, not a claim that the previous terminal advertised a save command.

The candidate saves the full snapshot at **turn 7, RNG calls 34** without advancing or replacing any career. The saved snapshot and metadata remain byte-identical across process restart. The new terminal forks that checkpoint into a fresh printed session and restores the exact full snapshot. Three subsequent bot actions match every full recorded state both before and after restart and against a separately forked native oracle, ending at **turn 10, RNG calls 49**. The oracle becomes active while terminal actions continue to target their own id. Main, sibling and checkpoint bytes remain preserved.

See `authored/native-baseline-v2/restart.json` and `authored/native-final/restart.json` for complete snapshots and terminal output.

## Authored gates

| Gate | Result | What it establishes |
| --- | --- | --- |
| Selected HTTP baseline challenges | 0/4 pass | Missing fork/list/save interface; malformed save strings still dispatched as choices. |
| Candidate child-process HTTP suite | 35/35 pass, no skips | 17 preserved session cases plus 18 checkpoint cases, local argument guards, create-only save bodies, exact fork identity, response failures and cancellation. |
| Meaningful native restart baseline | 0/1 pass | Actual progressed career has no terminal-created checkpoint after server restart. |
| Final actual-native package entry point | 11/11 pass, no skips | Durable disk save/restart/reopen, full continuation/RNG replay, actual refusal statuses, advisories, lost responses and EOF/SIGINT. |
| Workflow syntax and diff whitespace | Pass | Existing CI receives the Node 20 native gate and artifact upload after its release API build. |

Native refusal evidence covers **404** for a missing checkpoint, **409** for an existing saved name, **422** for incompatible saved metadata, and **500** from a deliberately blocked private snapshot writer. Invalid command-line and interactive names are refused locally before an HTTP mutation. The fixture suite has additional deliberate HTTP response shapes; these are not assertions that the current native API emits every fixture response.

Each dropped/invalid native save or fork response was observed after an actual HTTP 200 from the backend. The terminal sends exactly one POST, reports uncertainty with its checkpoint/session identity, and retains the actual committed state. EOF and SIGINT are also exercised after native commit with a held response; they stop waiting and do not replay the operation.

## Preserved failed receiving attempts

The first native harness used repeated `rest` actions while the career was in its mandatory debut race. Those actions did not advance the career, making its apparent continuation equality inadequate. Its file observer also tried to read the deliberately created writer-blocking directory as a checkpoint, causing an EISDIR harness failure. That attempt's ten passing groups and one failing group are **not** used as the qualified progressed-state result.

The corrected harness prepares real bot progression, requires a nonzero consumed RNG position and an advancing continuation, compares three complete continuation states, and observes regular published checkpoint files. Its meaningful original-terminal restart challenge still fails; the final candidate passes all eleven native groups. The only final harness adjustment after v2 restores the saved metadata's actual original schema value instead of a literal version 1.

Raw first-attempt outputs and captures remain under `authored/native-candidate-v1/` and `authored/native-baseline/`. The exact v1/v2 receiver bytes were recovered and verified against their already-recorded provenance hashes, then preserved as `authored/receiver-attempt-v1.native.mjs` and `authored/receiver-attempt-v2.native.mjs`. No failed or inadequate result has been relabeled as a pass.

## Replay and limits

```sh
npm --prefix packages/uma-sim-cli run test:tui
cargo build --locked --release -p uma-sim-core --bin uma-sim-api
UMA_SIM_NATIVE_API="$PWD/target/release/uma-sim-api" \
  UMA_TUI_RECEIPTS="/tmp/uma-terminal-checkpoint-receipts" \
  npm --prefix packages/uma-sim-cli run test:tui:native
```

For the original-client counterexample, set `UMA_TUI_UNDER_TEST` to the retained original terminal and run only the `actual process restart` native test. The native test runner requires an existing explicit API binary and fails rather than silently skipping when it is absent.

The API library remains relative to its server working directory. Acknowledged checkpoints preserve that saved moment; later play and quitting do not autosave. Creating a checkpoint-derived live session makes it active under the existing API contract. Live sessions themselves still disappear on API restart. Broader library replacement/transaction behavior remains the existing native owner's scope.

This packet establishes authored source and native qualification. Independent receiving, actual hosted CI, repository integration and any runtime installation are separate states. It makes no deployment claim.

## Subsequent independent and current-API receiving

The immutable first independent packet is now retained under independent/: 16 groups pass against the frozen 3077775/b8e3 source and its actual native API. It adds distinct exact-i64, simultaneous-fork, competing-create-only-save, metadata writer rollback, restart, and loss/interrupt controls.

Current main fe2e6fdcdf7625cb5beac0e0062ace0360288a02 subsequently added request-body admission and other owners' UI work. The composition at 92db1618cc014924e43a69b2dfe4d5891680b176 preserves those owners and the exact terminal source. Its fresh native API passes the unchanged authored 11-group suite. Independent current receiving also passes all original 16 groups plus three new admission-boundary groups without product edits. See current-fe2/README.md and independent-current-fe2/REVIEW.md for the precise subjects, full evidence, source comparisons, and limitations. Earlier failed/preparation results and baseline binary identities remain unchanged.

## Provenance scanning

The official scanner reported four verified Git object identities in immutable receiving evidence. The narrowly scoped existing-config extension and actual path, value, and other-rule controls are retained in scanner/README.md. Original source, native output, and both independent packets remain preserved. This does not disable any default scanner rule.

## Current publication base

Current main a3e04846521c8877d112842332bc4825a29edaf3 adds the MCP laboratory integration. current-a3/ records the ordinary composition and exact native-input preservation. The final workflow reuses the owner's Node 20 setup and adds ten terminal native test/receipt lines; all prior per-stage source and receiving pins remain historical, distinct records.

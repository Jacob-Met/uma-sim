# Independent receiving: durable terminal on current request-body admission

## Result

Accepted source composition `92db1618cc014924e43a69b2dfe4d5891680b176`, parent `fe2e6fdcdf7625cb5beac0e0062ace0360288a02`, on the actual native API binary SHA-256 `b5f4c2c7af89751b38a2be4a646c6a2b0a183d376002394e80b485f9c81709a4`.

The original independent **16 groups pass without changed expectations**: 31 actual terminal processes, two API processes including a real restart, and 305 HTTP requests. A separate **three-group admission-boundary run also passes**, using three terminal processes, one API process, and 90 HTTP requests. No product changes were requested or made by this reviewer.

The earlier `3077775807c1eb5c2af6937038bd26cf5f00c75b` / `b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c` result remains immutable. Its receipt hash is `b217ca32937cbd8d7a488886799ce69950cece3ea69164973c298ae855afa199`; its original preparation negatives and 16-group result retain their original classification.

## Exact receiving subject

| Item | Pin |
| --- | --- |
| Composed source tree | `4ee456989b96c3557f0c167f6d53ccc6631b6cd2` |
| Terminal SHA-256 | `653acbb0b858d7bdc420904e26452e7739dffcc9c2aa73168b8fa801a1d63b4e` |
| Current owner API blob | `eab9ccf7082ffc96c446f434bd76f313aff14bc9` |
| Current owner API SHA-256 | `31e51bf33c6cd5f51c7f7cb79a4560fad5c2e06a77f0345ff659cf114fabdd55` |
| Original owner API blob | `4ff7034bc5701d793cd93d645d89d40d73fc3273` |
| Current owner CI blob, before terminal hook | `247b92e6b0f2a4e6242435e228cf057bdf724ebd` |
| Composed CI blob | `a657f548d8a5bb2a7662e697c5f7555fc4b64582` |
| Unchanged library implementation blob | `4360cd3dbca832a9a22be138ca13a5deb4dfeb0f` |
| Cargo.lock blob | `1f99851c45524f129e8544a2d779b3c21594d816` |
| Replay driver SHA-256 | `6588974a3182048930512a8142731aaa37ae3b9559257766bfe0c763632f2e35` |
| Replay receipt SHA-256 | `eead9cca60504baff1e936a56564900b9a5397fd09bde122d7eb380eecd2b64e` |
| Admission driver SHA-256 | `2b5c45b3d77d3cfd843feeb24e1ec15727ab05e2ae54ebe61d723cea9921f8f5` |
| Admission receipt SHA-256 | `df2c1038820b737b7f24f4555cbbdc06a95ccec81d374f4daa3b56c9984fc4ad` |

Read-only native Git checks verified the exact composed HEAD, tree, parent, a clean source checkout, and every received file against both Git and the working tree. Ten files covering terminal/package/docs, API/library, owner admission tests, CI, and Cargo inputs were hashed before and after both runs. The old baseline TUI and actual API executable were also exact before and after.

There is no Rust or Cargo diff between the composed source and its fe2 parent. The composition retains all current owner CI lines and adds only the explicit 13-line terminal native test/receipt hook after the release build. The owner UI test additions are preserved. The reviewer did not edit the production checkout, API implementation, CI, or authored tests.

## API change and precise expected behavior

The owner change moves request-body admission before each known POST handler. Unreadable bodies (including invalid UTF-8), malformed JSON, and valid JSON values that are not objects return HTTP 400. Empty or whitespace-only bodies remain the existing shorthand for an empty object. Valid object fields retain the endpoint's existing defaults, coercion, and validation. Route/method selection still takes precedence; GET handlers and unsupported route/method behavior remain unchanged. This change introduces no body-size allowance or resource-bound promise.

Direct source comparison found all 17 changed POST handlers mechanically identical after normalizing the raw-string parameter to a borrowed JSON value, removing their local parsing line, and adjusting the corresponding borrow syntax. Fifteen read handlers are byte-identical. The exact owner diff and normalized comparison results are retained.

The prior 16-group driver sends valid JSON objects. Its only source change for this replay is its evidence-root directory; every helper, control, and assertion is otherwise exact. `replay-driver-delta.json` records both driver hashes and the single changed line.

## What the unchanged replay proves

The replay retains the original live-session and durable checkpoint controls: explicit targeting while another session is active; 64-byte names and an exact signed 64-bit seed; create-only saves and duplicate conflicts; concurrent independent forks and competing saves; actual API restart and loss of ephemeral live sessions; saved-state recovery and two subsequent deterministic bot steps; preservation of main and named bystander sessions; actual native checkpoint 404/400/422/500 failures; a real metadata temporary-write failure with rollback of the first snapshot write; lost or held post-commit responses; EOF and SIGINT; invalid arguments with no HTTP traffic; and a global no-reset/no-replay route audit.

All 156 terminal HTTP requests and all 123 addressed run/save requests match the original counts. The separate exact-address analysis verifies each run/save selector against its expected session IDs. Every save remains `overwrite:false`; the terminal never calls library load/delete or session activate/close.

Seven full state artifacts from the old and current runs compare equal when only JSON object-key ordering is ignored. Array order, every JSON numeric source token, and all other values are compared exactly, including the seed `-9223372036854775807`. The two restart continuation points still reach turn 9/RNG 44 and turn 10/RNG 48. Serialized bytes differ because object-key ordering differs between native executions. Checkpoint metadata also has a new wall-clock save time; each run independently requires its own saved metadata and snapshot bytes to remain exact through restart, actions, and failures. The cross-run comparison does not erase or reinterpret those differences.

## Separate new admission-boundary controls

A local recording proxy forwards real terminal requests to the actual native API. It changes exactly one outbound request body per group; the API itself produces the rejection. These are deliberate receiver-injected invalid requests, not bodies normally produced by the terminal.

1. **Non-object action body.** The proxy forwards `null` for one explicitly addressed terminal `auto`. The native API returns 400 with `JSON request body must be an object`. The terminal reports the known error and keeps the prompt; it does not report a post-commit uncertainty, replay, reset, or create a session. The selected state, main, bystander, and library bytes remain unchanged. A later explicitly typed `auto` performs exactly one real bot step matching an independently forked native oracle.
2. **Malformed save body.** The proxy removes the closing brace from one create-only save object. The native API returns 400 with `invalid JSON request body`. The terminal reports it and keeps the prompt. No default-named checkpoint or partial file is created; no save is repeated. The user's later explicit save succeeds once and contains the exact selected state.
3. **Unreadable checkpoint-fork body.** The proxy forwards byte `ff`. The native API returns 400 with `could not read request body`. The terminal exits 1 without a prompt, fallback start, replacement, or replay. The full session registry, active session, checkpoint bytes, and existing careers remain unchanged.

These current 400 outcomes are distinct from the original replay's intentionally lost post-commit responses, where mutation may have succeeded and the terminal must retain its uncertainty warning and no-replay guarantee. Both behaviors pass in the current composition.

## Reproduction and evidence boundaries

Native receiver root:

`/Users/me/Developer/hamon-mesh-author-c6cc965b64c2/terminal-durable-receiving-fe2e6fdc-v1`

Actual read-only executable:

`/Users/me/Developer/uma-terminal-durable-c6cc965b64c2/receiving/current-fe2e6fdc/target/debug/uma-sim-api`

The tests use Node 26.3.0 on macOS, a fresh private cwd for each API run, and `UMA_REPO_ROOT` pointing to the pinned current checkout with `UMA_POLICY_CMD` omitted. Each driver creates its output directory once and refuses an existing run directory. For another run, choose a fresh receiver namespace; do not overwrite these receipts. All receiver-owned API and terminal children were stopped by the harness.

The evidence packet contains both standalone drivers, both full receipts and logs, exact source and address manifests, original/current API impact and CI diffs, the single-line replay provenance, seven state artifacts plus checkpoint metadata, and the cross-run comparison driver/result. The native manifest also covers the complete private storage artifacts retained on disk.

This qualification covers the observed native terminal/API behavior. It is not a full UMA workspace or browser/UI CI result, a service deployment, or a guarantee of byte-identical serialization across separate server executions.

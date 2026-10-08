# Independent receiving: exact UMA MCP seeds and checkpoint JSON

**Verdict: PASS for production entry SHA256 `3d5f61a368bae5b4bf91ba839b925820ef949a4bb9274dcf7e9d91e9cc02a736`.** The independently frozen receiver passed **107 of 107 protocol controls and 45 of 45 native groups**, with no candidate failures, skipped groups, or receiver errors. Both driver processes exited 0. This is acceptance of the exact native source and runtime behavior described here; source publication, hosted CI, later-parent composition, and merge remain with the original author.

Source owner: [Jacob-Met/uma-sim #102](https://github.com/Jacob-Met/uma-sim/issues/102). Receiver reservation: [issue comment 6063342444](https://github.com/Jacob-Met/uma-sim/issues/102#issuecomment-6063342444). Receiver: `chatgpt:58d79b68c9e4:product_work`.

## What was received

The capability admits canonical decimal-string seeds throughout signed i64, while retaining existing safe integer Number inputs and the omitted default of 42. Successful JSON tool replies preserve the original native JSON text. The new `sim_library_import_json` accepts a complete object document through `snapshotJson` and carries its exact number literals into the native snapshot importer. The older object-input tool rejects unsafe integer or nonfinite Number leaves before dispatch.

The production closure is two files: `mcp-stdio.js` and its unchanged `package.json`. The entry imports only Node builtins. Its Git blob is `59df3ce6d6b18112ac9e25aed02a07346fca3f00`; package blob `9d2aaf8942be2908ac2380204dcb32d9bb8e73fc` remains canonical. Both were copied byte-for-byte into the receiver's isolated candidate directory after the producer froze the runtime. No producer implementation tests were used as an oracle.

The source base is `b026a21f437789d7a7cd04c5074258302e36814e`, tree `359cc10294331337e200c433d95d170ea439e087`. This parent differs from the earlier `4eab1798def5c19fbdee267eb2a6ea8a2726dbe7` only by the accepted PR98 test-helper normalization and its test. Production was unchanged between those baseline cuts.

## Independent specification and wire controls

The oracle and exact bindings were frozen before any proposed implementation, helper, or candidate test was read:

| Frozen artifact | Git blob |
| --- | --- |
| Pre-candidate oracle | `faf9ae7d8f2e937d58fdd31c015cf799589b1324` |
| Exact tool/seed binding | `319f34991cb0df5061fd916c23ae00b6e30ada51` |
| Separate Unicode addendum | `c478edb0b6c2fb4cad8b53164422550f4667fe04` |

The 107 protocol controls drive the real newline-delimited stdio MCP process through a private HTTP fixture. They cover 17 accepted decimal strings, seven ordinary numeric token forms, omission, canonical-invalid strings, unsafe numeric tokens and wrong types, exact raw JSON objects/arrays/scalars/quoted strings, decoded text resources, literal Markdown, malformed successful JSON, raw HTTP failures without retry, import wrapper identity, unsafe recursive object leaves, 12 explicit UTF-16/JSON-escape cases, and existing ID/session/report guards.

In particular, the final-newline string `"42\n"` is refused before HTTP even though a JavaScript regexp end anchor alone can accept that suffix. Literal lone surrogate code units are refused before UTF-8 substitution. ASCII JSON text containing an escaped surrogate is preserved for the native codec to decide; that text is a different input from a literal ill-formed argument string.

The unchanged accepted PR98 `McpClient` class supplies the basic child-process transport only. The independent observation wrapper records every request and response and can inject raw numeric tokens. Report-comparison helper functions are not imported as an oracle.

## Actual native career receiving

The native executable is the producer's single retained API build:

- Path: `/Users/me/uma-mcp-exact-json-proof-58d79b68/native-api/uma-sim-api`
- SHA256: `149620074b6fb756d1dc9c8775e2afbb68d82791e12d6a2870cd90e546f02c17`
- Size: 14,182,432 bytes.
- Native source/catalog base: `b026a21f437789d7a7cd04c5074258302e36814e`.
- Receiving platform: native macOS, Node v26.3.0. The receiver performed no Cargo build or installation.

The receiver independently verified all 136 selected native source, manifest, catalog, research, and content-pack leaves against their canonical Git blobs and modes before and after execution. The API binary also retained its exact hash. The executable is retained at the original proof path rather than duplicated in this packet.

All runs used new receiver-owned working directories and private loopback ports. `UMA_REPO_ROOT` selected the pinned catalogs and `UMA_POLICY_CMD` was removed from the API child environment. No existing career service, personal library, browser, or client configuration was touched.

The seven native values were:

`42`, `9007199254740993`, `-9007199254740993`, `9223372036854775806`, `9223372036854775807`, `-9223372036854775807`, and `-9223372036854775808`.

For each value, the receiver created a real native reference career, started the same seed through MCP, saved a checkpoint with literal Unicode labels/notes, compared the MCP export with the exact stored native bytes, imported that portable document directly through the native codec and through the new MCP text route, loaded a separate career, and applied actual available actions to both continuations. Complete snapshots, RNG state words, and call counts matched after the same race/training decisions. Original source checkpoint files and the independently live sibling remained unchanged.

Additional native controls check safe object compatibility, rejected seeds and unsafe object values before dispatch, missing fields, out-of-i64 snapshot literals, native rejection of escaped lone surrogates, overwrite refusal, exact HTTP error text, and preservation of every existing private library file on refusal. The independently read native API maps malformed snapshots to HTTP 400; compatibility errors are a distinct native category.

The exact-value JSON oracle uses integer coefficients and decimal exponents represented with BigInt. It compares every snapshot field while ignoring JSON object member order and formatting. Native HashMaps may serialize in a different order after import, so separate native serializations are compared structurally without ever converting seed literals to floating point. **The direct-versus-MCP export of the same stored checkpoint is separately required to be byte-identical.** Six oracle self-checks are recorded separately and excluded from the product acceptance count.

## Preserved baseline failures and receiver correction

The baseline is retained separately, without being relabelled as acceptance:

| Run | Passed | Target-contract failures | Receiver errors | Exit |
| --- | ---: | ---: | ---: | ---: |
| Baseline stdio protocol | 71 / 107 | 36 | 0 | 1 |
| Original native baseline | 14 / 45 | 30 | 1 | 1 |
| Targeted corrected native baseline control | 0 / 1 | 1 | 0 | 1 |
| Candidate stdio protocol | 107 / 107 | 0 | 0 | 0 |
| Candidate native receiving | 45 / 45 | 0 | 0 | 0 |

The old bridge exported `±9007199254740993` as `±9007199254740992`; native re-import accepted those altered seeds. Four i64 endpoint/adjacent exports became `±9223372036854776000`, and native import refused them with HTTP 400. The original native continuation control independently confirmed that race followed by speed training restores complete state and advances RNG calls to 4 and 10.

One original receiver control embedded pretty-printed snapshot JSON directly into a newline-delimited request. Its physical newlines split the frame, producing parse errors and an unanswered request. The original driver, invalid wire recording, failed result, and correction receipt remain intact. The correction replaces only physical CR/LF whitespace in that valid object document with spaces; escaped string content and every numeric literal remain unchanged. Only the blocked baseline control was replayed. It dispatched exactly once and changed the private checkpoint target's seed from 77 to `9007199254740992` for a raw input of `9007199254740993`. The candidate runs the corrected control and refuses before dispatch.

A later source-review-file write hit transient ENOSPC. The file was not created; its attempted execution reported MODULE_NOT_FOUND. Both completed candidate results were already present and intact. After a fresh native check showed available space, only that small missing evidence script was retried successfully. No behavior suite was rerun because of this custody failure.

## Source fence and legacy fixtures

The independent whole-parent comparison examined all 1,310 canonical source leaves. Exactly four existing leaves changed, and two dedicated tests were added. All **1,306 unrelated parent leaves** retain their Git blobs and modes, including Rust/API/UI/catalogs, dependency manifests and lockfiles, and the accepted PR98 helper.

The two legacy test files change only the intended public-contract expectations: add the new tool to discovery lists, replace the now-valid string seeds `7`/`42` with invalid leading-zero forms `07`/`042`, and require the HTTP fixture's actual compact JSON resource bytes instead of bridge-created pretty-printing. The exact diff is retained. No other legacy assertion, call path, or simulation expectation changed.

The production source review found no blocking defect. The code validates complete JSON while retaining its text, independently encodes optional import metadata, refuses unsafe object leaves, validates canonical seed range and spelling, and preserves the established JSON-string resource, Markdown, error, session, and ID boundaries.

## Evidence and replay

`manifest.json` lists every accompanying immutable packet file's path, size, mode, SHA256 and Git blob. Raw checkpoint documents, continued snapshots, direct HTTP records, proxy records, stdin/stdout/stderr, baseline failures, source freezes, source comparison, and custody corrections are included. The API binary is pinned and retained at the original native path.

From the original receiver directory, the candidate commands are:

```sh
/opt/homebrew/bin/node protocol-receiving.mjs --source candidate-v1/packages/uma-sim-mcp/mcp-stdio.js --out protocol-candidate-new --variant candidate
/opt/homebrew/bin/node native-receiving.mjs --source candidate-v1/packages/uma-sim-mcp/mcp-stdio.js --out native-candidate-new --variant candidate
```

Output paths must be new directories inside the receiver. The native command checks the exact binary and canonical input-tree identities before starting its private API. Copies of the packet can be replayed after supplying equivalent absolute source/catalog and binary paths with `--repo` and `--binary`; the admitted binary digest remains pinned.

This review does not claim Node 20 execution, hosted CI, a later publication head, installed estate adoption, a real user library import, or recovery of precision already lost in an external client. Those limits do not reduce the exact native receiving result. The original author retains source integration and the corresponding coordination claim.

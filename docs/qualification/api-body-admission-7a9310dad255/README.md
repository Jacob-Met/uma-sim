# Reject malformed API bodies before career mutation

This contribution corrects the Rust REST request boundary. It does not change simulation rules or a running estate service.

## Observed defect and corrected behavior

The old parser converted nonempty malformed JSON to an empty object. Successfully parsed nonobject JSON also reached handlers with no recognized fields. The request reader discarded I/O errors, so invalid UTF-8 could become an empty command. Actual HTTP receiving demonstrated default Start, Auto, Fast, fork and checkpoint-save behavior instead of refusal.

Matched POST routes now share one admission step. It propagates read failures, parses a body once, and requires a JSON object before invoking the handler. Refusals use the existing 400 JSON error response and outer CORS wrapper. Handlers receive the admitted object; their endpoint-specific value coercion, defaults and business behavior stay intact. Blank and whitespace-only request bodies retain their existing empty-object shorthand. Route selection happens first, preserving known-route405, unknown-route404, GET and OPTIONS behavior even when the supplied body is invalid.

The17 POST handlers receive the same mechanical parameter/parse/reference adaptation. The15 other top-level handlers, all three session-resolution functions owned by issue63, and the complete existing inline-test suffix are byte-identical. No engine, catalog, research, browser, MCP, CLI, dependency, lockfile or workflow source changes are included.

## Native verification

| Check | Observed result |
| --- | --- |
| New actual-process Rust HTTP suite on old source | 2 passed, 3 failed |
| Same suite on candidate | 5 passed, zero failures/skips |
| Existing core library tests | 45 passed |
| Existing career-lab integration tests | 14 passed |
| Existing positive external-policy REST integration | 1 passed |
| Whole-workspace formatting and diff whitespace | Passed |
| Independently frozen actual-HTTP receiver on candidate | 19 passed, zero setup/fatal errors |

The authored suite covers all17 POST routes, nonobject JSON, invalid UTF-8 both alone and after an otherwise valid object, complete snapshot/checkpoint/inventory preservation, valid numeric/string seed fields and nested extra fields, valid non-ASCII labels, default blank bodies and routing/CORS controls. Each test launches its own API process and disposable storage, then terminates that process.

The independent receiver was frozen before the reviewer saw candidate source. It compares complete active/named snapshots, inventories, text/telemetry, stored-file hashes and the next identical-seeded action, including random-stream state. It also exercises interrupted/truncated HTTP bodies. The two interrupted-body controls already refused safely on the baseline; they are preserved controls, not newly fixed failures.

On each native baseline, 7 controls passed, 11 cases explicitly failed incorrect-admission assertions, and one malformed-branch case reached a separate native panic under the deliberately empty external-command environment. The branch result is retained as a process failure, not an ordinary HTTP assertion or a claim to fix external-policy configuration. The candidate refuses that invalid request before the path executes.

[Independent frozen contract, full raw baseline/candidate archives, source review and immutable provenance](https://github.com/Jacob-Met/uma-sim/blob/474c1bbf8262dfa198b274b7fb66c85802161a8c/docs/receiving/api-admission-7a9310dad255/README.md).

## Exact source and build

Original API Git blob: `4ff7034bc5701d793cd93d645d89d40d73fc3273`.
Candidate API Git blob: `eab9ccf7082ffc96c446f434bd76f313aff14bc9`.
Candidate API SHA-256: `31e51bf33c6cd5f51c7f7cb79a4560fad5c2e06a77f0345ff659cf114fabdd55`.
Candidate API binary SHA-256: `926c50c94e8bee778d7630c9ceb6ea19171f3aa6a4b7373457cdb97dae5b2d1b`.

The isolated Linux build used Rust1.99.0/Cargo1.99.0, the unchanged lockfile, default features, and disabled debug symbols/incremental compilation. The first offline build failed because the copied registry lacked the regex index. A normal locked build obtained exactly the lockfile dependencies and completed; that negative build log is retained separately from successful execution.

The read-only source clone was pinned to `f5f9b29393731d18aee2d66a31d89c315aa79c60`. Every compiled/data input was compared with current main `0ac14602addd1a4610aa8359896915a34cce5659`: 282 entries, 13,812,012bytes, zero differences (the independent281-input scope plus rustfmt.toml). Main `0b5cc2342cd93141beb461edf6e83998388e67b8` subsequently added only checkpoint UI naming/portability changes. The source publication overlays only this API/test/qualification scope on the then-current main tree. The source/data manifest and exact test/binary manifest are retained beside the logs.

The independently tested candidate was uncommitted at execution time; its exact API and binary hashes identify it. Published-commit ancestry, actual-head hosted CI, integration and deployment must be recorded separately after publication.

## Reproduction

From a source checkout with the locked Rust dependencies available:

```sh
cargo test --locked -p uma-sim-core --test api_body_admission
cargo test --locked -p uma-sim-core --lib
cargo test --locked -p uma-sim-core --test career_lab --test rest_policy_external_stub
cargo fmt --all -- --check
git diff --check
```

The focused HTTP suite is an ordinary Cargo integration test, so the existing workspace CI runs it without workflow changes. Complete independent raw receiving archives and its replay command are linked above.

## Coordination

Owner: estate-7a9310dad255/production, under Jacob's standing autonomous execution mandate. The [exact adjacent scope was published and read back on issue63](https://github.com/Jacob-Met/uma-sim/issues/63#issuecomment-6056774672) before implementation. Session-resolution work remains with that owner; the held PR54 test cluster, existing MCP composition and all browser/CLI scopes are preserved. The local source/target/cache and generated test state are isolated; no other worker checkout or installed service is modified.

Published plaintext logs remove only surplus blank lines at EOF for repository whitespace checks. Original native logs remain unchanged in the isolated evidence directory; recorded outcomes and diagnostic text are unchanged.

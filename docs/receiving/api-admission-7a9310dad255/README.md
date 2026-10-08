# Independent Uma API request admission

Worker `7a9310dad255` independently froze the contract and actual HTTP receiver before seeing the API correction. This branch preserves the negative baseline and subsequent receiving evidence; it does not itself change the product.

## Why the request boundary matters

The existing API turns malformed nonempty JSON into an empty object. Several real routes then execute their default command. Actual native HTTP calls demonstrated a default career creation, career advancement, a fork and a checkpoint write. Invalid UTF-8 also reaches default Start because the body read error is ignored. A malformed Action returned200 without a visible change in the initial mandatory-race phase; that narrower observation is retained.

Baseline source is main `0ac14602addd1a4610aa8359896915a34cce5659`, with `uma-sim-core/src/api.rs` blob `4ff7034bc5701d793cd93d645d89d40d73fc3273`. All281 source/data inputs were independently compared against canonical Git blobs on each native host. The later publication parent `0b5cc2342cd93141beb461edf6e83998388e67b8` changes only checkpoint UI naming and portability tests/evidence; all281 inputs remain exact.

## Frozen receiving

Read [frozen-contract.md](frozen-contract.md) and [receiving.mjs](receiving.mjs). The runner uses Node built-ins and real HTTP/raw TCP against the supplied Rust executable. Each case has a fresh server, ephemeral loopback port, disposable cwd and two identical seeded careers. It compares complete snapshots, session inventory, active selection, text/telemetry, checkpoint and branch listings, stored-file hashes, and the next valid action's complete state including random-stream state.

Blank/whitespace bodies retain their established empty-object behavior. Valid UTF-8 objects and existing404/405 routing are controls. Session-resolution behavior owned by issue#63 is outside this correction's acceptance scope.

The same19 cases ran on Darwin/arm64 and Linux/x64. They are19 unique cases, not38. Each baseline completed with7 passing controls and12 failed cases:11 explicit bad-admission failures and one separately identified native-process failure. Both interrupted-body transport cases already refuse safely. All5 ordinary valid/routing controls pass. There were no setup errors, no harness-wide fatal error, and both supplied executable hashes remained unchanged.

The malformed branch case reaches the default scoring-policy launcher. With `UMA_POLICY_CMD=''` deliberately disabling external commands in the receiver's child environment, the API panics at `policy_external.rs:218`; no external command starts. This is preserved as a process failure, not reported as an ordinary HTTP assertion failure. The admission correction must refuse the invalid body before that path runs.

## Candidate receiving and source review

The exact Linux candidate API passed all19 unchanged independent cases: exit0 in16.67seconds, no setup errors or harness-wide fatal error, binary unchanged. All12 validly framed malformed/nonobject/invalid-UTF-8 requests return400 with JSON and CORS while preserving complete observed game/storage state and the next valid random outcome. The two interrupted transports still close safely. All5 ordinary valid/routing controls pass. The branch request is refused before the external-policy path.

Candidate binary SHA-256 is `926c50c94e8bee778d7630c9ceb6ea19171f3aa6a4b7373457cdb97dae5b2d1b` (14,092,352bytes). Candidate `api.rs` is Git blob `eab9ccf7082ffc96c446f434bd76f313aff14bc9`, SHA-256 `31e51bf33c6cd5f51c7f7cb79a4560fad5c2e06a77f0345ff659cf114fabdd55`. The281-input post-run comparison has zero unexpected differences: this API blob is the only source change among those inputs.

The receiver ran before the reviewer viewed candidate code. The subsequent complete source diff was accepted: routing validates object JSON once after matching a POST route; body read errors propagate; blank-body compatibility and the existing outer CORS wrapper remain. All17 modified handler bodies are exactly equivalent after the mechanical parameter/parse/reference adaptation. All15 other top-level handlers, the three session-resolution functions owned by issue#63, and the entire existing inline-test suffix remain exact. See [candidate-source-review.json](candidate-source-review.json).

[candidate-index.json](candidate-index.json) contains compact outcomes and the binding between source and executable. This candidate was uncommitted at execution time: the raw receiver's `sourceHead=0b5cc234…` labels its source base, not a commit containing the correction. Exact API/executable hashes establish the tested candidate; any later source commit must preserve those inputs. Source publication, fresh-head CI, merge and deployment remain separate.

[candidate-thinkpad-evidence.tar.gz](candidate-thinkpad-evidence.tar.gz) contains150 evidence/source-binding files plus its artifact manifest, including full raw receipt, complete stored/snapshot observations, API logs, source checks and reviewed diff. It is103,535bytes, SHA-256 `9a31ce12c618e049b87c2d08656eb46a1c01700a3da59d0c28ca441c236abcc6`. The original baseline evidence is retained unchanged.

## Exact native evidence

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| [baseline-mac-evidence.tar.gz](baseline-mac-evidence.tar.gz) |128627|`5fcf744e7534534dec6a485a5e3eb6e0732c51cd0bcc06a8acaeb10519c6b1d9`|
| [baseline-thinkpad-evidence.tar.gz](baseline-thinkpad-evidence.tar.gz) |131616|`a225c7de822891ae5a35c86c7644b01123c7457c77e66bb82069553668426029`|

[baseline-index.json](baseline-index.json) gives the exact binary identities and compact per-case outcomes. The two archives contain the unchanged receiver and contract, full raw receipts, complete per-case before/after/next snapshots where the process stayed available, synthetic stored files, API logs, and source/artifact hash manifests. The Mac archive additionally preserves the original retained binary's source/binary provenance and post-run recheck. The Linux archive preserves the independent post-run281-input check.

Extract each archive into a separate new directory using `tar -xzf`. Each contains its own artifact manifest with every included file's size, SHA-256 and Git blob hash. No Rust executable, live user career data, external provider credentials, build cache or owner checkout is included.

To replay with an appropriate native API build and matching data root:

```sh
node receiving.mjs /absolute/path/uma-sim-api /absolute/path/source /absolute/path/new-evidence 0ac14602addd1a4610aa8359896915a34cce5659
```

The output path must not exist. The final argument identifies the supplied source; it does not independently establish its provenance. Use the source and binary manifests alongside the observed execution. Candidate receiving, source review, fresh-head CI, source integration and deployment are separate states.

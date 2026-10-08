# Independent current session/import receiving

Evaluation_lane independently reviewed and executed the current [UMA PR116](https://github.com/Jacob-Met/uma-sim/pull/116) composition at **3397f142e397aa911c125261fac41158e7bc74cc**, tree **8220492e07834480ab9fc0823358c9ad3aa9f24b**. Its actual parents are prior qualified session source **63aca264ee8b47df14bcecf47cf1b390b244852f** and accepted import source **0ad4bd4d2ca0a133fe91080b6b7818efbaedb2fa**.

**Disposition: approved for this exact source-composition and React receiving gate.** Root retains ordinary hosted/current-head integration. No maintained product file changed during this review.

## Source preservation

The complete four-tree comparison finds exactly two overlapping maintained inputs:
- The final client is the whole accepted raw-JSON import client with one exact replacement: `if (!session) return path;` becomes `if (session === undefined) return path;`. All raw request-body handling and import APIs remain byte for byte.
- The final package is the complete accepted package plus the prior session-test argument, prior `prebuild: npm test`, and prior React test renderer dependency. Every other incoming package field is preserved.

All eight prior native/session contribution leaves outside those overlaps and all thirteen nonoverlapping incoming import leaves retain their exact blobs and modes. All prior and incoming paths remain present. The actual1586-leaf candidate preserves1547 incoming leaves unchanged. `source-preservation.json` records the exact paths, source rules and pins.

## Native execution

The existing20-case React receiver (original Git blob `f8ab830df82d86d9de193ecce5cf12f7296fb30c`) passes **20/20**, zero failed/skipped, exit0 and empty stderr. Only its source-root path and baseline metadata changed; reversing those two literal substitutions recovers the entire original receiver. Test cases, fixture data, assertions, compilation and loader behavior are unchanged.

The run covers delayed actions and follow-up reads, stale refresh/error/cleanup, reset/unmount/start retirement, duplicate submission, explicit empty selectors, displayed-session operation targeting, keyboard/telemetry, and the actual App→Lab→Activate→Run component path with current Conditions/Skills/history preserved.

Executed receiver SHA256: `407542673bcebcf4366e4c979e5e919802a4576164f7ea07c9497545866b03a7` (9151 bytes). Raw stdout SHA256: `aa78aaab9421d8ae2941b39e8aef582b95ba8562a0a0b11eb43edd41005f66af` (2558 bytes). Its47-input source-set SHA256 is `8a8047e667e3d64593355cf6d662b66e215e842851c6714f80f8ba4f84357662`.

All48 staged current Git inputs, including the separately pinned unexecuted import-test file, were verified before and after. Staging copied46 exact old inputs from the read-only shared source into a fresh per-invocation `/dev` directory and applied five exact published import/composition inputs. Existing dependency files were read without installation or mutation. Runtime versions and the full invocation appear in `execution.json`.

## Reproduction and limits

Use the exact reviewed source and existing compatible dependencies. Set the two environment paths at the top of `review-receiving.mjs` for an isolated source copy and an existing dependency root, then run it with Node. The authored run used the paths recorded in `execution.json`; those temporary process-local paths are not durable shared storage.

The receiver uses React test renderer and in-memory TypeScript transpilation. CSS bytes are pinned, while CSS imports are ignored. Rust source/test files are hashed, not executed; the existing116-test full UI build is not rerun. This does not claim a real browser, deployed/installed service, network provider or real user career operation.

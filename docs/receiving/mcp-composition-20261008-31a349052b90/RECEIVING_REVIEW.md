# UMA career-lab composition receiving

## Result and authority

PASS for the published owner composition in [PR #75](https://github.com/Jacob-Met/uma-sim/pull/75), head `4873c0958294173d342fba8bf0bf68d7a4b724de`, tree `9dd0e518e3357d3734f46233e807f458972d70c5`. The bridge is Git blob `24d5384c0a14fbdd07c4dd838b6a0d56b6d92ceb`, SHA256 `5f0dfdbb44595acb62026d3da33ca6ef5904ed1aec76dad4ef05cd418ec6f584`.

Discovery recovered the existing composition claim and accepted receiving route in [issue #66](https://github.com/Jacob-Met/uma-sim/issues/66#issuecomment-6056641639). The parent stopped the prospective local #69 composition after that discovery. This review exercises the published owner's source and does not modify an owner worktree or branch.

The reviewer authored the earlier #68 frame guard. This is independent receiving of another worker's composition and preservation of the career-lab contribution; it does not replace the parent's independent source review of that earlier guard. All tests here are unchanged published tests from the relevant pinned commits.

## Executed results

| Receiving target | Source | Tests | Pass | Fail | Skip |
| --- | --- | ---: | ---: | ---: | ---: |
| Complete unchanged #69 package suite | #75 bridge with #69 tests | 123 | 123 | 0 | 0 |
| Complete exact #75 package suite | Exact #75 package source/tests | 136 | 136 | 0 | 0 |

Both runs use the existing Node **20.19.0** binary, SHA256 `34bc6627675906f8431892630b5e91fc4cc9f1e03ce15e9716025aa551e5c823`, and the actual isolated Rust API built from `f5f9b29393731d18aee2d66a31d89c315aa79c60`, binary SHA256 `fdda7861a592987391e57b59d8dc0230b870b73268e9e3b02af0f330bffcab5e`. The earlier build receipt records Rust 1.99.0. No new build was performed.

All **246** native Rust, build, catalog and research files in the readable receiver were verified against the published #75 tree. The initial 123-case run had verified the 169 Rust/build/data/content-pack subset; the additional catalog/research bindings were completed before the 136-case run. The actual native code is unchanged between #69 and #75.

The 136-case run includes the eleven native checks: checkpoint export/import/conflict preservation, independent careers and branches, incompatible metadata refusal, safe default-session behavior, external-policy refusal and continuation, real comparison/report evidence, fork contention without an existing main career, full non-active target rollback after a real 503, and loaded content reaching two existing careers while a separate unloaded API remains a negative control. HTTP/stdio controls cover all 26 tools, strict nested schemas and selectors, raw status/body preservation, malformed success decoding, no automatic replay, fragmented UTF-8, correlation, notifications, invalid and unsafe numeric IDs, legacy JSON string payloads, and literal Markdown reports.

Each native API uses its own loopback port and private receiving directory. Native fixture data and outputs are retained under the two suite directories; the helpers stop their processes after execution. No installed simulator, provider, service, shared catalog or owner source was changed.

## Source preservation review

`source-preservation-review.json` records byte-identical regions between published #69 and #75:

- Protocol negotiation and version identity.
- All 26 tool declarations, the three resources, strict recursive argument validation, and query encoding.
- Every tool/resource REST route and request payload adapter.
- Stdio line framing, stream decoding, and parse-error behavior.
- Initialization, resource/discovery replies, and tool argument validation.
- Ping and unknown-method replies.

The declared changes are confined to the raw HTTP error class/decoder, initial frame/ID/object-params checks, failed tool-result formatting, and the owner-approved successful string formatter. The strict unknown-property/range policy remains intact. Numeric request IDs use `Number.isSafeInteger` after JSON parsing; string IDs and valid safe integer IDs are preserved. The bridge is not being accepted as an arbitrary-precision JSON parser or as a complete MCP-conformance implementation.

The #75 formatter preserves legacy JSON-encoded successful strings; only the Markdown lab report is literal text. This explicitly supersedes #69's broader raw-string output and follows the recorded owner decision. The raw HTTP failure body is preserved before successful JSON/text decoding. HTTP failure, malformed arguments and transport uncertainty remain distinct.

## Reproduction and evidence

`owner75-original-suite/receipt.json` records the first exact command, source/test hashes, binary identity and complete 123-case TAP output. Its tested source package is `owner75/packages/uma-sim-mcp` (the #75 bridge plus unchanged #69 tests).

`owner75-exact-suite/receipt.json` records the exact #75 run and all thirteen source/test/guide/workflow files verified by Git blob. `owner75-exact-suite/tests.tap` is the complete 136-case output. The package lives under `exact75/packages/uma-sim-mcp`; the adjacent workflow was inspected and runs `*native.test.mjs` in the Rust job, so the added native cases participate in hosted receiving.

To replay, use Node 20 or newer, provide a verified API binary, and point the repository-root variable at a checkout with the shipped native data. The source snapshots in this packet are package/receiving subsets, not complete native source checkouts or API binaries.

```sh
UMA_SIM_TEST_API_BIN=/absolute/path/to/uma-sim-api \
UMA_SIM_TEST_REPO_ROOT=/absolute/path/to/pinned/uma-sim \
UMA_SIM_TEST_OUTPUT_DIR=/new/writable/receiving-output \
UMA_SIM_TEST_RECEIVING_COMMIT=f5f9b29393731d18aee2d66a31d89c315aa79c60 \
node --test exact75/packages/uma-sim-mcp/tests/*.test.mjs
```

For the unchanged predecessor-suite preservation run, replace `exact75` with `owner75`. Use a writable TMPDIR if the default temporary filesystem is full. The native cases fail for an explicitly configured missing API binary; they skip only when the binary variable is absent.

The parent's separately executed negative probe on exact #69 used the unchanged #68 tests with the pattern `invalid JSON-RPC|malformed parameters|MCP null|numeric overflow`: two methods passed and two failed (null frame yielded -32603; fractional ID 1.25 executed and returned). That result is preserved as a parent-reported control, not presented as a new execution by this reviewer. No duplicate negative run was added.

Shared publication, owner integration and merges remain subject to the parent's fresh source/review/CI/ancestry gate. No merge, deployment or runtime adoption is claimed by this receiving review.

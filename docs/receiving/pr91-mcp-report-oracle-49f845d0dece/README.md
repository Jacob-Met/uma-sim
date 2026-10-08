# PR91 native MCP report receiving

This packet resolves the concrete native MCP gate failure in [uma-sim PR91](https://github.com/Jacob-Met/uma-sim/pull/91) at `bcd1df70c0681e7f9ae3a5a0bac75d9d4984ef84`. Original report implementation and issue73 authorship remain with `chatgpt-acd057031fb2`. Independent receiver: `estate-49f845d0dece / source_coordination`.

## Correction

The Rust report serializes punctuation as literal Markdown. The existing MCP helper applied a raw UTC regex directly to the escaped `Compared at` source, so a valid native report such as `2026\\-10\\-08T13\\:30\\:30Z` failed the integration gate.

Only `packages/uma-sim-mcp/tests/helpers/mcp-client.mjs::comparableReport` changes. It decodes colon/hyphen escapes in that one field, retains the anchored whole-second UTC format, and checks both finite parsing and an exact UTC ISO round trip. Nonexistent dates and normalized midnight rollovers refuse. It normalizes exactly the complete generated line. Matching text embedded inside report data is preserved, as are every other line, escape byte, identity and outcome. The shared format regex and JSON comparison helper remain unchanged; fractional seconds remain outside their existing format.

The controls show why unrestricted unescaping, broad string replacement, or finite Date.parse alone would be inadequate. The original helper could replace matching text inside a data line instead of the actual generated line and admitted invalid month/day/hour values by shape alone. The first correction still admitted normalized dates such as February 30. Root independently supplied that counterexample; its real native failure and the initial correction remain preserved. The final unchanged receiver accepts valid leap days while rejecting non-leap February 29, February 30, April 31, the 1900 leap-year exception, and 24:00 rollover.

No Rust production, MCP implementation, API/session, UI, library, dependency or workflow source changes. The additive focused test is included automatically by the existing MCP `*.test.mjs` gate.

## Actual results

Each before/after pair uses the same receiver bytes.

| Boundary | Before | After |
| --- | --- | --- |
| Initial seven focused controls, SHA `36de1c0e…` | Original helper: 4 pass, 3 fail | First correction `11c0ce6`: 7 pass |
| Final nine focused controls, SHA `be68c626…` | First correction: 8 pass, 1 rollover failure | Final correction: 9 pass |
| Unchanged native career-lab test, substantive subtests | Original helper: 6 pass, 1 escaped-timestamp failure | Final correction: 7 pass |
| Node raw native count, including parent | 6 pass, 2 fail | 8 pass |
| Other four published inputs, API binary and 59 staged data files | Exact | Exact |

The first correction's separate native 7/7 pass is also retained. It qualified the escaped-timestamp seam, before the independent calendar-normalization counterexample was added. The intermediate nine-case receiver without the last midnight example also retains its 8/9 result; the final receiver and its own 8/9 before-image are separately pinned.

The native run uses real Rust API processes, exact published MCP stdio source, actual checkpoint/branch creation, and direct HTTP versus MCP report comparison. API replies are not authored. Each run uses a fresh directory in this receiver's isolated namespace; the existing helper stops its own processes. There is no browser or live service/account action. Focused helper tests use explicitly authored report strings.

Final test SHA-256: `be68c626a43537f86c2810b6744f132c452086ea1ed6fec5246c3faf7e2f297a`.
Original helper SHA-256: `43eacdb0b83772be18a59c1a945731f792c0f506a057691e9adf8e0dc83a5c8d`.
Final helper SHA-256: `fe29f796eef63dcae1793d3869e7ed5b18fbf426258ecd1cbd9c8aee587b043b`.
Final helper Git blob: `928997e6b96fc07decbb9f7c09f7c968299dc4e6`.

## Source and runtime custody

Native receiver: `/home/jacob/uma-report-oracle-receiving-49f845d0dece` on the existing authorized ThinkPad. Node is `v22.22.1`. This deliberately partial native Git repository contains the exact five required published MCP inputs, the additive control, and receiving evidence.

- Original source snapshot: `c5b0dd43c25272c1c7482fd1aedb26a095360e96`.
- Initial control frozen before correction: `89ad1c0835109fd1ab5d8b2160b3f2fdf6ca0e1e`.
- First helper frozen before its passing replays: `11c0ce6bbd93c0f0cc38ae864c27783429b83c18`.
- Final nine-case receiver frozen against that first helper: `8d407459a66703d43f6a58d901607288c80c650b`.
- Final helper frozen before final passing replays: `c752c4d7a1435069f621bd5f2937b5ebe644cde0`.

The API is the author's retained, previously qualified release executable:
`/srv/hamon-estate/artifacts/chatgpt-acd057031fb2/release-receiving-fe0ff870/stage/uma-sim-api`,
SHA-256 `4bfb35d2c595daaa2afcd624e5e2cc8539f7c10bd33d018187039e7468d57f89`, 5,074,632 bytes, source `fe0ff870ee10f21e1a7cfa9264ef3196bc417854`. Its existing staged data map matches the published release receipt. The original source worktree is no longer present, so it was not copied or modified. The artifact is invoked in place and remains byte-identical.

This reuses the retained release artifact to qualify the isolated receiver correction. It is **not** a fresh build or whole-runtime qualification of PR91's later source, current main, or a merge. Final current-main integration and published-head CI remain with the existing author/root. The hosted Rust failure is preserved as an exact bounded excerpt from job113312182700. Unrelated hosted secret-scan failures are neither corrected nor waived.

The initial source-plus-full-log transfer was rejected with `spawn E2BIG` before a process started. The smaller exact-source/bounded-log transfer succeeded. That setup failure is retained separately and is not counted as a native test result.

## Replay

Use a qualified API binary, matching repository/staged data, and a new output directory. No npm install is required for these Node built-in tests.

```sh
node --test packages/uma-sim-mcp/tests/report-comparison.test.mjs

UMA_SIM_TEST_API_BIN=/absolute/path/to/uma-sim-api \
UMA_SIM_TEST_REPO_ROOT=/absolute/path/to/matching-data \
UMA_SIM_TEST_OUTPUT_DIR=/absolute/path/to/new-receiving-output \
node --test packages/uma-sim-mcp/tests/career-lab-native.test.mjs
```

The public packet contains the focused patch, command logs/receipts, actual candidate JSON/Markdown reports, provenance, historical test/helper bytes, and before/after hash checks. Disposable authored checkpoint/branch stores remain in native custody; they are not publication files. The manifest records exact bytes without relabeling earlier execution.

Source whitespace validation passes for the helper and new control test. A check over the entire raw evidence archive reports whitespace-only TAP failure lines and unified-diff context lines; those exact historical bytes are retained. `packaging-whitespace.json` records both command outcomes.

## Publication handoff update

A fresh pre-publication read found the author's successor `95fa942cc7ca913e38700fdc486f83a739792a03`, tree `0b188f888054ac2b5eb8cae3405a39dbf988f80b`, composed with accepted main `7a2e15f623a559b2d9d425e4e7b68c1e71f24cbb`. Its helper blob `552d6c5bd78278173e5fef02ea7137938562d70c` independently admits the fully escaped timestamp shape. That source observation is separate from the original failing head and the native replays in this packet. Its whole-report replacement and lack of a date round trip remain visible in the exact source; this packet does not claim a native execution of that successor.

The proposed helper/control and preserved evidence are handed to the original author through PR91 comments6060913809 and6061233389. This dedicated child branch keeps the historical failing base for a reviewable patch; it does not update the author's branch or replace the author's current-main composition. Final adoption and integration remain with the existing source owner/root.

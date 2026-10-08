# Current PR91 report-oracle receiving

Owner source: `95fa942cc7ca913e38700fdc486f83a739792a03`, tree `0b188f888054ac2b5eb8cae3405a39dbf988f80b`, in `Jacob-Met/uma-sim`. Original report author and integration owner remain `chatgpt-acd057031fb2`; independent receiver is `estate-49f845d0dece / source_coordination`.

The owner has corrected the original escaped-timestamp syntax mismatch. Its exact source now passes the unchanged actual API/MCP career-lab test. This packet receives the remaining report-value and date-validation boundaries against that current source.

## Current failures and applicable proposal

The owner's helper accepts a fully escaped whole-second UTC timestamp. The new companion preserves that exact contract: raw punctuation and fractional seconds still refuse.

On the current helper, the same nine controls give **6 pass / 3 fail**:

1. A complete generated-line string appearing earlier inside literal report data is replaced instead of the generated line.
2. The escaped digit shape admits an unparseable month, day or hour.
3. A finite JavaScript Date can normalize a nonexistent date or midnight rollover; these values must not disappear as harmless generated-time variation.

The proposed helper gives **9 / 9 pass**, including valid leap days, the 1900/2000 Gregorian distinction, malformed escapes, missing/duplicate generated fields, and changed identity/count/date/escape controls. It preserves the owner's exact escaped-only regex, decodes only that accepted field, checks a finite timestamp and exact UTC ISO round trip, and normalizes only the complete generated line. Every byte before `comparableReport`, including the JSON helper and shared UTC format, remains unchanged.

The only proposed existing-file change is `packages/uma-sim-mcp/tests/helpers/mcp-client.mjs::comparableReport`. The additive maintained test is `packages/uma-sim-mcp/tests/report-literal-normalization.test.mjs`. `current-helper.patch` applies to the inspected owner helper; `proposal-pins.json` binds both input and output. No production renderer, MCP implementation, API, session, UI, library, dependencies or workflow changes are included.

## Native results and exact pins

| Receiver | Current owner helper | Proposed helper |
| --- | --- | --- |
| Frozen current-format controls | 6 pass / 3 fail | 9 pass |
| Unchanged actual native career-lab subtests | 7 pass | 7 pass |
| Node native count, including parent | 8 pass | 8 pass |
| Current format regex, all earlier helper bytes, four other source inputs | Exact | Exact |
| Retained API executable and 59 staged data files | Exact | Exact |

The current-format receiver is byte-identical before/after:
`1bf95045787d1265d4c43c4b6c070eb90b4a8a4e9b465b1161890edf0ea97475`.

Owner helper SHA-256:
`cb1a606d0749f0aef0f791afa9ce611c0398a59b8c6d4ca60e31015794f579da`,
Git blob `552d6c5bd78278173e5fef02ea7137938562d70c`.

Proposal helper SHA-256:
`c0874ea6e8ab66a9baae28458c43f1f629398d6ca209ec0a70b86c096dc37532`,
Git blob `d7ee26eccd2601eb97f77790f7698d6dd8b4848f`.

Current source and controls were frozen at native `dfe84a6bc172245706891908b7577ba90b84ef6c`; the proposal was frozen before passing runs at `79e9ba5ab6e997764c5594794579bf0dfd78b2f5`. Native custody is `/home/jacob/uma-report-oracle-receiving-49f845d0dece/current-successor`. Node is `v22.22.1`.

The native tests invoke the real Rust API and exact current MCP stdio source, create actual disposable checkpoints/branches, and compare direct HTTP with MCP reports. No HTTP responses are authored. The API is the author's retained release executable, SHA-256 `4bfb35d2c595daaa2afcd624e5e2cc8539f7c10bd33d018187039e7468d57f89`, built from `fe0ff870ee10f21e1a7cfa9264ef3196bc417854`. This is a bounded helper receiving run with that known artifact; it is not a new Rust build or whole-runtime qualification of the later PR95fa source/main composition. All native processes/state directories belong to these disposable receiving runs.

## Historical compatibility observation

The [unchanged historical packet](https://github.com/Jacob-Met/uma-sim/tree/dd5a287a093460578f2f3c912a12dba0979526e9/docs/receiving/pr91-mcp-report-oracle-49f845d0dece) retains the original hosted/native escaping failure, initial helper correction, independent rollover counterexample, and all earlier source/test pins.

As requested, frozen historical receiver `be68c626…` was also run unchanged against the current owner helper. It gives 4 pass / 5 fail. Four failed tests assumed raw-format acceptance, which the owner explicitly changed; those are compatibility observations, **not current product defects**. The fifth reaches the escaped rollover case. The separate nine-case current-format companion removes those unsupported admission assumptions and isolates the three current failures above. Historical files and their failures remain intact rather than being relabeled as current-contract tests.

## Replay and integration handoff

No dependency installation is required for these Node built-in tests:

```sh
node --test packages/uma-sim-mcp/tests/report-literal-normalization.test.mjs

UMA_SIM_TEST_API_BIN=/absolute/path/to/qualified/uma-sim-api \
UMA_SIM_TEST_REPO_ROOT=/absolute/path/to/matching-data \
UMA_SIM_TEST_OUTPUT_DIR=/absolute/path/to/new-output \
node --test packages/uma-sim-mcp/tests/career-lab-native.test.mjs
```

The separate child branch is a qualified source proposal and receiving handoff to the active owner, with no competing PR or owner-branch mutation. Current-head adoption, hosted gates and final integration remain with the owner/root. The current escaped-field format must be retained during adoption. `source-pins.json`, `proposal-pins.json`, `preservation.json` and the manifest bind the proposed source to exact current inputs and native outcomes.

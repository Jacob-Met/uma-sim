# Hosted receiving of browser checkpoint source 618ad8f9

All required hosted checks completed successfully for PR114 source head `618ad8f9702a39d23965e0ff68065822629bb9ba`. This is additive evidence on a separate receipts branch; the tested source head is unchanged.

## Exact checkout and current source

The three raw CI logs each record checkout `d6b747d37b01dcc41d63fc82120a99eb0a8d119f`. Its fetched Git commit has parents `8d74413d7dbc3caf1745779c92d56b6eb7feea42` and `618ad8f9702a39d23965e0ff68065822629bb9ba`, with tree `8457386cf41c5530b39dec1fa6be127dfc430d01`: exactly the proposed source tree. Main was still `8d74413d` at the captured receiving read.

The source publication verified every one of the 15 contributed files, all 1,541 unowned incoming leaves/modes and the complete 1,556-leaf tree. Those observations and the exact PR-body readback are retained in `publication-readback.json`. The existing independent source review remains at [09c94cc5](https://github.com/Jacob-Met/uma-sim/blob/09c94cc504b6f51ef329cf765ca075e89e7beb72/docs/continuity/evidence/browser-checkpoint-import-independent-20261008-d98d06fb/README.md).

## Actual hosted results

CI run [37831034339](https://github.com/Jacob-Met/uma-sim/actions/runs/37831034339), attempt1, completed successfully at 2026-10-08T19:22:49Z.

| Job | Actual result |
| --- | --- |
| UI 113496118512 | 103 tests passed, zero failures/skips; includes all 15 new checkpoint transport tests. Current typecheck and production build passed. Existing browser receiver23/23 and trace receiver15/15 passed. |
| MCP 113496118830 | Protocol/HTTP suite134 passed, four native-only skips, zero failures. CLI suite35/35 passed. |
| Rust 113496545362 | Existing formatting, Clippy, workspace tests and release build passed. Native terminal checkpoint11/11 and native MCP11/11 passed with zero skips. Release-layout smoke and strict calibration passed. |

The seven captured check-runs on the exact source head all report success: UI, MCP, Rust, bugfix test gate, duplicate guard and two secret scans (source-branch push plus PR event). The separately captured combined-status list is empty; that empty list is not the basis for CI acceptance.

The UI artifact uploaded by the UI job and downloaded by the Rust job has the same recorded digest. The fetched artifact list also records native terminal and MCP evidence artifacts. Artifact listing and recorded digests are preserved; this packet does not claim that their ZIP payloads were downloaded or inspected.

## Preserved limits

These are the repository's existing hosted browser and native gates. They do not establish completion of the earlier Mac browser attempt that was intended to test actual exported checkpoints with unsafe integer seeds and subsequent native continuation. That original attempt remains unknown, has not been relaunched, and is not represented as a passing result.

This hosted record resolves the earlier pending current-source build/check gate. The original unpublished author packet is still unavailable. Root's source-integration decision and any later installed-package receiving remain separate. No runtime deployment, installed package update or production benefit is established here.

Every fetched UTF-8 job log is retained in full, including its original leading BOM, timestamps and escape characters. Raw fetched run/job/check/commit/artifact JSON is retained without rewriting its content. `source-binding.json` is the derived summary; its claims are checked against those actual fetched records.

# UMA MCP composition: independent receiving packet

This branch preserves receiving evidence for [PR #75](https://github.com/Jacob-Met/uma-sim/pull/75), the existing owner's composition of career-lab PR #69 and the frame/error contribution from PR #68. It adds only this documentation directory over main `0b5cc2342cd93141beb461edf6e83998388e67b8`; application source is unchanged.

The [commit-anchored receiving review](https://github.com/Jacob-Met/uma-sim/pull/75#pullrequestreview-5454714793) accepts exact source head `4873c0958294173d342fba8bf0bf68d7a4b724de`, bridge blob `24d5384c0a14fbdd07c4dd838b6a0d56b6d92ceb`, SHA256 `5f0dfdbb44595acb62026d3da33ca6ef5904ed1aec76dad4ef05cd418ec6f584`.

## Complete packet

[receiver-packet.tar.gz](receiver-packet.tar.gz) contains 71 verified members: exact tested source/test snapshots, unchanged native Node test runners and helpers, both full TAP outputs, private native result/server/provenance artifacts, source comparison, replay instructions, and source authority. No API binary or installed service state is included.

- Archive bytes: **121,583**
- Archive SHA256: `5a2281da54fc6b8d033c9ad0c6fb20b60a50829bf5d8b000b7547159bf13d433`
- Archive Git blob: `56cfb0dcdac4ed0ccca3dcfcd4203ec7dabb822a`
- [Manifest](MANIFEST.json) SHA256: `515f3accf6fece7e4777e11e089a6365231d29c5bec5bdc1358f6b99e165a8da`

Extract into a new directory:

```sh
tar -xzf receiver-packet.tar.gz
cd uma75-receiving
```

The extracted `RECEIVING_REVIEW.md` contains full reproduction instructions. All test runners are preserved in the archive under `owner75/packages/uma-sim-mcp/tests/` for the unchanged #69 suite, and `exact75/packages/uma-sim-mcp/tests/` for the exact #75 suite. The latter runners also remain directly reviewable at their [original immutable source paths](https://github.com/Jacob-Met/uma-sim/tree/4873c0958294173d342fba8bf0bf68d7a4b724de/packages/uma-sim-mcp/tests). They are not rewritten for this handoff.

## Receiving result

| Execution | Passed | Failed | Skipped |
| --- | ---: | ---: | ---: |
| Unchanged #69 complete package suite against #75 bridge | 123/123 | 0 | 0 |
| Exact #75 complete package suite | 136/136 | 0 | 0 |

Both runs use Node 20.19.0 and the existing real Rust API built from `f5f9b29393731d18aee2d66a31d89c315aa79c60`. The API binary SHA256 is `fdda7861a592987391e57b59d8dc0230b870b73268e9e3b02af0f330bffcab5e`. All 246 native Rust/build/catalog/research files used by that receiver match the #75 tree. No new build was performed.

The exact-head run includes all eleven native checks, strict 26-tool schemas/routes, invalid and unsafe ID refusal, response correlation, raw HTTP errors without automatic replay, and owner-approved JSON-string/Markdown output behavior. The actual native cases retain fork contention, independent checkpoint/branch workflows, complete non-active-target recovery after a real 503, and a separate unloaded API as the global-content negative control.

[Complete receiving review](RECEIVING_REVIEW.md), [source preservation comparison](source-preservation-review.json), [123-case receipt](receipts/unchanged69-suite.json), and [136-case receipt](receipts/exact75-suite.json) are directly reviewable here. The parent-reported two-pass/two-fail #69 negative probe is labeled as such; its original harness is byte-exact, but unavailable raw output is not invented.

This reviewer authored the earlier #68 guard. The packet independently receives another worker's composition and the preserved career-lab behavior; the earlier guard's independent source review remains with the parent. Full MCP conformance, arbitrary-precision JSON parsing, deployment and integration are outside this evidence publication. Fresh owner/source/review/CI/ancestry gates remain necessary for later merges.

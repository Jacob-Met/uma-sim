# Independent UMA #69 final ancestry and CI review

**Disposition: PASS for frozen head `e914b047973710bb1d50885730dea28bc8025b0b`
composed with base `b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c`.
Current-main integration remains held for the later `0bc58cb8` source refresh.**

Reviewer: `research_integration`, independently of the product implementation
lane. This review used authoritative GitHub objects and actual Actions logs and
artifact bytes. It did not edit another contributor's checkout or rerun the
already qualified standalone protocol suite.

## Source preservation

The reviewed head's second parent is the requested current native base
`b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c`. Its tree is
`43d39a70727bb75ba6c448f7639379682119f44c`. A complete recursive comparison
of both non-truncated trees found **524 base leaves / 580 head leaves**:

- **521** base leaf identities and modes remain identical.
- The only three modified files are the owned MCP implementation,
  its existing handshake test, and the composed CI workflow.
- All **56** added files are inside the declared MCP source/tests/docs and
  two receiving-evidence directories.
- There are **no deleted files** and **no out-of-scope differences**.

Thus all current-base Rust, CLI, UI and other unowned source bytes survive.
MCP implementation Git blob remains
`24d5384c0a14fbdd07c4dd838b6a0d56b6d92ceb`, independently recomputed SHA256
`5f0dfdbb44595acb62026d3da33ca6ef5904ed1aec76dad4ef05cd418ec6f584`.

## Composed CI

Head CI blob is `1ae99b56d4af0635f755b46a3c38d885806ba32b`; base is
`6355e324a5336ab5a8bcd4cdc34b5105fc787ade`. Removing the declared added
native receiving block and restoring the previous narrower MCP test command
reconstructs the base workflow **byte-for-byte**. Main's terminal CLI test step
is unchanged. Existing UI, Rust, release-layout, calibration, permissions and
pinned action steps are preserved.

The new native tests run after the current release API build and point to
`target/release/uma-sim-api`. Protocol tests retain the handshake file through
the broader test glob; native results are retained by the always-run artifact
step.

## Actual hosted receiving

[Run37762223137](https://github.com/Jacob-Met/uma-sim/actions/runs/37762223137)
completed successfully for head `e914b047`. The checkout was GitHub's merge
commit `4810377ccb6989a8faee8e31d3de67729ed0547e`, whose parents are the
requested `b8e3da0c` base and `e914b047` head. Its tree exactly equals the
reviewed head tree above.

The raw jobs report:

| Job portion | Result |
|---|---|
| Protocol/HTTP suite | 129 reported; 125 pass, 4 native tests intentionally skipped with no configured binary |
| Separate current-built native API receiving | 11 pass, 0 fail, 0 skip |
| Preserved terminal CLI suite | 17 pass, 0 fail, 0 skip |
| UI and remaining Rust/release checks | Successful job/step conclusions |

These are distinct execution groups, not new independent testcase claims.
The current native result is bound to the `b8` composition through the actual
checkout and freshly built binary, not the historical `f5f9b293` local receipt.

Downloaded artifact11542952144 has 8,758 bytes and independently matching SHA256
`a4a6e54f7162f87e112e576c91626d98a6cba49a5655fbc00872be81ae7b86e1`.
All sixteen archive files were inspected and retained. All five provenance
records agree on checkout `4810377c`, the exact MCP SHA256 above, and native
API executable SHA256
`10aae96b1859a895e98b3a49d7a1e7bbc7d368edb99383db2e53084471674afe`.
The artifact contains comparison JSON/Markdown, server logs and real native
session/rollback/content control receipts.

## Freshness disposition

During this review, main advanced to
`0bc58cb83e89f07c6af3056dd7c37436906d994f`, tree
`7c1b145124486f2a7911df3cecb620558abf84e1`.
This adds the request-body admission correction in
`uma-sim-core/src/api.rs` (blob `eab9ccf7082ffc96c446f434bd76f313aff14bc9`),
its tests and qualification records. It is a substantive native receiver
change beyond `b8`, though disjoint from the owned MCP source.

The reviewed `e914` head therefore cannot be described as current-main ready.
Preserve this exact passing receipt, conservatively carry the new native base,
and require the refreshed head's CI/native receiving before final integration.
Content mutations are currently held by the parent following GitHub's secondary
content limit. This review makes no remote write, merge, deployment or live
account claim.

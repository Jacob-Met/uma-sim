# Branch result publication receiving — issue #76

Two independent native processes could acknowledge different branch results with the same generated ID. The second save replaced the first JSON file, so a later read or restart returned the second outcome under the first acknowledged identity. This packet preserves the actual failure and the qualified repair.

## Behavior and scope

`BranchStore::save` creates a new immutable result. Its only runtime caller is POST `/v1/lab/branch`, immediately after a fresh run; there is no result-update endpoint. Reusing an ID now returns `LabError::AlreadyExists`, including for an identical payload. The existing API mapping returns HTTP 409. The error text describes refusal accurately for both checkpoint entries and branch results.

Generated opaque IDs now include seconds, nanoseconds, process identity and the full counter. The store also guards the publication itself: it exclusively creates a temporary file in the result directory, writes and syncs all bytes, closes the handle, then creates the final hard link. An existing final directory entry prevents publication atomically. The store removes only its own temporary file. There is no replacing-rename fallback.

The production diff is limited to imports/error wording, `branch_id` and `BranchStore::save` in `uma-sim-core/src/career_lab.rs`. Checkpoint save/import and the common `atomic_write` helper remain with issue #74; comparison/report code remains with #73. API/session, CLI, MCP and engine source are unchanged. The existing explicit checkpoint `overwrite=true` contract remains covered by the native domain suite.

## Native results

| Receiving case | Baseline | Candidate |
| --- | --- | --- |
| Actual API: two first branches in the same second, then native restart | 2 preservation assertions pass, 5 fail | 7/7 pass |
| Independent store tests: reused IDs, competing processes, reopening, dangling destination | 1 pass, 4 fail | 5/5 pass |
| Existing career-lab suite, including checkpoint overwrite, restore/RNG and sibling isolation | Existing tests retained | 14/14 pass |
| Final formatted test through Cargo | Not rerun against baseline | 5/5 pass |

The store suite contains one intentionally ignored worker entry, invoked by its parent test in four real publisher processes and a fresh reader. This is an executable helper, not skipped coverage. The dangling-link case is Unix-specific.

In the actual API baseline, both first branches returned HTTP 200 with `br-1791452919-0000`. The first saved bytes changed, and restart retained only two of the three acknowledged results. In the candidate, PIDs 23217 and 23218 both completed their first branch in the same second, received distinct IDs, and all three results survived restart. The first result stayed byte-identical; the prior sibling, checkpoint files and CLI-session fixture also stayed unchanged. All native processes were stopped and awaited.

The independent collision receiver deliberately supplied the same ID to four processes, separately from generated-ID behavior. Baseline acknowledged all four. Candidate acknowledged exactly one and returned three conflicts; a new process read the exact winner. Both changed and identical repeated payloads were refused, and a dangling destination entry was preserved. Ordinary publication left no temporary files.

## Source and artifact identity

Native source commit: `5fd6cfd0dda23ffefa0427a6d3762e0296391b54`, based on main `0b5cc2342cd93141beb461edf6e83998388e67b8`.

- Candidate domain SHA-256: `f12208e5796ac6ab42ba515d6d8e6734388810894b600420e4badfdeab250f08`.
- Candidate native executable SHA-256: `d45f5f80baf91d32ea8993b0740709723f2e2169f21868a8543587f0af4f31d5`.
- Baseline executable SHA-256: `61298b37b1ecdf7dfaaca892ac51de4b03ba22b233b962f1fa1304f5f0a2fd66`, originally built at `f5f9b29393731d18aee2d66a31d89c315aa79c60`; the relevant native source was unchanged through the inspected base.

`provenance.json` identifies the three production/receiver files by Git blob and SHA-256. `native-build.json` records the cached Cargo build: only `uma-sim-core` recompiled, in 2.09 seconds. `SHA256.json` inventories the packaged evidence.

The independent baseline/candidate pair used exactly the same test bytes, SHA-256 `e925eb000d256ce4545b4054d5740a1c46f1dae3a46cca85dc8a33f20777a7e2`. The final format gate required only rustfmt changes. `independent/career_lab_branch_publication.rs` retains the paired source; `formatting-custody.json` records its mapping to the tracked test, SHA-256 `7aae317e1e99944aa4c7709168ee8068c227865dc66c538f61f3d20b15140b21`. The final Cargo run qualifies those formatted bytes. The old rlib was not retained, so that final formatting-only revision is not misrepresented as another baseline execution.

## Repeat the receiving

From the repository root:

```sh
cargo build --locked -p uma-sim-core --bin uma-sim-api
cargo test --locked -p uma-sim-core --test career_lab_branch_publication -- --test-threads=1 --nocapture
cargo test --locked -p uma-sim-core --test career_lab -- --test-threads=1
UMA_LAB_REPO="$PWD" \
UMA_LAB_FIXTURE="$PWD/docs/receiving/checkpoint-portability-49f845d0dece/candidate/portable-checkpoint.json" \
node uma-sim-core/tests/branch_publication.api.mjs
```

The Node receiver uses loopback-only dynamic ports and a fresh storage directory. It defaults to the debug native API and expects preservation. `UMA_LAB_BINARY`, `UMA_LAB_OUTPUT_ROOT` and `UMA_LAB_SOURCE_REF` select an explicit binary, artifact root and source provenance. Use `UMA_LAB_EXPECT=loss` only when deliberately receiving the failing baseline; that diagnostic mode succeeds only after the targeted overwrite is reproduced while retaining all failed preservation assertions.

## Limits and adjacent work

The receiving proves ordinary publication, conflict refusal and restart preservation. It does not inject a power failure. The file is synced before publication, but no directory-fsync durability guarantee is added. Filesystems that cannot create hard links return a storage error without replacing an existing result. Generator uniqueness is supplemented by the atomic publication guard, rather than treated as a guarantee against every possible future collision.

PR #37 has a different explicitly named CLI design; its separately reproduced manual-history loss and portable failing controls remain on the existing author's contribution branch: https://github.com/Jacob-Met/uma-sim/pull/37#issuecomment-6057220980 . That history defect is not attributed to the landed result-ID implementation repaired here.

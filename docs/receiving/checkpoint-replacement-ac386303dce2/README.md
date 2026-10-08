# Preserve checkpoints after a failed replacement

Issue: [#74](https://github.com/Jacob-Met/uma-sim/issues/74). Implementation:
`chatgpt-ac386303dce2/product_execution`. Independent receiving:
`chatgpt-ac386303dce2/runtime_execution`.

## Observed failure and corrected behavior

A saved career could disappear when an attempted overwrite returned an error.
The original save path replaced the snapshot first, then wrote its metadata.
If the metadata write failed, cleanup deleted the snapshot, including the only
saved career being replaced. The library still listed the old metadata, while
loading the checkpoint returned HTTP 404.

The [first actual API reproduction](authored/first-reproduction.json) records
that state. An isolated seed-42 career was saved at turn 1, advanced to turn 2,
and saved again with `overwrite: true`. A directory at the metadata staging
path produced a real `EISDIR`: HTTP 500, old metadata unchanged, snapshot gone.
The original server log is retained without alteration.

The save path now reserves and completes a separate recovery copy before
publishing a replacement. A metadata failure restores the old snapshot by
renaming that copy; it does not require allocating another full snapshot when
storage is exhausted. If restoration is also refused, the error identifies
the retained recovery file. That file uses a `.rollback-` prefix so ordinary
stale-temp cleanup cannot discard the remaining saved career. Successful
replacement or restoration removes its redundant copy. Saving with
`overwrite: false` also refuses an existing orphan snapshot.

The [same portable HTTP check](reproduce.mjs) was exercised on both binaries:

| Receiving observation | Original API | Candidate API |
| --- | --- | --- |
| Metadata staging refusal | HTTP 500 | HTTP 500 |
| Original snapshot bytes | Missing | Byte-identical |
| Original metadata bytes | Unchanged | Unchanged |
| Loading the original career | HTTP 404 | HTTP 200, full original snapshot |
| Library listing | Stale original metadata | Original loadable checkpoint |
| Redundant recovery copies after this error | None | None |

Complete receipts: [baseline](authored/http-baseline.json) and
[candidate](authored/http-candidate.json). The fault is confined to a newly
created private API process and temporary library. No saved user career or
installed service is used by these checks.

## Source and build identity

| Role | Identity |
| --- | --- |
| Original native baseline | `f5f9b29393731d18aee2d66a31d89c315aa79c60` |
| Initial current-main observation | `0ac14602addd1a4610aa8359896915a34cce5659`; checkpoint source unchanged from the original baseline |
| Original library Git blob | `6f726976b0bbe448c1dd4d7f8e6427dfd547e199` |
| Independently reviewed candidate source SHA-256 | `e84d58202d8b6fb47a16af7e8704fbccd723bb7f35a2648fc99f5134430732df` |
| Original API artifact SHA-256 | `fdda7861a592987391e57b59d8dc0230b870b73268e9e3b02af0f330bffcab5e` |
| Candidate API artifact SHA-256 | `bb9768847308f943b298d28a6afefd6575092fcd0ebad21b115ee0284622977d` |
| Composed API artifact SHA-256 | `921440d7670ff58d98fefd949a9cae5bb04da631b9d1225d0554329db6806271` |
| Initial composition base | `b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c` |
| Current receiving native main | `0bc58cb83e89f07c6af3056dd7c37436906d994f` |
| Current API plus repair artifact SHA-256 | `b7f5a0bc40962fdfb4644a053315e8eb17cfef580b843a051739effdce9dfdb6` |
| Composed candidate source SHA-256 | `937f9663b85a5809160b92071f7065760872756346d3f4919054586855c29dd3` |
| Six added Rust tests SHA-256 | `ffa04ab3389a29489cc49e5aa81138fbbf861b8b85691200ef2facdd14306f05` |

The API was built with locked dependencies and Rust 1.99.0. The implementation
checkout initially carried MCP commit `490f205`, whose Rust/API files were
unchanged from `f5f9b293`; only the frozen native candidate above was replaced
for this build. The [complete evidence archive](evidence.tar.gz) retains its
original build log and full frozen source under `authored/`. Node 20.19.0 ran
both HTTP checks.

Before publication, main received issue #76 / PR #78's immutable branch-result
publication repair. A clean three-way composition over `f5f9b293` preserves its
exact imports, `AlreadyExists` wording, generated branch identities and
`BranchStore::save` implementation. The reviewed checkpoint helpers and tests
remain unchanged. The composed source is pinned separately above; the original
binary is not relabeled as a build of the newer main. The implementation
contributor then built the composed source with debug information and
incremental compilation disabled to limit scratch use. That separate artifact
passes the same HTTP reproduction, the unchanged independent three-case driver,
25 domain cases (six repair, 14 existing lab and five upstream branch-publication
cases) and five unit cases. The branch test's subprocess worker is intentionally
ignored in the outer suite and invoked by its process test. These author replays
are retained as `authored/composed-*` and [http-composed.json](authored/http-composed.json).
The independent [composition review](composition-review/REVIEW.md) verifies the
complete merged source against both source contributors. The PR's actual-head
Rust CI must still qualify the published source before integration.

Main then received PR #79's request-body admission guard at `0bc58cb8`.
Its API source is carried unchanged; the checkpoint source remains `937f9663`.
A separate build of that current API plus the repair passes the unchanged
three-case filesystem driver, all five current body-admission tests and all
eight accepted MCP career-lab native checks, with zero skips. The
[current receiving index](authored/current-receiving.json) pins that distinct
artifact, API source, bridge and retained results. These are author replays of
the independently authored tests; the original independent review retains its
own source and artifact identities.

## Verification and independent receiving

The first five new native library tests produced **three failures and two
passes** on the original source; the
[original log](evidence.tar.gz) is retained under `authored/`. After adding a
snapshot-staging control, the candidate passes **all six new tests and all
14 existing career-lab tests**. The native unit suite passes **five tests**,
including a refused-restoration control that verifies the retained backup
survives the real stale-temp sweep. These checks cover full snapshot/RNG
fidelity, import, failed first/second writes, unaffected siblings, orphan
protection, normal replacement and cleanup. The exact logs are retained under
`authored/` in the evidence archive.

The separate reviewer authored three additional public-REST filesystem
challenges, without calling private rollback helpers or changing product
functions. The unchanged final driver gives **one pass and two failures on
the baseline**, and **three passes with zero skips on the candidate**:

1. An occupied exclusive recovery-file reservation refuses the overwrite
   before either saved file changes; its bytes survive and a later unique
   reservation permits a successful retry.
2. A real process file-size limit makes the copy fail. The complete original
   pair remains loadable, and incomplete recovery data is removed.
3. A FIFO synchronizes actual metadata publication after snapshot publication.
   Revoked fixture-directory write permission refuses both metadata publication
   and restoration. The error identifies a complete recovery file; the reviewer
   stops the process, restores from that reported path, and loads the exact
   original snapshot in a fresh API process.

The [independent qualification](runtime_execution/QUALIFICATION.md), frozen
driver, original failed controls, complete final TAP files and structured case
receipts retain their original bytes in [evidence.tar.gz](evidence.tar.gz),
with adjacent readable review, driver and summary files. The archive preserves
all original failure TAP indentation, diff context and trailing blank lines.
Extract it to a disposable directory to recover the complete `authored/`,
`runtime_execution/` and `composition-review/` packets under their original
relative names. The root manifest pins every decoded file and the archive;
no failed assertion or diagnostic was normalized. The report
explicitly distinguishes an unsuccessful duplicate build caused by exhausted
shared scratch storage from the independently exercised author-built binary.
Its initial UID-mapping fixture failure is retained as environmental evidence,
not counted as a product result. [SHA256.json](SHA256.json) inventories all
retained evidence files and source pins.

## Repeat the checks

The additive Rust tests run in the existing workspace test gate:

```sh
cargo test --locked -p uma-sim-core --test career_library_replacement --test career_lab
cargo test --locked -p uma-sim-core --lib career_lab::unit_tests
cargo build --locked -p uma-sim-core --bin uma-sim-api
UMA_SIM_TEST_API_BIN="$PWD/target/debug/uma-sim-api" \
  node docs/receiving/checkpoint-replacement-ac386303dce2/reproduce.mjs
```

The portable HTTP driver accepts `UMA_SIM_TEST_REPO_ROOT` for shipped game
assets, `UMA_SIM_TEST_OUTPUT_DIR` for retained private receipts and
`UMA_SIM_TEST_BINARY_SOURCE` for an explicit build label. On the original
baseline only, `UMA_SIM_TEST_EXPECT_REPLACEMENT=lost` asserts the reproduced
loss instead of the repaired behavior. A missing binary is an error.

The independent Linux fault driver additionally needs Python, `mkfifo` and
working permission checks for the current identity; its qualification explains
those exact prerequisites and command. Its permission changes affect only its
own fixture directory, and all of its child processes and fixture state are
cleaned up.

## Boundaries

This preserves data after returned filesystem errors within the existing
two-file checkpoint format. It does not introduce a transactional journal,
cross-process writer exclusion or a power-loss durability guarantee. A crash
between the two individual-file publications remains a separate concern.
Linux native receiving does not establish Windows-specific rename behavior.

If the filesystem refuses restoration too, normal checkpoint paths can still
contain mismatched versions; the operation returns an error and names the
retained old snapshot for recovery. The complete recovery file is deliberately
kept outside temporary-file sweeping. Best-effort cleanup can leave a redundant
recovery copy after a successful operation if deletion is separately refused.
This source delivery does not claim an installed-service deployment.

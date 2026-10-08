# Independent checkpoint replacement recovery qualification

Contributor: ChatGPT HAMON execution, session `ac386303dce2`, runtime execution
lane. The reviewer did not author the checkpoint recovery implementation.

## Exact target and disposition

The frozen `career_lab.rs` source is
`e84d58202d8b6fb47a16af7e8704fbccd723bb7f35a2648fc99f5134430732df`.
The exercised candidate API artifact is
`bb9768847308f943b298d28a6afefd6575092fcd0ebad21b115ee0284622977d`.
The source was independently read in an isolated copy of current main
`0ac14602addd1a4610aa8359896915a34cce5659` with only that native file replaced.
The original native file is unchanged from
`f5f9b29393731d18aee2d66a31d89c315aa79c60`, SHA-256
`95cb00bd9418e889372b83eb998f8100887218d91b288af9180deff0843427c7`.
The baseline API artifact is
`fdda7861a592987391e57b59d8dc0230b870b73268e9e3b02af0f330bffcab5e`.

No source blocker was found for the stated returned-I/O-failure recovery scope.
The three independent public-REST filesystem challenges pass on the frozen
candidate. The unchanged final test fails two baseline cases and passes one
preservation control. This review does not claim transactional two-file commits,
cross-process exclusion, power-loss durability, Windows qualification, or a
deployed application update.

## Independent behavior challenges

The final test `checkpoint-filesystem.test.mjs` has SHA-256
`46ab3397ad67ee09ab317fbde7ecaf9d6fb14f928f19e710f40825e80ace5f81`.
It starts actual Rust API processes, creates original checkpoints through REST,
and supplies authored kernel/filesystem faults in private temporary directories.
It does not call private rollback helpers or patch production functions.

| Boundary | Baseline | Candidate |
| --- | --- | --- |
| An existing recovery reservation occupies the proposed backup name | Replacement succeeds despite the occupied reservation | Returns HTTP 500 before changing either saved file; preserves the existing recovery bytes; a subsequent unique reservation permits a successful retry |
| Copying cannot complete under a 1 KiB process file-size limit | HTTP 500 during new snapshot staging; old pair remains intact (control) | HTTP 500 identifies the preservation stage; old pair remains byte-identical and loadable; no incomplete recovery file remains |
| Metadata publication and rollback are refused after replacement snapshot publication | New snapshot remains, old metadata remains, and no prior snapshot recovery file exists | Error identifies the retained complete old snapshot; it survives API process exit and supports exact restoration followed by HTTP 200 load in a fresh process |

For the last case, an authored FIFO holds the real metadata writer after the
new snapshot has reached its final path. The test then removes parent-directory
write permission and releases that writer. The actual metadata rename and
restoration rename receive `EACCES`. The final baseline receipt explicitly
records `snapshotMatchesPrior=false`, `metadataMatchesPrior=true`, and no
recovery files. The candidate records the retained recovery filename, verifies
its complete bytes, stops the process, restores from the reported path, and
loads the original checkpoint through a new API process. The restored snapshot
and metadata hashes both equal their pre-save values.

The size-limit case uses an isolated Python process launcher to lower
`RLIMIT_FSIZE` and ignore `SIGXFSZ` before exec of the actual Rust binary. The
complete snapshot exceeds the limit, so preservation cannot finish. The
observed error is `EFBIG`; the test verifies cleanup and prior bytes, without
claiming a measured intermediate prefix length.

## Results and reproduction

Node `v20.19.0`, Linux x64:

- `baseline-final.tap`: **1 pass, 2 failures, zero skips**, 5.614 seconds.
- `qualified.tap`: **3 passes, zero failures, zero skips**, 5.701 seconds.
- Complete structured cases: `baseline-final/case-receipts.json` and
  `qualified/case-receipts.json`.

Run with an explicit candidate/baseline API path and an exact repository source
root for read-only game assets:

```bash
UMA_SIM_TEST_API_BIN=/path/to/uma-sim-api \
UMA_SIM_TEST_REPO_ROOT=/path/to/verified/uma-sim \
UMA_SIM_TEST_OUTPUT=/path/to/disposable/output \
UMA_SIM_TEST_RECEIVER_ID=exact-receiver-pin \
node --test checkpoint-filesystem.test.mjs
```

`UMA_SIM_TEST_PYTHON` can name Python explicitly for the file-size-limit launcher.
The configured test also requires `mkfifo` and Linux directory permissions to
apply to its current process identity. It skips when the API binary is unset
or the platform is not Linux. A configured invalid binary on Linux fails.
The qualified sandbox used UID 0 with no effective or bounding capabilities;
an actual read-only-parent probe returned `EACCES`. It did not remap users,
change accounts, or alter any non-fixture permissions.

## Source review and limits

The prior snapshot is opened and a fresh backup is exclusively reserved before
the replacement reaches its final path. A failed/incomplete copy is removed
before any primary mutation. On first-file publication failure, the old primary
is still present; on metadata failure, restoration uses the already complete
backup rather than allocating another full snapshot. A failed restoration keeps
the backup and reports its path. Successful restoration/replacement cleans its
own redundant backup; an unrelated existing recovery artifact is preserved.
The existing stale-temp sweep does not match the `.rollback-` prefix; the
author's separate native unit test covers that helper boundary.

The file format and existing native load/import behavior remain intact. Failure
can leave new snapshot bytes paired with old metadata when restoration is also
refused, so the returned recovery diagnostic is essential; the test does not
present that state as a successful save. Kernel rename and file-open behavior
can differ on other platforms. This Linux receiving result and conservative
source review do not replace their native qualification.

## Retained negative evidence

An independent duplicate build reached the actual Rust compiler but failed
while creating the core archive with `ENOSPC`; it did not produce a candidate
binary. Its log is retained at the review workspace's `build.log`. The completed
author build artifact above was then read by exact hash and independently
exercised. The review claims source inspection and independent receiving of
that artifact, not an independently reproduced successful compilation.

The first driver attempt tried to select UID 65534 and failed at fixture setup
with `EINVAL` because the sandbox maps only UID 0. No API started. The corrected
driver keeps the current identity, whose actual capabilities and permissions
were verified. `candidate.tap` retains this initial environment failure.
Two preservation-error wording assertions were removed before freezing the
final test so baseline outcomes distinguish data/authority behavior from message
format. The final rollback receipt records state before its assertion, exposing
the baseline's missing recovery bytes directly.

Fixtures and logs live in `/dev/shm` because the shared workspace overlay was
full. Every test removes its private application state and stops only its own
process. The integrating contributor should retain this packet and exact
source/binary identities in the existing issue/PR evidence destination.

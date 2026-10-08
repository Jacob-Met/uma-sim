# Independent terminal receiving on checkpoint-replacement main

## Accepted result

The current checkpoint-recovery API passes all 16 existing terminal groups and all three existing request-admission groups. The two frozen receivers keep every behavioral assertion and scenario; only their private root path and the Mac-specific Node launcher were adapted for Linux.

The successful runs exercised 34 terminal processes, three actual Rust API processes and 395 HTTP requests. All groups passed. The 16-group receipt covers exact signed-64-bit addressing, create-only saves, duplicate refusal, read-only listing, named session isolation, concurrent forks and saves, real API restart, checkpoint/RNG continuation, visible native errors, failed first-save cleanup, committed-but-lost responses, EOF/SIGINT, and absence of implicit replay or reset. The three admission groups retain malformed/nonobject/unreadable request controls and authoritative next-action behavior.

Seven complete state artifacts also match the previous accepted Mac/fe2 run after ignoring object key order only. Every numeric JSON source token, array position and other value is compared exactly. Actual save-time metadata is excluded from cross-run equality; each run independently proves its own saved metadata remains byte-identical through restart/action/error checks.

## Exact current native subject

Current main: `09df4c25cbd86c12d16a121481c01d1633a5efb6`.
Canonical tree: `8352de08ded783d6d23030b312c36c5b9a4b0fe1`.

The independently exercised existing Linux x64 executable is:

`/workspace/scratch/ac386303dce2/product-checkpoint-composed-target/debug/uma-sim-api`

Its SHA-256 is `b7f5a0bc40962fdfb4644a053315e8eb17cfef580b843a051739effdce9dfdb6`. No binary was rebuilt, modified or relabeled. The owner's published current receiving index and lossless archive bind that artifact to the later request-admission API and checkpoint repair.

The actual donor is `/workspace/scratch/ac386303dce2/product-checkpoint-recovery`. Its Git HEAD is the earlier `490f205d013abbf1774894ea23c5b1ac2c3d32c7`; the working files carry the later qualified composition. Independent before/after checks bind all 120 relevant Rust, Cargo and content files to the actual current-main Git blobs. The donor HEAD is not misrepresented as current main.

The API blob is `eab9ccf7082ffc96c446f434bd76f313aff14bc9`, SHA-256 `31e51bf33c6cd5f51c7f7cb79a4560fad5c2e06a77f0345ff659cf114fabdd55`.
The changed library blob is `83025a33775d3559c54c56c0183badd1c919cc50`, SHA-256 `937f9663b85a5809160b92071f7065760872756346d3f4919054586855c29dd3`.
The workspace lock remains `1f99851c45524f129e8544a2d779b3c21594d816`, SHA-256 `e2d929ee14f3b0a67bd8f53c7eda614eb4edf312b4ce6e2116f155b7d7be4f2c`.

The terminal is the frozen checkpoint implementation from source `3077775807c1eb5c2af6937038bd26cf5f00c75b`, also preserved in `f419b9e03db811777d2dea6dfaf830b7bb259dc2`. Its SHA-256 remains `653acbb0b858d7bdc420904e26452e7739dffcc9c2aa73168b8fa801a1d63b4e`.

The receiver-local source root is an explicitly declared composition of that terminal with current native/CI inputs and read-only donor links. It is not a complete named Git commit; the receipt's source_commit is therefore null, with all component identities recorded separately. All 11 staged source-file hashes, the baseline terminal and the executable are checked before and after the runs.

## Material source review

The current library now reserves a complete prior-snapshot recovery copy before replacement, restores through rename after metadata failure, and retains a reported recovery file if restoration fails. Create-only saving also rejects an orphan snapshot. The terminal continues sending overwrite:false and never switches to replacement implicitly. Its failed new-save, conflict, restart, selected-session and uncertain-action controls all pass unchanged. No new checkpoint format, API handler, TUI implementation or workflow alteration is inferred from this receiving.

The source change remains bounded by its owner's returned-I/O recovery contract: this packet does not establish cross-process writer exclusion, two-file power-loss atomicity or platform-independent rename behavior.

## Environment and preserved preparation failures

Runtime: Node v24.19.0 with native JSON numeric-source context, Linux x64. The workspace overlay had zero free bytes; this receiver used only its own bounded /dev/shm namespace. It made no donor writes, no Mac writes and no native builds.

The first Linux attempts retained the original /opt/homebrew/bin/node launcher and stopped with ENOENT before any terminal process began. Their actual API setup, logs and failed first-group receipts are retained under preparation/. A second namespace changes that launcher to process.execPath and changes the private root constant; driver-port-v2.json proves that all other bytes are identical to the previously qualified receivers. A staging-added trailing newline was corrected before those first runs; the small correction receipt and exact prepared beforeimages are also retained.

The original 3077775/b8e and 92db1618/fe2 receipts remain immutable. This is a new source-and-platform receiving record, not a rewrite of those earlier results.

## Evidence

The packet contains both successful raw receipts/logs, all captured state artifacts, the two portable drivers, exact port deltas, prior/current state comparison, donor/source before-and-after pins, and the preserved preparation failures. It is intended to be appended unchanged to PR89's qualification evidence. No service deployment or installed-user checkpoint mutation occurred.

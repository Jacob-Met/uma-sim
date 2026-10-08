# Current-main composition after checkpoint recovery

The append receives current main `f27829a266bb39fb1a4f298e2c2b3b32f380af0f` while preserving the published terminal head `78fa80ee0102ef46af1a3e46908fd41d227ee53c` as its first parent. Its additional parent is current main. All 140 previously qualified paths retain their exact published Git blobs and modes. All 674 unaffected current-main leaves remain exact, including the current UI, MCP, native CLI, checkpoint replacement implementation, training demo and their tests. There are no deletions.

## Newly received checkpoint implementation

The separate [independent current receiving packet](../independent-current-09df/README.md) is appended unchanged: 36 files, 565,850 decoded bytes, original container SHA-256 `676678b649d426360ac83dcfd144b223d78c304ce5e721517e8072609ba6d0a2`. It reports 16/16 terminal groups and 3/3 request-admission groups passing against the actual Linux API with 34 terminal processes, three API processes and 395 HTTP requests. The API process is genuinely restarted; saved bytes and exact state/RNG continuation are checked after recovery. Concurrent forks, competing create-only saves, native errors, post-commit response loss and EOF/SIGINT remain covered.

That receiving uses checkpoint-recovery main `09df4c25cbd86c12d16a121481c01d1633a5efb6` and the owner's exact API executable SHA-256 `b7f5a0bc40962fdfb4644a053315e8eb17cfef580b843a051739effdce9dfdb6`. It retains the unchanged request-admission API blob and newly received library blob `83025a33775d3559c54c56c0183badd1c919cc50`. The terminal continues to request create-only saves and never enters replacement implicitly.

## Subsequent main changes

Main `f7378bc1f60a0132d703aee9a1c36d6fef913217` differs from the received 09df tree in only three paths: the root README, `uma-sim-core/src/bin/uma-sim.rs` and its new `cli_seed_admission.rs` tests. That native CLI now rejects invalid seeds before changing a career or output. The later f278 advance changes only four `packages/training-demo/` files: README, builder, output receipt and build receiver. All seven owner path changes are preserved.

The receiver's broad inventory contained both native binary targets. Of its 120 recorded paths, 119 remain byte-identical. The one changed path is the separate `uma-sim` CLI entry. Cargo declares that target separately from `uma-sim-api`; the API entry calls the unchanged library's `api::serve`. The library module root, API implementation, checkpoint library, dependency manifests/lock and catalog are unchanged. The training-demo paths do not enter that native API target. The accepted actual-API receiving remains applicable without another native build or repeated runtime test. The receipt records the differing CLI blob explicitly and does not claim all 120 files remain identical.

## CI and scanner preservation

The existing reviewed terminal workflow insertion remains exact. It adds ten lines to the current owner's Rust job, reuses Node 20, runs the terminal native suite against the release API and retains its receipt. All owner workflow content remains recoverable byte-for-byte by deleting that insertion.

The existing scanner configuration is also exact. No new allowlist or exception is added for this evidence. Actual hosted checks on the appended head must pass before integration. Earlier native, HTTP and scanner results remain separately preserved under their original source identities.

[composition.json](composition.json) binds the parents, all 140 preserved paths, the complete 120-input comparison, source entry points, packet identities and expected 852-leaf combined tree. The material source boundary is the checkpoint owner's returned-I/O recovery contract; this work does not assert cross-process write exclusion or two-file power-loss atomicity. This is source qualification and proposed repository integration, with no deployment claim.

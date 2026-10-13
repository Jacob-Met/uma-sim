# Paired comparison reader — source and static receiving

The new [standalone HTML reader](../../PAIRED_COMPARISON_REPORT.md) consumes a complete saved native paired-comparison JSON report. It adds a readable summary and every exact paired row, with source-reviewed local search, outcome filtering and sorting. It changes no native comparison, career, simulator, frontend or existing telemetry reader.

**Qualification:** the Python command, admission/refusal behavior, complete static HTML and byte-exact embedded JSON passed. **Actual browser behavior is unqualified:** the available browser rejected local-file navigation under its security policy before a page loaded. No alternative surface, server or indirect-execution workaround was used. The interactive controls, actual download, disabled-JavaScript presentation and responsive layout have not been observed in a browser.

## Use the existing result

- [Historical sixteen-pair reader](examples/saved-sixteen-pairs.html).
- [Exact historical input JSON](examples/saved-sixteen.json).
- [Authored mixed fourteen-pair reader](examples/mixed-fourteen-pairs.html) and [its fixed input](examples/mixed-fourteen.json).
- [Command guide](../../PAIRED_COMPARISON_REPORT.md), including a digest-bound invocation and limits.

Download an HTML file and open it locally in your own browser. Repository code views may display its source instead of the page. Each HTML contains the exact input JSON as a download; no simulator binary, server or external assets are required. Keep this source context with the historical example.

### Historical example context

The sixteen-pair file is the unchanged 4,069-byte native comparison from the earlier [fixed-cohort study](https://github.com/Jacob-Met/uma-sim/blob/eb4f7de067824ed24818d93d5fdca87d3818a2da/docs/research/paired-policy-cohort-713adaab/REPORT.md), SHA256 `7b15171bbdec97c6bc52671b106deba787dfec3f5ecac50b79bf30f5faa4f473`. It describes seeds 0–15, URA / Special Week, default versus the then-existing local bot, with empty support/inheritance configuration. Its labels in this reader are `Default (saved study)` and `Local bot (saved study)`.

The historical producer used the accepted `4eab1798def5c19fbdee267eb2a6ea8a2726dbe7` composition plus its paired-comparison overlay, not this reader's current-main source. That model retained the study's stated URA/finale limitation. The original source owner and its unpublished integration boundary remain separate. This reader does not run or update that study.

All sixteen saved score differences are positive: mean 2,500.6875, median 2,570, minimum 1,066 and maximum 3,204. Those are descriptive terminal-score observations from that represented software. They are not race wins, an official game score, proof of independent sampling or a causal/general policy result. The full historical report supplies configuration, scorer and scientific limits.

The mixed fourteen-pair example is an authored receiving fixture: six positive, three zero and five negative deltas, signed seed endpoints and adjacent integers beyond JavaScript's exact-number range, both widened score-difference endpoints and literal unusual text. It is not a simulator study or observed policy performance.

## Source and admission

The qualification basis is `Jacob-Met/uma-sim@cd852b3be106855524e789c42670d38f419d3bd5`. The maintained native report structures and comparator are in `uma-sim-core/src/batch_pairs.rs`, Git blob `d083c65c64053a87da11a809de83d04184cc393b`. The new command is `scripts/render_paired_comparison.py`; its focused eight-method suite is `scripts/tests/test_paired_comparison_report_596c.py`.

The command verifies the selected file's hash, strict UTF-8/JSON/schema, complete ordered unique identities, native integer/string bounds, every widened delta and the complete descriptive summary before writing. It exclusively creates a new file and checks input custody before and after writing. Cleanup after a filesystem error is best effort and never authorizes deleting a substituted file. See the guide for the exact tolerance and cleanup boundary.

## Retained checks

| Evidence | Scope |
| --- | --- |
| [Maintained receipt](maintained-receipt.json) and raw streams | Eight methods on CPython 3.12.14; actual CLI output, exact integers/literals/download bytes, summary/schema/refusals and file-custody failure paths |
| [Historical launch](saved-sixteen-launch.json) | One actual invocation on the retained sixteen-pair JSON; source/input bytes and nanosecond mtimes unchanged |
| [Independent receiving protocol](receiving/PROTOCOL.md) and oracle | Frozen before reading the renderer; exact mixed fixture, refusal set and intended browser expectations |
| [Complete raw receiving](receiving/receiving-raw.json.gz) | One successful independent mixed render plus 31 refused commands; 125 recorded predicates, complete streams, fixtures and protected snapshots |
| [Owner reconciliation](independent-receiving-review.json) | Every raw stream and predicate, current protected byte/inode/mtime identity, and both complete static reports independently reconciled without another renderer course |
| [Source peer review](source-peer-review.json) | Complete new source/tests/guide and native schema, including the tiny-negative standard-deviation and cleanup precision corrections |
| [Browser boundary](receiving/browser-boundary.json) | Exact available-surface refusal; no loaded page or interaction acceptance |
| [Portable offline reconciliation](receiving/RECEIVING.md) | Fixed-pin checker for all 115 raw payloads and complete static reports; no renderer or browser invocation |

Static receiving checked all rows, exact seed and delta strings, caller labels as literal text, complete cards/legend/endpoints, the inline-script CSP hash, no external resource references, and original embedded JSON byte equality. Static checks are not browser execution.

The first author eight-method course passed before an explicit nonnegative standard-deviation guard was added. The final retained course includes the all-tie `-5e-10` refusal and before/after-output input-change cleanup controls. The independent final course used unchanged final source. An initial owner-only static replay compared Python tuples directly with JSON arrays; the sole serialization mismatch was corrected by comparing complete JSON serialization. It did not alter inputs, output, receiver expectations or source, and no renderer was repeated for it.

## Integration and delivery

[Issue165](https://github.com/Jacob-Met/uma-sim/issues/165) reserves only the new reader, tests, guide and this evidence/example directory. Publication is a reviewable branch, not a merge or installed adoption. The inherited workflow files remain unchanged; no PR is opened while its `pull_request_target` event would run. The independent no-Actions removal owner remains responsible for that separate change.

The source publication manifest binds the complete new contribution. Its immutable head and the source/example packet are linked from the issue and existing UMA Applications record after remote readback. No native executable, runtime assets or simulator course are added to this packet.

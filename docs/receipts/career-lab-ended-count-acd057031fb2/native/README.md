# Branch-ended count regression

`compare_branches` correctly selected which branch ended, then used the other
branch's timeline length in its explanation. Two recorded steps compared with
three therefore said the shorter branch ended after three steps. Reversing A
and B produced the same incorrect count; an empty branch also borrowed the
continuing branch's count.

The correction swaps only the two timeline-length references in that note.
Alignment, first-divergence selection, outcome metadata, and JSON structure
are unchanged.

Six native tests were committed before editing production source. They use
complete authored public `LabResult` values with identical common prefixes.
They cover both shorter-branch directions, changed shorter and continuing
lengths, empty branches, stale summary counts, equal-length controls, and
earlier decision/outcome divergence precedence. Assertions also tie the
explanation to the first missing row and its JSON representation.

| Native run | Passed | Failed |
| --- | ---: | ---: |
| Baseline before production edits | 2 | 4 |
| Baseline replay with compact build profile | 2 | 4 |
| Candidate with identical frozen tests and compact profile | 6 | 0 |

One initial candidate compilation exhausted shared storage before tests ran.
Its compiler log is retained separately. The successful candidate used a new,
exclusively owned tmpfs target with debug symbols and incremental caching
disabled. The baseline was also replayed using that compact profile.

`receipt.json` records the exact source/test commits, SHA256 values, environment,
and results. Each `.log.gz` file contains the exact raw log named by that
receipt; its SHA256 refers to the uncompressed bytes.

Replay with the provisioned Rust toolchain and repository lockfile:

```sh
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo test --locked --offline -p uma-sim-core --test career_lab_divergence -- --test-threads=1
```

This branch starts at main `f5f9b29393731d18aee2d66a31d89c315aa79c60`
and excludes the separate Markdown-renderer repair. Root composes both fixes;
the artifact worker owns independent natural API and rendered-Markdown
receiving. These native fixtures do not claim game-engine execution.

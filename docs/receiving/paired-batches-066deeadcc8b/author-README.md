# Paired batch comparison — author receiving packet

This packet binds the native `compare-batches` addition to canonical
`Jacob-Met/uma-sim` commit `7a2e15f623a559b2d9d425e4e7b68c1e71f24cbb`
(tree `895e790957919ee1d9e64f6336c0fc176bfb35cb`) and the exact nine-file
candidate in `candidate-freeze.json`.

## Outcome and source boundary

The command pairs the complete, unique recorded (scenario, trainee, signed seed)
identity sets from two bounded UTF-8 JSONL files. It reports input-minus-baseline
score differences, descriptive summaries, all JSON pairs, and the ten largest
absolute changes in text. Both complete inputs and the full cohort match are
admitted before primary output. Real Unix output errors return failure.

The only edits to existing source are three additive CLI hooks and one library
module export. Removing those exact additions recovers the canonical bytes.
Existing aggregate analysis, batch production, session handling, engine, scoring,
RNG, race, datasets, manifests, lockfiles, and workflow files are unchanged.

The initial native source capsule contains 273 exact canonical paths: both
crates' source and tests, manifests and lockfiles, and research/knowledge/content
data. A separate supplement binds all nine canonical example source files. They
were added after Cargo's formatting command reported seven explicitly declared
example paths absent from the initial projection. The CLI baseline build and
targeted candidate tests did not depend on those example targets. The final
native source projection therefore binds 282 canonical paths plus seven new
product paths, with two existing paths modified by the owned hooks.

Current receiving commit `2378365e48234c5ae3311e1b0dd7f1247a5a9071`
(tree `04539ad9db458a59f6b044cc7686b7769def9d94`) contains 903 blob leaves.
All 282 exercised canonical closure paths remain exact, and no new owned
path exists there. Its 226 changed/new leaves are unrelated UI, terminal UI,
CI and receiving evidence. The full tree and path delta are retained.

## Actual qualification

- The unchanged canonical CLI was built with ordinary-user Cargo 1.99.0 and
  Rust 1.99.0 on macOS using `cargo build --offline --locked -p uma-sim-core
  --bin uma-sim`. It succeeded. Its frozen binary is 14,795,088 bytes,
  SHA256 `59cfce9bac7007a6b1fc347fa58565d9130dc3d61fe38470dc3530432fe97039`.
- The final focused gate passed all **12 test methods**: seven library methods
  and five methods invoking the actual native CLI. They cover exact identity
  pairing, complete matching, duplicate and malformed admission, signed numeric
  endpoints, byte limits, literal Unicode/whitespace, deterministic text/JSON,
  real failing stdout/stderr descriptors, and unchanged input/session fixtures.
- `cargo fmt --all -- --check` passed after the canonical example supplement.
- `cargo clippy --workspace --all-targets --offline --locked` passed using the
  repository's existing warning policy. The complete warnings remain in its
  log, including Rust 1.99's nonfatal `is_multiple_of` style suggestion for
  the new median parity expression. This is not a zero-warning claim.
- The separate native consumer driver ran **six captured processes**: canonical
  default batch, candidate default batch, candidate bot batch, candidate paired
  comparison, and canonical/candidate aggregate analysis. Each batch used
  the maintained physics engine with seeds 42, 43, 44, URA, Special Week.
  The canonical/candidate default JSONL files were byte-identical. Both existing
  aggregate reports were byte-identical. The paired command's actual records
  matched an independent Python arithmetic calculation from those producer
  files. Input files, the authored session sentinel, all nine overlay files,
  and all 271 unmodified initial-capsule files were unchanged.

The three actual score changes in that small workflow were 2,964, 2,283, and
3,132; their mean was 2,793 and median 2,964. These are descriptive functional
evidence from the selected careers, not a performance, significance, authenticated
configuration, or common-randomness claim. The example JSONL files in the product
guide are separately authored illustrative records.

The final received executable is 15,061,840 bytes, SHA256
`ab563811709ef2788bd0bc563c14da6c842c7d1bc664218ba7c6a2e2f10b61ab`.
The nine-source freeze SHA256 is
`28029900f8ac9131ffe57e09c62212ec8761f93591fc5198802453c1d6ec753a`.
Native source root: `/Users/me/uma-paired-066deeadcc8b/source`.
Native binary: `/Users/me/uma-paired-066deeadcc8b/target/debug/uma-sim`.

## Preserved failed attempts

The first candidate compiled and passed the seven library methods and four CLI
methods, but its real read-only stdout descriptor control returned exit 0
instead of 1. The standard stdout wrapper suppressed EBADF. This was a real
delivery-status defect, not a changed test expectation.

The original nine files, failed gate output, command receipt and source freeze
are preserved in `evidence/candidate-attempt-1/`. The original executable remains
at its recorded native path; its byte identity is in `binary-custody.json`.
The corrected Unix adapter duplicates stdout safely into an owned File and
propagates its write error. No test expectation was weakened.

The initial formatting failure from the partial example projection and its
command receipt are retained under `evidence/projection-missing-examples-*`.
Adding the exact canonical examples resolved that preparation failure without
editing a manifest or unrelated source.

## Independent receiving and scope limits

Root's independent receiver was authored and frozen before implementation
inspection. Its baseline/candidate results and exact raw artifacts are a separate
peer packet; they must not be counted as author tests or pooled across source
revisions. This author packet does not assert full workspace test, embedded UI,
Windows, or hosted CI success. Existing hosted gates qualify the eventual PR
checkout independently.

All ordinary packet artifacts are listed in `packet-manifest.json`. Run:

```sh
python3 verify-packet.py
```

The portable verifier recomputes every listed byte length and SHA256, checks
the nine-file source freeze, confirms the preserved failed gate and successful
final gates, and recomputes the arithmetic from the six-process evidence.
Canonical public data are identified by exact Git blobs in the source capsule
and tree rather than duplicated. Native executable bytes remain at the preserved
native paths; binaries and build caches are not embedded in this compact packet.

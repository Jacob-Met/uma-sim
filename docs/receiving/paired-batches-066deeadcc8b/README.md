# Saved-career batch comparison receiving

The native `compare-batches` command compares every matching recorded career in
two saved JSONL batches. Usage, admission rules, schema and interpretation are
in [PAIRED_BATCHES.md](../../PAIRED_BATCHES.md).

The nine-file source overlay adds two Rust modules, two test files, two authored
JSONL examples, one guide, and hooks in the existing library and CLI. Removing
the three CLI hooks and one library export recovers the canonical source bytes.
The existing producer, aggregate analysis, engine, random-number generation,
data, manifests, lockfile and workflows are preserved.

## Accepted evidence

| Evidence | Result |
| --- | --- |
| Canonical native baseline | The maintained binary built from exact source; two independent baseline processes preserve absent-command behavior and aggregate-versus-paired median semantics |
| Original author qualification | 12 focused test methods, formatting and workspace/all-target Clippy pass; full warnings and the original failed output-status control are preserved |
| Actual maintained producer composition | Six processes; canonical/candidate default batch JSONL and aggregate output are byte-identical; paired arithmetic is recomputed from actual default/bot batch bytes |
| Original independent receiving | 84 regular cases, six exact/over resource limits, and two JSON/text consumers of actual native batches: 92 candidate processes, plus two separate baseline witnesses |
| Current source composition | All nine owned source files unchanged; 284 canonical paths independently bound to receiving tree `bcd00030f309a41cb5670b509d41381f12d1e621` |
| Current author qualification | The same 12 focused methods, formatting and workspace/all-target Clippy pass on the rebuilt current source |
| Current independent supplement | Three actual processes on binary `01d7e9aee08492579143a297d74be01816a10da8e40045ecf4436b619ebcc989`: paired JSON, paired text and existing aggregate output all match earlier accepted bytes |

The original author and peer archives remain unchanged. The current supplements
separately qualify the receiving parent `4eab1798def5c19fbdee267eb2a6ea8a2726dbe7`.
That parent adds maintained career-lab reporting changes and associated tests.
Their complete delta is retained and reviewed; those reporting routines are not
called by the paired consumer or the batch/aggregate compatibility workflow.

The original 92-case acceptance belongs to binary
`ab563811709ef2788bd0bc563c14da6c842c7d1bc664218ba7c6a2e2f10b61ab`.
The three-process supplement belongs to the current binary above. Repeated
compatibility cases are not presented as new original coverage.

## Packet layout and verification

- `author-packet.tar.gz`, its receipt and `author-README.md` contain the complete
  original source, raw qualification and first failed candidate evidence.
- `independent-packet.tar.gz`, its receipt and `independent-README.md` contain
  the independent receiver frozen before implementation inspection, all raw
  inputs/outputs, custody checks and source review.
- `author-current-packet.tar.gz` contains the exact incoming source delta,
  current closure, rebuilt-binary identity and all three new author gates.
- `independent-current-packet.tar.gz` contains the unchanged independent
  receiver, three new process records, raw outputs/custody and the independent
  284-path source audit.
- Readable receiving receipts, source reviews and transfer verification receipts
  are adjacent to the archives. `publication.json` pins the base, source,
  archives and complete receiving-tree preservation guard.

Each archive contains a portable Python verifier. Extract each archive to its
own directory and run `python3 -S verify-packet.py` for the original author,
`python3 -S verify-supplement.py` for the current author, or
`python3 -S verify_packet.py` for either independent packet. Preserve empty
fixture directories when extracting the original independent packet.
Verifiers check retained artifacts and semantics; they do not rerun native
producer processes. Executables remain at the hash-pinned native paths and
are excluded from the archives.

These are descriptive comparisons of recorded outcomes. Matching recorded
identities does not authenticate experiment settings, establish shared
counterfactual randomness or imply statistical significance. The existing
hosted PR checks and final integration are recorded separately from native
acceptance.

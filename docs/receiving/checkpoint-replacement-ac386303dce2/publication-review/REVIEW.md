# Independent receiving of checkpoint provenance serialization

Reviewer: `/root`, contributor `chatgpt-ac386303dce2`, 2026-10-08.

Accepted as a representation correction to the receiving index. This does not
change the checkpoint implementation, tests, or qualification attribution.

The failed candidate is `bcdc8b658607d9bfc2227ac3f85614ed54850d7c`, PR84.
Scanner run `37773304184`, job `113297917231`, reports the index's lines 4–6
under `generic-api-key`. Its output explicitly uses `--redact`; the log does
not expose the matched values. I retrieved the published original index
independently (Git blob `3bb9b2ca851aaa4f0455bee64c06d0f99374363c`) and read
the corresponding source and executable locally.

The three reported values are independently reproducible provenance digests:

| Artifact | Algorithm | Independently recomputed value |
| --- | --- | --- |
| `uma-sim-core/src/api.rs` | Git blob SHA-1 | `eab9ccf7082ffc96c446f434bd76f313aff14bc9` |
| `uma-sim-core/src/api.rs` | SHA-256 | `31e51bf33c6cd5f51c7f7cb79a4560fad5c2e06a77f0345ff659cf114fabdd55` |
| Native receiving executable, 14,130,248 bytes | SHA-256 | `b7f5a0bc40962fdfb4644a053315e8eb17cfef580b843a051739effdce9dfdb6` |

These values identify source and an authored test executable; they are not
authentication credentials. I also recomputed the native library digest as
`937f9663b85a5809160b92071f7065760872756346d3f4919054586855c29dd3`.

An exact JSON comparison verifies that all five original digests map one to one
to explicit artifact path, algorithm, and value records. No digest is added,
lost, or replaced. The remaining five fields—native base, attribution, results,
build configuration, and every native artifact entry—are deeply equal.

- Original index SHA-256: `bcddfd346bfda11ac32567b3d4615097bb2ffd00a0148b9948bf1a4ec169956f`.
- Replacement index SHA-256: `1e4d7651d4756de1844acf730f1ea9799357f40845ffc706b0320bf2f3526468`.

Preserve PR84, its original source object, independent review, and failed scanner
evidence. A normal fresh-base successor may publish this corrected index while
keeping the scanner, workflow, rules, and source scope intact. The successor
must pass its actual hosted checks; this review neither bypasses that gate nor
relabels the failed run as successful. Historical native qualification remains
bound to its original source and executable, regardless of later publication.

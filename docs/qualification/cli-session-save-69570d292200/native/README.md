# Native current-source CLI persistence receiving

Source candidate: acbe58aeface82d7e3cac1134c6d7126a910bb15. Baseline: 8d74413d7dbc3caf1745779c92d56b6eb7feea42. The native stage verifies every one of 300 selected Rust/content/research source leaves against Git before applying the two candidate source files. `current-source.json` and `candidate-source.json` retain that provenance and exact hashes; the latter includes the newly added test pin.

The installed Rust/Cargo 1.93.1 toolchain was used in a private ThinkPad tmpfs workspace. Missing locked dependencies were retrieved normally into this task's private Cargo cache, then all tests ran `--offline --locked`. No shared source, cache, installed service or live career was changed.

The corrected actual-process integration suite on unchanged baseline source produced **4 passes and 6 expected failures**: healthy commands, existing permissions, and both symlink cases passed; both obstructed-parent commands and all four size-limited saves incorrectly returned success. The retained current-source baseline executable is pinned in its receipt.

The first authored test run had an additional test-assumption failure: raw JSON byte comparison across separately started careers observes arbitrary HashMap key order. The healthy comparison now checks parsed complete snapshots. Exact-byte comparisons remain mandatory for failed-save preservation. The original 3-pass/7-failure run and receipt are archived as `baseline-initial-rust.*`; their receipt records the original log path before archival. This extra failure is not counted as a product regression.

Native candidate compilation and independent receiver replay are pending at this checkpoint. Reviewer controls were authored before candidate source review and are retained separately; no native candidate pass is claimed here. Directory durability on power loss and serialization of concurrent CLI commands are outside this change's guarantee.

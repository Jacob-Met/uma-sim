# Native batch output failure receiving

The native `uma-sim batch` output-open and output-write branches now return exit status 1. The production change is exactly two existing `return` statements replaced with `std::process::exit(1)`. Successful output, diagnostics, schema, seed order and direct partial-file handling retain their existing behavior.

## Evidence and review

The same final Rust integration test ran against the original and corrected native CLI: baseline **1 pass / 2 intended failures**, candidate **3 passes / 0 failures**. The real existing-directory error and child-only Linux file-size limit each returned 0 originally and 1 after correction. Both versions retain the same 30-byte directory sentinel, 311-byte partial file and 884-byte successful three-career output. Formatting passed.

`receiving.tar.gz` contains the original and final sources, unchanged final regression, native build/process receipts and retained output files, exact source intake, commands, first failed attempts and reproducible packet verification. All 74 archived members were checked. Its SHA-256 is **861eca29bf4bbc2f0adecc45aae0d2d81facc74a122a79d3aed49ac7d7659e98**.

The native author compiled the exact CLI using three pinned existing Rust library inputs. The borrowed source matches 76 authored-main source/manifests; a separately owned `career_lab.rs` differs outside this batch execution path. These scoped native results do not claim a fresh full-current-main Cargo build. The original qualification baseline is `0bc58cb83e89f07c6af3056dd7c37436906d994f`; current receiving source retains the same CLI baseline blob `2df06c16c8cb4bffe711b37e1afb3a8a57ebd528`.

`engine-review.json` records a separate source, runtime-receipt and retained-byte review. `root-review.json` records root's source/contract acceptance. Neither reviewer claims additional native executions. The original packet describes its frozen pre-publication state; subsequent reviews and integration do not rewrite that historical evidence.

## Replay

Run the repository-native regression with:

```sh
cargo test -p uma-sim-core --test batch_output_failures
```

The output-open and successful-output methods run on all supported platforms. The real midstream file-limit method is Linux x86-64 only. Packet replay details and the explicit original dependency boundary are in the archive's README and receipts.

## Integration boundary

Normal current-head PR checks, the repository's literal native Git ancestry gate, current ownership and guarded merge are required before integration. Source acceptance is not an installed-runtime or deployment claim. Output creation remains direct and may leave an incomplete final JSONL line after failure; no atomic replacement, rollback or durability guarantee is introduced.

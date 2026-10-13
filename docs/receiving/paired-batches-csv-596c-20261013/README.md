# Complete paired CSV export: native receiving

Source: [`392499683b3b52cf032222d4ed9ea6274fb10131`](https://github.com/Jacob-Met/uma-sim/commit/392499683b3b52cf032222d4ed9ea6274fb10131), branch `feat/paired-csv-596cb2f61c57`, parent `18d6cf1b9f1e6f7bfd947e9c04cac467bca5988a`. The CSV increment retains the previous analyzer arithmetic correction. This receiving increment adds evidence and instructions without changing product source.

## Use the delivered executable

The Windows executable, authored CSV, corresponding source projection and full native evidence are delivered together. [Download the Windows package](https://drive.google.com/file/d/15HchAEkuAvXLzsPUMNMLv4bfFNmdT3hI/view). `delivery-receipt.json` records its sealed ZIP identity. Extract into a new directory and use Command Prompt:

```bat
uma-sim.exe compare-batches --input=fixtures\input.jsonl --baseline=fixtures\baseline.jsonl --format=csv > my-comparison.csv
```

The package README also shows how to invoke CMD from PowerShell. The independently received file-write route was CMD redirection. The included table has fourteen authored pairs. CSV uses UTF-8 without BOM, CRLF records, exact decimal integers and always-quoted literal string fields. Embedded quotes are doubled; embedded newlines remain within values. Parse records with a CSV reader and import seed and literal string columns as text in spreadsheets. The [maintained comparison guide](../../PAIRED_BATCHES.md) specifies all fields, admission limits and interpretation. Shell redirection opens the destination before the program starts, so choose a new output name when preserving an existing report.

## Distinct qualification results

These are different observations, not one combined test count.

| Course | Recorded result |
| --- | --- |
| Unchanged original Windows executable | 23 command invocations, 73 predicates; CSV refused as expected |
| Actual maintained Windows Cargo tests | 14 methods: 7 pairing, 4 Windows command, 3 new CSV |
| Copied candidate Windows executable | 31 command invocations, 176 predicates |
| Native build/receiving wrapper | 12 checks, including both help routes and copied executable identity |
| Separate portable-use controls | 2 invocations, 10 predicates: CMD redirection and retained analyzer arithmetic |
| Author's isolated Linux adapter check | 4 invocations, 7 predicates; maintained module/private adapter with a small main wrapper, not a full Cargo build |

The original-first receiver was frozen before candidate inspection. Fourteen pairs in two input orders cover signed 64-bit seed endpoints, signed 32-bit score endpoints, commas, quotes, CR/LF, tabs, Unicode, whitespace and formula-looking literal strings. Eight refusal classes cover duplicate identities in either file, late invalid JSON, unmatched cohorts in either direction, empty input, seed overflow and score overflow.

Independent review recomputed all 171 payload hashes and 126 raw-stream bindings. Twenty-one original command results retain byte-identical stdout, stderr and exit status. Both CSV outputs equal the complete frozen 814-byte expectation; parsing recovers every literal field and exact integer. All eight CSV refusal cases emit no header or other stdout and retain the corresponding original diagnostic. No output normalization is used.

Source, executable, input and session snapshots remain unchanged in bytes and modification times, including direct cross-course equality. All 228 candidate source leaves independently match the published Git tree by blob, byte count and mode. The shell fixture retains 100755 metadata; Windows does not qualify Unix execution behavior. The Unix descriptor path was checked separately by the limited Linux adapter harness.

The separate CMD invocation wrote the exact received CSV, SHA256 `1bb11f90b36721b9ddf3c701efe4d289073f9cf5c27e476c1985a408c632fc6a`. The retained `analyze` command reports one record with `summary.shop_gain_mean = 4294967295.0` on the positive-endpoint fixture. This is one retained-capability control, not a repetition of the earlier analyzer course.

## Source and build boundaries

Native receiving used independently identified Windows 11 x64 device `9ea77fe9-bb87-46e0-a5a8-8b3e25f6ff7e`, MSI/Veria, Python 3.11.9 and Rust/Cargo 1.99.0 (`x86_64-pc-windows-gnu`). Source, targets and temporary output were isolated on D:. The existing bundled linker and crate cache were used; Cargo ran `--locked --offline` with default features, without downloads, toolchain installation or source edits during receiving.

```text
cargo test --locked --offline -p uma-sim-core --test paired_batches --test paired_batches_cli --test paired_batches_csv
cargo build --locked --offline --release -p uma-sim-core --bin uma-sim
```

The copied release is 8,274,114 bytes, SHA256 `9a57e48af9c23bb42a25593c74d6508ac428e75948bb18f0eeb875767cd86255`. The source projection includes both complete crate directories, unchanged root dependency pins, applicable instructions, licenses/notices and relevant guides. It is a focused 228-file workspace projection, not the entire repository or simulator/content assets.

An earlier original-control attempt disconnected on a different MSI device/user (`edc358ed…`, jacob); that attempt's result remains unknown. The qualified baseline here was restored from the exact previous portable artifact and source packet on the freshly identified Veria/D: device. A common hostname did not establish continuity.

## Replay retained evidence without native execution

The delivered package includes `evidence/native-receipts.json.gz`: 291,888 bytes, SHA256 `c99bb18ef451251ac22e9744c126a2590a7e65dc6665f0f296f4e6a643c7cdb2`. Its compressed JSON `files` mapping retains all 171 payloads with byte count, SHA256 and base64 bytes: full streams, receipts, fixtures, source manifests and exact control scripts.

From this directory, using standard Python:

```sh
python -B review_native_csv.py /path/to/package/evidence/native-receipts.json.gz \
  --source-bindings independent-source-bindings.json --output new-review.json
```

The verifier checks fixed packet/source-binding hashes, full raw-stream associations, complete CSV output, existing output parity, refusal behavior, source custody and declared counts. It writes only a new review file and runs no executable. `native-independent-review.json` records the successful replay. Independent source/publication review and the original Linux adapter receipt are retained separately alongside it. The frozen `receive_paired_csv.py` is also retained for a new explicitly configured native original/candidate course. Its original source/executable setup is recorded in the raw packet; the offline command above rechecks this delivered course without creating another one.

`linux-adapter-check/` preserves the exact historical wrapper, runner, build streams and receipt. Its runner names the author's Rust 1.93.1 compiler/dependency locations and expected `../candidate/` projection; it is not a standalone portable build script. Use the maintained Cargo commands above in a compatible full source projection for new qualification.

Exactly five product/test/doc files changed and one test was added; 1,735 previous leaves remained unchanged. The seven inherited workflow blobs are unchanged. Source publication produced zero observed Actions runs. No PR is opened because `pull_request_target` would run; source and receiving use separate skip-marked branches, with no merge or installation implied.

These results qualify the saved-batch workflow and explicit controls above. They do not claim a full workspace-suite pass, simulation/server/UI qualification, production data, another machine's behavior or installed adoption. All example records are authored controls.


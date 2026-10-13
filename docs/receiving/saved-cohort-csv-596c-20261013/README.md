# Delivered companion

[Download the saved-cohort CSV companion](https://drive.google.com/file/d/1nDJON-K6vRPC3kdk4dt2YEUOJ0qePp7K/view): 79,019 bytes, SHA256 `5975300c3265493f66d0647fccc58b2d97ce2239fd63eef5b0d9b10f9f778e13`.

The data, complete raw evidence and packet inventory are in that archive. The opening and verification commands below run from its extracted directory. This Git directory preserves the reviewed receiving code and compact delivery evidence.

# Saved UMA cohort: paired CSV companion

The current CSV tool has exported all **16 pairs** from the existing accepted policy study. Open `comparison.csv` to inspect every seed, both final scores and the signed **bot minus default** difference. `comparison.json` is byte-identical to the historical comparison. This companion makes saved results easier to consume; no new careers were run.

See [REPORT.md](REPORT.md) for the complete score table. The records describe `ura`, `Special Week`, seeds 0–15. Their mean score difference is **2500.6875**, median **2570**, and range **1066–3204**. The JSON field `wins` counts positive score differences, not races won.

## Open and export

Import `comparison.csv` as UTF-8 CSV in a spreadsheet. It has no BOM, an unquoted header, quoted string fields and CRLF record endings. Treat scenario, trainee and grades as text. Seed is an identifier; import it as text for other cohorts with large integers. The columns are:

`scenario,trainee,seed,baseline_score,input_score,score_delta,baseline_grade,input_grade`

The original records are in `data/default-cohort.jsonl` and `data/bot-cohort.jsonl`. This companion contains no executable or runtime assets. Use the already delivered [Windows CSV analyzer](https://drive.google.com/file/d/15HchAEkuAvXLzsPUMNMLv4bfFNmdT3hI/view) for repeat exports. That ZIP is 3,512,008 bytes, SHA256 `58558c930502cdabedefecdbda027118500dd24349c6ddd15c36e3e3491c8d1c`. Its executable is 8,274,114 bytes, SHA256 `9a57e48af9c23bb42a25593c74d6508ac428e75948bb18f0eeb875767cd86255`, from [source 392499683b3b52cf032222d4ed9ea6274fb10131](https://github.com/Jacob-Met/uma-sim/commit/392499683b3b52cf032222d4ed9ea6274fb10131).

From Windows **Command Prompt**, change into the extracted companion. Replace the example executable location with its actual path. Choose unused output names: shell redirection replaces existing files.

```bat
"D:\Tools\uma-sim-csv\uma-sim.exe" compare-batches --input=data/bot-cohort.jsonl --baseline=data/default-cohort.jsonl --format=csv > comparison-received.csv
"D:\Tools\uma-sim-csv\uma-sim.exe" compare-batches --input=data/bot-cohort.jsonl --baseline=data/default-cohort.jsonl --format=json > comparison-received.json
```

These comparisons need no simulator assets, external policy, installation or new careers. Native receiving used exactly two comparator invocations and preserved the staged inputs and both executable copies. The example path is a placeholder; exact received paths and streams are in the evidence.

## Verify without running UMA

With standard Python 3, run from the extracted companion:

```bat
python -B VERIFY.py
```

The verifier checks every declared payload, fixed native evidence and historical Git blobs. It reconstructs the exact CSV and every JSON row/statistic from all 32 terminal records. It writes no files and starts no UMA process. User-added files are listed separately. `FILES.json` is an inventory, not a signature; the separately published archive checksum identifies the complete download.

For a new standalone review, choose an unused output filename:

```bat
python -B tools/review_simulated_pairs.py --baseline data/default-cohort.jsonl --input data/bot-cohort.jsonl --csv comparison.csv --json comparison.json --scenario ura --trainee "Special Week" --seed 0 --seed 1 --seed 2 --seed 3 --seed 4 --seed 5 --seed 6 --seed 7 --seed 8 --seed 9 --seed 10 --seed 11 --seed 12 --seed 13 --seed 14 --seed 15 --output fresh-output-review.json
```

This creates only the new review file and checks input bytes and modification times before and after reading. Frozen reviewer SHA256: `d92817b14a5b74f618d5ef19d0869487909645f262fef4c72f7612289e4d54e6`.

## Provenance and interpretation

The earlier native Mac producer used `4eab1798def5c19fbdee267eb2a6ea8a2726dbe7` plus its qualified paired-comparison overlay, executable SHA256 `01d7e9aee08492579143a297d74be01816a10da8e40045ecf4436b619ebcc989`. The current Windows comparator reads those saved records; it did not regenerate them.

The original study used empty deck/legacy, the physics model and no external policy. Equal initial seeds do not establish equal subsequent random-number consumption. Its historical URA finale, spark-order and mood-direction limitations remain. These descriptive scores do not establish race wins, actual-game fidelity or general policy superiority.

Read the complete [original report](https://github.com/Jacob-Met/uma-sim/blob/eb4f7de067824ed24818d93d5fdca87d3818a2da/docs/research/paired-policy-cohort-713adaab/REPORT.md), [independent historical review](https://github.com/Jacob-Met/uma-sim/blob/eb4f7de067824ed24818d93d5fdca87d3818a2da/docs/research/paired-policy-cohort-713adaab/independent-root-review/README.md) and [issue #132](https://github.com/Jacob-Met/uma-sim/issues/132). Eight unchanged historical files are also inside the native evidence; their old relative links refer to the complete original study and may not resolve inside this subset.

`evidence/native-receipts.json.gz` contains all 24 receiving payloads: 59,954 bytes, SHA256 `276dc8bba83cbb7aa7d50aa37b5469a42d512162fd72753774ac61a39a95544a`. The additional independent review binds all eight historical files to their exact remote Git tree, recomputes the complete outputs, reconciles all command streams and confirms historical JSON byte equality. The analyzer's [source and receiving guide](https://github.com/Jacob-Met/uma-sim/blob/87a551fefb56ee2229253ac3fe158c95619ac395/docs/receiving/paired-batches-csv-596c-20261013/README.md) covers its wider qualification.

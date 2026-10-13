# Existing 16-pair study exported through the CSV analyzer

## Result

The delivered Windows analyzer consumed all saved `ura` / `Special Week` pairs for seeds 0–15 from the accepted historical study. The new CSV contains every pair in numeric seed order and is **850 bytes**. The new JSON is **4,069 bytes**, exactly equal to the historical comparison output.

Each difference below is **bot score minus default score**, independently derived from the two terminal records and reconciled with both outputs.

| Seed | Default score | Bot score | Difference |
|---:|---:|---:|---:|
| 0 | 790 | 3311 | 2521 |
| 1 | 854 | 3746 | 2892 |
| 2 | 947 | 3566 | 2619 |
| 3 | 819 | 3172 | 2353 |
| 4 | 653 | 3526 | 2873 |
| 5 | 1154 | 3596 | 2442 |
| 6 | 1064 | 4268 | 3204 |
| 7 | 1099 | 3260 | 2161 |
| 8 | 900 | 3526 | 2626 |
| 9 | 1051 | 3155 | 2104 |
| 10 | 1864 | 2930 | 1066 |
| 11 | 838 | 3588 | 2750 |
| 12 | 737 | 3620 | 2883 |
| 13 | 693 | 3750 | 3057 |
| 14 | 834 | 2938 | 2104 |
| 15 | 822 | 3178 | 2356 |

Differences sum to **40,011**. Mean **2500.6875**; median **2570**; minimum **1066**; maximum **3204**; population standard deviation **491.74024122879143**. All 16 differences are positive. The JSON fields `wins`, `ties` and `losses` compare final scores, not races.

## Receiving and independent review

On 2026-10-13, the existing portable analyzer at source `392499683b3b52cf032222d4ed9ea6274fb10131` ran exactly two comparison commands on the verified MSI / Veria Windows consumer. Both exited successfully with empty stderr. This receiving course ran **zero simulator calls** and needed no build, dependency installation or session creation.

Independent review verified all 24 payloads of the raw evidence and bound all eight historical input/context files to Git tree `8025414e21f6b793678eec9cf7448e5bdd21f9ec`, study commit `eb4f7de067824ed24818d93d5fdca87d3818a2da`. It checked both complete command argument lists and every retained stdout/stderr hash, reproduced the native offline result, and confirmed unchanged input/executable byte/hash/modification-time snapshots.

The reusable reviewer independently reconstructs CSV quoting with Python's CSV library, parses the result back, and checks every JSON row and statistic. Only independently evaluated standard deviation allows a floating-point tolerance of 1e-12.

| Artifact | SHA256 |
|---|---|
| `comparison.csv` | `f6bf094b84bda463e79357e4757529597ea001e74f7ca49ecb4f4deb0ce53131` |
| `comparison.json` | `7b15171bbdec97c6bc52671b106deba787dfec3f5ecac50b79bf30f5faa4f473` |
| Complete native evidence gzip | `276dc8bba83cbb7aa7d50aa37b5469a42d512162fd72753774ac61a39a95544a` |

## Interpretation stays with the original study

The records came from an earlier native Mac producer based on `4eab1798def5c19fbdee267eb2a6ea8a2726dbe7` plus its qualified paired-comparison overlay. The newer export tool does not change the simulator version that generated them. The fixed cohort used empty deck/legacy, the physics model and no external policy. Same initial seeds do not imply identical random-number consumption across policies.

The original study's model restrictions, URA finale boundary, spark ordering and the unincorporated mood-direction source finding remain relevant. This export does not establish general policy superiority, race wins, latest-mechanics acceptance or fidelity to the actual game. It supplies a complete, verifiable spreadsheet view of the accepted saved data.

The authoritative [historical report](https://github.com/Jacob-Met/uma-sim/blob/eb4f7de067824ed24818d93d5fdca87d3818a2da/docs/research/paired-policy-cohort-713adaab/REPORT.md) and [independent historical review](https://github.com/Jacob-Met/uma-sim/blob/eb4f7de067824ed24818d93d5fdca87d3818a2da/docs/research/paired-policy-cohort-713adaab/independent-root-review/README.md) retain the full experiment and source history. See [README.md](README.md) for opening, verification and repeat-export commands.

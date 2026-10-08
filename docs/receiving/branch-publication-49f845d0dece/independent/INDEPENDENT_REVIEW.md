# Independent branch-publication receiving

## Verdict

Accept candidate `career_lab.rs` SHA-256 `f12208e5796ac6ab42ba515d6d8e6734388810894b600420e4badfdeab250f08` for the issue76 publication contract.

Exact base: main `0b5cc2342cd93141beb461edf6e83998388e67b8`; original domain SHA-256 `95cb00bd9418e889372b83eb998f8100887218d91b288af9180deff0843427c7`.

The source review found only the intended imports, reusable conflict wording, identity generator and branch-local publication changes. The shared checkpoint `atomic_write` helper is unchanged. The main API invokes `BranchStore::save` only when creating a newly run branch; its existing error mapping turns `AlreadyExists` into HTTP409. There is no branch-result update endpoint. Named checkpoint overwrite remains the separate `CareerLibrary::save(..., overwrite=true)` contract.

The candidate closes the fully written/synced temporary file before creating the final hard link. The final directory entry atomically rejects an existing destination. ID entropy reduces collisions; publication independently prevents replacement if one still occurs. Unsupported hard links return a storage error without replacing an existing result. This review does not establish directory-sync or power-loss durability.

## Independent native results

Native Mac Rust1.99.0 linked the same receiver source against the preserved baseline core cache and then the rebuilt candidate cache. Compiled outputs stayed in this receiving directory, outside the shared Cargo target.

| Case | Baseline | Candidate |
|---|---|---|
| Different payload under an acknowledged ID | Fails: second save returns success | Passes: conflict and unchanged original bytes |
| Identical payload submitted again | Fails: second save returns success | Passes: creation conflict |
| Four independent processes publish one forced ID | Fails: four successful acknowledgments | Passes: one success, three conflicts |
| Existing dangling destination | Fails: link replaced | Passes: existing link preserved |
| Distinct IDs reopen, with sibling and external sentinel files unchanged | Passes | Passes |

The process case verifies exact complete winner bytes, absence of stray final/temp files, and readback by another fresh process. The checkpoint/session control here uses authored opaque sentinel files; the product API receiver separately exercises actual checkpoint/session data.

Original paired receiver SHA-256: `e925eb000d256ce4545b4054d5740a1c46f1dae3a46cca85dc8a33f20777a7e2`.
Baseline: **1 passed / 4 failed**, exit101.
Candidate: **5 passed**, exit0.
The one ignored test is the subprocess entry point, invoked explicitly by the process case.

Baseline binary SHA-256: `1b26cc2fc5ed3cd2cea13cc98f4c85a443289bba879bc2c195de6eb524cd5007`.
Candidate binary SHA-256: `04d229942623be528bf9581602b50afda94c60fdacbd509082f03ace7d7ad734`.

## Final formatting gate

The repository formatting check required rustfmt on the new test. The original paired source and raw outcomes remain preserved; `formatting-custody.json` and `rustfmt-only.patch` record the transformation.

Final tracked test SHA-256: `7aae317e1e99944aa4c7709168ee8068c227865dc66c538f61f3d20b15140b21`.

`cargo fmt --check` passed. The final tracked test then passed through Cargo:

```sh
cargo test -p uma-sim-core --test career_lab_branch_publication -- --test-threads=1 --nocapture
```

Result: **5 passed / 0 failed / 1 subprocess entry ignored**, exit0. The candidate domain hash remained unchanged. The baseline rlib was not retained as another large artifact, so the baseline was not rebuilt after formatting; its exact original paired test source, linked binary and compile/result receipts retain that boundary.

## Compact source evidence

Retain these files with the production contribution:

- `baseline-compile.json` and `baseline-results.json`
- `candidate-compile.json` and `candidate-results.json`
- `career_lab_branch_publication.rs` — original paired receiver
- `formatting-custody.json` and `rustfmt-only.patch`
- `final-cargo-results.json`
- this independent review

The final formatted test is the tracked integration-test file. The JSON result records contain the actual stdout/stderr, native process identities and source/binary pins. No shared runtime state was used.

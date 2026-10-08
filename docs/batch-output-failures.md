# Batch output failures

`uma-sim batch` returns exit status **1** when it cannot open its JSONL output
or a write to that output fails. The existing error message remains on stderr;
the command does not print its successful `Batch wrote ...` summary for either
failure. A successful batch still returns 0 and writes the same career records.

This lets an ordinary shell pipeline stop before analyzing an incomplete batch:

```sh
uma-sim batch --seeds=42,43,44 --output=out/careers.jsonl &&
  uma-sim analyze --input=out/careers.jsonl
```

## What remains on disk

Output handling is unchanged: the command creates or truncates its requested
file directly. A failure after some writes can leave complete JSONL records and
an incomplete final line. The nonzero result signals that the requested export
did not finish. Inspect or retain that partial file before choosing another
output destination or starting a new batch. The command does not delete it,
retry the batch, or reconstruct missing records.

This is failure reporting at the existing open/write boundary. It introduces no
temporary-file replacement, filesystem transaction, durability guarantee, record
schema change, policy change, or alteration to `analyze`.

## Native regression checks

```sh
cargo test -p uma-sim-core --test batch_output_failures
```

The tests launch the actual native CLI. An existing output directory exercises
a real open error and retains its sentinel contents. A normal three-seed batch
checks the JSONL record count, requested seed order, and successful status.
On Linux x86-64, a file-size limit applied only to the child process allows one
complete record and part of the next before a real write returns `EFBIG`.
`SIGXFSZ` is ignored in that child so the test checks the CLI's own exit status,
not a signal termination. The partial file must match the successful reference
prefix exactly. Other platforms run the open-error and successful-output cases.

For a retained receiving packet, set `UMA_BATCH_RECEIVING_ROOT` to an isolated
directory before running the tests. Individual process stdout, stderr, status,
and output files remain there. `UMA_BATCH_DATA_ROOT` may select an existing
read-only simulator data tree; normal Cargo tests use the repository root.

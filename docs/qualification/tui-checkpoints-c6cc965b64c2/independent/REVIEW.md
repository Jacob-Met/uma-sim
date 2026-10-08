# Independent durable terminal receiving

## Decision

Accepted on the frozen source `3077775807c1eb5c2af6937038bd26cf5f00c75b` with the unmodified native API from parent `b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c`. The independent run passes **16 groups**, exercising **31 terminal processes**, **two native API processes with an actual restart**, and **305 HTTP requests**. No product source or authored test helper was edited or imported. No new API build was needed.

The terminal source SHA256 is `653acbb0b858d7bdc420904e26452e7739dffcc9c2aa73168b8fa801a1d63b4e`; the API executable SHA256 is `6f94216933f76a08caeade29087f50e0e49da46dabdea8d98d89a200d4835754`. The independent driver SHA256 is `b5c79ba9e8aa53ded1c6cadf650ef2e99908e277fe92e49438ade74a9504ef50`. [source-manifest.json](source-manifest.json) binds the TUI, package metadata, README, API, career-library source, and original baseline TUI. All five owner/received source files, the baseline, and the executable match before and after execution.

## Native behavior proved

The unmodified baseline sends `save baseline-sentinel` to the ordinary action endpoint, advances its career, and creates no checkpoint. The exact before/after snapshots are retained. The candidate instead sends one explicit `/v1/library/save` with its selected nonempty session and `overwrite: false`.

A 64-character checkpoint name beginning with `--` and a 64-character session ID both work. The saved career uses seed `-9223372036854775807`, outside JavaScript's exact integer range. The receiver compares complete JSON structures while preserving every original numeric token; it does not round the seed through JavaScript numbers or omit mutable fields. Snapshot/metadata files are also compared by SHA256. The exact seed and snapshot survive an actual stop/restart of the Rust API with the same private working directory. The old live session correctly returns native HTTP 404 after restart.

Two terminal processes fork the same checkpoint concurrently into distinct UUID sessions. One takes a real bot step while the other remains unchanged. The advancing branch matches an independently forked native oracle. Both then compete to save different career states under one new name: exactly one succeeds and the other receives native HTTP 409, and the stored snapshot exactly matches the winner. Original main, bystander, source career, and checkpoint data remain unchanged by those forks and saves.

After restart, the reopened checkpoint takes two real bot steps matching an independent native continuation at turn 9/RNG 44 and turn 10/RNG 48. A new default terminal career also preserves the existing main and named careers. Interleaved explicit changes to the API's active selection do not redirect terminal actions or saves.

Missing, malformed-metadata, malformed-snapshot, and incompatible-schema checkpoints produce actual native HTTP 404, 500, 400, and 422 responses. No failed resume starts a fallback career or replaces a session. The corrupted test inputs remain intact. A deliberately blocked metadata temporary file produces a real save HTTP 500 after the snapshot write; the native library removes the partial snapshot and retains the prior library and live state.

A transparent receiver deliberately drops or holds responses only after the real native API has returned success. A lost save response leaves exactly one saved checkpoint and prints the checkpoint name plus selected session for recovery. EOF during a held save exits 0 with uncertainty disclosed; the committed checkpoint survives. SIGINT during a held fork exits 130, prints the proposed session ID, and retains exactly one recoverable fork. Explicitly resuming that ID succeeds. A lost action response applies exactly one real bot step to the selected career, matching a separate native oracle. No case automatically replays a mutation.

Five invalid interactive save names and nine invalid mode combinations are rejected locally; the mode combinations make zero HTTP requests. The full route audit covers 156 terminal HTTP requests and 123 explicitly addressed run/save requests. Every save sends `overwrite: false`; the terminal never calls load, delete, activate, or close. The source review confirms that the same closed-over session value feeds both reads and actions. [exact-address-analysis.json](exact-address-analysis.json) additionally checks every recorded run/save selector against the expected session for its process group.

## Evidence and limits

[receive-v1.mjs](receive-v1.mjs), [run-v1/receipt.json](run-v1/receipt.json), and [run-v1.log](run-v1.log) preserve the exact commands, arguments, PIDs, process results, native responses, request counts, and assertions. All actual checkpoint files and selected full state snapshots remain under the private native run directory. [evidence-manifest.json](evidence-manifest.json) inventories their exact bytes.

This is an independent macOS/Node 26.3.0 receiving run with the supplied native Rust API, not a rerun of the author's harness. The fault receiver preserves the real upstream status before intentionally dropping/holding a response; those transport faults are not presented as native endpoint failures. The concurrent save experiment uses two terminal clients against one API instance; it makes no claim about concurrent API instances sharing a filesystem. Process shutdown is exercised; power-loss durability is not claimed. No deployment or service on another worker's directory was changed.

Preparation-only failures are retained separately: an initial missing receiving directory caused ENOENT before source staging, and an outer V8 template interpolation mistake prevented the first driver-generation expression from executing. Both were corrected before this single native receiving run. They are not product failures or successful test cases.

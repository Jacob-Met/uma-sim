# CLI session persistence — original native witness

Contribution: [issue #113](https://github.com/Jacob-Met/uma-sim/issues/113), owner estate-69570d292200/production_lane.

At public source `8d74413d7dbc3caf1745779c92d56b6eb7feea42`, `RunSession::save` overwrites the current session directly, and four CLI callers continue to successful output/exit status after save errors. Both the session module and the complete four affected caller functions are byte-identical to public baseline `6245d48132df3f07aaa314f5faa9cb31536449b1`.

The retained native baseline executable was located through the existing batch owner's recorded receiving evidence, read and hashed, and executed without modifying its owner workspace. Its actual SHA-256 was `eb1fc0410dc86362af577d302ff06a27091ebc233999590ffe816a7145923551`, unchanged after this run. This is a source-bound retained-binary witness; it is not a build of the entire later current main, and candidate current-source compilation remains a separate gate.

The attached driver made five actual CLI observations in a new exclusive tmpfs fixture. Healthy start exited 0 and wrote valid JSON. Obstructed start/fast reported save errors but also exited 0 with normal result output. In two separate child processes, an inherited file-size limit caused step and deck placement to truncate a valid 2,954-byte prior session to invalid 1,024-byte JSON; both commands printed a save error and nevertheless exited 0. Deck placement also printed its ordinary success line. The size limit applied only to each child, never to the receiving process, host, or other workers.

`baseline-v1/receipt.json` is the native file read back after the run. Its healthy/refusal fields describe observed behavior rather than expected acceptance. The byte lengths, digests, stderr, status, and post-write JSON validity preserve the original failure. Actual fixture files and stdout/stderr remain under the explicitly named native receiving directory. No installed service, other worker's build, real career, game account, or live API was changed.

Source publication contains no product correction yet. Implementation, independent review, current-main composition and exact-head CI remain separate gates. Session replacement will preserve the previous successful snapshot on handled preparation/publication failure and report failures before normal command-success output; it will not add command serialization or a power-loss durability guarantee.

## Candidate receiving in progress

The correction writes a complete, synced sibling before atomically replacing the saved career. Handled preparation, write, sync and rename errors preserve the previous snapshot and clean the attempt's temporary file. Cleanup failures identify the remaining temporary file. Existing regular-file permissions and final symlink destinations are preserved; new Unix session files use mode 0600. This does not serialize concurrent commands or promise directory durability on power loss.

All four CLI save callers (`start`, `step`, `fast`, `deck place`) now exit 1 before normal result output on a save error. The session encoding and location, engine, RNG and other command behavior are unchanged. The new `cli_session_persistence` integration suite exercises the actual CLI in isolated directories, including child-only file-size limits.

At source publication, current-source native baseline and candidate receiving are still in progress; no candidate pass is claimed in this checkpoint. The Rust source staged from `8d74413d7dbc3caf1745779c92d56b6eb7feea42` was verified against all 300 selected Git blobs. Receiving main `0ad4bd4d2ca0a133fe91080b6b7818efbaedb2fa` changes only browser importer/docs paths from that baseline, with no Rust source or dependency changes. Its tree is retained in the source composition. Independent reviewer qualification and exact-head hosted CI remain required before integration.

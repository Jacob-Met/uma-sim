# Independent native report qualification

The production worker authored `uma-sim-core/tests/career_lab_reports.rs`
against main `f5f9b29393731d18aee2d66a31d89c315aa79c60`, then independently
reviewed and executed root's frozen renderer. No production source was edited
by this reviewer. The exact source, test, toolchain, and raw-log hashes are in
`receipt.json`.

| Native run | Passed | Failed |
| --- | ---: | ---: |
| Original eight-test baseline | 4 | 4 |
| Final frozen nine-test baseline | 4 | 5 |
| Same nine tests on the candidate renderer | 9 | 0 |
| Existing career-lab suite before source-expectation maintenance | 13 | 1 |
| Existing career-lab suite after root's localized expectation update | 14 | 0 |

The compatibility failure required the raw unescaped hyphenated heading.
Root changed that one expectation to the equivalent escaped Markdown source
and added a comment. The reviewer copied the exact pinned test file and reran
the existing fourteen-test suite; the renderer and new tests stayed unchanged.

The new tests cover all 26 displayed string locations, 65 changed-input code
field cases, A/B table columns, document structure, numeric and timeline
semantics, missing cells, absent divergence, hidden identifiers, and typed/JSON
source immutability. They introduce no dependency.

The original test helper incorrectly applied prose escape/entity handling
inside code spans. Before any candidate execution, it was refined to recognize
matched code delimiters and literal contents; dedicated delimiter and padding
cases were added. The refined test file was committed and the baseline rerun
before the candidate. Both versions' receipts are preserved. These bounded
helpers do not replace the separate real GFM/browser receiver.

The source review found all displayed values routed through the appropriate
prose/table or code serializer. Entity ordering, CRLF handling, delimiter
selection, and significant padding preserve the tested data. Code-context
fields occur outside tables. JSON and simulation behavior remain unchanged.

One attempt to rebuild the refined baseline ran out of shared tmpfs space
during compilation, before tests executed. Its compiler log is retained and is
not a test result. Only the reviewer's target/cache were moved into its own
overlay directories before a successful offline rebuild.

All `.log.gz` files are deterministic gzip copies of the raw logs named in
`receipt.json`; that file's log SHA256 values refer to the uncompressed bytes.

Replay with a provisioned Rust toolchain and the repository lockfile:

```sh
cargo test --locked --offline -p uma-sim-core --test career_lab_reports -- --test-threads=1
cargo test --locked --offline -p uma-sim-core --test career_lab -- --test-threads=1
```

This receipt qualifies the listed source bytes. Root owns integration and its
required head checks; the artifact worker owns actual API, download, GFM, and
Chromium receiving.

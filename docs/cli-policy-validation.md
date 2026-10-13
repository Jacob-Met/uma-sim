# Native CLI policy validation

The native career commands accept exactly `--policy=default`, `--policy=bot` and `--policy=external`. Omit the flag to keep the default policy. Names are case-sensitive and are not trimmed.

An empty or unknown value exits with status **2** during argument parsing. For example:

```console
uma-sim batch --policy=bto
Error: unknown policy 'bto'; expected one of: default, bot, external
```

This refusal occurs before the selected command starts an engine, writes a session, creates or truncates batch output, exports telemetry or executes an external policy. It applies to the shared parser used by `start`, `fast`, `batch` and `export-telemetry`.

Every repeated policy argument must be valid. `--policy=bot --policy=default` keeps the existing last-valid-value behavior. `--policy=bto --policy=default` fails on `bto`; a later valid flag does not erase an invalid request. Other existing argument and policy semantics are unchanged.

## Why this matters

Before this correction, the parser stored any requested string. The policy dispatcher explicitly recognized `bot` and `external`, then used default behavior for every other string. Batch and fast summaries printed the requested string. A typo could therefore run the default policy while labeling the result with an unrecognized policy name.

Validation is at the CLI boundary. The engine dispatcher, external policy implementation, scoring, RNG and dependencies are unchanged. This change does not validate arbitrary policy labels in separate services or the career laboratory's separate save-label parser.

## Maintained checks

`uma-sim-core/tests/cli_policy_validation.rs` exercises the actual Cargo-built CLI in four tests:

- All four career commands refuse a typo while preserving existing output and session bytes and modification times.
- Refusal does not create an output file or missing parent directory.
- Empty, mistyped, case-changed and whitespace-decorated names fail exactly; invalid first and last repeated values both fail.
- Known values, omission and repeated valid values pass parsing and reach an intentionally failing output-open operation before any engine or external policy executes. An unrelated unknown argument retains its existing behavior.

The admission control uses an existing directory as the batch output with a positive one-career count. It does not depend on zero-count behavior, which belongs to the separate batch-admission work.

Run the focused maintained tests from the repository:

```console
cargo test --locked -p uma-sim-core --test cli_policy_validation
```

The separately retained native receiving records bind the original typo/default witness, candidate refusal and preservation checks, and the actual repeated-valid `bot` then `default` result. Those receipts identify the tested source and executable; this guide alone is not a claim that a branch has been installed or merged.

Tracked in [issue #160](https://github.com/Jacob-Met/uma-sim/issues/160).

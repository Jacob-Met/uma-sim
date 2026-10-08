# Full native workspace qualification

This historical qualification belongs to the browser-session contribution claimed in [issue #63](https://github.com/Jacob-Met/uma-sim/issues/63).

- Qualified published source: `fa8711b8baeb48bb2ba2732a7d47084bff86bf7b`.
- Qualified source tree: `ec9d9001e0d0b79456d0847959049b63c63c99c1`.
- Receiving baseline at qualification: `f5f9b29393731d18aee2d66a31d89c315aa79c60`.
- Command: `cargo test --workspace --locked`.
- Result: exit 0, 344 tests passed, zero failed or ignored, 70 test-result summaries including zero-test targets and doc tests.
- Runtime: approximately 70 seconds, completed on 2026-10-08 before 08:18:47 UTC.
- Environment: isolated macOS checkout, Rust/Cargo 1.99.0.
- Process handle recorded by the remote runner: 38483; its completion reported exit 0 and runtime 69.59 seconds.

The isolated native checkout's receiving baseline and both changed Rust files matched the qualified candidate. `uma-sim-core/src/api.rs` had SHA-256 `1bd4ae827fbbfe72d3c59f339bde09c4b2599039e48dfd75d17d49e85c61dcd9`; `uma-sim-core/tests/session_targeting.rs` had SHA-256 `c42b468d8974a104a0f49df7dc94a551b2d19bc8d4b5ed5e18e172d08e78aee3`. Native tests launched and terminated their own API processes; no installed server was used.

`rust-workspace-qualified.log` preserves the complete retained runner output, including warnings and every test-result summary. It has 33,607 UTF-8 bytes and SHA-256 `90bd202b6cd1e82812f27ccec6e6575b4e78c770c681f4e64af8d42d91e1229c`. This preservation step made no textual edits to the retained log. Qualification-source hashes and the earlier targeted positive/negative evidence are alongside this file.

This records qualification of the stated historical source. Main has advanced since that run; the result does not qualify later composed code, establish GitHub CI on a future PR head, or establish deployment or visual gameplay acceptance. The evidence branch changes documentation only and must not replace current-main source during integration.

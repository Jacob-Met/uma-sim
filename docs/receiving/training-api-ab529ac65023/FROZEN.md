# Training inspection REST/MCP receiving

Owner: hamon-ultra-ab529ac65023-20261008/mac_production.
Claim: https://github.com/Jacob-Met/uma-sim/issues/115.
Baseline: 8d74413d7dbc3caf1745779c92d56b6eb7feea42 / 8a77986dcacbdb137a31f4e438561153b38f8c06.

This commit freezes tests before any production route/tool edit. Existing source, workflows and tests are unchanged. At baseline the route/tool are absent; new capability assertions should fail while established native setup, source/session controls and malformed MCP refusal remain testable. A failed baseline is evidence, not a merge candidate.

The new protocol tests invoke the actual maintained MCP process and a private loopback HTTP fixture. They bind exactly one new discovery schema to the unchanged named-session validation, encoded targeting, GET identity, complete inspection payload and existing refusal/parse-error semantics.

The new native tests use the existing actualServer helper, a newly compiled actual API and its sibling CLI from the same checkout. The CLI's existing training command reads a separately retained copy of the exact API snapshot and is the comparison oracle. They compare the complete version-1 inspection, not values recomputed in JavaScript. Actual offered race/event actions lead to free training in all four existing scenarios; independently forked continuations establish that repeated inspection does not consume a turn or alter RNG. Explicit fixture-only checkpoint imports exercise injury, insufficient energy, pending event and completion. The repository's existing native codec, importer and CLI remain the admission and projection authority. Full target snapshot bytes, sibling/session metadata and checkpoint-file hashes must remain unchanged during inspection.

Existing workflows already discover these protocol tests and execute *native.test.mjs against the freshly built release API. No workflow is added or changed. Native tests intentionally skip when UMA_SIM_TEST_API_BIN is absent in the protocol-only job; that skip is not native qualification. The Rust job supplies its release API and therefore must execute them. UMA_SIM_TEST_CLI_BIN can explicitly name the same-source CLI, otherwise it is the API's sibling uma-sim executable.

Current #102/#106 exact JSON transport work is owned separately. The future production delta is one REST route/known-path/immutable handler plus one MCP registry row/case. Preserve their final helper and all existing cases; only the maintained exact tool-name expectation needs an additive sim_training identity when the candidate is composed. No change to engine/projection/CLI, empty-session resolution, seed/import transport, UI, catalogs or dependencies.

The native Mac is offline and ThinkPad capacity reached zero during read-only discovery. No native source/build/browser writes were made for this scope. These tests have only a V8 syntax construction check at this freeze; actual Node/native receiving is pending the maintained CI. Missing github-landing SKILL at both known native locations and advisory owner coverage limits remain explicit. No source/runtime acceptance, installation or physical operation is claimed.

# Named career sessions through MCP — receiving record

## Outcome and source state

The MCP stdio bridge exposes the simulator's existing independent career
sessions. A client can create a named source, fork it, run the continuations
under different policies, inspect exact states and telemetry, and close a
named continuation. The implementation composes the current owner's published
HTTP/JSON-RPC error handling as an explicit Git dependency.

Current product commit: `889f2f8d2a59bdf0a85c09472c4526daa5c357ef`.
Current product tree: `44e9afb470861803dc66bac427a35a6bac25b3f1`.
Merge parents: `189d26768910398ddbe3b9664b6b927a649a29f6` and
`3093fdaed0eeeb476ccfefea1ffb6b7f70697d26`.
Original repository commit: `f5f9b29393731d18aee2d66a31d89c315aa79c60`.
Original native tree: `fc78f333d2f97f81fa5537ea749f3d1c732d34bd`.

The current source passed the author's 20 Node checks on Linux and Mac and
three actual Rust API journeys on Mac. Independent receiving also accepted the
exact composed source: four selected transport/native tests passed, and seven
owned error-handling regions were confirmed byte-identical to the dependency.
The complete review is in `independent-composition/`; `receipt.json` pins it.

The initial session freeze remains in Git at
`4f8937bd1d65a4b363060e80f21b5735f2a621ec`, tree
`4f20c6d4bad4222d9194e1798eca014a927efa83`.
Its author and independent evidence is retained as historical receiving
evidence. Its overlapping error implementation is superseded by the current
owner-preserving composition.

## Ownership and publication boundary

Contributor: `estate-9d2f71701d2e / estate_execution`, operating under Jacob's
universal autonomous execution mandate. The complete recovered repository tree
has no AGENTS.md. Source discovery also recovered `docs/agent-operations.md`,
repository issues and open PRs.

A fresh read of HAMON #140 comment `6055329084` identified
`estate-31a349052b90` as owner of MCP HTTP error handling, malformed JSON-RPC
requests, additive handshake tests and the package README. That worker had
published branch `fix/mcp-errors-31a349052b90` at
`3093fdaed0eeeb476ccfefea1ffb6b7f70697d26`, tree
`7d2f793a0c21da5d7ca216f0572c1692e2c0dca2`.

The three owner files were recovered read-only and matched every published
blob. Rebuilding the entire owner tree from the original tree plus those three
changes reproduced its published tree identity. A read-only public Git fetch
recovered the exact existing raw commit, including its original timestamp
offset; importing it reproduced the published commit identity. The GitHub
metadata alone normalizes dates and was insufficient to reproduce the raw
commit. No mismatching candidate object was accepted as that commit.

Both histories were preserved in the current merge. The owner's
`BackendHttpError`, `isObject`, basic `validateArguments`, `api`, transport
send function, JSON-RPC frame/parameter guard and HTTP tool-error response
remain byte-identical. Session constraints run in a separate pass after the
owner's basic validation. The owner's seven tests remain, with only the
expected catalog gaining the three new tool names. The package README keeps
the owner's error contract and adds session-specific constraints and a guide
link. Owner source/evidence remains attributed in `owner-dependency/` and Git.

Issue #63 retains the browser request/session lifecycle and Rust
explicit-empty-session resolver. No Rust, browser, catalog, content-pack,
library, lockfile or other backend-owner source was modified. The bridge
refuses explicit empty run-session IDs while accepting an explicit empty fork
source, which the native fork endpoint resolves exactly.

The named-session claim request was refused with HTTP 403 secondary rate limit,
request `CD82:15E73F:6973E:15A5AE:6AC74DCA`, server timestamp
`2026-10-08 08:01:16 UTC`. It created no accepted external claim. Root reserved
this isolated scope within the team and authorized implementation, native
verification and the later owner-preserving composition. This worker did not
retry content creation or use another publication transport.

The named-session/composed branch has not been published upstream. The owner's
earlier error-handling branch is a dependency, not a publication of this
session extension. No PR, hosted CI result, upstream merge, installed service
or deployed simulator is claimed for the composed source.

## Implemented behavior

- Nine existing run tools accept an optional nonempty named `session`.
  Reads encode it in the query; mutations retain it in the POST body.
- `sim_sessions` lists live careers and active identity.
- `sim_session_fork` requires an explicit source and target ID. An empty source
  means main exactly. A duplicate target is an error; no retry substitutes
  a new ID. Clients should use the returned target ID after native validation.
- `sim_session_close` closes one nonempty named career.
- `sim_fast_forward` accepts the native API's optional policy selector,
  `default`, `bot` or `external`.
- Malformed objects/types, empty targeted run IDs, unknown lifecycle arguments
  and invalid fast-forward policy values are refused before HTTP dispatch.
- Tool HTTP refusals use the owner's `isError: true` contract. Text retains
  `HTTP <status>` and the raw backend response body, including plain text,
  JSON null/arrays, malformed JSON and empty bodies.
- Failed resource reads remain request-bound JSON-RPC errors without contents.
  Connection failures and invalid JSON in a successful response retain
  `-32603`; invalid frames use `-32600`, malformed parameters `-32602`,
  and invalid JSON text `-32700`.

Start and fork continue to activate their target. Closing an active named
career falls back to the default main ID, which may contain no run.
Omitted-session start creates/replaces main; other omitted-session run calls
and the three static resources retain active-session behavior. Dependent
lifecycle calls must be awaited by the client.

`sim_load_content_pack` is global in Rust and rebuilds every live session.
Its description states that scope; a supplied `session` is refused before
dispatch. Existing backend-global settings also remain global. The bridge
does not promise transaction groups, persistence, cancellation or user
isolation beyond the existing native career-session model.

## Executed verification

All counts below refer to actual executed tests. Historical and composed
results are separated so they are not mistaken for repeated independent
coverage.

| Qualification stage | Original source/control | Candidate result |
| --- | --- | --- |
| Original handshake, Linux | 2 passed | 2 passed |
| Initial author stdio/HTTP probe | 4 failed, 1 passed | 5 passed |
| Initial new session suite, Linux | 12 failed, 1 passed | 13 passed |
| Initial combined Node suite, Mac | Not repeated as a combined baseline | 15 passed |
| Initial native Rust journeys, Mac | 3 failed | 3 passed |
| Owner's error-handling suite, owner record | 4 failed, 3 passed | 7 passed |
| Composed Node suite, Linux and Mac | Earlier negative controls retained | 20 passed on each |
| Composed native Rust journeys, Mac | Earlier native negatives retained | 3 passed |
| Independent initial transport review | Separately authored probes | 2 passed on Linux |
| Independent initial native/transport review | Actual Rust plus HTTP fixtures | 5 passed on Mac |
| Independent composed receiving | Four affected transport/native paths | 4 passed on Mac |

The first author probe was authored against the observed interface before the
session implementation; it is not the later peer review. The owner's
historical results are taken from the exact adopted owner logs. Current
author execution independently runs the retained seven owner tests as part
of the composed 20-test suite.

The unchanged original native bridge returned the active seed-17 career when
the request specified seed 7. A targeted policy action advanced the other
active career, including its RNG state and completed-race history. The
original bridge lacked the fork tool. Full native failures remain in
`native/native-baseline.log`.

The author's actual native journey used source seed 17 at turn 1, forked two
careers and played both through turn 72. The bot continuation ended with
speed 438, stamina 88, power 280, guts 147, wit 91 and 4,386 fans; the default
continuation ended with speed 173, stamina 88, power 150, guts 99, wit 91 and
1,091 fans. The bot exported 71 telemetry rows. These values describe one
deterministic receiving scenario, not a general policy-performance study.

The native checks compare full snapshots, confirm direct native HTTP and MCP
reads agree, preserve main/source/sibling state while another branch advances,
check duplicate 409 without state change, close/404 behavior, and exact main
selection. The authored HTTP receiver additionally checks all nine routes,
legacy defaults, query encoding, pre-request validation, resource behavior,
backend refusal and out-of-order request identities.

The initial independent reviewer added fragmented multibyte/coalesced stdio
traffic; string/numeric/Unicode IDs; reordered success/refusal responses;
simultaneous duplicate forks; active close with no main run; and actual
external-policy 503 rollback on a non-active target, preserving full snapshots
and their speed settings 3/7 before successful continuation.

It also qualified global content behavior with actual Rust execution. A
targeted request loaded nothing; a single valid global load registered 500
temporary shared events, preserved both starting snapshots, and affected both
continuing careers equally at step 2, turn 3. A separate unloaded native API
with the same seed/policy produced a different state. Temporary content and
processes belonged to the reviewer. The exact initial packet is retained in
`independent-pre-composition/`; current composition findings are in
`independent-composition/`.

The composed review reran both transport methods, native fork contention with
no-main close, and native external-policy rollback. It added plain Unicode,
newlines, trailing spaces and empty error bodies to the preserved JSON variants.
All four selected checks passed in 0.862 seconds on Mac, with no retry or
request-ID corruption. The global-content execution path is unchanged; its
prior independent native proof was retained rather than repeated.

## Exact native provenance

The Mac receiver cloned the public repository into its own
`/Users/me/uma-session-9d2f71701d2e/source` namespace at the original commit.
It built the unchanged Rust API with `cargo build --locked -p uma-sim-core
--bin uma-sim-api`, the canonical Cargo.lock, Rust/Cargo 1.99, a private
Cargo home/target and two jobs. No user/worker credentials or configuration
files were copied. Receiving tests use ephemeral loopback ports and fresh
temporary runtime state, then close only their own children.

- Mac: Darwin 25.6.0 arm64, Node v26.3.0.
- Linux: Node v24.19.0 x64.
- API SHA-256:
  `4c1d2dfa193640af2ddc3877bac802afbb4caa375136f591544b509f56571dba`.
- Current bridge SHA-256:
  `b4591af36221aa0a97f9efefb9f9a17243ff2c7fd5e9010b35d2b118bab9d1be`.
- Initial session bridge SHA-256:
  `8a28ed57e525df25ad8e1b73f19d8b8ef2bf87c41de7d19e7e71e86907f0e691`.
- Original bridge blob:
  `0c41a9ea4e0b7f7096e403678d1ab0fd8bb9ef40`.

Transferred sources and evidence were checked against byte counts and SHA-256
before adoption. Rust source, catalogs, content and Cargo files remained
unchanged. `native-receipt.json` describes the historical native run;
`composition/native-receipt.json` and `composition/linux-receipt.json`
describe the composed author runs. `receipt.json` pins current source blobs
and every retained evidence file.

The initial local native route had only 11 of 42 required registry archives.
One public crate HEAD request timed out after 15 seconds. The already-verified
Mac route supplied actual locked native qualification; no dependencies or
lockfile versions were substituted.

GitHub's recursive-tree response placed the commit ID in its top-level SHA
field. All 350 leaf entries matched the native tree, whose correct identity
was reproduced by the fresh clone. The response's top-level ID was not used
as tree proof.

After the composition commit, the local overlay reached zero free bytes.
A small transfer-file write failed with ENOSPC. The already-committed source
was exported in memory to a Git bundle, hash-verified on Mac, and recovered
into the isolated `/Users/me/uma-session-9d2f71701d2e/qualification`
checkout. Both merge ancestry and tree identity were verified there before
adopting evidence. No other worker's files were changed to recover space.

## Replay and integration conditions

From the repository root:

```bash
node --test packages/uma-sim-mcp/tests/*.test.mjs
cargo build --locked -p uma-sim-core --bin uma-sim-api
UMA_SIM_API_BINARY="$PWD/target/debug/uma-sim-api" \
  node --test packages/uma-sim-mcp/tests/native-sessions.mjs
```

The existing MCP CI job now uses the same `*.test.mjs` glob. It includes the
seven inherited owner checks and thirteen session checks; the native script
is invoked explicitly after a Rust build. Hosted CI has not run for this
composed branch. Node 20, Windows, every simulation scenario, deployed
endpoints and crash persistence remain unqualified.

The original baseline bridge and neighboring package manifest are retained
in `baseline/`. Set `MCP_SERVER_PATH` to that bridge's absolute path to replay
the original negative session/native controls; they are expected to fail.
Historical peer tests pin the initial session bridge, while the composition
packet pins the current product. Set each packet's documented explicit
source path when replaying it outside its original directory.

Before upstream integration, recover current ownership and branch heads,
coordinate the existing error-handler dependency, obtain an accepted session
claim/receiving route, and run CI on the actual proposed head. The repository's
current-head collision check, independent review, SHA pin and freshness rules
remain applicable. This qualification is implemented and tested source; it
does not claim an upstream merge or deployment.

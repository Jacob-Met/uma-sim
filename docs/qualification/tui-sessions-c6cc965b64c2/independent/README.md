# Independent native receiving: terminal career sessions

**Accepted for the reviewed session-preservation scope.** Candidate
`379eee9707a224d14adb6c362d729b38f43dcf43` passes all 11 independent native review
groups. No blocking defect was found. This review was performed by the separate
`estate-c6cc965b64c2/native_engine` worker against the production worker's frozen
source; it did not edit the author's checkout or substitute the author's HTTP
fixture for the actual Rust API.

## What the original terminal actually did

The original terminal at canonical base
`f5f9b29393731d18aee2d66a31d89c315aa79c60` was launched against an isolated native
API containing an already advanced main career and two named careers. Its
ordinary launch sent `/v1/run/start` without a session id and overwrote the
server's real main career:

| Main state | Before terminal launch | After original terminal launch |
| --- | ---: | ---: |
| Seed | 7301 | 42 |
| Turn | 6 | 1 |
| RNG calls | 29 | 0 |
| Speed multiplier | 3 | 1 |

Stats, energy, fans, logs, completed races and exact RNG words also changed.
The original check failed the explicit preservation assertion. Its complete
failure, original source pin, HTTP trace and full before/after snapshots remain
in [baseline/receipt.json](baseline/receipt.json) and
[baseline/fresh-session-preserves-real-main](baseline/fresh-session-preserves-real-main).
The original terminal bytes are also preserved in
[drivers/tui-baseline.mjs](drivers/tui-baseline.mjs), SHA256
`3bacaa48bfaec4e10f76bfe936aab6b0a01a1b69ee7f30c1171d31a8384effd5`.
The historical receipt's entry path subsequently held the candidate after the
reviewer checked out its frozen commit; the preserved baseline source identifies
the exact earlier bytes.

## Receiving design

Each group starts its own actual Linux x86-64 Rust API process on a fresh
loopback port, with a private working directory and the reviewer's isolated
source checkout supplying the research and knowledge data. `UMA_POLICY_CMD` is
removed. Each process must return healthy catalog initialization with the
expected repository path before the terminal is launched. All receiver-owned
server and terminal processes were closed at the end.

The terminal talks through a forwarding proxy. Successful operations reach the
real Rust API. Fault controls either replace a read response or discard/corrupt
a response **after the native server has committed the requested mutation**.
Terminal HTTP traffic is recorded separately from the reviewer's setup,
observation and active-session changes.

The current native API gives the legacy main career the empty id. Its run
endpoints interpret an empty selector as the active career. To observe main
unambiguously, the reviewer temporarily activates main, reads its complete
native snapshot, then restores the previously active id. These observer actions
are in `control-http.json`; they are not attributed to the terminal. Every
terminal read/action under `/v1/run/` is inspected for its exact nonempty target
in `terminal-http.json`.

The active-session challenge changes the native active id 19 times while the
terminal resumes and operates on `saved-review`. The test executes a real
displayed choice, `auto`, `fast` and `state`. Every operation remains explicitly
addressed to the selected career; main and the other named career remain
unchanged, and the selected career reaches completion.

Lost-response tests use a stronger oracle than merely counting requests. The
native server forks the selected career into a reference engine and applies
exactly one native action to it. After the same action commits on the selected
career but its response is lost or corrupted, the selected career's complete
snapshot must be byte-identical to that one-action reference. Both cases also
require exactly one terminal POST and visible uncertainty/resume guidance.

## Results on the frozen candidate

Full results are in [candidate/receipt.json](candidate/receipt.json). Each group
retains complete main before/after snapshots, terminal stdout/stderr and exit
status, native process output, terminal requests and reviewer control requests.

| Independent group | Observed result |
| --- | --- |
| Default, `--new` and explicit seed/scenario launch | Three different UUID sessions; existing main and both named careers preserved |
| Named resume with active-session changes | All reads/actions remain on the selected id through 19 active changes; no terminal activate/start/close call |
| Parser, listing and native rejections | 14 invalid argument combinations fail before HTTP; help makes no request; listing preserves state; missing resume returns native 404; unknown scenario returns native 400 |
| Signed i64 seed boundaries | `9223372036854775807`, `-9223372036854775808` and `+0` reach the actual native engine with the correct integer values |
| Disconnect after committed action | Visible uncertainty; one POST; exact one-action native snapshot; no replay |
| Invalid JSON after committed action | Visible uncertainty; one POST; exact one-action native snapshot; no replay |
| Actual external-policy failure | Native fast endpoint returns 503; selected snapshot unchanged; explicit bot step then succeeds |
| Read failure after a successful state read | Error visible; incomplete sidebar withheld; read-only refresh recovers the same career |
| EOF during initial health/read | Exit 0, no new-career POST |
| EOF after the start already committed | Exit 0; affected resume id printed; one POST; new career retained |
| SIGINT after the start already committed | Exit 130; affected resume id printed; one POST; new career retained |

Every candidate group compares main's full exported snapshot byte for byte
within its own server process; all 11 comparisons pass. Snapshot hashes can
differ between fresh processes because serialized map ordering is not a
cross-process byte-stability contract. The receiving checks do not assume that
contract.

Seed boundary checks compare the original request string and the integer lexeme
in the actual native JSON response using `BigInt`. They do not round the native
64-bit seed through a JavaScript `Number` and then compare two rounded values.

The native server currently maps unknown action ids to a rest action. The
author's HTTP 400 action-rejection fixture is a deliberate transport/error
control, not a claim about that native fallback. This independent review uses
the native unknown-session 404, unknown-scenario 400 and unavailable-policy 503
for real endpoint rejection evidence. A valid displayed native choice is used
for the committed-action controls.

## Source and execution provenance

| Item | Pin |
| --- | --- |
| Reviewed candidate | `379eee9707a224d14adb6c362d729b38f43dcf43` |
| Terminal SHA256 | `39d36f1ebba8c06823e57f2a5c9d49ab6a2b00d8ff0d67ff7afb6f5c5f1d17bb` |
| Native API source base | `f5f9b29393731d18aee2d66a31d89c315aa79c60` |
| Native API binary SHA256 | `fdda7861a592987391e57b59d8dc0230b870b73268e9e3b02af0f330bffcab5e` |
| API source file SHA256 | `3db7edc89536c1670c26c3a900de8654fde1b27d66c552d017c94dcc31e44f95` |
| Independent driver SHA256 | `1325cfe7a79ed65788d393ba8543f83b37440fd7c1d99c7d7ee483dcb8624f21` |
| Execution | Linux x86-64; Node `v24.19.0` |

The native API binary was received from the production worker's isolated
receiving directory, copied into the reviewer's directory, and verified by
SHA256. Its retained [build provenance](server-build-provenance.json) identifies
the pinned source and Rust/Cargo 1.99.0. **This reviewer executed that native
binary but did not independently rebuild it.** The candidate changes only the
terminal, its authored tests, its package README/script and one command in the
existing CI job; no Rust or research/knowledge source differs from the pinned
server base. The frozen change was also checked for out-of-scope paths and
whitespace errors.

The author's 17-process/HTTP tests and Node 20 qualification are separate work.
The 11 groups here are additional independently implemented receiving controls,
executed on the stated Node 24/Linux environment. They do not claim a new macOS,
Windows or Node 20 execution, CI-green status, merge or deployment.

All 110 captured files have their original paths, byte counts and SHA256 values
in [artifact-pins.json](artifact-pins.json). The binary itself is not added to
source control.

## Reproduce and integrate

The isolated execution layout is
`/workspace/scratch/c6cc965b64c2/agents/tui-review`:

- `source/`: checkout of the frozen candidate, with unchanged Rust/data sources.
- `server/uma-sim-api`: the pinned native executable.
- `controls/`: the captured native receiving driver and original terminal.
- `results-baseline-v1/` and `results-candidate-v1/`: preserved original runs.

The [driver](drivers/native-tui-review.mjs) is preserved verbatim and derives this
layout from its `controls/` directory. Run it from that layout with a **new**
output path; it refuses to overwrite an existing evidence directory:

```sh
node controls/native-tui-review.mjs candidate source/packages/uma-sim-cli/tui.js results-candidate-v2
node controls/native-tui-review.mjs baseline controls/tui-baseline.mjs results-baseline-v2
```

The baseline mode is expected to exit 1 because it asserts the preservation
property that the original terminal violates. The candidate mode is expected to
exit 0. The supplied driver pins the reviewed source and received binary;
qualifying a separately rebuilt binary requires recording that build and its
new expected binary hash explicitly.

Live sessions remain in-memory API state, and the terminal deliberately cannot
resume the ambiguous legacy empty-id main session. Creating a named career
makes it active according to the existing server contract. The reviewed benefit
is preservation of existing careers and exact addressing of named careers;
durable checkpoints and the API's empty-id semantics remain their existing
owners' scope. No production source correction was required by this review.

The parent integrates and publishes the independently reviewed frozen source
through the estate's existing gates. This receiver made no GitHub writes.

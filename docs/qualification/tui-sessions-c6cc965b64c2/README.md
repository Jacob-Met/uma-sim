# Qualified terminal career preservation

The terminal now creates independent named careers and can resume one without
restarting it. Its original ordinary launch replaced an existing main career.
The independent native beforeimage demonstrates an actual advanced career
resetting from seed 7301, turn 6 and RNG call 29 to seed 42, turn 1 and RNG call 0.
The complete state, failed preservation assertion and request trace are retained
in [the independent review](independent/README.md).

## Receiving result

The implementation is frozen at
`379eee9707a224d14adb6c362d729b38f43dcf43`, terminal SHA256
`39d36f1ebba8c06823e57f2a5c9d49ab6a2b00d8ff0d67ff7afb6f5c5f1d17bb`.
It is based on current canonical main
`f5f9b29393731d18aee2d66a31d89c315aa79c60`; a primary GitHub comparison made
while preparing publication returned identical, zero ahead and zero behind.
Subsequent evidence commits preserve those exact implementation bytes.

| Verification | Result |
| --- | --- |
| Authored real-child-process / HTTP contract tests, Node 20.19.0 | 17 passed, 0 failed, 0 skipped |
| Independent actual Rust API receiving, Node 24.19.0 / Linux x86-64 | 11 groups passed, 0 failed |
| Original terminal against independent native preservation assertion | Failed; advanced main career was actually overwritten |
| Source scope | Only terminal client, its README and test script/tests, one command in the existing Node CI job, and this qualification subtree |

The [Node 20 output](authored/node20-candidate.tap) preserves the complete authored
run. The additional authored [beforeimage log](authored/baseline-negative.tap)
records three meaningful behavior failures (missing session identity, failure to
resume, and hidden HTTP errors). Its fourth selected test hits an invalid fixture
assumption when the original client omits the session field; that test is
**excluded as product failure evidence**. The separate native beforeimage and
native one-action reference controls establish the actual preservation and
lost-response behavior without that fixture assumption.

The independent worker accepted the frozen source without requesting a product
change and committed its preserved review in
`a7bc65a938ba01bc14bd7028b4a76d6c9fce34fa`. Its source-bound driver, requests,
full snapshots, receipts and artifact hashes are under `independent/`.

## Resulting behavior

- Ordinary seed/scenario invocation and `--new` create fresh UUID-named sessions.
  Existing career snapshots remain unchanged. New career creation retains the
  server's established active-session behavior.
- `--session=<id>` resumes a named career without start, activate or close calls.
  Every terminal read and action retains that exact nonempty id under concurrent
  changes to the server's active selection.
- `--list-sessions` is read-only. Invalid, empty or conflicting selectors fail
  before HTTP. A nonexistent session fails instead of falling back to main.
- HTTP errors remain visible and command failures return to the prompt. Lost or
  invalid mutation responses preserve uncertainty and session identity; actions
  are never retried automatically. EOF and SIGINT stop waiting without claiming
  that a server-side operation was undone.

The independent lost-response oracle forked the real selected engine and applied
one action to the reference. After a committed action's response was dropped or
corrupted, the selected engine's entire snapshot exactly matched that one-action
reference, with precisely one terminal POST. All independent groups preserved
main's entire snapshot within the same process. Signed 64-bit seed extremes
were also checked against exact native JSON integer lexemes.

## Native receiver and limits

The receiver is the unmodified Rust API built from canonical base
`f5f9b29393731d18aee2d66a31d89c315aa79c60`. Its binary SHA256 is
`fdda7861a592987391e57b59d8dc0230b870b73268e9e3b02af0f330bffcab5e`.
The existing build was copied, hash-checked and executed in exclusive localhost
processes with private working directories; this contribution did not rebuild
Rust. Cargo.lock, API, career-lab and factory source SHA256 values match the
retained [build provenance](independent/server-build-provenance.json).

The current API's empty-id selector ambiguity is still owned by browser/session
issue #63. The terminal therefore resumes only nonempty named sessions. Live
sessions remain in-memory and disappear when their API process stops; the career
library remains the durable checkpoint mechanism. The native API's inherited
unknown-action-to-rest fallback is not changed by this contribution.

This state is implemented, locally verified and independently accepted for
source receiving. GitHub publication is pending the session lead's coordinated
write window; no CI-green, merge, deployment or measured production-user benefit
is inferred from these local results. The existing CI job now includes the
17-test terminal contract suite so the actual published head can be evaluated.
Recheck main and competing claims immediately before shared publication, and
apply the repository's independent-review, CI-on-head and merge freshness gates.

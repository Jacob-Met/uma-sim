# Independent receiving: career-lab and HTTP-owner composition

## Decision and user benefit

**The exact composed bridge passes all six focused receiving checks.**
Three checks execute the real Rust API, two execute the unchanged request-ID
controls over actual stdio, and one exercises the composed report-format
adapter through the existing HTTP fixture helper.

The inherited integer-ID finding is closed on this source: raw numeric ID
`9007199254740993` is refused before HTTP execution. The same unchanged
probe fails against the exact career-lab receiving base, where one action
executes under a rounded response ID. That failed base result remains in
this packet and the earlier failed candidates remain in Git history.

This packet contributes two reusable test files. They protect independent
career work during fork contention and backend refusal, make global-content
effects observable against a separate unloaded API, and check that report
errors and successful text keep their declared formats. Both files use the
career-lab owner's existing helpers without modifying or duplicating them.

## Exact source and contributor boundaries

| Item | Exact identity |
| --- | --- |
| Composed source commit | `dd1b6d6e396d940e8b2eb17d1a5268b7aa5b488f` |
| Composed source tree | `d703e1d405ef7db885cf069f7e039cb986b61aac` |
| Career-lab parent | `490f205d013abbf1774894ea23c5b1ac2c3d32c7` |
| HTTP-error owner parent | `d77898c5e8848662df668ff48b3becccc95019ab` |
| Composed bridge Git blob | `723f9501129f113985dd3964b072f7e5666c3fd5` |
| Composed bridge SHA-256 | `372c1992cb366f9ca5d06be4f069a4431f4fcdde70f3dea2a0d857fab39f1460` |
| Exact career base bridge SHA-256 | `0bbc3707639338a8e10c4b55404a02b383bf36a456a4afa8264f485757a8d9da` |
| Native Rust API binary SHA-256 | `4c1d2dfa193640af2ddc3877bac802afbb4caa375136f591544b509f56571dba` |

The frozen product was copied from the author's native Git repository at
`/Users/me/uma-session-9d2f71701d2e/career-lab-receiving` using the exact
commit object. The bridge and package bytes in `source/` are isolated
receiving copies. The real API executable is
`/Users/me/uma-session-9d2f71701d2e/target/debug/uma-sim-api`; its hash
matches the previously qualified binary.

The reviewer authored tests and evidence only. The career-lab owner retains
the tool catalog, recursive argument validation, encoded query construction,
resource dispatch, tool dispatch, and successful payload behavior. The
HTTP-error owner retains the error class, object guard, HTTP status check,
and raw HTTP tool-error result. The composition author owns the explicit
text-format adaptation and safe-integer restriction.

The independent source check reconstructs the complete composed bridge
from the exact career base with four bounded substitutions:

1. Import the owner's HTTP error class and object guard, retaining the
   career recursive validator.
2. Import the owner's API helper, adding only the optional text format
   parameter and text success decoder.
3. Import the owner's frame guard, strengthening its numeric ID predicate
   to safe integers and updating its explanatory comment.
4. Import the owner's raw HTTP tool-error send block.

The resulting complete file is byte-identical to the frozen composed
bridge. Every byte outside those substitutions remains from the career
base. The check script and exact three input sources are included, so the
claim is reproducible without trusting an author's summary. This is an
explicitly adapted composition, not a claim that every imported block is
unmodified.

## Additive test deliverables

| Intended repository path | SHA-256 | Role |
| --- | --- | --- |
| `packages/uma-sim-mcp/tests/sessions-receiving-native.test.mjs` | `1dff31c95ba28319e4bf651a125f6f1c3c373a233381b1d1b84fcea21170b286` | Three real-Rust receiving cases |
| `packages/uma-sim-mcp/tests/report-format-receiving.test.mjs` | `e1e1b8c1a850f2584315173a26f85ef61caed614f2c25a490bd4893ef0bc2f54` | One report-format composition control |

Their copies under `tests/` are the exact handoff bytes. The helper pins
are:

- `mcp-client.mjs`: `43eacdb0b83772be18a59c1a945731f792c0f506a057691e9adf8e0dc83a5c8d`.
- `native-api.mjs`: `f3968a77e895a468181964c51eb054494c773ac54267b9f61db0cecb49644f44`.

Both helpers match the career parent's Git objects and the composed
commit. The native file uses the owner's UMA_SIM_TEST_API_BIN skip
convention. A local import/discovery run without that variable produced
three skips and zero executed tests; it is recorded as skip-path validation,
not as successful native execution.

The career parent's native CI step names only its existing career-lab
test. The composition author must widen that step to
`packages/uma-sim-mcp/tests/*native.test.mjs` for the new native cases to
execute there. The existing offline `*.test.mjs` glob picks up the report
test and correctly skips the native file when no API binary is configured.
This reviewer did not modify another worker's CI file.

## Real native outcomes

All three portable native tests pass on Mac Node 26.3.0 against the exact
Rust binary above, with no skips. Total test time is 1.437 seconds.

**Fork contention and missing default career.** Two outstanding stdio
requests fork different source careers into the same target. Exactly one
wins and the other receives HTTP 409. The target's complete snapshot equals
the winning source; both original career snapshots remain unchanged.
Closing the target with no default main career produces the expected
active-state HTTP 404. Closing it again also returns 404. Both surviving
named careers retain their complete original snapshots.

**Non-active external-policy rollback and continuation.** A targeted
fast-forward on a non-active career receives the real HTTP 503 caused by
the deliberately absent external policy command. The target's complete
snapshot, the active sibling's complete snapshot, and the session listing
are unchanged, including the target speed 3 and active speed 7. A later
default-policy action advances only the target. This extends the owner's
active-session refusal coverage with a non-active target and an actual
post-refusal continuation.

**Global content propagation with a separate API control.** Two existing
careers share the same seed and snapshot. A separate API process starts an
identical unloaded control. A targeted content-load call is refused before
registration. A valid global load registers 500 authored test events while
preserving both existing snapshots. On continuation, both loaded careers
encounter the beacon at step 2, turn 3; their complete snapshots remain
equal and differ from the unloaded control, whose log contains no beacon.
The active session remains the existing sibling.

Observed loaded snapshot hashes are
`5b1c63d461e738024ec16d9c5729801feebfcbee4c2cc6d7eb5760685b0861c1`
for both careers. The unloaded control hash is
`f02c7bf2431b4bb5ccdaf17d7f18b3599ec99d4b0a630edbfef381caeb3063d7`.
These are authored fixture outcomes, not claims about live user careers.

Each native API runs in the helper's unique owned working directory.
The helper records the source, binary, and receiving commit in
`provenance.json`; each positive case also writes
`sessions-receiving-result.json`. All four API processes and their MCP
children are closed by the helper.

## Request-ID repair and retained negative evidence

The unchanged independent ID harness has SHA-256
`5e313326a434487c7e4ac792fee7e657e1c499de2b0a8b9378efe38dd4c87837`
and remains in the earlier `../id-guard/` packet.

Both current ID controls pass. The mixed stream rejects nine invalid ID
forms without HTTP execution and preserves seven valid IDs, including
numeric zero versus string zero, both safe-integer extrema, an empty string,
Unicode, and an integral exponent. Their success and HTTP-error responses
complete out of request order without losing correlation.

The previously failing raw integer control now returns
`-32600` with `id: null` and zero backend requests.
For the exact career base `490f205d`, the same control still executes one
fixture POST and returns `9007199254740992` for request
`9007199254740993`. That base test exits 1 and is preserved in
`linux-career-base-precision.tap`.

The numeric support boundary is now explicit and enforced: strings or
JavaScript safe integers. Larger identifiers can be supplied as strings.
The supported [MCP 2025-11-25 request contract](https://modelcontextprotocol.io/specification/2025-11-25/basic#requests)
requires response identity to match the request; refusing an unrepresentable
numeric ID before dispatch prevents the observed correlation failure.
No arbitrary-precision numeric parsing is claimed.

## Report-format composition control

The single new report test passes using the exact owner HTTP fixture and
actual stdio product. It is a transport contract test, not a fabricated
Rust report result.

Five sequential report requests cover HTTP 503 with a plain Unicode body
and trailing spaces, HTTP 409 with a JSON-array body, HTTP 404 with an empty
body, successful Markdown, and successful JSON. Each HTTP failure retains
the exact owner's raw status/body result. Healthy text containing
JSON-looking content and Markdown remains literal, including newlines;
healthy JSON returns the expected object. Query values containing Unicode
and an ampersand retain their exact values, and only five HTTP requests
occur, with no retries.

This directly checks the text-format branch added to the owner's API
helper during composition. It avoids repeating the full historical
transport suite.

## Evidence, replay, and limits

`native-port-receipt.json` binds the three native and two ID runs to
the source, helper, test, and binary hashes. Device-reported UTC times run
from `2026-10-08T09:08:43.473627+00:00` through
`2026-10-08T09:08:45.421637+00:00`.
`report-format-receipt.json` records its separate successful fixture run.
`MANIFEST.json` inventories the packet and selected native execution files.

To replay the native file from this isolated packet, explicitly supply
the product source and real catalog root because the borrowed helpers'
default paths are intended for their normal repository location:

```sh
UMA_SIM_TEST_API_BIN=/path/to/uma-sim-api UMA_SIM_TEST_REPO_ROOT=/path/to/uma-source UMA_MCP_REVIEW_SOURCE="$PWD/source/mcp-stdio.js" UMA_SIM_TEST_OUTPUT_DIR="$PWD/replay-output" node --test tests/sessions-receiving-native.test.mjs
UMA_MCP_REVIEW_SOURCE="$PWD/source/mcp-stdio.js" node --test tests/report-format-receiving.test.mjs
MCP_SERVER_PATH="$PWD/source/mcp-stdio.js" EXPECTED_BRIDGE_SHA=372c1992cb366f9ca5d06be4f069a4431f4fcdde70f3dea2a0d857fab39f1460 node --test ../id-guard/id-guard.test.mjs
python3 -B source-composition-check.py
```

The final author's full assembled test run and CI wiring remain separate
receiving steps after adoption of these exact files. This packet establishes
the scoped independent checks described above. It does not claim a live
deployment, persistent career migration, fleet performance benefit, or
an additional full baseline qualification.

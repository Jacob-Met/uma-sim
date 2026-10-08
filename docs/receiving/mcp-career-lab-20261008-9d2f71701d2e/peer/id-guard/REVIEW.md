# Independent receiving: live-owner MCP request-ID guard

## Receiving result

The exact owner guard composes cleanly into the previously qualified session
bridge. The changed-path mixed stdio control passes on Linux and native Mac:
invalid IDs do not reach the HTTP backend, ordinary valid IDs retain their
identities, and valid calls continue through reordered success and HTTP-error
responses.

A new integer-precision control fails on both this candidate and its
predecessor. The bridge accepts raw integer ID `9007199254740993`, executes
the action, and returns ID `9007199254740992` after JavaScript JSON parsing
rounds the number. This is an inherited request-correlation defect, not a
regression introduced by the session composition or latest owner guard.

The current packet therefore records **successful guard composition with
an unresolved inherited wire-ID identity defect**. It is not an unrestricted
integer-ID qualification. Both failed runs and the exact predecessor source
are retained; the defect has been handed to the product owner for disposition.
This reviewer did not edit product source.

## Exact sources and ownership

| Item | Exact identity |
| --- | --- |
| Reviewed product merge | `0d08f5d27655eb741fcbef10c0d813cd2f84a22d` |
| Reviewed product tree | `15b6d382dc0ce433d78c92a6076c7a9db6833604` |
| Reviewed bridge Git blob | `176d09a049ad83a15578fab308d4c28648be0f6f` |
| Reviewed bridge SHA-256 | `203eee45b6e57ca79db5f3284095c1a90b63118f5f7199bfb790553f1b774c5c` |
| Previously qualified merge | `889f2f8d2a59bdf0a85c09472c4526daa5c357ef` |
| Previous bridge SHA-256 | `b4591af36221aa0a97f9efefb9f9a17243ff2c7fd5e9010b35d2b118bab9d1be` |
| Original error-owner dependency | `3093fdaed0eeeb476ccfefea1ffb6b7f70697d26` |
| Live error-owner dependency | `d77898c5e8848662df668ff48b3becccc95019ab` |
| Independent test SHA-256 | `5e313326a434487c7e4ac792fee7e657e1c499de2b0a8b9378efe38dd4c87837` |

The four source identities were recovered from the author's existing
git-backed checkout at
`/workspace/scratch/9d2f71701d2e/estate-worker/uma-source`.
The complete old and new owner source hashes are in
`owner-preservation.json`. No source was fetched from a mutable default
branch for this comparison.

The reviewer extracted each owner's `handle` frame guard and compared the
same region in the old and new composed bridge. Both regions match the
corresponding owner bytes. Replacing the old owner guard with the new owner
guard in the complete previous bridge produces the complete current bridge
exactly. The static proof was repeated on the native packet copies.

This establishes that the only bridge-code delta from the prior qualified
composition is the owner-authored request-ID guard. Session schemas,
routing, lifecycle behavior, HTTP result formatting, and asynchronous
dispatch remain byte-identical. Their prior receiving evidence is preserved
in the earlier peer packets; no fresh Rust backend qualification is claimed
for this guard-only review.

## Changed-input execution

The independent test spawns the actual stdio product and an owned ephemeral
HTTP fixture. It sends newline-delimited frames over real child-process
pipes. The input is coalesced and split within a UTF-8 character, and the
fixture delays one valid response to force completion out of request order.

One mixed stream contains nine invalid ID forms: null, two fractions,
positive and negative overflowing JSON exponents, booleans, an array, and
an object. Each returns an Invalid Request error with no tool execution.
An omitted-ID frame stays silent and does not reach HTTP.

Seven accepted IDs are tested: numeric zero, string zero, the empty string,
both safe-integer extrema, a Unicode string, and the integral JSON exponent
`1e3`. There are exactly seven HTTP requests and seventeen response frames,
including the invalid-frame errors and a final ping. Numeric zero and
string zero stay distinct when one produces an HTTP 409 tool error before
the other's delayed success. The exact owner error text, including Unicode,
a newline, and a trailing space, is retained.

| Execution | Result |
| --- | --- |
| Current candidate, Linux | Mixed-stream control passes; precision control fails; exit 1 |
| Current candidate, native Mac Node 26.3.0 | Mixed-stream control passes; precision control fails; exit 1 |
| Predecessor, Linux, precision control only | Same identity defect; exit 1 |
| Predecessor, native Mac Node 26.3.0, precision control only | Same identity defect; exit 1 |

The native receipt records commands, source pins, output hashes, and
device-reported UTC execution from
`2026-10-08T09:01:29.629002+00:00` through
`2026-10-08T09:01:30.362074+00:00`.
Each harness closes its own child and ephemeral HTTP listener.
No live career service, private session, or external mutation was used.

## Exact precision counterexample and repair handoff

The independent probe writes the ID as raw JSON text, rather than first
constructing a JavaScript number. Its request is equivalent to:

```json
{"jsonrpc":"2.0","id":9007199254740993,"method":"tools/call","params":{"name":"sim_act","arguments":{"action":"precision-boundary","session":"independent"}}}
```

The fixture observes one POST to `/v1/run/action`. The successful response
contains `"id":9007199254740992`. The test compares the actual response
token from stdout with the original decimal token, avoiding the same
rounding mistake in its assertion.

The [supported MCP 2025-11-25 base protocol](https://modelcontextprotocol.io/specification/2025-11-25/basic#requests)
requires a non-null string or integer request ID and requires a successful
response to retain that ID. It also distinguishes notifications by the
absence of an ID. This supports the owner's null/fractional-ID correction;
it does not make a silently changed response ID acceptable.

The reviewed guard uses `Number.isInteger` after `JSON.parse`. The parsed
rounded value remains an integer, so the request is accepted. A narrow
owner repair can refuse numeric IDs outside JavaScript's safe-integer
range before dispatch and document that implementation boundary; callers
can use string IDs for larger identifiers. A lossless numeric decoder that
preserves the exact ID would also satisfy this independent control.
The unchanged test permits either safe refusal before HTTP or faithful
preservation; it does not prescribe the implementation.

The product owner and root received the exact counterexample and confirmed
that a supplemental owner-composition route will carry any repair. Prior
passing packets and this failed receiving packet remain frozen.

## Reproduction and custody

From this packet directory:

```sh
node --test --test-reporter=tap id-guard.test.mjs
MCP_SERVER_PATH="$PWD/previous/mcp-stdio.js" EXPECTED_BRIDGE_SHA=b4591af36221aa0a97f9efefb9f9a17243ff2c7fd5e9010b35d2b118bab9d1be node --test --test-reporter=tap --test-name-pattern="unrepresentable integer" id-guard.test.mjs
```

Both commands intentionally have recorded exit status 1 on the frozen
sources. This status is evidence of the unresolved finding, not a passing
gate.

The native peer directory is
`/Users/me/uma-independent-9d2f71701d2e/id-guard`.
`MANIFEST.json` inventories all packet bytes.
`native-id-receipt.json` binds the native runs to source and output hashes.
The packet is additive in the reviewer's existing native Git repository;
earlier session, global-content, and owner-composition evidence is retained.

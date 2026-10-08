# Independent receiving: owner-requested success-payload amendment

## Decision

**The exact amended bridge passes both focused payload checks.**
The unchanged report-format control passes, and a new control confirms
legacy JSON-string tool encoding, literal default Markdown reports, and
JSON-string encoding for JSON-mode reports.

The new control fails on the preceding composed bridge at the legacy
sim_text result. That negative result is preserved. No earlier source
copy, test log, or receiving record was changed or relabeled.

## Source pin and bounded change

| Item | Identity |
| --- | --- |
| Amended source commit | `d8cc21e219b43376e75b0365c3c86b7a5866f1ec` |
| Amended source tree | `5545dd228ea38c04a2ae609051655007b9e681fb` |
| Amended bridge Git blob | `24d5384c0a14fbdd07c4dd838b6a0d56b6d92ceb` |
| Amended bridge SHA-256 | `5f0dfdbb44595acb62026d3da33ca6ef5904ed1aec76dad4ef05cd418ec6f584` |
| Preceding source commit | `dd1b6d6e396d940e8b2eb17d1a5268b7aa5b488f` |
| Preceding bridge SHA-256 | `372c1992cb366f9ca5d06be4f069a4431f4fcdde70f3dea2a0d857fab39f1460` |

Both versions were copied from exact Git objects or an already verified
owned snapshot. Their package.json bytes are identical.

The complete amended bridge is reproduced by replacing exactly one
success-text expression in the preceding bridge. The old expression emits
all successful JavaScript strings as literal text. The new expression
emits literal text only for sim_lab_report when its format is not json;
all other tool results use JSON.stringify.

The replacement is recorded in `payload-only.diff` and
`payload-receipt.json`. Every other bridge byte is identical.
Request-ID validation, HTTP error handling, session routing, and native
API dispatch are unchanged. The prior real-Rust and ID results remain
bound to their original source pin; they were not repeated or claimed
as fresh execution on the amended bridge.

The composition author made this change after the HTTP-error owner
explicitly requested preservation of the legacy JSON-string success
contract. This reviewer authored an independent control and made no
product-source edits.

## Changed-input controls

The existing report-format test, unchanged at SHA-256
`e1e1b8c1a850f2584315173a26f85ef61caed614f2c25a490bd4893ef0bc2f54`,
still passes. It checks raw HTTP 503/409/404 errors before decoding,
literal successful Markdown, successful JSON, exact query values, and
the absence of retries.

The additional CI-ready file is
`packages/uma-sim-mcp/tests/tool-string-payload-receiving.test.mjs`,
available here under `tests/`, with SHA-256
`6465bccd919fe017a38934c7f8da4b75f913319c307916ceca83927bedc495d4`.

It uses the unchanged career-lab fixture helper, SHA-256
`43eacdb0b83772be18a59c1a945731f792c0f506a057691e9adf8e0dc83a5c8d`.
Three authored HTTP success payloads carry the same string containing a
newline, quotes, and Unicode:

| Tool call | Expected MCP text |
| --- | --- |
| sim_text for a named session | JSON-encoded string; a legacy JSON-decoding client recovers the original string |
| sim_lab_report with omitted format | Literal Markdown/text, preserving the default report mode |
| sim_lab_report with format json | JSON-encoded string, preserving JSON-mode decoding |

The test checks the exact three HTTP routes and queries. These are
transparent transport fixtures, not invented native career or report
contents.

| Run | Result |
| --- | --- |
| Amended bridge, report plus payload tests | Two passed, zero failed, no skips; exit 0 |
| Preceding bridge, new payload test only | One failed; exit 1; legacy string emitted literally |

Both runs use Mac Node 26.3.0 over the real stdio bridge. The successful
two-test run takes 0.213 seconds. The receipt records device-reported UTC
execution from `2026-10-08T09:19:45.969901+00:00` through
`2026-10-08T09:19:46.517249+00:00`, commands, source hashes, test hashes,
and output hashes. Each fixture closes its owned HTTP listener and child.

## Replay and receiving scope

From this packet directory:

```sh
UMA_MCP_REVIEW_SOURCE="$PWD/source/mcp-stdio.js" node --test tests/report-format-receiving.test.mjs tests/tool-string-payload-receiving.test.mjs
UMA_MCP_REVIEW_SOURCE="$PWD/previous-source/mcp-stdio.js" node --test tests/tool-string-payload-receiving.test.mjs
```

The second command has the recorded failing exit status on the predecessor.

`MANIFEST.json` inventories the exact packet bytes.
`../RECEIVING_INDEX.md` maps all retained receiving stages to their
source pins. The final peer Git bundle includes the full receiving history
with no prerequisite bundle or commit required.

The author's assembled test run and publication remain separate steps.
This receipt qualifies the bounded payload amendment. It does not repeat
or replace the earlier native, concurrency, global-content, or request-ID
evidence.

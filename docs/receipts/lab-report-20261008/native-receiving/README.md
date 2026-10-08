# Native comparison report receiving

> Historical renderer-only receiving. Its six-check UI composition used an
> alternate clear-on-selection UI, which was superseded when PR65 landed.
> Current accepted-main composition and replay commands are documented in
> `docs/receipts/lab-ended-branch-20261008/native-receiving/README.md`.
> The original native UI harness is preserved unchanged in
> `historical/verify-lab-native-ui.mjs`; the canonical command now dispatches to
> the accepted PR65 receiver. These historical measurements remain intact.

This receipt checks the report that a user actually downloads from `uma-sim-api`.
The receiver creates branches through the real API, downloads the Markdown and
JSON attachments in Chromium, parses the saved Markdown bytes with marked in GFM
mode, and examines the resulting visible heading and table cells in Chromium.

**The repaired renderer passes all nine receiving cases and all 93 assertions.**
The original passes one case and 74 assertions. An additional smoke of the built
career-lab UI against the actual repaired native API passes all six checks,
including exact native-response-to-browser-download byte equality.

The accepted renderer source SHA-256 is
`d214179b4389ccaf2b3ac0992c97364b86ea8a2ffac9762daa5086174ea1b7d4`.
It was independently copied into a checkout of the original base, verified
before and after copying, built with the locked offline dependencies, and frozen
as executable SHA-256
`4c9ad36a01fe9d3a0abefc0dea237dd5e7a84288bd36c62de0b54c7a33b425a7`.
Both receiving runs use identical receiver bytes:
`a046b0e4eb95929c2eb96c57c64cfb0a1cf067f522b21215e4fd7e7b4cb20fff`.

| Receiving case | Original | Repaired |
| --- | --- | --- |
| Ordinary branch names | Pass | Pass |
| Pipe inside a name | Fail | Pass |
| Literal Markdown and backticks | Fail | Pass |
| Literal HTML text | Fail | Pass |
| CRLF, CR, and multiple LF breaks | Fail | Pass |
| Backslashes, pipes, and backtick runs | Fail | Pass |
| Unicode name with a pipe | Fail | Pass |
| Literal entity spelling | Fail | Pass |
| Authored persisted metadata and outcome strings | Fail | Pass |

The downloaded JSON is deeply equivalent across all nine original/repaired
pairs after excluding only the fresh branch IDs and comparison timestamp. Each
run separately verifies those exact IDs against its own API-created branches.
All original names, timeline values, outcomes, and caveats remain equal; the
details and canonical hashes are in `json-equivalence.json`.

## Original defect

The original source at commit `f5f9b29393731d18aee2d66a31d89c315aa79c60`
accepts the branch name `Rest | train`. Its JSON report retains that name and the
ordered comparison IDs. Its Markdown report inserts the raw name into a table:

```markdown
| Branch | Rest | train | Plain control |
```

The receiving parser renders A as `Rest` and B as `train`; it loses the actual B
name `Plain control`. Embedded line breaks produce extra headings and destroy
the final-outcome table. Markdown syntax, HTML, backticks, and entity syntax can
also change a name's visible meaning.

The final baseline receiver run preserves one ordinary-name passing control,
seven failing adversarial name cases, and one failing persisted-metadata case.
All 18 attachments really download with HTTP
200 and the expected content type, filename, and content length. The failures
occur when the downloaded Markdown is read as a report.

## Receiving method

The receiver starts the supplied native executable on an ephemeral loopback port
with a fresh filesystem state directory. `UMA_REPO_ROOT` points at the supplied
repository, and `UMA_POLICY_CMD` is removed from the child environment. A career
uses seed 4242, scenario `ura`, trainee `Special Week`, the stub race model, and
the built-in bot. The public library API saves a checkpoint named
`report-receiving`. For each case, the public branch API runs two actions from
that checkpoint and admits the requested branch names.

The first eight cases change names only through the API. The ninth case is an
explicitly authored persisted-input extension: after generating two real
branches, the receiver changes string fields only in those two disposable local
branch files. It preserves the original and authored bytes. The altered fields
exercise checkpoint and action metadata, dynamic code-span fences, action
labels, date labels, moods, race names, learned skills, and sparks. The actual
API then loads those files and generates the report. This case is not presented
as naturally generated simulation content.

Chromium clicks links to the actual `/v1/lab/report` endpoints and saves both
attachments. The receiver checks the ordered IDs and original names in the JSON
attachment, hashes both saved files, checks their UTF-8 round trip, and records
the actual response headers. The Markdown attachment is parsed by marked, then
rendered in Chromium with a simple receipt stylesheet. The receiver checks:

- One comparison heading containing both original names as literal data.
- The four existing report sections and two existing tables.
- Eight cells per timeline row and three cells per final-outcome row.
- The complete twelve-row outcome table and exact ordered Branch cell text.
- Numeric, boolean, energy, and mood cells against the downloaded JSON.
- Code-span metadata against its original characters, with line breaks converted
  to spaces according to the renderer's documented code-span contract.
- The authored divergence's four list items and timeline/race/skill/spark cells.
- No browser page errors or requests beyond the native loopback origin.
- Identical source and executable hashes before and after receiving.

CRLF and CR are normalized to LF when comparing visible line breaks in prose or
table cells. No HTTP report payload or simulator response is replaced. The
ninth case's explicit persisted string edits are the only authored input beyond
normal branch API requests.
The HTML files contain a restrictive content policy so the baseline's embedded
HTML cannot request resources or run scripts during inspection.

This check uses local native simulation and local browser receiving. It does not
exercise a deployed estate service, external policies, or production race data.
The generated state directory is disposable and is not included in the receipt;
the exact branch-creation inputs are recorded in each full run receipt.

## Reproduction

Build the original or repaired native API from its pinned checkout:

```sh
cargo build --locked --offline -p uma-sim-core --bin uma-sim-api
```

Run the same receiver for each binary and its source checkout, using a new output
directory each time. The receiver never installs dependencies:

```sh
PLAYWRIGHT_MODULE=/absolute/path/to/playwright-core \
MARKED_MODULE=/absolute/path/to/marked/lib/marked.esm.js \
CHROMIUM_EXECUTABLE=/absolute/path/to/chromium \
node scripts/verify-lab-report-receiving.mjs \
  /absolute/path/to/uma-sim-api /absolute/path/to/uma-sim /absolute/new/output
```

Exit status 1 represents a rejected run; exit status 0 means every planned case
passed and both the source and executable remained unchanged. Each case records
individual failed assertions instead of stopping after the first defect.

The recorded runtime is Rust and Cargo 1.99.0, Chromium 153.0.8010.0,
Playwright Core 1.62.1, and marked 17.0.5. The original API executable was built
with the locked offline Cargo registry and has SHA-256
`d3f4fa2f4266f99260e160effb34971f2cb5e2348d8b36a03286e903a5e5d198`.
Every receipt includes the exact source hashes, executable hash, browser hash,
parser module hash, and receiver hash used for that run.

## Evidence layout

Each run directory contains `receipt.json` and the native server log. Each case
directory contains the admitted branch summaries, actual Markdown and JSON
downloads, rendered HTML, visible DOM text/cells, a full-page screenshot, and the
case assertions. These artifacts retain the original failure as evidence.

`baseline/` and `candidate/` hold the nine-case runs. `candidate-build.json`
records the independent source-to-binary build and its completion result.
`manifest.json` hashes the receiving scripts and every file in this receipt.
The original Markdown attachments intentionally retain CR/CRLF bytes. Scoped
Git attributes preserve those exact downloads and recognize CR line endings in
whitespace checks; the failing report bytes are not cleaned up or normalized.

## Actual native-backed UI composition

`scripts/verify-lab-native-ui.mjs` serves the actual compiled UI and transparently
proxies its `/v1/` requests to the actual native server on a second loopback
port. This matches the repository's Vite development topology. Responses are
forwarded byte-for-byte; the smoke does not replace an API response or edit
persisted branch data.

The browser creates two branches through the rendered controls:
`Rest | **early**` and `Control <em>steady</em> &amp;`. It selects A and B,
compares them, downloads both reports through the rendered links, and verifies
the saved files against the exact native response hashes. The repaired Markdown
then renders the same literal names in its heading and final Branch row.
Selecting the same branch for A and B clears the obsolete comparison/report
controls and disables the invalid comparison.

The smoke's six checks pass with the repaired renderer. With the repaired UI and
the original native renderer, the first three checks pass and the Markdown
receiving assertion rejects the lost literal name content. Both runs observe
15 proxied native API responses plus two successful setup requests and no
browser page errors. The real catalog
bootstrap attempts 133 public portrait image requests; the receiver blocks and
records all of them. No unexpected external requests occur. This qualifies the
local UI/native composition; it is not an embedded release build or deployment
claim.

The compiled UI pins are:

- `src/state/labStore.ts`:
  `235fa75f0758b2bacc4b56bb188a54f6cbedc61df95ab36e0085f4903f5e2f28`.
- `src/components/ComparePanel.tsx`:
  `ba6e9ca851fda7c1d31b1299e476730d7f2a5ebf51fa5e4f88b0e425591264b6`.

The complete built-file hashes, native request traces, screenshots, downloaded
bytes, and received name cells are in `integrated/baseline/` and
`integrated/candidate/`. To replay:

```sh
PLAYWRIGHT_MODULE=/absolute/path/to/playwright-core \
MARKED_MODULE=/absolute/path/to/marked/lib/marked.esm.js \
CHROMIUM_EXECUTABLE=/absolute/path/to/chromium \
node scripts/verify-lab-native-ui.mjs \
  /absolute/path/to/uma-sim-api /absolute/path/to/uma-sim \
  /absolute/path/to/uma-sim-ui /absolute/new/output
```

## Receiver development note

An earlier scratch-only receiver invocation used an incorrect expected outcome
row count of thirteen. That oracle was corrected to the source's twelve rows
before the preserved baseline comparison. The preserved baseline uses the final
receiver and is the authoritative negative run.

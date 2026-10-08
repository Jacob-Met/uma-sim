# uma-sim-ui

Browser SPA for interactive `uma-sim` careers (Vite + React + TypeScript).

## Scripts

```bash
npm ci
npm test
npm run dev        # http://localhost:5173 — proxies /v1 -> :8765
npm run typecheck
npm run build      # writes dist/ (embedded by Rust with --features embed-ui)
npx playwright install chromium
npm run test:browser # built app, local HTTP fixtures, real Chromium downloads
npm run test:trace # recorded override flags, full JSON, delayed reads and deletion
```

Requires `uma-sim serve` (or `uma-sim-api`) on port 8765 for API calls.

See the root [README](../../README.md) for release / embed instructions.

## Reviewing career-lab experiments

The branch form validates the complete draft before it can send a run request.
Use a whole-number action limit from 1 to 500, and one `turn:actionId` override
per line. Blank lines and surrounding whitespace are allowed. Turns must be
nonnegative integers that fit the engine's signed 32-bit representation;
duplicate turns, missing fields and malformed lines show their source line
numbers. No part of an invalid draft is submitted. The review lists every
accepted override in authored order. The simulator still decides whether an
action is legal at the requested turn and records rejected choices in its trace.

The A/B radio buttons select the next comparison to request. The displayed
result and its Markdown/JSON downloads retain the IDs of the comparison that
actually returned, even when a new selection is made or a later request fails.
A completed comparison does not overwrite a selection edited while it was
pending. Deleting a branch clears a displayed comparison involving that branch
and clears only its selected IDs; deleting an unrelated branch preserves the
result. This takes effect as soon as deletion is acknowledged, including if
refreshing the branch list subsequently fails.

Run `npm test` for the native draft-validation regressions, `npm run typecheck`
for static checks and `npm run build` for the production bundle. The browser
receiver uses pinned Playwright as a development dependency. It serves the built
app and explicit API fixtures on a temporary loopback port, verifies actual
Markdown/JSON downloads, and writes its receipt under `test-results/career-lab/`.
It does not require a running simulator. An existing compatible Chromium can be
selected with `npm run test:browser -- /absolute/path/to/chromium`.

The existing UI CI job runs the validation, comparison and trace regressions. Linux environments that
need browser system libraries can use `npx playwright install --with-deps chromium`.
The production application has no additional runtime dependencies.

## Inspecting saved branch traces

Choose **Inspect trace** beside a saved branch to read its existing recorded
steps. The inspector displays the authored overrides and the per-step applied
and rejected flags separately. Multiple phases may share one turn; a configured
turn with no recorded step has no observed override outcome. The step table
shows the recorded action, with an option to include steps that have no override
flags. Older duplicate overrides retain the simulator's first-entry precedence.

**Download full trace JSON** exports the full response displayed in the inspector,
including fields not rendered in its tables. Changing the inspected branch,
closing the panel or deleting the branch invalidates an earlier pending read.
Read failures retain the requested branch ID and offer a retry. Inspection does
not change the active career or the A/B comparison selection.

## Reading a career

The **Career event log** shows entries retained in the current career snapshot,
including recorded training, event choices and race outcomes. Numbers show entry
positions. The engine records selected outcomes; this is not a complete transcript
of every action or narration. **Current state summary** expands the existing
summary of the current turn, stats and scenario resources.

Use **Search retained events** to find literal text without case sensitivity.
Matching entries keep their original positions and wording. The status reports
both matching entries and individual occurrences. Repeated entries stay separate.

Scrolling back or searching pauses automatic following. **Follow latest** returns
to the newest retained entry. When a search is active, the button explicitly clears
the search before following. Loading another retained history replaces the displayed
entries; the reader does not create an additional history store.

## Log reader checks

After `npm ci`, run `npm test` with Node 20 or newer. The shared test command
includes the log reader and career-lab regressions. For the log reader alone,
run `node --test test/log-view.test.mjs`.
This uses the existing TypeScript compiler and needs no additional dependencies.
`npm run typecheck` and `npm run build` also cover the receiving React component.

The optional browser check uses a built UI, Node 22+, an installed Chromium and the
real `uma-sim-api` binary. From the repository root:

```bash
cargo build --locked -p uma-sim-core --bin uma-sim-api
UMA_LOG_CHROMIUM=chromium node packages/uma-sim-ui/test/career-log-browser.mjs
```

The check starts isolated loopback servers, its own browser profile and temporary
career/checkpoint state, then removes its state after finishing. Set
`UMA_LOG_API_BINARY`, `UMA_LOG_PROFILE_ROOT`, `UMA_LOG_STATE_ROOT` or
`UMA_LOG_EVIDENCE_DIR` for an alternate binary, writable browser profile parent,
temporary career state parent or evidence folder. It never connects to
an existing uma-sim server. See [the receiving record](../../verification/career-log/README.md)
for the exact source baseline, comparison and browser results.

### Download retained career events

The Career event log includes **Download all N entries**. Choose **Plain text**
for a numbered, readable transcript or **JSON** for the exact retained string
array. Download always includes all entries currently retained by the displayed
career, even while search shows only matching entries. It captures those entries
at the click; later career changes do not change that prepared file.

The file represents retained history and can omit events already discarded by the
engine. Entry numbers are positions in the exported sequence. It contains no
restorable career state or inferred run/session metadata. The JSON format is
`uma-sim-retained-log/1`, with `scope: "displayed-retained-history"`, `entryCount`
and `entries`; duplicates, empty strings, order and original string contents
survive JSON parsing. Plain text keeps readable numbered blocks; JSON is the
lossless choice for embedded control characters or exact entry boundaries.

Downloads use the displayed data locally and issue no API request. The browser
handles file saving; the app does not claim that a requested download was saved.
An empty retained log disables the action. A preparation failure leaves an inline
error and can be retried explicitly.

Run the focused format, download-boundary and native React rendering tests with:

```sh
node --test test/log-export.test.mjs
```

## Reuse a career setup

The **New career** form can download and reopen an editable setup file.

1. Choose the scenario, trainee, seed, speed, dialogue, race model and policy.
   Select up to six deck supports and configure inheritance as desired.
2. Choose **Download setup** to request a local `uma-sim-setup-v1.json` file.
3. In a new form or browser session, choose **Open setup** and select that file.
   Review the seed, choices, ordered deck and inheritance details.
4. Choose **Replace setup** to replace all setup choices in the form. **Cancel**
   keeps the current form. Replacement clears the two search filters so the
   loaded selections can be inspected.
5. Edit any choice, then explicitly choose **Start run**. The run receives the
   form's selected policy, and the run toolbar starts with that same policy.

Opening, reviewing, downloading and replacing a setup make no simulator API
request and do not start a career. Setup files include disabled inheritance
choices, all six ancestors and thirty factor/star slots, and the retained
compatibility score. Enable inheritance again to inspect or use those choices.
To resume an in-progress career, use the **Career library** checkpoint controls.

The seed is stored and submitted as decimal text so the full signed 64-bit
range, from `-9223372036854775808` through `9223372036854775807`, survives
without JavaScript number rounding. Numeric JSON seeds, fractions, exponent
notation and out-of-range seeds are refused. The form also checks the seed
before starting a run.

Version 1 files are UTF-8 JSON of at most 64 KiB, with an exact
`{"schema":"uma-sim.run-setup","version":1,"setup":{...}}` envelope.
All form fields are required. Missing, additional, unsupported or malformed
values are refused as a whole. The current catalog must still contain each
scenario, trainee name, support ID, ancestor name and factor ID in its proper
category. No unavailable selection is silently replaced. The current
trainee/support and enabled direct-parent exclusions also apply. A setup
records choices against the available catalog; later catalog or engine changes
can change a simulated outcome.

A form edit, changed available choices, a busy transition, a newer file
selection or leaving the form invalidates an outstanding file read or review.
Open the file again to review it against the current form. File read failures
and refused files leave all current choices intact. This is an explicit local
download/open workflow; closing a form does not automatically save its choices.

Run the focused file-admission tests with
`node --test tests/run-setup-file.test.mjs`, or use the existing `npm test`
command for all maintained UI tests.

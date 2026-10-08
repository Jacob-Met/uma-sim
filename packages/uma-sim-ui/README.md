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


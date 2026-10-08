# Career-lab experiment and comparison integrity

Contribution: `estate-7e7ad6f8e5bd`, [issue #64](https://github.com/Jacob-Met/uma-sim/issues/64).
Historical discovery baseline: `f5f9b29393731d18aee2d66a31d89c315aa79c60`.
UI counterexample baseline: `0ac14602addd1a4610aa8359896915a34cce5659`.
The initial publication is composed on `5d32d0b06de88141ecd4c84922e77553a45a522f`,
preserving the later checkpoint import label, native portability receiver,
named terminal sessions and their CI regression gate.

[PR #65](https://github.com/Jacob-Met/uma-sim/pull/65), by
`estate-49f845d0dece`, landed the comparison/report identity correction before
this contribution was published. Its production repair, native-API browser
receiver and evidence are retained. The overlapping unpushed repair was
reconciled with that commit. This contribution supplies the remaining complete
draft validation, caught deletion-refresh failure/local row removal, and an
inspector for the branch trace already produced by the simulator.

The browser previously displayed one comparison while its report links could
download the newly selected branches. Deleting a displayed branch consulted
the draft selection, and a delayed comparison could overwrite later keyboard
selections. The override parser silently removed invalid lines and accepted
turn values that the native API could discard or cast to a different turn.

The existing TypeScript UI now validates the entire authored experiment before
submission. It identifies malformed and duplicate overrides by source line,
preserves valid action text and order, and checks the existing action limit and
signed 32-bit turn representation. Downloads use the IDs in the displayed
comparison. Successful deletion clears matching results and selections before
the fallible list refresh; request and branch revisions prevent obsolete
comparison results from being restored.

## Receiving evidence

An independent reviewer froze a browser receiver against the original app. Its
final acceptance rule allows either disabling invalid submission or handling a
click with visible validation, while requiring zero branch-creation requests.
That same receiver produced:

| Receiving source | Passing cases | Failing cases |
| --- | ---: | ---: |
| Original pinned UI | 5 | 18 |
| Current main after PR #65 | 11 | 12 |
| Qualified input/error candidate | 23 | 0 |

`canonical-main.json` records the residual counterexamples at the receiving
base: seven malformed override inputs and four invalid action limits still
reach the mutation API, while a confirmed deletion followed by a list-refresh
503 produces an unhandled error. Comparison invalidation itself already passes
in PR #65 and is not claimed as this contribution's improvement.

`baseline.json` and `candidate.json` retain the actual requests, download
identities, failures and valid controls. Receiver SHA-256:
`71774c18bfd7115047f5c2d0d3d3435743f7cddd188a4779a331b35b7fbf2e1d`.
Browser: Chromium `153.0.8010.0`; Playwright `1.62.1`; Node `v24.19.0`.

The checks execute the built React app against explicit local HTTP fixtures,
including actual Markdown and JSON downloads. They verify UI and request
behavior; they do not claim a simulator run, installed deployment or measured
user benefit. The independent root receiving copy also completed a clean
`npm ci`, 29 native validator regressions, production build and the same 23
browser cases using the pinned development dependency.

## Reproduce

From `packages/uma-sim-ui`:

```sh
npm ci
npm test
npm run typecheck
npm run build
npx playwright install --with-deps chromium
npm run test:browser
npm run test:trace
```

The existing UI CI job runs these gates. The comparison browser receiver is
`tests/receiving/uma-lab-receiver.mjs`; it also accepts a different built `dist`
directory and output directory to reproduce the baseline negative controls.
The production app has no new runtime dependencies. The work leaves the
concurrent session-targeting contribution in issue #63 and held API-test PR
#54 unchanged.

## Recorded branch trace inspector

Saved branch rows now expose the existing `labBranchGet` response in the actual
Library and lab tab. The inspector shows authored overrides, recorded action
choices, per-step applied/rejected flags, unobserved configured turns, and the
full displayed response as a JSON download. The native producer checks the
first configured override for a turn on each step; multiple phases may share
a turn. Flags are mutually exclusive within a native step but may differ
between steps at one turn. The display does not claim that a selected action
necessarily succeeded: the producer records its choice while ignoring the
`engine.step` return value.

The read lifecycle binds loading, errors, retry, late success and full-response
export to the requested branch. Closing, unmounting, switching branches and an
acknowledged deletion retire prior reads. Failed deletion preserves the view.
Native producer source and root wiring received a separate source-contract
review, retained in `native-trace-contract-review.md`.

Nine additional actual-component rendering checks qualify the trace display
(38 native Node tests total). The independent built-application trace receiver
passes all 15 cases, and all 23 comparison/input receiver cases still pass on
the composed UI. `trace-candidate.json` and `trace-composition.json` preserve
those receiving results. Exact final source/build and CI pins are recorded
with the PR; source/fixture verification does not imply installed deployment.

The first GitHub claim attempt was rejected by a secondary content-creation
rate limit at `2026-10-08T08:10:22Z`. Isolated work continued, the single retry
after backoff succeeded as issue #64, and its body was read back. Native
`hamon-ie` discovery did not return; no resident identity or goal claim was
inferred. This record preserves source qualification; the PR and CI receipts
establish subsequent integration status.

# Native shorter-branch note and report composition receiving

## Defect and repaired behavior

An actual two-action branch compared with an otherwise identical three-action branch reported that the shorter branch ended after **three** steps. Reversing the ordered pair produced the same contradiction for branch B. The timeline and final-outcome table already showed the correct counts; the first-divergence note selected the continuing branch's length.

The production correction swaps only the two mistaken length references in `compare_branches`. It was composed onto the independently qualified Markdown renderer, which preserves report strings as literal text and keeps embedded line breaks within their intended block or table cell. No API schema, simulation decision, score, timeline, branch identity, or accepted UI behavior is changed by the two native repairs.

## Exact source and executable pins

The first receiving baseline is the renderer-only candidate, so the shorter-branch failure is isolated from Markdown formatting. Its `career_lab.rs` SHA256 is `d214179b4389ccaf2b3ac0992c97364b86ea8a2ffac9762daa5086174ea1b7d4`; executable SHA256 is `4c9ad36a01fe9d3a0abefc0dea237dd5e7a84288bd36c62de0b54c7a33b425a7`.

The count patch is production commit `22ad948351b4aad1e3c08fa7d4220188370581a0`, following its tests at `c20aa7b36547edc3178a296816a040e57e4b52f9`. Applying only that two-line source patch to the renderer produces `career_lab.rs` SHA256 **`49d68441482fa907364292bb25b43fc06be6e95b13937cbbba898a393f4e4935`**. The composed native executable SHA256 is **`149600d040481e6eb2530c2a3081d1c680f223d7a16d6bcd54feb247ab50579d`**. `ended-count-source.json`, `ended-count-source.patch`, and `composed-build.json` record the exact composition and locked, offline build.

The early isolated receiving checkouts retain base HEAD `f5f9b29393731d18aee2d66a31d89c315aa79c60` plus the declared source patches. Their HEAD alone is not a claim that the modified native source was committed there. The before/after file hashes and executable hashes identify the actual tested product.

## Independent native receiving

`scripts/verify-lab-ended-branch.mjs`, frozen at SHA256 `88d300ecf8f462d8c1229623d9a4169f1d0fee626b2e21c93dcf5178b6512d49`, starts the real Rust API in a fresh disposable state directory. It starts a seeded run using the built-in bot and stub race model, saves a checkpoint, and creates branches with actual lengths two, three, and two through the branch API. There are no authored HTTP responses or persisted-store edits in these cases.

The receiver compares both shorter-branch directions and an equal-length control. For each, it receives the comparison JSON and actual JSON/Markdown attachment responses, saves their bytes, verifies headers, lengths and ordered identities, and renders the saved Markdown using marked GFM and Chromium. It checks the first-divergence note against the actual shorter branch and independently checks the final-outcome Steps cells.

| Case | Renderer-only baseline | Renderer plus count correction |
| --- | --- | --- |
| A has 2 steps, B has 3 | Wrong note: A ended after 3 | Correct note: A ended after 2 |
| A has 3 steps, B has 2 | Wrong note: B ended after 3 | Correct note: B ended after 2 |
| Both have 2 steps | No divergence; passes | No divergence; passes |
| Total | 1/3 cases; 12/18 checks | 3/3 cases; 18/18 checks |

`semantic-delta.json` verifies that the received comparisons and JSON reports differ only in the intended first-divergence note after excluding freshly assigned branch IDs (`aId`, `bId`) and comparison timestamps (`comparedAt`). Every other JSON field remains deeply equal. All requests succeeded, and the Chromium report consumer recorded no page errors or external requests. Source and executable hashes remained unchanged through each run. The baseline and candidate screenshots visibly corroborate the numerical contradiction and its correction.

The unchanged nine-case report receiver was also replayed on this same composed source and executable: **9/9 cases, 93/93 checks**. The first eight cases use naturally API-created names containing pipes, Markdown delimiters, HTML-like text, literal entities, Unicode and line breaks. The ninth explicitly authors string fields in disposable persisted branch results, then receives them through the real API; it covers checkpoint/code-span metadata, action fields and outcome strings that are not naturally user-editable branch names. This fixture boundary is declared in its receipt. `composition/report/` contains that final native report replay.

## Accepted UI integration

The alternate clear-on-selection UI was superseded after a fresh shared-tip check found PR65 accepted on main `0ac14602addd1a4610aa8359896915a34cce5659`. The native fixes preserve PR65's behavior: the displayed A/B comparison and its downloads remain valid while a user drafts another pair. Historical alternate-UI receipts are not final product evidence for this branch.

The PR65 UI source, build and receiving receipts are recorded separately under `composition/accepted-ui/`. These runs use immutable PR65 commit **`1cf2fc6e5e82aefa9d940c8a873c17fc97f9a875`**, plus the declared native source patch. Its `labStore.ts` SHA256 is `aeccd1b983ff988f185b2bc5ce1cfffcbf1f9d7020929fce4e6117b0e21a926a`; `ComparePanel.tsx` SHA256 is `5d1497dd64f0e43fb751bd9d2f1e46d94f2842da8a90421d78a56d669fdc853a`. An active successor working tree was observed and deliberately not copied. The post-build tracked files and all three built assets remained unchanged through receiving.

**Accepted-main equality is confirmed.** The complete committed trees of PR65 `1cf2fc6` and main `0ac14602addd1a4610aa8359896915a34cce5659` both equal `e7d10088a713d8fbd7391c86050a92898652bdd6`. Every one of the 32 committed UI files matches by SHA256, and those files also match the native integration base `f6bbea9f1480a1ed6db2a7b5f974743e94bc2e2f` (tree `253bc986f9b78bcb03beb0f589f964ca61712d11`). The integration base's native source, API and Cargo lockfile match the received native composition. `accepted-main-equality.json` records every comparison.

The build regenerated one tracked TypeScript incremental cache, `tsconfig.tsbuildinfo`. Its committed and post-build hashes are recorded separately; all 31 other tracked UI files, including runtime source, configuration and lockfiles, match accepted main exactly. This expected build output is not an implementation change. The recorded UI and native receiving can therefore be joined to accepted main by concrete source equality without repeating an identical artifact test.

The PR65 native interaction sequence passes **9/9 cases**: save checkpoint, fork, close/resume, run/compare/download, JSON and Markdown binding after drafting another pair, delayed-response draft preservation, draft-only deletion retention, and displayed-branch deletion invalidation. The downloaded Markdown is parsed with marked and Chromium instead of searching raw escaped source. The original nine interactions remain unchanged; the original committed script and exact adaptation diff are preserved.

The report-specific supplement passes **6/6 checks** using adversarial branch names, real browser downloads and an unmodified loopback proxy. It checks native response-byte hashes against browser-saved bytes, exact JSON identities, literal rendered heading and Branch cells, and another actual JSON download after the user changes the draft to A/A while the displayed report remains A/B. All 16 proxied responses succeeded. The browser recorded 133 blocked catalog portrait requests, zero unexpected external requests and zero page errors; native source, UI source, built assets and binary stayed unchanged.

The first supplemental receiver passed its four report checks, then failed an unrelated expectation copied from the superseded UI: that the same-pair Compare button would be disabled. PR65 performs its pair validation when clicked. The corrected receiver verifies the actual selected draft and preserved displayed report/download without imposing that alternate behavior. The first receipt and its exact initial harness are preserved under `adversarial-initial-oracle/` and `adversarial-initial-oracle.mjs`; `harness-provenance.json` explains the bounded oracle correction. No product source changed between those runs.

## Receiving boundary and replay

The tested topology is a production Vite build served locally with a transparent `/v1/` proxy to the actual Rust executable, matching the repository's existing development topology. This is not an embedded release binary, deployed service, external policy run, or performance benchmark. Public catalog portrait images are blocked and recorded during UI receiving; the test does not depend on them.

Use a fresh output directory for every replay. The runtimes used here were Rust/Cargo 1.99.0, Chromium 153.0.8010.0, Playwright Core 1.62.1 and marked 17.0.5. Set `PLAYWRIGHT_MODULE`, `MARKED_MODULE`, and `CHROMIUM_EXECUTABLE` to available installations before invoking the native receivers:

```sh
node scripts/verify-lab-ended-branch.mjs API_BINARY REPO_ROOT OUTPUT_DIR
node scripts/verify-lab-report-receiving.mjs API_BINARY REPO_ROOT OUTPUT_DIR
node scripts/verify-lab-native-ui.mjs API_BINARY REPO_ROOT UI_ROOT OUTPUT_DIR
```

The canonical `verify-lab-native-ui.mjs` command now dispatches to the tested `verify-lab-accepted-ui.mjs` receiver. Its historical clear-on-selection implementation is preserved byte-for-byte under the earlier report receipt's `historical/` directory. The earlier manifest points to that preserved file; the old receiving measurements and hashes remain intact.

The PR65-derived receiver uses `UMA_LAB_REPO`, `UMA_LAB_API_BINARY`, `UMA_LAB_OUTPUT_ROOT`, `UMA_LAB_PLAYWRIGHT_MODULE`, `UMA_LAB_CHROMIUM_EXECUTABLE` and `LAB_STAGE` environment variables, plus `MARKED_MODULE`. Its interaction cases are retained; the Markdown assertion consumes parsed report text so legal escaping of literal branch names does not cause a false failure. The original script pin and exact adaptation are included with the final composition metadata. The existing `packages/uma-sim-ui/tests/lab-comparison.browser.mjs` entry point now uses that same parsed-text oracle, retaining its original repository-path default and nine interactions. Provide an installed marked ESM module through `MARKED_MODULE`, alongside the existing explicit browser runtime. No package manifest or runtime UI source is changed.

`manifest.json` records byte counts and SHA256 hashes for the committed evidence and independent harnesses, excluding the manifest itself. Disposable runtime-state directories and reproducible build dependencies are not deliverable evidence. Original negative results remain intact.

# Retained race results: author receiving

The Race panel now reads the loaded career snapshot's retained log. It presents
observed physics results and earlier occurrences, preserves the original text,
and leaves missing or unrecognized fields unknown. The existing mandatory race
action is unchanged. Source ownership is [Uma #86](https://github.com/Jacob-Met/uma-sim/issues/86),
with the adjacent App seam recorded in [#63](https://github.com/Jacob-Met/uma-sim/issues/63#issuecomment-6059689224).

## Source and scope

The production freeze is **ab287fed5e5f30b2b999c31c054b234ca1211461**, tree
**5738dfd755b6bb6c7b328dd142f55d1e7ac7bf5a**, directly on native baseline
**4dc00f155d29fb37887524455f0627399191c915**. The seven source paths and exact
hashes are in `source-freeze.json`. That record was frozen before browser review;
the final author results are in `author-receipt.json` and the reports below.

The sole App change supplies `cs.log` to RacePanel. Its Event Log prop remains
unchanged. The existing package test entries are preserved, with the new race
checks appended. RacePanel, its small parser and stylesheet, a native regression
file, and its README section complete the source scope. No engine, API, session,
career-lab, catalog, training-demo, lockfile, storage or deployment change is part
of this contribution. Fresh-main composition and independent acceptance are
recorded separately by the receiving owner; these original pins remain historical.

The original pre-writing contract was on a3e04846521c8877d112842332bc4825a29edaf3.
`contract-current-main.json` records the move to 4dc00f and preservation of the
merged Event Log. Both contracts are retained unchanged.

## Established baseline and result

On the unchanged native API and built App, a real seed29 physics career retained:

```text
Race debut 9th +321 fans [physics t=75.400s course=10601 seed=230167224 field=9 margin_win=7.867s margin_ahead=0.000s]
```

After the race, the App displayed no Race panel. Passing the correct retained line
to the old parser still returned no result. A separate actual stub career emitted
`Race debut +1072 fans`, which the old parser displayed as first place even though
that record contains no placing. `baseline-native/baseline-witness.json` binds
these observations to the exact native binary, browser transitions and source.

The candidate passes all **61 UI tests**, TypeScript checking and the Vite
production build. The unchanged baseline passed its 48 incumbent tests and both
build gates. The seven browser groups passed against the real native API:

1. The actual mandatory entry is disabled while its native response is pending;
   the resulting place, field, time, fan gain and original record match the API.
2. Three real race occurrences include repeated optional race IDs and retain the
   exact chronological identities in newest-first history.
3. An offered ordinary Rest changes career state while preserving those results.
4. Enter and Space open the native disclosures without an action POST or any
   native state mutation.
5. At a 390px viewport, all facts and expanded original records fit without
   horizontal overflow, including the visible keyboard focus indication.
6. Starting a real stub career replaces the prior history and shows only its
   recorded fan gain, with placing, finish time and field size unrecorded.
7. Starting a subsequent empty career removes the previous results completely.

The final browser report is `candidate-browser-r2/browser-receipt.json`. It also
contains the actual snapshots, request transcript, source hashes, blocked external
catalog-image URLs and absence of browser exceptions. All tested source bytes
remained unchanged. The original r1 report is preserved in the complete author
archive. The final helper adds an observed checkout identity, so replays do not
mislabel a later composition as the original author commit; the seven behavior
groups were rerun to bind those final helper bytes.

## Parser and interpretation boundaries

Only the retained `engine::do_race` formats are interpreted. Physics values must
be finite and nonnegative, with positive finish time, a valid ordinal and a
placing within a positive field size. Integer values must be exactly representable;
the recorded course and seed must fit the native unsigned 32-bit fields. The
parser preserves finish-time precision and the complete original string.

Fan-only records do not identify their internal backend or supply a placing.
Unrecognized or malformed Race records remain literal records, including as the
newest occurrence. Older recognized results are not promoted past them. Repeated
IDs and identical text stay separate by their retained log positions. These are
selected retained outcomes; the component does not promise a complete career
archive or derive additional race facts from current settings or fan gain.

The native API binary was built independently from all 646 exact 4dc00f blobs,
using the unchanged Cargo lock and offline cached dependencies. Its SHA256 and
build/source manifests are retained. No embedded UI was compiled into that binary;
the browser loaded this contribution's real Vite production bundle through a
transparent loopback proxy. All careers, state, ports and browser contexts were
created for the receiving process. This record does not claim a live user career,
external policy service, account, or a different platform was exercised.

## Reproduce the primary check

From an actual repository checkout with Node, the declared npm dependencies,
Rust and Chromium available:

```bash
cd packages/uma-sim-ui
npm ci
npm test
npm run typecheck
npm run build
cd ../..
cargo build --locked -p uma-sim-core --bin uma-sim-api
UMA_RACE_TMP=$(mktemp -d)
TMPDIR="$UMA_RACE_TMP" UMA_RACE_CHROMIUM=/absolute/path/to/chromium \
  node docs/receiving/race-results-afe225d6c6be/receive-race-results.mjs \
  "$PWD" "$PWD/target/debug/uma-sim-api" "$UMA_RACE_TMP/results"
```

The helper's three positional arguments are the repository root, an executable
native API path, and an output directory. The root must contain the built
`packages/uma-sim-ui/dist`, installed declared UI dependencies, and the unchanged
research/knowledge catalogs used by the API. `TMPDIR` must be writable; the helper
starts its own API on an ephemeral loopback port and never connects to an existing
server. `UMA_RACE_CHROMIUM` selects the installed Chromium executable. Reports,
source hashes and screenshots go into the requested output directory.

For the original negative witness, materialize **4dc00f**, install or reuse its
unchanged declared UI dependencies, build its UI, and run `reproduce-baseline.mjs`
with the same three arguments. That helper intentionally expects the inherited
parser and missing panel. It is not a candidate acceptance test.

For this estate receiving run, no dependency install was needed. `environment.json`
and `ui-dependencies.json` identify the read-only package donor, all matched lock
versions and package-manifest hashes. Each reused package was linked into an owned
node_modules root, leaving caches and outputs in the private receiving directory.
Playwright and playwright-core 1.62.1 came from the existing primary Node runtime.
The repository-tracked TypeScript incremental cache changed during `npm run build`;
its generated diff is retained, and its original bytes were restored before the
clean seven-file source freeze. Production tests were not changed to accommodate it.

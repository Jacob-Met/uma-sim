# Career conditions and simulator titles

The Run screen shows **Conditions & titles** immediately below Status. It reads
the retained status collection in the selected career, including a completed
career resumed from the library. The panel updates with the existing career
snapshot after an action or a library resume.

## Read the current state before choosing an action

An injury record produces a visible training warning. The current engine treats
an exact case-insensitive `injured` ID, or an ID containing `injury`
case-insensitively, as an injury. This also applies to unfamiliar imported IDs:
`LEFT-INJURY` and `epithet:future_injury_win` have the same training consequence.
The original ID remains visible below the Injury label.

**Rest clears injury conditions when that choice is available.** The panel does
not add or trigger an action. Continue to use the existing Choices panel; event,
race and completed-career rules still determine which choices are available.
A completed career keeps its injury record and shows a retained-record message
without recovery guidance.

The engine currently recognizes these three simulator title IDs:

| Retained ID | Display label |
| --- | --- |
| `epithet:sim_g1_win` | G1 win |
| `epithet:sim_climax_win` | Climax win |
| `epithet:sim_ura_finale_win` | URA finale win |

Each is marked **Simulator title**, and its exact ID remains visible. These IDs
are the simulator's current epithet stubs, not a complete official game title
catalog. The panel does not assign new awards or infer additional bonuses.

Other retained strings appear literally. Their presence alone does not identify
a positive or negative effect, duration, or recovery method. Empty and
whitespace-only IDs are displayed with quotes so they remain distinguishable.
All entries, including duplicates, remain in their recorded order. Long lists
scroll, and long IDs wrap within the panel.

An empty collection says **No conditions or titles recorded.** Missing or
malformed collection data says that the career snapshot did not report
conditions and titles; it does not claim the career is free of injury.

## Implementation boundary

`ConditionsPanel.tsx` receives the existing `CareerState` and uses a pure
`conditionsView.ts` projection. It has no requests, effects, action callbacks or
state mutation. The App integration adds one import and one placement after
StatsPanel. Existing session, keyboard, training, race and import/export
handlers remain intact.

The injury predicate follows
[`CareerState::is_injured` and `without_injury`](../uma-sim-core/src/state.rs).
The title labels correspond only to the three IDs returned by
[`RaceOutcomeConfig::epithet_for_win`](../uma-sim-core/src/config.rs).
The backend remains authoritative about action legality and recovery.

## Qualification

The focused presentation controls are included in the existing UI test command:

```sh
cd packages/uma-sim-ui
npm ci
npm test
npm run build
```

The native receiver runs the actual built UI against a separately launched
native API, using a new private storage directory. It starts a real career,
plays the mandatory debut through the existing Race choice when needed, and
imports explicitly authored checkpoints through the existing library API,
resumes them through the actual UI, and uses the existing Rest choice. The fixture
blocks every off-origin browser request and records the existing setup screen's
attempted catalog portrait requests separately from unexpected requests.

```sh
cargo build --locked -p uma-sim-core --bin uma-sim-api
node packages/uma-sim-ui/tests/receiving/conditions-titles-receiver.mjs \
  packages/uma-sim-ui/dist /absolute/new/receiver-output \
  target/debug/uma-sim-api /absolute/path/to/chromium
```

The output directory must not already exist. Omit the final executable argument
to use Playwright's configured Chromium. `UMA_CONDITIONS_EXPECT=absent` records
the missing panel in a separately built pre-feature UI while using the same
native authored-checkpoint flow. Every fixture is identified as authored, not
an earned career outcome. No real provider, existing user career, or installed
native package is changed.

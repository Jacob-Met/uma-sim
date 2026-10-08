# Branch trace: native contract review

Reviewed against `Jacob-Met/uma-sim` main commit `0ac14602addd1a4610aa8359896915a34cce5659`, after main advanced through PR #65. This is a source review; it does not claim a Rust execution or native API integration test.

## Producer evidence

| Source | Exact Git blob |
| --- | --- |
| `uma-sim-core/src/career_lab.rs` | `6f726976b0bbe448c1dd4d7f8e6427dfd547e199` |
| `uma-sim-core/src/engine.rs` | `7ece3123aea3ba3f17326ceb033fc53e796b3566` |
| `uma-sim-core/src/session.rs` | `c407f70093b955801126d1f3c93af79a279d2346` |
| `uma-sim-core/src/api.rs` | `4ff7034bc5701d793cd93d645d89d40d73fc3273` |

The career-lab and API producer blobs are unchanged from the original source pin `f5f9b29393731d18aee2d66a31d89c315aa79c60`.

## What the recorded flags establish

`choose_action` checks the first configured override whose turn equals the current pre-step turn. When that override's action ID appears in the current choices, it returns applied=true/rejected=false. When the ID is absent, it selects `policy_action` and returns applied=false/rejected=true. With no matching override it returns both flags false. Both flags cannot be true in one step produced by this implementation. Applied is evidence of choice availability; it does not establish that the action improved the outcome. [Producer](https://github.com/Jacob-Met/uma-sim/blob/0ac14602addd1a4610aa8359896915a34cce5659/uma-sim-core/src/career_lab.rs#L650-L695)

The runner performs this lookup on every recorded step. Training that creates a pending event and scenario lesson actions can leave the turn unchanged, so multiple phases or actions can share one turn. The same turn can therefore contain both applied and rejected flags across different steps. A configured turn that is never recorded has no observed outcome; absence does not establish rejection. Legacy duplicate-turn configurations use the first entry, although the new UI prevents authors from submitting duplicates. [Branch loop](https://github.com/Jacob-Met/uma-sim/blob/0ac14602addd1a4610aa8359896915a34cce5659/uma-sim-core/src/career_lab.rs#L716-L764), [turn advancement](https://github.com/Jacob-Met/uma-sim/blob/0ac14602addd1a4610aa8359896915a34cce5659/uma-sim-core/src/engine.rs#L396-L435)

## Action, fallback, and outcome wording

A rejected override always invokes the configured policy selector. Both policy getters return `None` only when choices are empty; the branch loop already stops on an empty choice list. The recorded action ID and label describe the selected action passed to `engine.step`. The runner ignores the step result, and the engine can return early or report an invalid action. Consequently the panel now says **Recorded action** and **action selected for each recorded step**, without asserting that the recorded selection completed successfully. The trace contains the rejection flag but no detailed rejection message. [Policy getters](https://github.com/Jacob-Met/uma-sim/blob/0ac14602addd1a4610aa8359896915a34cce5659/uma-sim-core/src/engine.rs#L267-L291), [step handling](https://github.com/Jacob-Met/uma-sim/blob/0ac14602addd1a4610aa8359896915a34cce5659/uma-sim-core/src/engine.rs#L379-L448)

Turn, date, phase, and RNG-before are recorded before the action; RNG-after and outcome measurements come from the subsequent engine state. `outcome.steps` is the number of appended timeline rows. `careerComplete` is copied from the final native state. The panel's completion text uses that boolean and does not infer that an incomplete run exhausted its budget. The configuration is retained in the result, so **Requested limit** describes the recorded configuration rather than an inferred stop reason. [Recorded fields and outcome](https://github.com/Jacob-Met/uma-sim/blob/0ac14602addd1a4610aa8359896915a34cce5659/uma-sim-core/src/career_lab.rs#L724-L805)

## Integration review

Reviewed root's full-app wiring in `LabTab.tsx` and `ComparePanel.tsx`: the inspector has its own hook, opens saved-row IDs, and closes when the confirmed local branch list loses its selected ID. The delete path removes the row before its fallible list refresh; an acknowledged deletion therefore also retires a pending or loaded trace when refresh fails. Deleting an unrelated row keeps the selected ID present. Request tickets suppress stale results after a different selection, close, or unmount. The component exports the complete loaded object without a second fetch. No session, A/B selection, API/client, or Rust mutations were added by this inspection capability.

Final wording change was verified by the nine focused panel tests. The independently authored full-app browser receiving suite is owned by the runtime reviewer; its separate receipts distinguish HTTP fixture qualification from native backend execution.

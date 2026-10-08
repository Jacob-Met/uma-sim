# URA finale participation-class model

New URA careers retain each completed race occurrence and fix a finale class before the existing qualifier is executed. The physics adapter then uses that surface/distance class for all three rounds. This closes a native inconsistency: the current generic optional/debut races use Tokyo turf sprint, while the previous fixed finale courses were Tokyo mile, medium and long.

This is an explicit simulator modeling policy grounded in community research. It is **not official game parity**. The existing approximate November/December finale schedule, generic race chooser, NPC model and physics remain in place. Mixed canonical race histories are supported through native scenario callbacks and retained states; the engine does not gain a canonical race picker.

## Participation, ties and courses

Each successful invocation of the engine's existing `on_race_complete` hook before the finale boundary records one occurrence, regardless of placing or `won`. Repeated IDs and optional races count. `completed_races` remains the existing deduplicated mandatory-completion vector and is never used to reconstruct history.

The maximum **joint** surface/distance count wins. Ties use this table's order, not separate surface and distance majorities. These are the seven admitted classes; dirt-long is deliberately excluded.

| Priority | Surface | Distance class | Representative course | Actual metres |
|---|---|---|---|---|
| 1 | Turf | Sprint | 10601 | 1400 |
| 2 | Turf | Mile | 10602 | 1600 |
| 3 | Turf | Medium | 10604 | 2000 |
| 4 | Turf | Long | 10607 | 2500 |
| 5 | Dirt | Sprint | 10609 | 1300 |
| 6 | Dirt | Mile | 10611 | 1600 |
| 7 | Dirt | Medium | 10612 | 2100 |

The checked-in course metadata is authoritative. A round keeps its existing course if that course already matches the selected class: qualifier 10602 (turf mile, 1600 m), semifinal 10606 (turf medium, 2400 m), finals 10608 (turf long, 3400 m). Other rounds use the representative above. A missing or mismatched representative falls back to the legacy round course. No venue fidelity is implied by choosing these existing Tokyo courses.

The recognized native symbolic IDs `debut` and `optional` count their actual interim 10601 course. Numeric catalogue IDs, optionally prefixed once with `race:`, count their known course. Unknown IDs do **not** count the generic physics fallback as a known sprint occurrence.

## Retention, freezing and old saves

The existing new-career URA initialization hook creates version 1 resources: `ura_finale_history_version`, `ura_finale_history_complete`, seven `ura_finale_{surface}_{distance}` counters, and `ura_finale_frozen_class`. Unrelated scenario resources are preserved. Every required field must be present; counters must be nonnegative.

Only the qualifier boundary can select a class. Selection is fixed after the existing mandatory-race decision and before its physics call. The frozen class is 1–7 in table order; 0 means not yet frozen and -1 means unavailable. Frozen selection must agree with its retained counts. Finale rounds and every later callback, including unknown IDs, leave the frozen history unchanged.

An empty, legacy, incomplete, malformed or invalidated history retains the legacy per-round courses. An unknown course, unsupported class or counter overflow invalidates the complete history rather than selecting a partial majority. A later-final state without an earlier freeze cannot establish a selection retroactively. Missing trackers are never initialized from the turn number, completed IDs or logs, including after subsequent known race callbacks. The finale boundary reports unavailable-history fallback.

Resources travel through the existing serialized career state. There is no new save format, migration, provider, account or browser route. The context-free `course_id_for_race` API retains its existing results; only valid frozen URA finale physics receives the contextual override. Other scenarios and non-finale races keep their existing resolution.

## Determinism and verification boundary

Race seed derivation, NPC-generation algorithms, career RNG use, action admission and physics formulas are unchanged. A different course can change race outcomes and downstream career results; preserving algorithms and seeds does not promise identical results.

Maintained tests in `uma-sim-core/tests/ura_finale_history.rs` cover all seven actual metadata classes, retained matching round courses, joint counts, tie precedence, repeated participation/losses, legacy restore, invalid history, frozen save/reopen and unrelated-scenario behavior. Existing parity and race-physics gates remain relevant. Historical golden data must be reviewed explicitly when an intended course change affects it; it is not silently regenerated.

## Domain evidence and limits

- [Official Cygames scenario page](https://umamusume.jp/contents/game/scenario/ura/) supports the qualifier, semifinal and final tournament structure. It does not specify this implementation's counting, tie or course policy.
- [VIP community observations](https://wikiwiki.jp/vip_umamusu/コメント/URAファイナルズ) include first-hand April/May 2021 category examples distinguishing joint counts from surface totals. The [scenario guide](https://wikiwiki.jp/vip_umamusu/URAファイナルズ) provides community context.
- [Community scenario discussion](https://w.atwiki.jp/aniwotawiki/pages/52107.html) describes participation-based selection. Seven-bin tie precedence was corroborated in indexed community FAQ/scenario material; direct retrieval of the wikiru article normalized to its front page, so a full article read is not claimed.

Participation, tie precedence and freeze timing are the documented community-grounded policy above. Official confirmation of those exact rules, full game finale venue/calendar fidelity and reconstruction of unrecorded old-save occurrences remain outside this increment. The independent receiving evidence records both the source limitations and the actual original engine observations.

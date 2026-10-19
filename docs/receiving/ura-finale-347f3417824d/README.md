# URA finale-history qualification

New URA careers retain known race-completion occurrences and fix a joint surface/distance class before the existing qualifier. The ordinary native route previously raced debut/optional on turf sprint, then switched to mile, medium and long in its finals. It now uses turf sprint for all three rounds. Repeated participation and losses count; finals do not feed the history.

The exact policy, representative course table, old-save behavior and domain evidence limits are documented in [URA_FINALE_HISTORY.md](../../URA_FINALE_HISTORY.md). This is a community-grounded simulator policy, not official game parity.

The qualified component changes three runtime files, adds maintained public-hook/physics tests, updates the previous gap-status test and documents the bounded behavior. Engine/state source, scenario calendar, NPC and RNG algorithms/seeds, physics formulas, Cargo/dependencies and all golden files are unchanged. Changed courses can change outcomes; identical outcomes are not a general promise.

## Receiving

- The exact original source was recovered and Git-blob verified before authoring. The ordinary original engine witness completed 71 actions with debut/optional 10601 and finales 10602/10606/10608. This demonstrated the actual reachable discrepancy.
- Independent expectations and two native receiver sources were sealed before candidate exposure. Their public-API original control passed 6/20 cases; the feature cases failed meaningfully.
- Component native gates passed **86 methods**: 31 focused mechanics/race/history checks and 55 existing core/golden/parity checks. The three golden methods include all 200 unchanged career summaries.
- The unchanged independent receivers passed: the ordinary career uses 10601 for every finale while preserving its pre-finale physics, completion vector, 71 actions and 272 career RNG calls; the API receiver passes **20/20 cases**. All 302 component inputs remained exact.
- Main then advanced to `38291c1aaf915da17229eca0afbbd58d9b3ce453`, adding explicit browser-session targeting. Independent receiving found only one changed native input, `api.rs`, and one added native test. All nine authored leaves and the model closure remained exact. The separate composition passed the incoming **5/5 API tests**. No unchanged feature matrix was repeated.

[receipt.json](receipt.json) binds the exact source, counts, independent receipts and [native-evidence.tar.gz](native-evidence.tar.gz). The deterministic archive contains 53 regular members, including an unchanged nested 51-member independent packet. Manifests and raw logs retain actual child exits and hashes, source/contract seals, original negative controls and the first receiver import correction.

The original offline Cargo attempt selected an existing cache without `tiny_http`; retry used the already installed complete cache. The first candidate compile hit default `/tmp` ENOSPC; an owned temporary directory allowed the unchanged source to compile. These failures remain in the evidence. Existing lib tests created two synthetic lab records; their identities are recorded and the files are retained natively, outside source publication.

The recovered native shell fixture `tests/fixtures/policy_stub.sh` lacked its upstream executable bit. Its bytes are bound, none of these executed gates invokes it, and native executable mode is not claimed as qualified. Publication inherits its existing Git `100755` mode unchanged. Original sources and fixtures were not modified.

## Replay and boundary

The receipts list the native offline/locked Cargo commands and build environment. From the qualified repository source, the meaningful native gates are:

```sh
cargo test --offline --locked -p uma-sim-core --test ura_finale_history --test ura_mechanics --test race_physics_r87 --test race_npc_r86 --test race_outcomes --test race_scheduler -- --test-threads=1
cargo test --offline --locked -p uma-sim-core --lib --test parity --test golden_seeds -- --test-threads=1
cargo test --offline --locked -p uma-sim-core --test session_targeting -- --test-threads=1
```

Independent driver sources and their invocation receipts are in the nested packet. They use synthetic careers and the real public native hooks/physics adapter. A current compatible Rust toolchain and the checked-in course/catalogue data are required.

This source qualification does not claim full game venue/calendar parity, a canonical engine race chooser, legacy-history reconstruction, installed-runtime adoption or real-account execution. Hosted checks and final merge/tree verification are separate receiving steps.

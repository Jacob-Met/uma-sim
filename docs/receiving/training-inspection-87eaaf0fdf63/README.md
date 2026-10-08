# Native training inspection qualification

## Subject and player workflow

Source commit: `cbeb87df63271197650466d2be381a4b2ade5d84`.
Tree: `5e23bc907a03570c4b159a07657ddb0715fa1595`.
Parent: `b026a21f437789d7a7cd04c5074258302e36814e`,
tree `359cc10294331337e200c433d95d170ea439e087`.
[Scope reservation #100](https://github.com/Jacob-Met/uma-sim/issues/100) preceded source edits.

The native player can run `training` between actions to inspect five training
samples, energy, failure risk and blockers. `training --format=json` exposes
the same version-1 result. The actual private native career in
`candidate-player.json` starts at a mandatory race, follows the existing
race/event gates, and reaches a normal training phase. Its four inspection calls
preserve the exact before/after saved bytes. The final text and JSON are retained
separately for review; both saved career inputs are authored fixtures.

Sample stat gains come from the existing resolver with its fixed local seed 0.
They are before caps and subsequent scenario/event effects and are not promised
next-action outcomes or expected values. The display uses the engine's effective
facility levels through a cloned preview state. This covers Unity rank resources
without changing its saved facility map or the generic policy preview.

Failure chance reuses the existing execution branch, including current energy,
level and Unity/Trackblazer readiness. `extraction-review.json` verifies the
branch's text is identical after removal of only its local return binding.
The training resolver, failure configuration, scenario modules, session codec
and dependency lock remain exact to the parent.

## Native gates

All execution used an isolated ordinary macOS arm64 checkout on Mac.lan:
`/Users/me/.hamon-rdc-seats/seat-5/uma-training-87eaaf0fdf63.jo98f7ht/source`.
The compiler and Cargo are 1.99.0. Commands use a private target directory,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0`; dependencies were available offline.
No installed service, browser, real saved user career or external policy ran.

| Gate | Result |
| --- | --- |
| Original native build | Pass; original `training` prints general help |
| New focused integration target | 7 tests passed, 0 failed; 2.13 s |
| Existing affected regression targets | 48 tests passed, 0 failed |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --offline --locked` | Pass; preexisting warnings retained |
| Real native player capture | Four inspections preserve session bytes; text/JSON include five ordered rows in the free phase |
| Incremental Git bundle | Verified; requires canonical parent b026a21f |

Focused command:

```text
cargo test --offline --locked -p uma-sim-core --test training_inspection
```

Existing regression command:

```text
cargo test --offline --locked -p uma-sim-core \
  --test golden_seed --test golden_seeds --test sim_engine \
  --test training_resolver --test training_failure_config \
  --test training_failure_penalty --test grand_live_l1_training \
  --test unity_trackblazer_mechanics
```

The two golden targets retain their existing golden fixtures. These are
55 selected native tests, not a full workspace-suite result. The full hosted
workflow and independent receiving remain separate integration gates.

## Boundaries and preserved holds

The first regression-build preflight saw 212,033,536 free bytes and stopped
before Cargo was invoked. Its receipt is preserved. Capacity later recovered
without cleanup by this worker; the same ordinary native route then ran all
eight regression targets successfully. No shared source or another worker's
files were removed.

The complete baseline tree has no AGENTS.md. The source
`docs/agent-operations.md` was read. Its referenced github-landing skill
was absent at the two stated native workspace locations; its concrete exact-head,
independent-review, source-readback and CI requirements remain in force.
An earlier read of another worker's native coordination handoff returned
PermissionError; it was not retried through another device or identity.
Fresh supported GitHub coordination established the disjoint #100 scope.

Original source and binary identities, the missing-command witness, exact native
stdout/stderr and saved-input bytes, raw gate logs, unchanged-branch evidence,
patch and incremental Git bundle are retained. The original bundle requires
b026a21f and contains the source commit above; it does not claim a standalone
copy of the repository's full history. The adjacent manifest hashes this packet.

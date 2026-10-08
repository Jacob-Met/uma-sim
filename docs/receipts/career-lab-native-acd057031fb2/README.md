# Literal comparison reports and correct ended-branch counts

Contribution: `chatgpt-acd057031fb2`, addressing [uma-sim issue 73](https://github.com/Jacob-Met/uma-sim/issues/73).

## Product changes

Downloaded comparison reports preserve names and other data as literal text. A
name containing a pipe stays in its existing table cell; backticks, Markdown
punctuation, literal entities/HTML, and line breaks do not create unintended
report structure. Code-styled metadata uses a delimiter that can contain its
actual backticks. Numeric comparison values and the JSON representation are
unchanged by serialization.

When one recorded timeline ends before the other, the first-divergence note now
reports the ended branch's actual length. Previously it named that branch but
used the continuing branch's length. Earlier divergence and equal timelines keep
their existing behavior.

Production changes are confined to `uma-sim-core/src/career_lab.rs`. The other
native edits add two focused regression modules and update two existing raw
Markdown heading assertions, including one inside `api.rs`'s test module. The
production API prefix is unchanged. The existing browser comparison receiver
now checks the rendered meaning of Markdown instead of searching its escaped
source for unescaped names.

## Receiving evidence

| Boundary | Original control | Qualified candidate |
| --- | --- | --- |
| Independently authored Rust report cases | 4 pass / 5 fail | 9 / 9 pass |
| Rust ended-count cases | 2 pass / 4 fail | 6 / 6 pass |
| Existing career-lab integration suite | Preserved | 14 / 14 pass |
| Actual API Markdown downloads, parsed in Chromium | 1 / 9 cases; 74 / 93 checks | 9 / 9 cases; 93 / 93 checks |
| Actual unequal-branch JSON and Markdown reports | 1 / 3 cases; 12 / 18 checks | 3 / 3 cases; 18 / 18 checks |
| Accepted PR65 UI with actual native API | Accepted displayed-result contract | 9 / 9 interaction cases |
| Adversarial-name UI and actual downloads | Historical oracle failure retained | 6 / 6 checks |
| Native workspace tests | Original failure and control retained | 354 pass; zero failed, ignored, or filtered |

The workspace source pin is `97754679972a3128b5b84cde2159b43a4d85c56e`.
Formatting and workspace/all-target Clippy pass. The initial raw-heading test
failure, its passing accepted-main control, and storage-exhaustion attempts are
retained alongside the successful run. Clippy's pre-existing warnings were not
silenced. See [workspace receiving](workspace/README.md), its full compressed
logs, source-scope manifest, and suite counts.

The actual browser-qualified native executable has SHA-256
`149600d040481e6eb2530c2a3081d1c680f223d7a16d6bcd54feb247ab50579d`.
Its combined renderer/count source has SHA-256
`49d68441482fa907364292bb25b43fc06be6e95b13937cbbba898a393f4e4935`.
This executable predates the test-only API assertion maintenance; receipts do
not claim it was built from the later test-file bytes. Actual attachments,
rendered DOM, screenshots, request/response identities, source pins, and the
explicitly authored persisted-field fixture are preserved in the native report
and ended-branch packets.

- [Report serialization and original native receiving](../lab-report-20261008/native-receiving/README.md)
- [Composed native count/report and accepted-UI receiving](../lab-ended-branch-20261008/native-receiving/README.md)
- [Independent source and receiving review: APPROVE](independent-review/README.md)

## Accepted work and provenance

The accepted PR65 UI deliberately preserves a displayed comparison while the
user changes the draft pair. Its downloads remain bound to that displayed A/B
result. This contribution preserves that behavior. Our older alternative UI
proposal is superseded; its historical receiving script is retained under the
report packet's `historical/` directory and is not the current verification
entry point.

The complete PR65 source tree equals accepted main
`0ac14602addd1a4610aa8359896915a34cce5659`; the receipt verifies all 32 committed
UI leaves. Accepted PR71 at `0b5cc2342cd93141beb461edf6e83998388e67b8` changes a
distinct checkpoint import label and adds its own portability receiver. Main
`5d32d0b06de88141ecd4c84922e77553a45a522f` additionally integrates terminal CLI,
its CI step, scan configuration, and evidence. Both accepted increments were
merged with their original ancestry, and neither changes the qualified Rust
source or comparison runtime files. No claim is made that the earlier browser
run included the later label or terminal changes.

The receiving composition reviewed independently at tree
`e87746af0d7ac749cb9c82511588285fc20f6657` is unchanged in its product and test
files by the subsequent evidence additions and accepted-base merges. The
publication-scope record verifies this and the complete diff against current
main. Residual generic UI lifecycle evidence is a separate contribution for
the existing issue64 owner, not an additional UI implementation in this branch.

## Execution status

This packet records implemented and independently verified source. GitHub PR
publication, exact published-head CI, current-base review, and any merge remain
separate integration steps. At this checkpoint GitHub has imposed a secondary
content-creation limit; publication is paused through its recorded backoff.
No deployment or operational-benefit measurement is claimed. The estate's
existing contribution record and the eventual issue/PR conversation carry the
later integration state.

Current verification entry points and runtime prerequisites are documented in
the linked receiving packets. Frozen original failures and source pins remain
historical evidence; they are not relabeled as runs against newer commits.

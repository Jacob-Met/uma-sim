# Race-results provenance: independent Gitleaks receiving

The eight findings at PR #95 head `373646ab050f355ef990088f55550177d28b18cb`
are recomputed source, executable, and source-commit identities. A native run of
the same Gitleaks version as the failed jobs reproduced all eight findings. The
proposed policy removes only the verified path/value pairs and retains the
default detectors and all inherited allowances.

## Exact subject and proposed change

The failed pull-request run is 37788580137 (job 113349619733); the failed push run
is 37788483609 (job 113349288689). Their unmodified redacted logs are preserved in
`original-hosted-logs/`. Both used Gitleaks 8.24.3 and reported eight
`generic-api-key` findings in five files.

`source-bindings.json` pins those five files by their original Git blobs and
records the eight path/line/rule observations. `classification.json` records the
recomputed identities and their meaning. The source digest was recomputed from
the exact source object. The executable digest was recomputed after decompressing
the earlier qualified native artifact. Its source commit was resolved as a real
Git commit. The source-commit field is not a source-file blob identity.

The policy adds three allowlists to the existing `generic-api-key` rule:

| Identity | Exact allowed evidence paths | Actual findings |
| --- | --- | ---: |
| Source digest | Baseline native witness; candidate browser receipt; current-044 browser receipt | 5 |
| Executable digest | Source freeze; current-044 composition record | 2 |
| Native build source commit | Current-044 composition record | 1 |

Each entry requires `condition = "AND"`, an explicitly anchored full path, and
an explicitly anchored complete value with `regexTarget = "secret"`. This is
six path/value pairs covering eight observed occurrences. The complete current
policy prefix, existing exceptions, default rules, and all original source and
evidence bytes remain unchanged.

## Actual native controls

The official standalone release archive matched the publisher's checksum before
extraction. Archive names, member kinds, duplicates, size, and executable format
were checked before execution. The executable reported 8.24.3. The acquisition
receipt and exact acquisition script are included. No global installation or
runtime change was made.

All ten groups in `native/receipt.json` passed:

| Group | Findings | Exit |
| --- | ---: | ---: |
| Published policy, exact five files | 8 | 2 |
| Current-237 policy, exact five files | 8 | 2 |
| Qualified policy, exact five files | 0 | 0 |
| Different whole values at the allowed paths | 8 | 2 |
| Original values at different paths | 8 | 2 |
| Extra character before each value | 8 | 2 |
| Extra character after each value | 8 | 2 |
| Verified identities exchanged across differently allowed paths | 3 | 2 |
| Same allowed identity under a separate test-only rule | 1 | 2 |
| Unassigned offline GitHub-pattern probe under the default rule | 1 | 2 |

Each scan used a real local Git commit containing the stated fixture files.
Commands, fixture commit identities, input-file digests, redacted reports, and
redacted logs are retained. The alternate rule was added only to a separate
test configuration. It is absent from the proposed policy.

`negative-fixtures.tar.gz` preserves the exact seven negative-control source
sets and their test configuration. It intentionally contains detector probes
and wrong-path/wrong-value examples. Keep this raw evidence compressed in the
repository; the public driver regenerates it for native replay. No control value
was used in a request or a live account.

## Current-main continuity

The ten-group run qualified the policy based on main
`2378365e48234c5ae3311e1b0dd7f1247a5a9071`. Main subsequently advanced to
`4eab1798def5c19fbdee267eb2a6ea8a2726dbe7`, which added PR #91's own policy
entry. The final policy preserves that entire newer prefix and appends the same
three qualified entries. A separate native scan of the same five exact files
returned zero findings. That one-run continuity receipt and exact executed
script are in `current-4eab/`; the original ten-group receipt is unchanged.

## Replay

Use Python 3.11 or later, Git, the checksum-verified Gitleaks 8.24.3 executable,
and a new work directory with at least the recorded capacity floor. Point
`--source` at a checkout containing the five exact bound evidence files:

~~~sh
python3 review_gitleaks.py \
  --source /path/to/uma-sim \
  --bindings source-bindings.json \
  --published-config policy-controls/published-373646.toml \
  --current-config policy-controls/current-2378365.toml \
  --candidate-config policy-controls/qualified-2378365.toml \
  --scanner /path/to/gitleaks \
  --work /path/to/new-scanner-receiving
~~~

The driver verifies input blobs, scanner bytes/version, policy continuity, and
source immutability. It preserves reports under the new work directory.

## Limits and retained failures

This is a bounded reproduction of the five exact finding files, using the same
native scanner and Git-history mode observed in the hosted jobs. Exact-head
full-repository hosted scanning remains the final integration gate. This packet
does not claim that the full current repository was scanned locally.

The first local HTTPS attempt timed out. The first isolated ThinkPad attempt
downloaded the checksum but hit a user quota before completing the archive.
Neither attempt executed an unverified binary. Both outcomes and partial-file
identities are preserved in `acquisition-attempts.json`. The successful attempt
used a different ordinary owned temporary directory on the existing filesystem,
with a 1 GiB capacity floor. No prior failure or original artifact was overwritten.

The public acquisition receipt omits expiring redirect-query parameters from
the download URLs. The original native acquisition receipt is preserved in its
owned receiving directory and identified by digest in the public derivative.

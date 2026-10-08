# Hosted checksum classification

The initial connector head affeddfc9a9d4d466c6d36ff4daa2ce4eb4232f8 received two findings from the default generic-api-key rule. Both were executable checksum values in README lines 33 and 34. The values were verified against the actual retained candidate and baseline native binaries; the structured receiving records contain those same artifact digests.

The correction changes only the prose artifact labels from API wording to executable wording. No checksum value, production source, receiver, scanner configuration, workflow, default rule or allowlist was changed. The source and native receiving qualifications therefore remain unchanged.

The exact failing hosted job is https://github.com/Jacob-Met/uma-sim/actions/runs/37760572737/job/113255700158 . It scanned one commit and reported two findings in this README; the structured digest records were not reported. The corrected owned branch is published as one commit on its verified current-main base so the PR's history scan evaluates the corrected artifact representation. Original native source/integration commits and the failed connector head remain recorded for provenance.

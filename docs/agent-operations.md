# Agent Operations Handbook — uma-sim

Short, repo-specific operating notes for agents working on `Jacob-Met/uma-sim`.
Docs-only; no behavior change. Lessons are dated snapshots — verify against live
repo state before acting on a stale one.

## Canonical operating rules

The estate's canonical agent-operating rules live in the `github_landing` skill
(`~/workspace/skills/github-landing/SKILL.md` in the agent workspace). Summary:

- Verify the exact branch tip SHA immediately before each shared mutation;
  re-check in the push turn itself (collision checks go stale in minutes).
- Read back after every push/merge/comment; back off once on 429/403.
- Merges only for independently reviewed, CI-green PRs: pin the head with the
  `sha` body param on PUT /merge, and run the rebase-freshness gate.
- Never force-push, rewrite history, or delete branches.
- Holds: never write Mission Control's `queue/`; never touch
  `HELDOUT_SEALED.md`; respect active lane/queue owners' branches and surfaces.

## Dated repo-specific lessons (newest first)

- **2026-10-05 — Merge-turn 4-check checklist.** Before any merge: (1) re-read
  review state at merge time and surface any blocking dissent as a material
  exception; (2) duplicate guard — a live PR-list read in the mutation turn;
  (3) head-pin check — sha-guarded PUT /merge so a moved head fails instead of
  merging; (4) CI-on-head honesty — never substitute "mergeable clean" for CI
  green.
- **2026-10-05 — Pagination note format.** `gh-pat-api` injects the pagination
  note INLINE mid-line on some list endpoints (seen on closed-pulls), not just
  as a bare comment line. Strip the substring
  `# NOTE: paginated response, more pages available` before JSON-parsing;
  line-based `grep -v '^#'` no longer works.
- **2026-10-04 — Rebase-freshness gate (the uma-sim #42 lesson).** A textually
  clean merge is not a safe merge: a branch cut from a stale checkout merges
  cleanly while silently reverting other lanes' work. In the merge turn
  itself, `git merge-base --is-ancestor <base-tip-sha> <head-sha>` must hold;
  on failure, rebase onto the current base tip and re-run the affected test
  suite. Also scan `git diff <base-tip>...<head> --stat` for out-of-scope
  files.
- **2026-10-04 — Collision checks expire in minutes.** PR #43 duplicated
  sibling PR #41 (opened ~16 min earlier) because the pre-push collision check
  ran ~10 min before the push; #43 was withdrawn as a duplicate. Re-check the
  live PR list immediately before pushing, not just at mission start.
- **2026-10-04 — The 503 saga is resolved.** PR #48 (handle_fast restore +
  regression test) squash-merged; re-verified 2026-10-05 at tip `f1a8f3bf`
  (regression test PASS, api::tests 12/12). PR #46 closed unmerged as
  superseded. PR #54 (`test(api): harden handle_fast regression harness`)
  flagged DO-NOT-MERGE on a failing CI head — observation only, do not land.
- **2026-10-04 — npm wrappers.** PR #41 merged (GPL-3.0-only SPDX into the
  three npm wrapper manifests); PR #40 closed unmerged as a duplicate by
  Jacob. Packages are `"private": true` — registry 404s on publish are
  expected, not a gap.

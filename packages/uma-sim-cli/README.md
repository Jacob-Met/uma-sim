# Terminal career sessions

The terminal client uses Node 20 or newer and the existing Rust REST API.
No npm dependencies are required. Build the API from the repository root with
`cargo build --release -p uma-sim-core`, or run `uma-sim serve` yourself.
`UMA_SIM_API` selects a different API URL (default `http://127.0.0.1:8765`).

## Start and resume

```sh
# Start a new career without replacing an existing career.
node packages/uma-sim-cli/tui.js 7 unity

# The optional --new flag makes the same fresh-career behavior explicit.
node packages/uma-sim-cli/tui.js --new

# List the server's live careers and copy a named session id.
node packages/uma-sim-cli/tui.js --list-sessions
node packages/uma-sim-cli/tui.js --session=tui-<id-printed-at-start>
```

Every new terminal career receives a random UUID session id, which is printed
along with the resume command. Seed and scenario positional arguments retain
their meanings; they default to `42` and `ura`. Creating a career makes it the
server's active session, as `/v1/run/start` specifies, but preserves the existing
main and named careers. Leaving the terminal preserves its career for later
resumption while that API process remains running.

`--session=<id>` resumes only that existing named career. It never creates,
restarts, activates or closes a session, and all its reads and actions keep
targeting that id if another client changes the server's active session.
A missing session is an error. Combining `--session` with `--new`, a seed,
a scenario or `--list-sessions` is also an error; there is no replace-existing
mode. Use the printed id to resume, or a new invocation to start another career.

`--list-sessions` is read-only. `*` marks the server's active session and
`(main)` identifies its legacy empty-id session. This client accepts only
nonempty named ids: empty, whitespace or malformed ids are rejected before
any HTTP request. The current API treats an explicit empty selector as the
active session, so this client cannot safely resume `(main)`.

Session ids permit 1–64 ASCII letters, digits, periods, underscores and hyphens;
`.` and `..` are reserved. Both `--session=<id>` and `--session <id>` work.
Use the equals form for a session id beginning with `--`.

The API stores live sessions in memory. Use a checkpoint to retain a career
across API restarts. Listing and either resume mode require an already running
API. New-career mode can start a built local API after a refused loopback
connection; it will not start a local service in response to a remote connection
failure or an HTTP error from the configured server.

## Save and recover after a restart

In the running terminal, save the selected career before leaving:

```text
save before-race
checkpoints
quit
```

After the API has restarted, list its library and open that saved career:

```sh
node packages/uma-sim-cli/tui.js --list-checkpoints
node packages/uma-sim-cli/tui.js --checkpoint=before-race
```

`save NAME` writes the selected session's current snapshot to the API server's
existing durable library. The name follows the same 1–64 character rules as
session ids. Each save is create-only: an existing name returns HTTP 409 and
retains its checkpoint. Use a different name to save later progress.
`checkpoints` and `--list-checkpoints` show names, turns, scenarios, trainees and
save times without changing careers.

`--checkpoint=NAME` (or `--checkpoint NAME`) forks the saved snapshot into a
fresh, uniquely named live session. It prints that session's `--session` resume
command and displays any compatibility advisories returned by the API. The
checkpoint, existing main and other live careers keep their state. Forking makes
the new session active as the native API specifies; subsequent terminal reads,
actions and saves remain bound to its printed id even if another client changes
the active selection. Missing or incompatible checkpoints fail visibly without
starting a new career. Do not combine this mode with `--session`, `--new`,
listing options, a seed or a scenario.

The library is stored under `.uma-sim/library/` relative to the API process's
working directory. Restart that API with the same working directory to reopen
the same library. When `UMA_SIM_API` points to another machine, saves live on
that server. Checkpoints contain the saved snapshot and RNG position; later
actions do not update them, and quitting does not automatically save.
`--session` continues to address live sessions only, so it returns an error
for a session lost during an API restart.

## Commands and failures

Enter a displayed choice id, `auto` (one bot step), `fast` (finish the career),
`state` (show the displayed snapshot as JSON), `save NAME`, `checkpoints`, or
`quit`/`q`.

HTTP failures, including a missing session or an unavailable external policy,
are displayed with their status and server message. An interactive command
failure returns to the prompt and refreshes the same session. Failed starts
and resumes exit with status 1; they never fall back to another career.

Transport failures and invalid responses after a mutation can leave its result
uncertain. The terminal displays the affected session and asks you to inspect
its state before retrying. It never automatically repeats an action. EOF and
Ctrl-C stop waiting for an outstanding request, with the same uncertainty
message for a pending mutation; they cannot undo work already received by
the API. EOF exits with status 0, Ctrl-C with status 130.

An uncertain save also prints the checkpoint name and asks you to inspect the
library before saving again. An uncertain checkpoint fork retains the proposed
session id so you can inspect it with `--session`; it never retries the fork or
replaces another career. A malformed success response is treated as uncertain
rather than reported as a completed save or fork.

## Verification

```sh
npm --prefix packages/uma-sim-cli run test:tui
```

The Node tests launch the real terminal executable against disposable HTTP
receivers. They verify career preservation, session addressing, argument
validation, read-only listing, startup and command failures, lost responses,
recovery, checkpoint addressing and create-only saves, EOF and interruption. Fault responses in these tests are deliberate
HTTP contract fixtures; they are not claims that every native endpoint emits
every tested status. The existing Node CI job runs this suite.

A separate suite exercises the real Rust API with a private on-disk library,
actual API process restarts, exact saved state and three-step RNG replay,
native refusals, compatibility advisories, and deliberately lost or held
post-commit responses:

```sh
cargo build --locked --release -p uma-sim-core --bin uma-sim-api
UMA_SIM_NATIVE_API="$PWD/target/release/uma-sim-api" npm --prefix packages/uma-sim-cli run test:tui:native
```

The native suite requires an explicit existing binary and fails if it is missing.
It does not skip when unconfigured. The existing Rust CI job runs it after its
release API build and retains the receiver's artifacts.

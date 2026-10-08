# Browser session targeting qualification

Contribution claim: [issue #63](https://github.com/Jacob-Met/uma-sim/issues/63).
Receiving base: `f5f9b29393731d18aee2d66a31d89c315aa79c60`.

## Observed failures

The browser stored a career snapshot without its session identity. Every
action and subsequent read followed the server's mutable active-session
default. A delayed action response could overwrite a newly selected career,
and the old snapshot could appear beside the new session's text. Its error
and busy cleanup could also overwrite newer request state. Reset did not
invalidate outstanding work, and two calls before a disabled-control render
could submit the same action twice.

At the API boundary, the default main session is identified by an empty
string, but both session resolvers interpreted that explicit value as an
omitted selector. A request explicitly targeting main consequently read or
mutated the active named fork. The browser's URL helper dropped the empty
selector too.

## Change

- Carry the displayed session ID through the run store and every run action,
  follow-up read and telemetry download. Resolve active once when loading a
  session, then use that explicit ID for all component reads.
- Use a request identity to admit only the current request's view updates,
  errors and cleanup. Selection, reset and unmount invalidate earlier view
  requests. Clear the old playable state while a selection loads; failure
  leaves no stale action target. A synchronous guard prevents duplicate run
  operations before React updates the controls. Initial catalogs and health
  remain tied to the mounted app while their busy cleanup stays fenced, so
  selecting a career during initialization neither drops the catalogs nor
  releases the selection's pending state.
- Limit numeric shortcuts to the visible Run tab, respect editing/modifier/
  repeat/composition events, and use the same numbered choices as the UI.
- Preserve explicit empty session IDs through the browser client and both
  Rust run resolvers. Omission still follows active; a missing explicit
  target returns 404. The body selector continues to take precedence over
  the query selector.

## Executed verification

| Check | Exact baseline | Candidate |
| --- | --- | --- |
| Initial real-React store/client regressions | 8 failed | 8 passed |
| Additional main/start/keyboard/export regressions | 4 failed | 4 passed |
| Initialization overlapping a pending selection | 1 failed | 1 passed |
| Native TCP API session-targeting regressions | 5 failed | 5 passed |
| Existing API unit suite | 12 passed | 12 passed |
| UI typecheck and production build | Not repeated for baseline | Passed |
| Rust formatting | Not repeated for baseline | Passed |
| Native clippy for the new integration target | Not repeated for baseline | Completed with engine-source warnings |

UI qualification used Node 24.19.0, React/test-renderer 18.3.1 and the
repository's installed TypeScript/Vite toolchain. The tests execute the
actual hook, API client and App through React, using deterministic fixture
responses and controlled response delays; they do not claim real game
rendering or visual layout acceptance.

Native qualification used an isolated macOS checkout with Rust/cargo 1.99.0.
The new tests start their own API child processes and terminate them after
each test. The receiving API and test file SHA-256 values matched the local
candidate exactly. No installed server was used or changed.

`source-sha256.json` identifies the qualified code and tests. Logs preserve
the actual negative and positive results. Absolute checkout paths are
replaced with `<isolated-checkout>` and trailing whitespace is removed;
result values are otherwise retained.

## Reproduce

```sh
cd packages/uma-sim-ui
npm ci
npm test
npm run typecheck
npm run build
cd ../..
cargo test --locked -p uma-sim-core --test session_targeting
cargo test --locked -p uma-sim-core --lib api::tests -- --test-threads=1
cargo fmt --all -- --check
```

The package's `prebuild` runs the regression suite, so the existing UI CI
job executes it before producing the embedded browser assets. The new Rust
integration test is included by the existing workspace test job.

## Limits and receiving state

An obsolete response is discarded from the UI; an action already accepted
by the server is not cancelled or replayed. The API still allows another
client to advance the same session between separate reads; this change
does not introduce snapshot transactions or remote action cancellation.
Independent review and CI on the final published head are integration
gates. This record establishes authored source qualification, not a live
deployment or a measured user outcome.

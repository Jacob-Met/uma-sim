# Required career event choices

An event with pending options is a required decision, not an ordinary page
control. The browser uses a native `<dialog>` in the modal top layer with its
heading as an accessible label. The existing event title, literal option text,
order, duplicates, and `event_N` dispatch IDs remain unchanged.

While an event is open, Escape cancellation is prevented. Focus enters the
dialog and Tab remains inside it; the underlying career page is inert. The
application's window-level numbered-choice shortcuts are also suppressed:
native modality alone does not prevent an event bubbling to a window listener.
A busy state keeps the dialog open, disables its choices, and announces
`Working…`. Resolving the event or unmounting closes the modal.

## Running the checks

Builds use the repository's existing dependencies and Rust workspace:

```bash
cd packages/uma-sim-ui
npm ci --ignore-scripts
npm run build           # Runs all UI tests, including the controlled dialog suite
cd ../..
cargo build -p uma-sim-core --features embed-ui --bin uma-sim

# Real Chromium check: launches the built Rust server and embedded UI,
# starts a career, advances to a pending event, and tests actual browser behavior.
UMA_EVENT_BINARY="${CARGO_TARGET_DIR:-target}/debug/uma-sim" \
UMA_EVENT_CHROME="/path/to/chrome" \
node packages/uma-sim-ui/tests/event-dialog-live.browser.mjs
```

The live test uses a temporary local port and shuts its server and browser down.
It checks the native modal top layer, focus and Tab containment, Escape,
background shortcut suppression, first-option dispatch, close behavior,
mobile viewport fit, and uncaught page errors. Set `UMA_EVENT_SCREENSHOT` to
save an actual paused event screenshot.

The controlled suite in `tests/event-dialog-modal.test.mjs` additionally
covers ten literal states and five lifecycle timelines using compiled component
protocol doubles. It does **not** replace the real React/browser check.
`tests/run-session.test.mjs` covers the window shortcut suppression.

## Evidence and current limits

This integration builds on the earlier unreferenced source offer
`5d10437953c9da0f597c6e6ed8b7c80e1a3c87ec` associated with
[issue #171](https://github.com/Jacob-Met/uma-sim/issues/171). Its controlled
component checks were useful, but a real Chrome playthrough exposed a
background shortcut that was still active. The application-level guard is
the integration correction.

On the source implementation, an actual browser career triggered
**Extra Training** after 15 automatic steps with two options. The modal
accepted choice `event_0`, blocked Escape and background key `1`, kept
focus inside, and closed after selection without a page exception. All
130 UI tests and the production UI bundle build passed.

The verified browser is Chromium/Chrome on Linux. A screen-reader assessment,
Safari/Firefox runs, and comprehensive cross-device visual review are not
claimed. Character artwork, rules and rewards are not changed by this patch.

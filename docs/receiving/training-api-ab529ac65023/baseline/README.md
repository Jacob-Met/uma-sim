# Actual baseline receiving

The frozen tests-only PR head `aa917c00` ran as checkout `b3759407`, exact tree `f4a65876`, in unchanged CI run [37833173081](https://github.com/Jacob-Met/uma-sim/actions/runs/37833173081).

The MCP job passes 135 checks, fails the four new missing-tool cases, and deliberately skips five native tests. The new malformed-selector control passes. The Rust job formats, checks, tests and builds the existing workspace, passes 11 terminal-checkpoint tests, and then passes all 11 existing native MCP checks. All seven new training subtests fail at the absent route: embedded HTML is returned with HTTP 200. Actual CLI inspection succeeds and the four scenario setups reach free training, but no successful endpoint inspection is claimed. The Node parent reports the seven subtests plus their failed parent as eight failures.

The UI job passes 88 Node tests, 23 career-lab browser checks and 15 trace-browser checks. This is an expected-negative baseline, not candidate acceptance. Later Rust release-layout/calibration steps are skipped after native failure.

The complete three decoded raw job logs are retained here with byte counts and SHA-256 hashes in manifest.json. The existing upload step retained native evidence as artifact 11574159723 (9,851 bytes, service-reported SHA-256 a24a3b2f970d77872e19426c7051596c03e22a07a320cdd946c8270e3011c1e4). A request to the connector-returned download reference failed HTTP 403; no ZIP-byte or member verification is claimed. The exact failure is preserved in the manifest, without retaining the temporary signed URL.

Frozen test blobs remain `9de14a0f` and `ffeee00e`. No assertion, timeout, helper, workflow or production source was changed to obtain this result.

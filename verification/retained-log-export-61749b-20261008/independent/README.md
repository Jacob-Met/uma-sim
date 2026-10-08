# Independent retained-log export receiving review

**Disposition: accept the frozen source at the declared native receiving boundary.** No source correction is requested. Actual browser download/save behavior remains unestablished.

Source base: `09df4c25cbd86c12d16a121481c01d1633a5efb6`, tree `8352de08ded783d6d23030b312c36c5b9a4b0fe1`. Frozen manifest SHA256: `d237f53ecfa7af46cbe4a09a63c6a2ff56bb50e59ac41364aa5d78f31e0162bc`.

## Review and evidence

[review.json](review.json) records exact runtime/source pins, static custody, reviewed author evidence, and qualifications. All 48 supplied baseline inputs (47 UI files plus agent instructions) match their immutable Git blob manifest. All 44 inputs outside the seven-file scope remain byte-identical. Removing only the new import and full-history download component from LogPanel reproduces the entire original reader. The summary remains separate from `cs.log`; filter, follow, scroll, status, API and store paths remain unchanged.

The author’s twelve focused checks, seven React event scenarios, typecheck and build were inspected through their source and hash-verified receipts; this reviewer did not rerun them. Their positive and original-absence evidence keeps its own authorship.

[The independent native receipt](native-dispatch-refusal/receipt.json) is **one composed scenario with seven assertion groups**, not seven independent test scenarios. It adds a refusal after URL allocation and attached-anchor dispatch to the author’s earlier pre-URL preparation refusal:

- The actual LogPanel callback receives a simulated synchronous `anchor.click()` exception after a real Node Blob and object URL exist.
- The alert appears synchronously, the reader stays unchanged, the anchor is removed immediately, the URL is revoked on the timer, and no automatic retry occurs.
- An explicit retry after history replacement captures only that replacement and clears the error.
- The actual JSON Blob round-trips duplicates, empty entries, all C0 controls, DEL, BOM, line/paragraph separators, Japanese/emoji, combining text, distinct lone high/low surrogates, and literal markup/escape text.

The two generated files describe an attempted request and a successful simulated dispatch, respectively; neither is an actual browser download. Real React callbacks, Node Blob, object-URL registration/revocation, and timers were exercised. DOM nodes and dispatch were authored simulations. No browser, native DOM, screen reader, browser profile, provider, host, or network request was used.

## Replay

The runner is [review-dispatch-refusal.mjs](review-dispatch-refusal.mjs); [command.json](command.json) preserves the exact executed command and raw-log hash. Supply the qualified UI source with its existing TypeScript/React dependencies and a matching React TestRenderer dependency root that resolves the same React instance. This run used Node 24.19.0, React/TestRenderer 18.3.1 and TypeScript 5.9.3; no product dependency was changed. The runner verifies the four compiled source hashes before use and again before success.

```sh
node review-dispatch-refusal.mjs   /absolute/path/to/packages/uma-sim-ui   /absolute/path/to/matching/react-test-renderer   /absolute/path/to/new-output-directory
```

The output directory must not exist. The runner refuses to overwrite prior evidence. It compiles the actual TypeScript modules in memory, executes the actual React component callbacks, and writes a receipt plus exact JSON Blob bytes. It does not invoke browser tooling.

No production source edits or Git writes were performed. A receiving owner must preserve this boundary when describing browser acceptance.

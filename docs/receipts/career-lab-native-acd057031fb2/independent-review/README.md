# Independent native change review

**Decision: APPROVE for the exact composition and receiving boundary below. No blocking source or verification-command finding remains.** This is a static change review by the artifact receiving worker, independent of the authors of the native renderer, count correction and native regression tests. It does not repeat the earlier browser or API cases.

## Exact reviewed object

The reviewed local composition is commit `f5af3b5f7825138c74b83d337f4c9f8417b636b9`, tree `e87746af0d7ac749cb9c82511588285fc20f6657`. It consists of native candidate `97754679972a3128b5b84cde2159b43a4d85c56e` plus receiving commits `8ae55a80d703a4c1e34ee28062421d23cc97f44e` and `2be516138c9fecd0145c1f150dc6f8176f80bbb5`. Root's independently composed commit `042e475092a60b3bc6b3a982a14f9737654a497d` has the identical complete tree. The comparison base is accepted main `0ac14602addd1a4610aa8359896915a34cce5659`, tree `e7d10088a713d8fbd7391c86050a92898652bdd6`.

`review.json` records source hashes, protected-path equality, the API production-prefix equality and the independently read native gate receipt. This review commit adds evidence only; its own commit hash is consequently distinct from the product tree being reviewed.

## Native semantics

`markdown_text` preserves report data in its existing heading, list item or GFM table cell. It escapes ASCII punctuation, encodes literal ampersands and angle brackets once, and converts CRLF, CR and LF to an explicit `<br>`. The order handles a CRLF pair once and prevents an authored entity or HTML tag from becoming report markup. Backslashes before pipes are escaped in a way that retains literal text without introducing a column separator. The helper is applied to all rendered prose and outcome fields, including the branch names, divergence labels and note, timeline labels, races, learned skills, sparks and caveats. Static headings and column order remain unchanged.

Code-styled checkpoint and divergence metadata use a separate helper because Markdown code spans do not decode prose escapes or entities. Its fence is one backtick longer than the longest run in the data. Padding protects backtick edges and significant leading/trailing spaces, while all-space content follows code-span whitespace semantics. Embedded line breaks intentionally normalize to spaces. Those code spans occur in list metadata, outside GFM tables, so a literal pipe in a code value cannot split a report table. Empty values remain visually empty. The native suite independently supplies 65 delimiter/padding cases across the five code fields; the separate actual API/browser persisted-field case checks their interpretation by a real Markdown consumer.

The count correction selects the timeline length of the side that actually ended. It does not change alignment, action/outcome classification, summary metadata or simulation. Both missing-side directions, zero-length branches, changed continuing lengths, deliberately stale summary counts and earlier-divergence precedence are covered. The actual API receiver independently created two-, three- and two-action branches and checked both the JSON note and rendered Steps cells.

The renderer borrows its comparison and does not rewrite stored or JSON data. Native typed/JSON immutability controls and the received JSON equivalence comparison support this boundary. The intended JSON change from the count fix is confined to the previously incorrect first-divergence note.

## Oracle and command review

The existing native report test and the API module's embedded test now expect escaped hyphens in raw Markdown source. Their names, route flow, content-type and surrounding assertions remain intact. The API change lies wholly within `#[cfg(test)]`; the complete production prefix has the same SHA256 as accepted main. The native report oracle was refined before candidate execution to distinguish literal code spans from prose escaping. Its limited inline/source parsing is stated explicitly and is complemented by the actual marked GFM and Chromium receiver.

The existing PR65 browser runner retains its nine interactions. Its Markdown download assertion now parses the saved file and checks the exact rendered heading and Branch cells instead of searching escaped source substrings. The derivative used for receiving and the existing entry point have the same executable body after their documented repository-path default and comments are normalized. The new `MARKED_MODULE` prerequisite is explicit; no package manifest silently acquires a runtime dependency.

The canonical `scripts/verify-lab-native-ui.mjs` forwards its four positional arguments unchanged to the qualified accepted-UI receiver. The receiver requires an explicit Chromium installation and documents the Playwright/marked module paths, built UI package directory and fresh output directory. It preserves PR65's displayed comparison while the user drafts another pair. The historical clear-on-selection script, its prior measurements and the initial supplemental-oracle failure are preserved under explicitly historical evidence paths; they are not advertised as current product behavior.

The artifact worker authored these receiving harnesses, so this review is not an independent-author review of that portion. The production worker separately reviewed exact receiving commit `2be5161` and returned **APPROVE**, including the wrapper's argument forwarding, explicit dependencies, retained nine interactions, parsed-text oracle and historical evidence boundaries. That second review was static and did not claim another browser run.

## Evidence and source scope

The independent production gate receipt for exact native commit `9775467`, tree `da863f9b1df928ed5bd1ac85642ee67bfd1f4cdc`, records successful formatting, workspace/all-target Clippy and default-feature workspace tests: **354 passed, 0 failed, 71 result blocks**. Its SHA256 is `b942698791f3d5c60da69b1e49011107c2fef793d53e708e594fe75b1e912183`. The complete accepted test log and the earlier raw-heading mismatch, baseline control and storage-exhaustion attempts are retained. This review reads that evidence; it does not claim to have rerun the gates.

The final report receiver records 9/9 cases and 93/93 checks on composed native source SHA256 `49d68441482fa907364292bb25b43fc06be6e95b13937cbbba898a393f4e4935` and executable SHA256 `149600d040481e6eb2530c2a3081d1c680f223d7a16d6bcd54feb247ab50579d`. The shorter-count receiver records 3/3 cases and 18/18 checks. Accepted PR65 UI integration records nine workflow cases plus six adversarial report checks. Exact accepted-0ac UI equality and the one regenerated TypeScript build cache are declared in the receiving source receipts.

Against accepted `0ac`, the reviewed composition changes production behavior only in `career_lab.rs`. The other changed Rust source path contains the one embedded test assertion. UI runtime source, UI manifests/lockfile, Cargo manifests/lockfile and workflows are unchanged. The UI package does contain the declared browser-test oracle adjustment, so a broader claim that every UI-package file is unchanged would be inaccurate.

## Fresh-main continuity and limits

A fresh direct GitHub comparison of `0ac14602` to `0b5cc2342cd93141beb461edf6e83998388e67b8` found only the accepted LibraryPanel generated-import-name label change, its separate checkpoint-portability browser script and associated evidence. This is PR **71**, addressing issue **70**. It changes neither the comparison browser runner nor the comparison UI state/panel or native Rust. The publication composition must retain that accepted work; this review does not authorize overwriting it. Root will record the resulting exact ancestry and final source union separately.

Receiving used the production Vite build and actual native API through a transparent loopback proxy. It qualified marked 17.0.5 and Chromium 153.0.8010.0, including BR-aware literal display. It does not establish identical behavior in every Markdown engine, exact preservation of arbitrary HTML whitespace, an embedded release layout, deployed service, hardware, external policy or performance benchmark. The accepted-0ac browser evidence precedes PR71's separate label change. No MCP connection, session/tunnel/auth repair, or shared publication was performed by this reviewer.

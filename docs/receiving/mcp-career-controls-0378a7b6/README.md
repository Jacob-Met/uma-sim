# MCP catalog and career style receiving

Issue #119; contributor chatgpt-0378a7b6b7c2/msi_product.

MCP career players can obtain actual catalog identities and change the preferred race style of an active or named career. The implementation adds two schema rows and two dispatch cases (13 lines) to the existing adapter. Catalogs use the existing native read route. Style uses the existing mutation route; Auto sends the native empty style to clear a previous preference. It takes no turn, and no better-race-outcome claim is made.

## Source and ownership

The authored source commit is aff7450bb97f3ba85e0c637d3c31363282596a57. The canonical adapter SHA-256 is a6e5ca16f5bf411a7cacc5d43cb944644c375f47fbdb5ce039e13edc2103e4ea. Removing only the recorded additions recovers the prior adapter exactly. Two exact discovery lists add the two new names, two focused test files are new, and the package guide explains usage.

Current-parent composition 827ca7f8cafc0851ae63f5ec244a7b3531828139 receives main38291c1aaf915da17229eca0afbbd58d9b3ce453 (PR116). All six owned files remain byte-identical, and all1582 unowned current-parent leaves/modes are exact. PR116 changes native omitted/empty session resolution. This MCP adapter retains its existing nonempty named-session admission; omission still follows active. The rebuilt native API and full maintained MCP suite qualify this composition.

Root's fresh20:18:12Z native ownership scan found352 readable records and20 denied, including36 UMA/TestPilot matches. No sim_catalog, sim_style or preferredRunningStyle ownership hit was observed. The separate #102 transport, #66 career-lab and #115 training owners remain preserved. This is partial access, not a native lease. The original local claim and public claim publication are retained.

## Actual receiving

- The unchanged native baseline has working catalog/style routes while actual MCP discovery lacks both controls and calls reject them.
- The maintained MCP suite passed156/156, zero skips, on the authored candidate. The canonical LF adapter then passed the focused protocol/native tests. The rebuilt current-parent API passed the full suite again; exact counts and runtime provenance are in composed-gate.log/json.
- New protocol cases cover all five catalog routes, exact style mapping, named/active targeting,19 malformed requests making no HTTP request, and preserved backend errors.
- Native cases select identities returned by the catalog, start a real career, fork/save it, exercise all styles and Auto, and compare complete states/RNG/siblings/checkpoint bytes.
- Root independently accepted the exact authored adapter with eight changed-input native groups: seed-17, Zenno Rob Roy, support20011, factor21028, inherited late preferences and a different active fork. Auto clears inherited preference; repeated Auto is idempotent; missing/invalid targets preserve state. The independent subtree is copied byte for byte.

## Retained corrections and limits

The first candidate native receiver used a Unicode fork ID that native admission refuses and missed one existing exact discovery list. The second incorrectly expected Auto to insert null; native Auto removes the optional preferredRunningStyle field. Both original failures, fixtures and corrections remain. Production adapter bytes were unchanged by those receiver corrections.

The original Windows checkout used CRLF. Four owned files were restored to canonical LF before committing, with exact raw-to-canonical mapping and original tested raw bytes retained. The later canonical tests and current-parent suite use the committed bytes.

Native build/tests ran on MSI with Node24.19.0 and Rust/Cargo1.99.0, in an isolated checkout and private API processes. There was no installed-service mutation, package-registry publication, scientific validation or race-strategy quality measurement.

## Files

Top-level scripts, receipts and complete gate logs retain reproducible commands and chronology. root-independent-20261008T2020Z is exact independent evidence. native-runs.json.gz is a gzip JSON envelope of every baseline/candidate/canonical/composed native-run file: each entry records its relative path, byte length, SHA-256 and base64 bytes. This avoids duplicating many small generated snapshots in the tree without discarding originals. Decompress and decode entries to restore them; compare each hash before use. manifest.json pins all packet files except itself.

Hosted CI and integration are required after this packet is committed. Their exact final-head results belong in the PR/issue integration receipt, not in these prepublication observations.

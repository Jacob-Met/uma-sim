# Exact provenance classification

The official Gitleaks 8.24.3 Darwin arm64 executable was downloaded into this worker's private receiving directory and verified against its official release checksum. The release archive and installed system tools were not changed; only the bounded local scanner was executed.

The initial actual Git scan of fe2e6fdc..97d3423 found four generic-api-key matches in the 1.46 MB qualified source/evidence history. Each was a verified Git object identity: one original source blob and the two pinned source commits. The raw redacted report and log remain here. No credential was removed or replaced in the evidence.

| Exact evidence path (relative to the qualification directory) | Verified whole Git object value |
| --- | --- |
| README.md | 4ff7034bc5701d793cd93d645d89d40d73fc3273 |
| independent-current-fe2/source-manifest.json | fe2e6fdcdf7625cb5beac0e0062ace0360288a02 |
| independent-current-fe2/api-current-impact.json | b8e3da0c6e152f77aa5c3e083ffa7b248bd74f8c; fe2e6fdcdf7625cb5beac0e0062ace0360288a02 |

Three per-rule allowlists extend the existing generic-api-key rule. Every block requires both an anchored full file path and that path's anchored whole verified value. The existing PR72 exceptions and all built-in rules remain intact. There are no directory exclusions, broad hexadecimal patterns, line suppressions, baselines, or disabled rules.

Actual scanner controls prove that the four exact provenance values pass at the three exact paths. A second control changes each path's value, moves one allowed value to a sibling path, and puts an intentionally unusable synthetic private-key-shaped fixture at an allowed path. All five negative findings remain detected: four by generic-api-key and one by private-key. The fixture contains no usable key material. The driver and redacted reports make the path/value/rule boundaries reproducible.

With this exact classification loaded, the same actual candidate history scans with zero findings. assessment.json distinguishes that pre-freeze source head and config identity from later evidence commits. Actual final-head scan and hosted CI remain separate recorded gates; this report does not treat synthetic detector controls as product failures.

The authored current build receipt also clarifies that only Rust/Cargo build inputs remained unchanged after the API build; the terminal and evidence overlay was applied afterward. The original and current independent packets and all raw native captures remain byte-identical.

All scanner reports use complete redaction. The copied driver writes only a fresh private control repository and refuses reuse, so a repeat should use a separate receiving namespace. Compiler caches were not inputs to these scanner controls.

# Exact-seed MCP independent receiving capsule

**PASS for native source `ec96217c590087b645d2715ee1c84fd5768563a4`**, tree `7597d189f1a914e0cb179af9914652cd627cf91c`, parent `b026a21f437789d7a7cd04c5074258302e36814e`. The runtime entry is SHA256 `3d5f61a368bae5b4bf91ba839b925820ef949a4bb9274dcf7e9d91e9cc02a736`.

The independent receiver passed **107/107 actual stdio protocol controls and 45/45 actual native groups**, with no candidate failures, skips, or receiver errors. Seven signed-seed cases retain exact native checkpoint text, native importability, restored state, and actual race/training continuation with exact RNG words and call counts. Refusal cases preserve the existing private checkpoint files and live sibling. All 1,306 unrelated source-parent leaves remain exact.

## Contents

- `REVIEW.md` is the exact README from the sealed native packet. It explains the frozen independent oracles, results, source fence, retained baseline failures, receiver framing correction, custody failure, and limits.
- `native-review.json.gz` contains the complete packet: **296 manifested files plus its manifest**, including executable receiving scripts, exact source copies, raw native documents, wire recordings, process logs, source comparison, and all historical negatives. The gzip payload is a JSON envelope of relative paths, modes, and base64 bytes; it does not contain the separately retained native API binary.
- `manifest.json` is the exact manifest for the files inside that archive, not a claim that every packet file is expanded in this Git directory.
- `review-result.json` and `packet-receipt.json` are readable exact native receipts.
- `source-commit-binding.json` is a later independent addition. It verifies that all six reviewed source blobs were committed unchanged in native `ec96217c`; it does not rewrite the earlier immutable packet.
- `unpack-review.mjs` checks the complete capsule and can materialize it in a new directory.

Archive SHA256: `19c8fd460cda9c03efd32b696ffa18286b67aea6f879dc7706334abf1dd7d316`; 638,323 bytes. Manifest SHA256: `b662798f8ed9dc4759ed16d7eb39d084bf3e58f7279a507ae043a25203764c3d`. Native archive Git blob: `5c35b13629637be52110499110b3eb497629ee1c`.

## Read or replay

No package installation is needed. First verify the archive without writing files:

```sh
node unpack-review.mjs
```

To materialize every packet file with its recorded mode:

```sh
node unpack-review.mjs --out /absolute/path/to/a-new-review-directory
```

The destination must not already exist. Follow the packet's README for the original protocol/native commands. Native replay requires the pinned API binary and canonical source/catalog files; this capsule does not download or rebuild them.

Source owner: [uma-sim #102](https://github.com/Jacob-Met/uma-sim/issues/102). [Independent receiving handoff](https://github.com/Jacob-Met/uma-sim/issues/102#issuecomment-6063973529).

This evidence commit changes only this documentation subtree. It does not move the author's source branch or claim a later-main build, hosted Node 20/CI, installed adoption, or merge. The original author owns current-parent composition and integration. The ThinkPad Conscience route was unavailable during the independent reservation, so the exact external reservation and completed receiving handoff are used without inventing a native event sequence.

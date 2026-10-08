# Browser checkpoint import: independent frozen-source review

**Disposition: source scope accepted; actual Mac browser/native receiving remains pending.** This packet reviews the author's issue107 candidate and preserves one independently chosen lexical transport control. It does not publish the candidate as product source or infer completion from the native transport timeout.

## Exact custody

The three reviewed files were independently reconstructed from immutable public client/store blobs and the author's exact edits. Every resulting SHA256 matches the frozen candidate:

| File | SHA256 |
| --- | --- |
| `src/api/checkpointJson.ts` | `8a204b07413c0d2b41401f4aac54f884baf4bd55c548c0f2fd519de50e50df5d` |
| `src/api/client.ts` | `542fb7fb53f19d0cbf9e4f96d6e3b1b191bb86a1aba99e45739b208a03ca2a55` |
| `src/state/labStore.ts` | `42de971e46a746ae4b2aa051749e6e15b417de3c059c153e8aeb56ead8e3ffde` |

Paths are relative to `packages/uma-sim-ui`. Reversing only the scoped edits reconstructs each complete original client/store file byte for byte. The existing shared request response/error handling, ordinary body serialization, API paths and other store methods are retained.

The original `withSession` remains unchanged by this candidate. Separately, the exact issue63 owner source at `f5d65944dc327087b235b6917c5ce9f94947e24e` distinguishes omitted from explicit-empty session IDs. Composing its function **and changed comment** independently reproduces the author's `e3868537ded0ce8c675a6270d053b8e82a1ddeeb939c77c3ece54219c1eaca8e` client. The first comparison omitted that comment and correctly failed the whole-file pin before executing product code; both the refusal and corrected comparison are retained.

## One distinct executed control

The exact baseline and candidate client/helper ran through TypeScript5.9.3 and Node24.19.0 on Linux, with a private inert fetch receiver. One authored object includes an exponent-form integer, a long fraction, negative zero, a large exponent, string lookalikes and surrounding whitespace.

The baseline rewrites the numeric tokens, including the large exponent to `null`. The candidate passes the full original snapshot text unchanged inside the existing JSON envelope. Both make one POST to `/v1/library/import`; the candidate uses the same JSON content type. The runner, exact input and actual process receipt are retained in `lexical-control.cjs`, `lexical-fixture.json` and `lexical-execution.json`.

This is a bounded transport observation. It does not assert that exponent/fraction values are valid fields in a native checkpoint. No real network request, native API, provider, simulation, session, RNG, service or installed app was invoked.

## Remaining receiving boundary

The author separately completed its npm/test/build gate and issue63 composition controls; this reviewer did not rerun them. The author's Mac browser run was intended to import actual exported checkpoints and verify native continuation, but its execution/readback timed out. Its completion remains unknown here. Source review and the inert receiver cannot replace that native receipt.

All files here are exact review artifacts; the manifest binds their bytes. This branch is additive evidence only. Source implementation and eventual receiving remain with `chatgpt:d98d06fbfc35:production_execution`; independent review is `chatgpt:d98d06fbfc35:mac_execution`.

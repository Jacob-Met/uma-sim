# Skills and hints

The Run tab shows a **Skills & hints** panel whenever a career is loaded, including a completed career or a resumed checkpoint. It displays every retained entry. Long lists scroll and can be focused with the keyboard.

The panel separates three kinds of information:

- **Learned skills** come directly from the snapshot's `learnedSkillIds`, retaining their order and any duplicate entries.
- **Skill hints** are entries in `hintLevels` with an explicit `skill:` key. A hint can also exist for a learned skill; the two lists represent separate snapshot fields.
- **Training & other hints** retain facility keys, generic hints, and unresolved event names. Familiar training keys receive readable labels, while the original keys remain visible.

Names and descriptions come from the repository's canonical skill catalog. Display names prefer a nonempty official English name, then the fan English name, then the Japanese name. Missing catalog entries keep their literal IDs. Catalog descriptions provide reference text; this view does not compute purchases, prices, hint discounts, availability, or a skill's realized effect in the simulation.

If the catalog request fails or the server returns an older HTML fallback, the panel keeps showing retained IDs and hint levels. **Retry names** retries only the catalog read. Resuming another career replaces the displayed skill and hint lists with that career's snapshot.

## Read-only API

`GET /v1/catalog/skills` returns an `items` array in stable numeric skill-ID order. Each item has:

| Field | Value |
| --- | --- |
| `id` | Canonical `skill:<number>` key |
| `name` | Catalog display name, or `null` |
| `description` | Catalog English description, or `null` |

The endpoint is available before starting a career. It uses the existing catalog initialization and does not create or advance a career. Unsupported methods return HTTP 405. Missing catalog data yields an empty array, allowing clients to keep their literal-ID fallback.

## Verification

Run the focused Rust catalog tests and the UI's normal test/build commands:

```sh
cargo test --locked -p uma-sim-core --test skill_catalog_display --test skill_hint_routing
cargo build --locked -p uma-sim-core
cd packages/uma-sim-ui
npm ci
npm test
npm run build
```

The native browser receiver launches its own API process and disposable storage, then tests the built application against it. From the repository root, with Chromium installed for Playwright:

```sh
node packages/uma-sim-ui/tests/receiving/skills-hints-receiver.mjs \
  packages/uma-sim-ui/dist /tmp/uma-skills-receiving target/debug/uma-sim-api
```

An optional fourth argument selects a Chromium executable. The receiver checks actual catalog data and an initial career, full snapshot/RNG equality across reads, native checkpoint import/resume, long and empty retained lists, literal text rendering, mobile layout, catalog fallback/retry, and pending-request cancellation. Its large completed-career checkpoint is an authored display fixture, not a claimed simulation outcome.

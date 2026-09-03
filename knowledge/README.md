# Uma Musume Global Knowledge Base

Canonical, versioned game knowledge for Global (EN). Bot `src/data/*.json` is a **generated export**, not the source of truth.

## Layout

| Path | Purpose |
|------|---------|
| `schema/` | Entity JSON Schema + ID conventions |
| `glossary/` | Official Global terminology |
| `scenarios/` | Per-scenario deep mechanics |
| `mechanics/` | Training/race formulas |
| `ingest/` | Fetch + normalize pipelines |
| `exports/` | Emit bot JSON, RAG chunks, instruction-tune JSONL |
| `raw/` | Cached GameTora hashes/payloads (gitignored) |
| `canonical/` | Normalized entity stores (generated) |

## Principles

1. **Namespaced IDs** — never use bare English display names as primary keys (`skill:200011`, `support:30072`, `trainee:100602`).
2. **Dual English** — `name_en_official` vs `name_en_fan` (Global localization diverges).
3. **Provenance** — every fact: `source`, `content_hash`, `as_of`, `server` (`global` | `jp`).
4. **No silent JP→Global backfill** — skill effects differ by server; mark gaps explicitly.
5. **Global gating** — prefer `release_en` / `active_en` / `playable_en` / `scenariosByServer.en`.

## Quick start

```powershell
# From repo root, with venv activated
python knowledge/ingest/gametora_fetch.py
python knowledge/ingest/consolidate_local.py
python knowledge/exports/export_all.py
```

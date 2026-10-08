# Knowledge base release workflow

1. **Ingest:** `python knowledge/ingest/gametora_fetch.py`
2. **Validate:** `python knowledge/validate/validate.py` (schema + cross-refs)
3. **Research sync:** update `research/*.json` when formulas change
4. **Sim calibration:** `python scripts/calibrate_sim.py` (telemetry fit)
5. **Content packs:** drop JSON under `content_packs/`; sim loads at startup when wired

Canonical entities live in `knowledge/canonical/by_kind/`. Bump `schema_version` in research files when breaking changes occur.

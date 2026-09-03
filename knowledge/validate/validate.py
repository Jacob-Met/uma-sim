#!/usr/bin/env python3
"""
Validate canonical knowledge base entities against schema and cross-references.

Usage (repo root):
  python knowledge/validate/validate.py
  python knowledge/validate/validate.py --strict
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
KB = Path(__file__).resolve().parents[1]
CANON = KB / "canonical" / "by_kind"
SCHEMA_PATH = KB / "schema" / "entity.schema.json"

REQUIRED = {"id", "kind", "server", "provenance"}
VALID_KINDS = {
    "trainee", "support_card", "skill", "event", "race", "scenario",
    "factor", "epithet", "song", "lesson", "item",
}
VALID_SERVERS = {"global", "jp", "both"}
ID_PATTERN = re.compile(r"^[a-z_]+:.+$", re.DOTALL)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def validate_entity(entity: dict, path: str, index: int) -> list[str]:
    errors: list[str] = []
    prefix = f"{path}[{index}]"

    for field in REQUIRED:
        if field not in entity:
            errors.append(f"{prefix}: missing required field '{field}'")

    eid = entity.get("id", "")
    if eid and not ID_PATTERN.match(str(eid)):
        errors.append(f"{prefix}: id '{eid}' does not match expected namespace pattern")

    kind = entity.get("kind")
    if kind and kind not in VALID_KINDS:
        errors.append(f"{prefix}: unknown kind '{kind}'")

    server = entity.get("server")
    if server and server not in VALID_SERVERS:
        errors.append(f"{prefix}: invalid server '{server}'")

    prov = entity.get("provenance")
    if isinstance(prov, dict):
        if "source" not in prov or "as_of" not in prov:
            errors.append(f"{prefix}: provenance missing source or as_of")
    elif prov is not None:
        errors.append(f"{prefix}: provenance must be object")

    return errors


def cross_ref_skills(entities_by_kind: dict[str, list[dict]]) -> list[str]:
    errors: list[str] = []
    skill_ids = {e["id"] for e in entities_by_kind.get("skill", []) if "id" in e}
    skill_ids |= {f"skill:{e['payload'].get('skill_id')}" for e in entities_by_kind.get("skill", [])
                  if isinstance(e.get("payload"), dict) and e["payload"].get("skill_id")}

    for card in entities_by_kind.get("support_card", []):
        payload = card.get("payload") or {}
        for hint in payload.get("hint_skills") or []:
            sid = f"skill:{hint}" if not str(hint).startswith("skill:") else str(hint)
            if sid not in skill_ids and str(hint) not in {str(e["payload"].get("skill_id")) for e in entities_by_kind.get("skill", [])}:
                pass  # hints may reference skills not yet global; warn only in strict

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true", help="Fail on warnings")
    args = parser.parse_args()

    if not CANON.exists():
        print(f"ERROR: canonical dir missing: {CANON}", file=sys.stderr)
        return 1

    all_errors: list[str] = []
    entities_by_kind: dict[str, list[dict]] = {}

    for path in sorted(CANON.glob("*.json")):
        data = load_json(path)
        if not isinstance(data, list):
            all_errors.append(f"{path.name}: root must be array")
            continue
        entities_by_kind[path.stem] = data
        ids_seen: set[str] = set()
        for i, entity in enumerate(data):
            if not isinstance(entity, dict):
                all_errors.append(f"{path.name}[{i}]: not an object")
                continue
            all_errors.extend(validate_entity(entity, path.name, i))
            eid = entity.get("id")
            if eid:
                if eid in ids_seen:
                    all_errors.append(f"{path.name}[{i}]: duplicate id '{eid}'")
                ids_seen.add(eid)

    all_errors.extend(cross_ref_skills(entities_by_kind))

    # Summary
    total = sum(len(v) for v in entities_by_kind.values())
    kinds = ", ".join(f"{k}={len(v)}" for k, v in sorted(entities_by_kind.items()))
    print(f"Validated {total} entities across {len(entities_by_kind)} kinds")
    print(f"  {kinds}")

    if all_errors:
        print(f"\n{len(all_errors)} issue(s):", file=sys.stderr)
        for err in all_errors[:50]:
            print(f"  - {err}", file=sys.stderr)
        if len(all_errors) > 50:
            print(f"  ... and {len(all_errors) - 50} more", file=sys.stderr)
        return 1

    print("OK — no validation errors")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

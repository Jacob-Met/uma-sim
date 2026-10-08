import json

skills = json.load(open("knowledge/canonical/by_kind/skill.json", encoding="utf-8"))
by = set()
for s in skills:
    by.add(str(s["payload"]["skill_id"]))
    g = s["payload"].get("gene_version")
    if g and g.get("id") is not None:
        by.add(str(g["id"]))

c = next(
    x
    for x in json.load(open("research/race_checkpoint_v3_sample_1000.json", encoding="utf-8"))[
        "cases"
    ]
    if x["id"] == "v3_394_10914_modeled_only"
)
for sid in c["skills"]:
    print(sid, "OK" if sid in by else "MISSING")

c2 = next(
    x
    for x in json.load(open("research/race_checkpoint_v3_sample_1000.json", encoding="utf-8"))[
        "cases"
    ]
    if x["id"] == "v3_690_10810_modeled_only"
)
print("---690---")
for sid in c2["skills"]:
    print(sid, "OK" if sid in by else "MISSING")

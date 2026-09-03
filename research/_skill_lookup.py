import json

skills = json.load(open("knowledge/canonical/by_kind/skill.json", encoding="utf-8"))
by = {}
for s in skills:
    by[str(s["payload"]["skill_id"])] = s
    g = s["payload"].get("gene_version")
    if g:
        gid = g.get("id") or g.get("skill_id")
        if gid is not None:
            by[str(gid)] = {"payload": g, "name_en_fan": s.get("name_en_fan")}

for i in ["200352", "202662", "202181", "201401", "100791", "101171", "202711", "900711"]:
    s = by.get(i)
    print("===", i, (s or {}).get("name_en_fan") or (s or {}).get("name_en_official"))
    if not s:
        print("  MISSING in skill.json")
        continue
    p = s["payload"]
    for g in p.get("condition_groups") or []:
        print("  pre", g.get("precondition"))
        print("  cond", g.get("condition"))
        print("  time", g.get("base_time"), "eff", g.get("effects"))

d = json.load(
    open(
        r"C:\Programming\umalator-ref\uma-tools\uma-skill-tools\data\skill_data.json",
        encoding="utf-8",
    )
)
for i in ["200352", "202662", "100791", "101171", "900711", "202711"]:
    print("UMA", i, json.dumps(d.get(i), ensure_ascii=False)[:320] if i in d else "MISSING")

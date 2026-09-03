import json
from pathlib import Path

sample = {c["id"]: c for c in json.load(open("research/race_checkpoint_v3_sample_1000.json", encoding="utf-8"))["cases"]}
fails = json.load(open("research/race_checkpoint_v3_triage_1000.json", encoding="utf-8"))["failures"]
for f in fails:
    c = sample[f["id"]]
    print(f["id"], "pace", c["pace_effects"], "g", c["ground"], "strat", c["horse"]["strategy"],
          "spd", c["horse"]["speed"], "ours-exp", round(f["ours"]-f["expected"], 4),
          "nskills", len(c["skills"]), "course", c["course_id"])

import pathlib,json,hashlib,time,difflib
root=pathlib.Path("/dev/shm/c6cc965b64c2-terminal-replacement-09df-v2")
prior=pathlib.Path("/dev/shm/c6cc965b64c2-terminal-replacement-09df-v1")
h=lambda data:hashlib.sha256(data).hexdigest()
manifest=json.loads((root/"source-manifest.json").read_text())
before=json.loads((prior/"native-input-readback-before.json").read_text())
donor=pathlib.Path(before["donor_root"]);after=[]
for f in before["files"]:
 data=(donor/f["path"]).read_bytes();sha=h(data)
 assert sha==f["sha256"],"donor changed "+f["path"]
 after.append({"path":f["path"],"git_blob":hashlib.sha1(b"blob "+str(len(data)).encode()+b"\0"+data).hexdigest(),"sha256":sha,"bytes":len(data)})
binary=pathlib.Path(manifest["binary"]["path"])
assert h(binary.read_bytes())==manifest["binary"]["sha256"]
for f in manifest["files"]:
 for base in [pathlib.Path(manifest["source_repo"]),root/"source"]:
  assert h((base/f["path"]).read_bytes())==f["sha256"],str(base/f["path"])
assert h((root/"baseline/packages/uma-sim-cli/tui.js").read_bytes())==manifest["baseline_tui"]["sha256"]
post={"schema":"hamon.uma.replacement.native-input-after.v1","source_main":manifest["source_parent"],"files":after,"all_120_donor_inputs_unchanged":True,"all_11_staged_inputs_unchanged":True,"baseline_terminal_unchanged":True,"binary":manifest["binary"],"binary_unchanged":True}
(root/"native-input-readback-after.json").write_text(json.dumps(post,indent=2)+"\n")
old=(root/"prior-career-lab.rs").read_text();new=(root/"source/uma-sim-core/src/career_lab.rs").read_text()
(root/"career-library-current.diff").write_text("".join(difflib.unified_diff(old.splitlines(keepends=True),new.splitlines(keepends=True),fromfile="fe2/uma-sim-core/src/career_lab.rs",tofile="09df/uma-sim-core/src/career_lab.rs")))
for name,groups in [("run-v1/receipt.json",16),("admission-boundary-v1/receipt.json",3)]:
 q=json.loads((root/name).read_text());assert q["qualified"] and q["summary"]["passed"]==groups and not q["summary"]["failures"]
assert json.loads((root/"cross-platform-snapshot-comparison.json").read_text())["all_full_state_snapshots_equal"]
paths=["README.md","source-manifest.json","native-input-readback-after.json","driver-port-v2.json","receive-v1.mjs","admission-boundary-v1.mjs","run-v1/receipt.json","run-v1.log","admission-boundary-v1/receipt.json","admission-boundary-v1.log","career-library-current.diff","compare-snapshots.cjs","cross-platform-snapshot-comparison.json","prior-mac-snapshots.json","freeze-receiving-v1.py"]
sources=[(root/p,p) for p in paths]
sources.extend((p,str(p.relative_to(root))) for p in sorted((root/"run-v1/artifacts").iterdir()) if p.is_file())
for p in ["native-input-readback-before.json","staging-receipt.json","driver-staging-correction.json","stage-v1.py","run-v1/receipt.json","run-v1.log","admission-boundary-v1/receipt.json","admission-boundary-v1.log","receive-v1.mjs","admission-boundary-v1.mjs","receive-v1.mjs.preparation-extra-newline","admission-boundary-v1.mjs.preparation-extra-newline"]:
 dest="preparation/"+p if p!="native-input-readback-before.json" else p
 sources.append((prior/p,dest))
files=[]
for src,dest in sources:
 data=src.read_bytes();files.append({"path":dest,"content":data.decode(),"bytes":len(data),"sha256":h(data)})
inventory={"schema":"hamon.uma.replacement.independent_manifest.v1","created_utc":time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime()),"current_main":manifest["source_parent"],"binary":manifest["binary"],"results":{"terminal_groups":16,"admission_groups":3,"terminal_processes":34,"native_api_processes":3,"http_requests":395,"full_state_comparisons":7},"donor_source_files":120,"staged_source_files":11,"files":[{k:v for k,v in f.items() if k!="content"} for f in files]}
data=(json.dumps(inventory,indent=2)+"\n").encode();(root/"evidence-manifest.json").write_bytes(data)
files.append({"path":"evidence-manifest.json","content":data.decode(),"bytes":len(data),"sha256":h(data)})
packet={"schema":"hamon.uma.terminal_replacement.independent_packet.v1","files":files}
data=(json.dumps(packet,separators=(",",":"))+"\n").encode();(root/"receiving-packet.json").write_bytes(data)
meta={"path":str(root/"receiving-packet.json"),"sha256":h(data),"bytes":len(data),"files":len(files),"decoded_bytes":sum(f["bytes"] for f in files),"core_hashes":{f["path"]:f["sha256"] for f in files if f["path"] in ["README.md","evidence-manifest.json","run-v1/receipt.json","admission-boundary-v1/receipt.json","native-input-readback-after.json","driver-port-v2.json"]},"native_api":manifest["binary"],"main":manifest["source_parent"]}
(root/"receiving-packet-meta.json").write_text(json.dumps(meta,indent=2)+"\n")
print(json.dumps(meta))

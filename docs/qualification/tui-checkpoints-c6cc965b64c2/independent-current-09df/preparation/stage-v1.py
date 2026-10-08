import pathlib,json,hashlib,os,subprocess,time
root=pathlib.Path("/dev/shm/c6cc965b64c2-terminal-replacement-09df-v1")
packet=json.loads((root/"staging-packet.json").read_text())
donor=pathlib.Path(packet["composition"]["native_donor_root"])
composition=pathlib.Path(packet["source_repo"])
overrides={f["path"] for f in packet["files"]}
def mirror(src,dst,prefix=""):
 dst.mkdir()
 for p in src.iterdir():
  rel=prefix+p.name
  if not prefix and p.name in [".git","target"]:continue
  if any(x==rel or x.startswith(rel+"/") for x in overrides):
   if p.is_dir():mirror(p,dst/p.name,rel+"/")
  else:(dst/p.name).symlink_to(p,target_is_directory=p.is_dir())
mirror(donor,composition)
for f in packet["files"]:
 data=f["content"].encode()
 assert hashlib.sha256(data).hexdigest()==f["sha256"],f["path"]
 assert hashlib.sha1(b"blob "+str(len(data)).encode()+b"\0"+data).hexdigest()==f["git_blob"]
 for base in [composition,root/"source"]:
  p=base/f["path"];p.parent.mkdir(parents=True,exist_ok=True)
  assert not p.is_symlink(),str(p)
  p.write_bytes(data)
baseline=root/"baseline/packages/uma-sim-cli/tui.js";baseline.parent.mkdir(parents=True)
baseline.write_text(packet["baseline_tui"]["content"])
assert hashlib.sha256(baseline.read_bytes()).hexdigest()==packet["baseline_tui"]["sha256"]
assert hashlib.sha256(pathlib.Path(packet["binary"]["path"]).read_bytes()).hexdigest()==packet["binary"]["sha256"]
for f in packet["native_input_readback"]["files"]:
 data=(donor/f["path"]).read_bytes()
 assert hashlib.sha256(data).hexdigest()==f["sha256"],"donor changed "+f["path"]
manifest={k:v for k,v in packet.items() if k!="native_input_readback"}
manifest["files"]=[{k:v for k,v in f.items() if k!="content"} for f in packet["files"]]
manifest["baseline_tui"]={k:v for k,v in packet["baseline_tui"].items() if k!="content"}
(root/"source-manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
(root/"native-input-readback-before.json").write_text(json.dumps(packet["native_input_readback"],indent=2)+"\n")
receipt={"created_utc":time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime()),"composition":packet["composition"],"binary":packet["binary"],"source_files":len(packet["files"]),"actual_donor_native_inputs_exact":len(packet["native_input_readback"]["files"]),"scripts":{},"source_manifest_sha256":hashlib.sha256((root/"source-manifest.json").read_bytes()).hexdigest(),"native_builds":0,"donor_writes":0}
for name in ["receive-v1.mjs","admission-boundary-v1.mjs"]:
 receipt["scripts"][name]=hashlib.sha256((root/name).read_bytes()).hexdigest()
(root/"staging-receipt.json").write_text(json.dumps(receipt,indent=2)+"\n")
print(json.dumps(receipt))


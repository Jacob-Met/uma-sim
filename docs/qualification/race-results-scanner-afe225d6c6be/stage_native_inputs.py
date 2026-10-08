import json, pathlib, hashlib, subprocess, sys
root=pathlib.Path("/var/tmp/hamon-gitleaks-afe225d6c6be-yy3nog5g")
payload=json.loads((root/"receiving-payload.json").read_text())
source=root/"source"
source.mkdir()
def blob(data):
    return hashlib.sha1(f"blob {len(data)}\0".encode()+data).hexdigest()
for relative,item in payload["files"].items():
    p=pathlib.PurePosixPath(relative)
    assert not p.is_absolute() and ".." not in p.parts
    data=item["content"].encode()
    assert blob(data)==item["git_blob"],relative
    out=source.joinpath(*p.parts)
    out.parent.mkdir(parents=True,exist_ok=True)
    with out.open("xb") as f:f.write(data)
    out.chmod(0o400)
policies=root/"policies"
policies.mkdir()
for name,content in payload["policies"].items():
    out=policies/(name+".toml")
    out.write_text(content)
    out.chmod(0o400)
bindings=root/"source-bindings.json"
bindings.write_text(json.dumps(payload["bindings"],indent=2)+"\n")
logs=root/"original-hosted-logs"
logs.mkdir()
for name,content in payload["hosted_logs"].items():
    (logs/(name+".log")).write_text(content)
assert blob((policies/"current.toml").read_bytes())=="a23640b909b35f001fac2680b53d752a5c3f9e49"
print(json.dumps({"staged_files":len(payload["files"]),"driver_sha256":hashlib.sha256((root/"review_gitleaks.py").read_bytes()).hexdigest(),"candidate_sha256":hashlib.sha256((policies/"candidate.toml").read_bytes()).hexdigest()}),flush=True)
argv=[sys.executable,str(root/"review_gitleaks.py"),"--source",str(source),"--bindings",str(bindings),"--published-config",str(policies/"published.toml"),"--current-config",str(policies/"current.toml"),"--candidate-config",str(policies/"candidate.toml"),"--scanner",str(root/"release/gitleaks"),"--work",str(root/"native-review-r1")]
proc=subprocess.Popen(argv,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
with (root/"native-review-r1-console.log").open("x") as out:
    for line in proc.stdout:
        out.write(line);out.flush();print(line,end="",flush=True)
rc=proc.wait()
print(json.dumps({"native_driver_exit_code":rc}),flush=True)
raise SystemExit(rc)

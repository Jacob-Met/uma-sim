import pathlib,json,hashlib,subprocess
repo=pathlib.Path("/Users/me/Developer/uma-terminal-durable-c6cc965b64c2/receiving/current-fe2e6fdc")
commit="92db1618cc014924e43a69b2dfe4d5891680b176"
parent="fe2e6fdcdf7625cb5beac0e0062ace0360288a02"
def git(*args):return subprocess.run(["/usr/bin/git","-C",str(repo),*args],capture_output=True,check=True).stdout
assert git("rev-parse","HEAD").decode().strip()==commit
assert git("rev-parse","HEAD^{tree}").decode().strip()=="4ee456989b96c3557f0c167f6d53ccc6631b6cd2"
assert git("rev-parse","HEAD^").decode().strip()==parent
paths=["packages/uma-sim-cli/tui.js","packages/uma-sim-cli/package.json","packages/uma-sim-cli/README.md","uma-sim-core/src/api.rs","uma-sim-core/src/career_lab.rs",".github/workflows/ci.yml","uma-sim-core/tests/api_body_admission.rs","Cargo.lock","Cargo.toml","uma-sim-core/Cargo.toml"]
files=[]
for name in paths:
 b=(repo/name).read_bytes()
 blob=git("rev-parse",commit+":"+name).decode().strip()
 assert b==git("show",commit+":"+name),name+" worktree differs"
 f={"path":name,"sha256":hashlib.sha256(b).hexdigest(),"git_blob":blob,"bytes":len(b)}
 if name in [".github/workflows/ci.yml","Cargo.lock","Cargo.toml","uma-sim-core/Cargo.toml"]:f["content"]=b.decode()
 files.append(f)
binary=repo/"target/debug/uma-sim-api"
binary_sha=hashlib.sha256(binary.read_bytes()).hexdigest()
assert binary_sha=="b5f4c2c7af89751b38a2be4a646c6a2b0a183d376002394e80b485f9c81709a4"
print(json.dumps({"repo":str(repo),"source_commit":commit,"source_parent":parent,"tree":"4ee456989b96c3557f0c167f6d53ccc6631b6cd2","binary":{"path":str(binary),"sha256":binary_sha},"files":files,"changed_from_parent":git("diff","--name-status",parent,commit).decode(),"ci_diff":git("diff",parent,commit,"--",".github/workflows/ci.yml").decode(),"owner_rust_diff":git("diff",parent,commit,"--","uma-sim-core","Cargo.toml","Cargo.lock").decode(),"status":git("status","--short").decode()}))

import os,json,subprocess,datetime,hashlib
from pathlib import Path
r=Path("/dev/shm/uma-checkpoint-source-receiving-d98d06fb-20261008/root-merge-v1");r.mkdir(mode=0o755,parents=True,exist_ok=False)
gitdir=r/"graph.git"
env={"PATH":"/usr/bin:/bin","LANG":"C","GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null","GIT_TERMINAL_PROMPT":"0"}
records=[];passed=False
try:
 for argv,limit in [(["/usr/bin/git","init","--bare",str(gitdir)],15),(["/usr/bin/git","-C",str(gitdir),"-c","maintenance.auto=false","-c","gc.auto=0","fetch","--filter=blob:none","--depth=2","--no-tags","--no-write-fetch-head","https://github.com/Jacob-Met/uma-sim.git","618ad8f9702a39d23965e0ff68065822629bb9ba"],60),(["/usr/bin/git","-C",str(gitdir),"merge-base","--is-ancestor","8d74413d7dbc3caf1745779c92d56b6eb7feea42","618ad8f9702a39d23965e0ff68065822629bb9ba"],10),(["/usr/bin/git","-C",str(gitdir),"show","-s","--format=%H%n%T%n%P","618ad8f9702a39d23965e0ff68065822629bb9ba"],10)]:
  start=datetime.datetime.now(datetime.timezone.utc).isoformat();p=subprocess.run(argv,env=env,capture_output=True,timeout=limit)
  rec={"argv":argv,"started_at":start,"finished_at":datetime.datetime.now(datetime.timezone.utc).isoformat(),"returncode":p.returncode,"stdout":p.stdout.decode(),"stderr":p.stderr.decode()};records.append(rec)
  if p.returncode:raise RuntimeError("native Git graph gate refused")
 need="618ad8f9702a39d23965e0ff68065822629bb9ba\n8457386cf41c5530b39dec1fa6be127dfc430d01\n8d74413d7dbc3caf1745779c92d56b6eb7feea42\n"
 if records[-1]["stdout"]!=need:raise RuntimeError("native Git source parent/tree mismatch")
 passed=True
except BaseException as e:error={"type":type(e).__name__,"message":str(e)}
receipt={"passed":passed,"records":records,"error":None if passed else error,"no_worktree_checkout":True,"no_repository_source_or_remote_ref_mutation":True}
b=(json.dumps(receipt,sort_keys=True,indent=2)+"\n").encode()
with (r/"git-ancestor.json").open("xb") as f:f.write(b);f.flush();os.fsync(f.fileno())
if (r/"git-ancestor.json").read_bytes()!=b:raise RuntimeError("ancestor receipt readback")
print(json.dumps({"root":str(r),"receipt_sha256":hashlib.sha256(b).hexdigest(),"result":receipt},sort_keys=True))

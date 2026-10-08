import os,json,subprocess,hashlib,shutil,time,datetime
from pathlib import Path
r=Path('/Users/me/uma-conditions-20261008-5f566b5ec8ef');s=r/'source';ev=r/'evidence'
freeze=json.loads((ev/'source-freeze-v1.json').read_text())
def digest(p):
 data=p.read_bytes();return {'size':len(data),'sha256':hashlib.sha256(data).hexdigest()}
for f in freeze['files']:assert digest(s/f['path'])['sha256']==f['sha256'],f['path']
manifest=json.loads((r/'current-source-manifest.json').read_text())
inputs=[x for x in manifest['leaves'] if x['path'] in ('Cargo.toml','Cargo.lock') or (x['path'].startswith(('uma-sim-core/','uma-race-core/')) and (x['path'].endswith('/Cargo.toml') or '/src/' in x['path']))]
pins=[]
for f in inputs:
 data=(s/f['path']).read_bytes()
 assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==f['sha'],f['path']
 pins.append({'path':f['path'],'git_blob':f['sha'],**digest(s/f['path'])})
assert shutil.disk_usage(r).free>220_000_000,'Insufficient API build headroom'
env=os.environ.copy();env.update({'CARGO_TARGET_DIR':str(r/'target'),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_DEV_INCREMENTAL':'false','CARGO_BUILD_JOBS':'2'})
cmd=['/opt/homebrew/bin/cargo','build','--locked','-p','uma-sim-core','--bin','uma-sim-api']
log=ev/'native-api-build.log';start=time.monotonic();started_at=datetime.datetime.now(datetime.timezone.utc).isoformat()
print(json.dumps({'starting':'native-api-build','free':shutil.disk_usage(r).free}),flush=True)
with log.open('w') as f:result=subprocess.run(cmd,cwd=s,env=env,stdout=f,stderr=subprocess.STDOUT,timeout=240)
receipt={'started_at':started_at,'command':cmd,'environment_overrides':{key:env[key] for key in ('CARGO_TARGET_DIR','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_DEV_INCREMENTAL','CARGO_BUILD_JOBS')},'native_source_head':subprocess.run(['/usr/bin/git','-C',str(s),'rev-parse','HEAD'],capture_output=True,text=True,check=True).stdout.strip(),'canonical_base':'205178f1bf3bf5f34c04bbda88c3e4df2102f535','exit_code':result.returncode,'seconds':round(time.monotonic()-start,3),'log_sha256':digest(log)['sha256'],'compiled_inputs':pins,'source_freeze_sha256':digest(ev/'source-freeze-v1.json')['sha256']}
if result.returncode==0:receipt['binary']={'path':str(r/'target/debug/uma-sim-api'),**digest(r/'target/debug/uma-sim-api')}
receipt['source_unchanged']=all(digest(s/f['path'])['sha256']==f['sha256'] for f in freeze['files']) and all(digest(s/f['path'])['sha256']==f['sha256'] for f in pins)
receipt['source_status']=subprocess.run(['/usr/bin/git','-C',str(s),'status','--porcelain'],capture_output=True,text=True,check=True).stdout
receipt['versions']={name:subprocess.run(args,capture_output=True,text=True).stdout.strip() for name,args in [('cargo',['/opt/homebrew/bin/cargo','--version']),('rustc',['/opt/homebrew/bin/rustc','--version'])]}
receipt['free_after']=shutil.disk_usage(r).free
out=ev/'native-api-build.json';out.write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps({'receipt':str(out),'receipt_sha256':digest(out)['sha256'],'exit_code':result.returncode,'source_unchanged':receipt['source_unchanged'],'source_status':receipt['source_status'],'binary':receipt.get('binary'),'free':receipt['free_after']},indent=2),flush=True)
raise SystemExit(result.returncode)

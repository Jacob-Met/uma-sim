import os,json,subprocess,shutil,hashlib,datetime,time,tarfile,io
from pathlib import Path
r=Path('/Users/me/uma-conditions-20261008-5f566b5ec8ef');s=r/'source';ui=s/'packages/uma-sim-ui';ev=r/'evidence'
receipt={'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'canonical_base':'205178f1bf3bf5f34c04bbda88c3e4df2102f535','native_baseline':'ac242eb732eaad0e21032d76d5a25582cda90612','steps':[]}
def digest(p):
 d=p.read_bytes();return {'size':len(d),'sha256':hashlib.sha256(d).hexdigest()}
def save(): (ev/'initial-qualification.json').write_text(json.dumps(receipt,indent=2)+'\n')
def run(name,args,cwd,env=None,timeout=180):
 log=ev/(name+'.log');start=time.monotonic()
 print(json.dumps({'starting':name,'free':shutil.disk_usage(r).free}),flush=True)
 with log.open('w') as f:
  p=subprocess.run(args,cwd=cwd,env=env,stdout=f,stderr=subprocess.STDOUT,timeout=timeout)
 row={'name':name,'command':args,'cwd':str(cwd),'exit_code':p.returncode,'seconds':round(time.monotonic()-start,3),'log':str(log),'log_sha256':digest(log)['sha256'],'free_after':shutil.disk_usage(r).free}
 receipt['steps'].append(row);save();print(json.dumps(row),flush=True)
 if p.returncode:raise RuntimeError(name+' failed; retained log and no dependent replay')
assert shutil.disk_usage(r).free>300_000_000,'Native dependency headroom unavailable'
env=os.environ.copy();env['PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD']='1'
run('npm-ci-offline',['/opt/homebrew/bin/npm','ci','--offline','--no-audit','--no-fund'],ui,env)
run('ui-tests',['/opt/homebrew/bin/npm','test'],ui,env)
build_info=ui/'tsconfig.tsbuildinfo';build_before=build_info.read_bytes()
run('candidate-ui-build',['/opt/homebrew/bin/npm','run','build'],ui,env)
build_after=build_info.read_bytes();receipt['generated_build_info']={'before_sha256':hashlib.sha256(build_before).hexdigest(),'after_sha256':hashlib.sha256(build_after).hexdigest(),'restored_exact_input':True};build_info.write_bytes(build_before)
assert not (r/'candidate-dist').exists();shutil.move(str(ui/'dist'),str(r/'candidate-dist'))
# Build the exact current-main UI separately for the same native API fixture.
baseline=r/'baseline-ui';assert not baseline.exists();baseline.mkdir()
paths=subprocess.run(['/usr/bin/git','-C',str(s),'ls-tree','-r','--name-only','HEAD','--','packages/uma-sim-ui'],check=True,capture_output=True,text=True).stdout.splitlines()
for path in paths:
 rel=Path(path).relative_to('packages/uma-sim-ui')
 data=subprocess.run(['/usr/bin/git','-C',str(s),'show','HEAD:'+path],check=True,capture_output=True).stdout
 out=baseline/rel;out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data)
(baseline/'node_modules').symlink_to(ui/'node_modules',target_is_directory=True)
run('baseline-ui-build',['/opt/homebrew/bin/npm','run','build'],baseline,env)
shutil.move(str(baseline/'dist'),str(r/'baseline-dist'))
assert shutil.disk_usage(r).free>180_000_000,'Native API build headroom unavailable after UI qualification'
env['CARGO_TARGET_DIR']=str(r/'target');env['CARGO_PROFILE_DEV_DEBUG']='0';env['CARGO_PROFILE_DEV_INCREMENTAL']='false';env['CARGO_BUILD_JOBS']='2'
run('native-api-build',['/opt/homebrew/bin/cargo','build','--offline','--locked','-p','uma-sim-core','--bin','uma-sim-api'],s,env)
binary=r/'target/debug/uma-sim-api';receipt['native_api_binary']={'path':str(binary),**digest(binary)}
receipt['versions']={name:subprocess.run(args,capture_output=True,text=True).stdout.strip() for name,args in [('node',['/opt/homebrew/bin/node','--version']),('npm',['/opt/homebrew/bin/npm','--version']),('cargo',['/opt/homebrew/bin/cargo','--version']),('rustc',['/opt/homebrew/bin/rustc','--version'])]}
receipt['finished_at']=datetime.datetime.now(datetime.timezone.utc).isoformat();save()
print(json.dumps({'qualified':True,'receipt':str(ev/'initial-qualification.json'),'receipt_sha256':digest(ev/'initial-qualification.json')['sha256'],'binary':receipt['native_api_binary'],'free':shutil.disk_usage(r).free}),flush=True)

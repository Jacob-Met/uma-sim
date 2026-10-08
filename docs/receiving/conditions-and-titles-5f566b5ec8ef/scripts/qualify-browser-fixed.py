import os,json,subprocess,time,hashlib,datetime
from pathlib import Path
r=Path('/Users/me/uma-conditions-20261008-5f566b5ec8ef');s=r/'source';ev=r/'evidence'
binary=r/'target/debug/uma-sim-api'
assert hashlib.sha256(binary.read_bytes()).hexdigest()=='283c46e2a698dd1e258ef726ffd3816536c2f1f8fec7240168a4b0063e16a69a'
runner=s/'packages/uma-sim-ui/tests/receiving/conditions-titles-receiver.mjs'
assert subprocess.check_output(['/usr/bin/git','-C',str(s),'rev-parse','HEAD'],text=True).strip()=='7683f629a152be0b94249bf1527c3cd8e0f4e5e3'
assert subprocess.check_output(['/usr/bin/git','-C',str(s),'status','--porcelain'],text=True)==''
assert hashlib.sha256(runner.read_bytes()).hexdigest()=='ee2b921ab9856f9e41fba9838a7e5785da5c32ad23c8cf92064be645f3361d08'
receipt={'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_head':'7683f629a152be0b94249bf1527c3cd8e0f4e5e3','runner_sha256':hashlib.sha256(runner.read_bytes()).hexdigest(),'phases':[]}
for name in ('candidate',):
 env=os.environ.copy()
 if name=='baseline':env['UMA_CONDITIONS_EXPECT']='absent'
 else:env.pop('UMA_CONDITIONS_EXPECT',None)
 output=r/(name+'-browser-fixed')
 assert not output.exists(),'Receiver output must be new'
 cmd=['/opt/homebrew/bin/node',str(runner),str(r/(name+'-dist')),str(output),str(binary),'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome']
 log=ev/(name+'-browser-fixed-process.log');start=time.monotonic();print(json.dumps({'starting':name,'output':str(output)}),flush=True)
 with log.open('w') as f:result=subprocess.run(cmd,cwd=s,env=env,stdout=f,stderr=subprocess.STDOUT,timeout=180)
 phase={'name':name,'command':cmd,'exit_code':result.returncode,'seconds':round(time.monotonic()-start,3),'process_log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()}
 if (output/'receipt.json').exists():
  data=(output/'receipt.json').read_bytes();inner=json.loads(data);phase.update({'receipt_sha256':hashlib.sha256(data).hexdigest(),'cases':len(inner['cases']),'error':inner.get('error'),'source_unchanged':inner['sourceUnchanged'],'binary_unchanged':inner['binaryUnchanged']})
 receipt['phases'].append(phase)
 (ev/'candidate-browser-fixed-qualification.json').write_text(json.dumps(receipt,indent=2)+'\n')
 summary={**phase}
 if summary.get('error'):summary['error']={key:summary['error'].get(key,'')[:300] for key in ('name','message')}
 print(json.dumps(summary,indent=2),flush=True)
 if result.returncode:raise SystemExit(result.returncode)

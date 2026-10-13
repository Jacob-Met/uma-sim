# SPDX-License-Identifier: GPL-3.0-only
"""Receive the existing fixed-cohort saved study with the delivered CSV exporter."""
import base64,datetime,hashlib,importlib.util,json,os,pathlib,platform,shutil,socket,subprocess,sys,time,traceback
handoff=pathlib.Path(r'D:\hamon-consumer-596c-uma-csv-20261013')
root=pathlib.Path(r'D:\hamon-consumer-596c-uma-saved-cohort-20261013')
sha=lambda b:hashlib.sha256(b).hexdigest()
blob=lambda b:hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
def require(c,label):
 if not c:raise RuntimeError(label)
def put(p,b):
 p.parent.mkdir(parents=True,exist_ok=True)
 with p.open('xb') as f:f.write(b)
def serial(v):return (json.dumps(v,ensure_ascii=False,sort_keys=True,indent=2,allow_nan=False)+'\n').encode()
def snap(p):
 b=p.read_bytes();return {'bytes':len(b),'sha256':sha(b),'git_blob':blob(b),'mtime_ns':p.stat().st_mtime_ns}
require(socket.gethostname().lower()=='msi' and str(pathlib.Path.home())==r'C:\Users\Veria','Device identity')
require(not root.exists(),'Create-only course destination collision')
packet=json.loads((handoff/'saved-study-transfer.json').read_bytes())
require(packet['study_commit']=='eb4f7de067824ed24818d93d5fdca87d3818a2da' and packet['study_tree']=='8025414e21f6b793678eec9cf7448e5bdd21f9ec' and len(packet['files'])==8,'Historical study identity')
fixed={'raw/default-cohort.jsonl':('10b460ee53414411865316731c79093b38372cb506069ad3d61d58eac3b530ae',4707),'raw/bot-cohort.jsonl':('29c742c08dcabc93c52ea899232d7447cf3815564962027884edbe250ab7399b',4754),'raw/paired-comparison.stdout':('7b15171bbdec97c6bc52671b106deba787dfec3f5ecac50b79bf30f5faa4f473',4069)}
decoded={}
for row in packet['files']:
 p=pathlib.PurePosixPath(row['path'])
 require(not p.is_absolute() and '..' not in p.parts and ':' not in row['path'] and '\\' not in row['path'],'Unsafe study path')
 b=base64.b64decode(row['base64'],validate=True)
 require(len(b)==row['bytes'] and sha(b)==row['sha256'] and blob(b)==row['git_blob'],'Historical Git source mismatch')
 if row['path'] in fixed:require((sha(b),len(b))==fixed[row['path']],'Fixed raw input mismatch')
 require(row['path'] not in decoded,'Duplicate study path');decoded[row['path']]=b
require(set(fixed)<=set(decoded),'Missing retained inputs')
source_exe=handoff/'candidate-portable/uma-sim.exe';source_exe_before=snap(source_exe)
require(source_exe_before['bytes']==8274114 and source_exe_before['sha256']=='9a57e48af9c23bb42a25593c74d6508ac428e75948bb18f0eeb875767cd86255','Delivered executable identity')
review_bytes=(handoff/'review_simulated_pairs.py').read_bytes()
require(len(review_bytes)==8156 and sha(review_bytes)=='d92817b14a5b74f618d5ef19d0869487909645f262fef4c72f7612289e4d54e6','Frozen reviewer identity')
root.mkdir()
for path,b in decoded.items():put(root/'study'/path,b)
put(root/'review_simulated_pairs.py',review_bytes)
put(root/'receive_saved_cohort.py',pathlib.Path(__file__).read_bytes())
shutil.copy2(source_exe,root/'uma-sim.exe');exe=root/'uma-sim.exe'
require(snap(exe)==source_exe_before,'Copied executable mismatch')
source_manifest={k:v for k,v in packet.items() if k!='files'}
source_manifest['files']=[{k:v for k,v in row.items() if k!='base64'} for row in packet['files']]
put(root/'study-source-manifest.json',serial(source_manifest))
put(root/'run-started.json',serial({'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'pid':os.getpid(),'simulator_execution':False,'rule':'Never rerun an existing course directory; retain any failure.'}))
spec=importlib.util.spec_from_file_location('reviewer',root/'review_simulated_pairs.py');reviewer=importlib.util.module_from_spec(spec);spec.loader.exec_module(reviewer)
expected={('ura','Special Week',n) for n in range(16)}
for kind in ['default','bot']:reviewer.records(decoded['raw/'+kind+'-cohort.jsonl'],expected,kind)
before={str(p.relative_to(root).as_posix()):snap(p) for p in root.rglob('*') if p.is_file()}
env=dict(os.environ)
for k in list(env):
 if k.startswith('UMA_'):env.pop(k)
runs=[];errors=[];success=False
try:
 for fmt in ['csv','json']:
  args=[str(exe),'compare-batches','--input='+str(root/'study/raw/bot-cohort.jsonl'),'--baseline='+str(root/'study/raw/default-cohort.jsonl'),'--format='+fmt]
  label='compare-'+fmt;started=datetime.datetime.now(datetime.timezone.utc).isoformat();t=time.perf_counter()
  put(root/(label+'-started.json'),serial({'args':args,'cwd':str(root),'started_utc':started,'timeout_seconds':30}))
  timeout=False
  try:
   r=subprocess.run(args,cwd=root,env=env,capture_output=True,timeout=30);out,err,status=r.stdout,r.stderr,r.returncode
  except subprocess.TimeoutExpired as e:out,err,status=e.stdout or b'',e.stderr or b'',None;timeout=True
  put(root/(label+'-stdout.bin'),out);put(root/(label+'-stderr.bin'),err)
  runs.append({'label':label,'args':args,'cwd':str(root),'started_utc':started,'elapsed_seconds':time.perf_counter()-t,'timeout_seconds':30,'timed_out':timeout,'exit':status,'stdout_bytes':len(out),'stdout_sha256':sha(out),'stderr_bytes':len(err),'stderr_sha256':sha(err)})
  require(not timeout and status==0 and not err,label+' did not complete with zero/empty stderr')
  put(root/('comparison.'+fmt),out);print(json.dumps({'phase':label,'exit':status,'bytes':len(out)}),flush=True)
 require((root/'comparison.json').read_bytes()==decoded['raw/paired-comparison.stdout'],'Current JSON differs from original study comparator complete bytes')
 args=[sys.executable,'-B',str(root/'review_simulated_pairs.py'),'--baseline',str(root/'study/raw/default-cohort.jsonl'),'--input',str(root/'study/raw/bot-cohort.jsonl'),'--csv',str(root/'comparison.csv'),'--json',str(root/'comparison.json'),'--scenario','ura','--trainee','Special Week']
 for n in range(16):args+=['--seed',str(n)]
 args+=['--output',str(root/'independent-output-review.json')]
 r=subprocess.run(args,cwd=root,env=env,capture_output=True,timeout=30)
 put(root/'offline-review-stdout.bin',r.stdout);put(root/'offline-review-stderr.bin',r.stderr)
 offline={'args':args,'exit':r.returncode,'stdout_bytes':len(r.stdout),'stdout_sha256':sha(r.stdout),'stderr_bytes':len(r.stderr),'stderr_sha256':sha(r.stderr)}
 require(r.returncode==0 and not r.stderr,'Independent offline output reconciliation failed')
 require(json.loads((root/'independent-output-review.json').read_bytes())['success'],'Offline reconciliation disposition')
 success=True
except Exception as e:errors.append({'type':type(e).__name__,'message':str(e),'traceback':traceback.format_exc()})
finally:
 after={path:snap(root/path) for path in before}
 custody={'all_staged_inputs_and_exe':before==after,'original_executable':source_exe_before==snap(source_exe),'no_session_directory':not (root/'.uma-sim').exists()}
 success=success and all(custody.values())
 receipt={'success':success,'errors':errors,'device_id':'9ea77fe9-bb87-46e0-a5a8-8b3e25f6ff7e','host':socket.gethostname(),'user_home':str(pathlib.Path.home()),'python':platform.python_version(),'study_source':source_manifest,'historical_producer_source':'4eab1798def5c19fbdee267eb2a6ea8a2726dbe7 plus qualified paired-comparison overlay','historical_producer_executable_sha256':'01d7e9aee08492579143a297d74be01816a10da8e40045ecf4436b619ebcc989','current_comparator_source':'392499683b3b52cf032222d4ed9ea6274fb10131','current_comparator_executable':source_exe_before,'requested_saved_cohort':{'scenario':'ura','trainee':'Special Week','seeds':list(range(16)),'direction':'bot minus default'},'environment':'Inherited parent HOME unchanged; UMA_* removed from child environment; no runtime assets supplied or needed for compare-batches.','native_cli_invocations':len(runs),'simulator_invocations':0,'runs':runs,'offline_review':locals().get('offline'),'before':before,'after':after,'custody':custody,'historical_json_output_byte_equal':(root/'comparison.json').exists() and (root/'comparison.json').read_bytes()==decoded['raw/paired-comparison.stdout'],'scope':'Downstream CSV/JSON export of the existing accepted saved cohort. No new simulator study, source repair, race-win or finale claim, policy superiority, latest-mechanics acceptance or installed adoption.'}
 put(root/'saved-cohort-receipt.json',serial(receipt))
 print(json.dumps({'success':success,'native_cli_invocations':len(runs),'simulator_invocations':0,'historical_json_output_byte_equal':receipt['historical_json_output_byte_equal'],'custody':custody,'errors':errors}),flush=True)
if not success:sys.exit(1)

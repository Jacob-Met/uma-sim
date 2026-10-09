from pathlib import Path
import hashlib,json,stat
ROOT=Path('/home/jacob/uma-continue-3dc74ef3b04d-root-receiving')
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def put(name,obj):
    with (ROOT/name).open('x') as f: json.dump(obj,f,indent=2);f.write('\n')
old=ROOT/'FROZEN-V3-MANIFEST.json'
assert sha(old)=='72a7e04c6e2130512ad7e90bfdd1ddb3a807ef1e9a8a16d38e6b2d614bec7ba2'
for entry in json.loads(old.read_text())['files']:
    p=ROOT/entry['path']
    assert sha(p)==entry['sha256'] and p.stat().st_size==entry['bytes'],entry['path']
    assert oct(stat.S_IMODE(p.stat().st_mode))==entry['mode'],entry['path']
v3=ROOT/'receiver-v3/receive-continue-v3.py'
v4=ROOT/'receiver-v4/receive-continue-v4.py'
assert sha(v3)=='b01b3358258c3e9cecb34844df2b300f55d982a096f1d9c028379ac47164ce37'
assert sha(v4)=='38bd092e2a8005d54d1e3ee02f0637b5cef9446d33b3122130daeb1aae4f0bd8'
assert v3.read_text().split('sha = lambda b:',1)[1]==v4.read_text().split('sha = lambda b:',1)[1]
rp=ROOT/'receiver-v4/RECEIPT-v1.json'
assert sha(rp)=='f81dab6ee8430d6f55cdee93190328c33c6917c82b0ad2878df537be3f529166'
receipt=json.loads(rp.read_text())
assert receipt['passed']==5 and receipt['failed']==0 and receipt['command_count']==10
assert receipt['policy_marker_exists'] is False
pins=json.loads((ROOT/'REPAIRED-SOURCE-PINS-v1.json').read_text())
candidate=Path('/home/jacob/uma-continue-3dc74ef3b04d-local-policy')
for path,expected in pins.items(): assert sha(candidate/path)==expected,path
binary=Path('/home/jacob/uma-continue-3dc74ef3b04d-proof/local-policy-v1/candidate/candidate-uma-sim')
assert sha(binary)==receipt['binary_sha256']
put('FINAL-RECEIVING-RECEIPT-v1.json',{
 'actor':'hamon-3dc74ef3b04d-20261008/estate_coordination',
 'status':'Independent source review and unchanged V4 receiving accepted',
 'source_review_sha256':sha(ROOT/'LOCAL-POLICY-SOURCE-REVIEW-v1.json'),
 'source_pins_sha256':sha(ROOT/'REPAIRED-SOURCE-PINS-v1.json'),
 'candidate_binary':str(binary),'candidate_binary_sha256':receipt['binary_sha256'],
 'receiver_method_sha256':sha(v4),'receiver_receipt_sha256':sha(rp),
 'primary_assertions_byte_identical_to_v3':True,
 'reused_event_checkpoint_sha256':'aa52629d8c9b9367ca155f904e647116f47d9f273c709201c8534f84a6924a5b',
 'observed_process':{'pid':2924000,'exit':0,'runtime_seconds':2.35,'explicit_completion_readback':True},
 'result':{'groups_passed':5,'groups_failed':0,'actual_cli_commands':10,'external_marker_absent':True},
 'earlier_negatives':{'v1':'Receiver speed field adapter error; immutable.',
  'v2':'Receiver support ID type adapter error; immutable.',
  'v3':'One confirmed external policy invocation from bot, exit101. Later completed/FIFO marker assertions contaminated by that same shared marker; original raw2/5 result remains immutable.',
  'all_36_frozen_files_verified_unchanged':True,'frozen_manifest_sha256':sha(old)},
 'limits':['No cross-process full-outcome determinism claim.',
  'No new authored start/step/reset in V4; actual prior transitions remain preserved.',
  'No compiler, GitHub, network or product mutation by receiver.',
  'Unix native receiver does not claim Windows Uma binary qualification.',
  'Current-main ownership, unmerged writer dependency and publication gates remain separate.']
})
entries=[];nonregular=[]
for p in sorted(ROOT.rglob('*')):
    s=p.lstat()
    if stat.S_ISDIR(s.st_mode):continue
    if not stat.S_ISREG(s.st_mode):nonregular.append(str(p.relative_to(ROOT)));continue
    entries.append({'path':str(p.relative_to(ROOT)),'bytes':s.st_size,'mode':oct(stat.S_IMODE(s.st_mode)),'sha256':sha(p)})
assert not nonregular,nonregular
total=sum(e['bytes'] for e in entries)
assert total<2*1024*1024,total
put('FINAL-RECEIVING-MANIFEST-v1.json',{'actor':'hamon-3dc74ef3b04d-20261008/estate_coordination',
 'scope':'Complete independent receiving tree including prior negatives; manifest excludes itself.',
 'regular_file_count':len(entries),'regular_bytes':total,'nonregular_paths':nonregular,'files':entries})
print(json.dumps({'regular_files':len(entries),'regular_bytes':total,
 'receipt_sha256':sha(ROOT/'FINAL-RECEIVING-RECEIPT-v1.json'),
 'manifest_sha256':sha(ROOT/'FINAL-RECEIVING-MANIFEST-v1.json')}))

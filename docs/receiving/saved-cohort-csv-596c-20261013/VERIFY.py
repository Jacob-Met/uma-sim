# SPDX-License-Identifier: GPL-3.0-only
"""Verify the saved-cohort CSV companion without running UMA or writing files."""
import base64,gzip,hashlib,importlib.util,json,pathlib,sys
sys.dont_write_bytecode=True
root=pathlib.Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
blob=lambda b:hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
def require(c,label):
 if not c:raise RuntimeError(label)
def safe(name):
 p=pathlib.PurePosixPath(name)
 require(not p.is_absolute() and '..' not in p.parts and '\\' not in name and ':' not in name,'Unsafe payload path')
 return root.joinpath(*p.parts)
inventory=json.loads((root/'FILES.json').read_bytes())
require(inventory['study_commit']=='eb4f7de067824ed24818d93d5fdca87d3818a2da' and inventory['comparator_source']=='392499683b3b52cf032222d4ed9ea6274fb10131','Source identity mismatch')
rows=inventory['files'];names={r['path'] for r in rows}
require(len(names)==len(rows),'Duplicate inventory path')
actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()}
require(names|{'FILES.json'}<=actual,'Missing payload')
for row in rows:
 b=safe(row['path']).read_bytes()
 require(len(b)==row['bytes'] and sha(b)==row['sha256'] and blob(b)==row['git_blob'],'Payload mismatch '+row['path'])
raw=(root/'evidence/native-receipts.json.gz').read_bytes()
require(len(raw)==59954 and sha(raw)=='276dc8bba83cbb7aa7d50aa37b5469a42d512162fd72753774ac61a39a95544a','Native evidence identity mismatch')
envelope=json.loads(gzip.decompress(raw));files=envelope['files']
require(len(files)==24,'Native evidence inventory mismatch')
decoded={}
for name,row in files.items():
 safe(name);b=base64.b64decode(row['base64'],validate=True)
 require(len(b)==row['bytes'] and sha(b)==row['sha256'],'Native payload mismatch '+name)
 decoded[name]=b
bindings={'data/default-cohort.jsonl':'study/raw/default-cohort.jsonl','data/bot-cohort.jsonl':'study/raw/bot-cohort.jsonl','comparison.csv':'comparison.csv','comparison.json':'comparison.json','tools/review_simulated_pairs.py':'review_simulated_pairs.py','evidence/native-output-review.json':'independent-output-review.json','evidence/study-source-manifest.json':'study-source-manifest.json'}
for path,native in bindings.items():require(safe(path).read_bytes()==decoded[native],'Native data binding mismatch '+path)
receipt=json.loads(decoded['saved-cohort-receipt.json'])
require(receipt['success'] and not receipt['errors'] and receipt['native_cli_invocations']==2 and receipt['simulator_invocations']==0,'Native course disposition mismatch')
require(receipt['before']==receipt['after'] and all(receipt['custody'].values()),'Native custody mismatch')
require(receipt['historical_json_output_byte_equal'] and decoded['comparison.json']==decoded['study/raw/paired-comparison.stdout'],'Historical JSON parity mismatch')
require(receipt['current_comparator_source']==inventory['comparator_source'] and receipt['current_comparator_executable']['sha256']=='9a57e48af9c23bb42a25593c74d6508ac428e75948bb18f0eeb875767cd86255','Recorded executable identity mismatch')
for run in receipt['runs']:
 require(run['exit']==0 and not run['timed_out'],'Native invocation failed')
 for stream in ['stdout','stderr']:
  b=decoded[run['label']+'-'+stream+'.bin']
  require(len(b)==run[stream+'_bytes'] and sha(b)==run[stream+'_sha256'],'Raw stream binding mismatch')
 require(run['stderr_bytes']==0,'Native diagnostic present')
manifest=json.loads(decoded['study-source-manifest.json'])
require(manifest['study_commit']==inventory['study_commit'] and manifest['study_tree']=='8025414e21f6b793678eec9cf7448e5bdd21f9ec','Historical source binding')
for row in manifest['files']:
 b=decoded['study/'+row['path']]
 require(len(b)==row['bytes'] and sha(b)==row['sha256'] and blob(b)==row['git_blob'],'Historical Git blob mismatch')
helper=root/'tools/review_simulated_pairs.py'
require(sha(helper.read_bytes())=='d92817b14a5b74f618d5ef19d0869487909645f262fef4c72f7612289e4d54e6','Frozen independent reviewer mismatch')
spec=importlib.util.spec_from_file_location('cohort_review',helper);reviewer=importlib.util.module_from_spec(spec);spec.loader.exec_module(reviewer)
expected={('ura','Special Week',n) for n in range(16)}
result=reviewer.review(reviewer.records((root/'data/default-cohort.jsonl').read_bytes(),expected,'default'),reviewer.records((root/'data/bot-cohort.jsonl').read_bytes(),expected,'bot'),(root/'comparison.csv').read_bytes(),(root/'comparison.json').read_bytes())
native_review=json.loads(decoded['independent-output-review.json'])
for key in ['comparisons','summary','recorded_shop_gains']:require(result[key]==native_review[key],'Recomputed result differs '+key)
print(json.dumps({'success':True,'payloads_verified':len(rows),'native_evidence_payloads':24,'saved_pairs':len(result['comparisons']),'csv_bytes':len(decoded['comparison.csv']),'csv_sha256':sha(decoded['comparison.csv']),'historical_json_byte_equal':True,'new_uma_invocations':0,'extra_files_not_verified':sorted(actual-(names|{'FILES.json'}))},sort_keys=True))

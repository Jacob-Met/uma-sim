"""Reconcile immutable CLI/static receiving; never launch renderer or browser."""
import argparse,base64,gzip,hashlib,importlib.util,json
from pathlib import Path

PACKET_SHA='6f457aef80cf35e6f93aff7b3a5921d97e2f4bbecacdf71403712693b47d19d2'
DECODED_SHA='d7a566abc4eecb0adeb9c3a20446ec5c397867b1ca88d65cd22ac9fddb849ff4'
RECEIVER_SHA='7de8e76003d34a7c8b9f1bc4478bfd3c955512c0d63b851fa6a05de3249fe942'
SOURCE_SHA='f4a7aa5c1de55880f76028fdfc7d0cd32f5184f4f40d625ec73024390e985cf8'
def need(value,message):
    if not value:raise ValueError(message)
def sha(b):return hashlib.sha256(b).hexdigest()
def unique(pairs):
    out={}
    for k,v in pairs:need(k not in out,'duplicate JSON field');out[k]=v
    return out
def forbidden(value):raise ValueError('nonfinite JSON')
def parse(b):return json.loads(b.decode('utf-8'),object_pairs_hook=unique,parse_constant=forbidden)

def review(packet_path,receiver_path):
    raw=packet_path.read_bytes();need(len(raw)==73622 and sha(raw)==PACKET_SHA,'fixed packet mismatch')
    decoded=gzip.decompress(raw);need(len(decoded)==408870 and sha(decoded)==DECODED_SHA,'decoded packet mismatch')
    packet=parse(decoded);need(packet['schema']=='uma-paired-html-receiving-raw/1'and packet['source_sha256']==SOURCE_SHA,'source binding')
    files={}
    for name,row in packet['files'].items():
        need(isinstance(name,str)and '\\'not in name and ':'not in name and not name.startswith('/')and all(x not in ('','.','..')for x in name.split('/')),'unsafe packet name')
        b=base64.b64decode(row['base64'],validate=True)
        need(len(b)==row['bytes']and sha(b)==row['sha256']and hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==row['git_blob_sha']and row['mode']=='100644','payload identity '+name)
        files[name]=b
    need(len(files)==115,'payload count')
    rb=receiver_path.read_bytes();need(sha(rb)==RECEIVER_SHA and rb==files['receive_html.py'],'fixed static receiver mismatch')
    spec=importlib.util.spec_from_file_location('bound_uma_html_static_receiver',receiver_path);receiver=importlib.util.module_from_spec(spec);spec.loader.exec_module(receiver)
    oracle=parse(files['ORACLE.json']);fm=parse(files['fixture-manifest.json'])
    for name,row in fm.items():
        b=files['fixtures/'+name];need(len(b)==row['bytes']and sha(b)==row['sha256'],'fixture binding '+name)
    need(len(fm)==28 and len(oracle['malformed_cases'])==26,'fixture counts')
    rows=oracle['mixed']['comparisons'];need(len(rows)==14,'mixed row count')
    orders=dict(identity_indices=list(range(14)),delta_desc=sorted(range(14),key=lambda i:-rows[i]['score_delta']),delta_asc=sorted(range(14),key=lambda i:rows[i]['score_delta']),absolute_delta=sorted(range(14),key=lambda i:-abs(rows[i]['score_delta'])),wins=[i for i,r in enumerate(rows)if r['score_delta']>0],ties=[i for i,r in enumerate(rows)if r['score_delta']==0],losses=[i for i,r in enumerate(rows)if r['score_delta']<0])
    need(all(oracle[k]==v for k,v in orders.items()),'fixed expected orders')
    receipt=parse(files['course/receipt.json']);need(receipt['result']=='PASS'and receipt['source_sha256']==SOURCE_SHA,'course result/source')
    need(receipt['before']==receipt['after']and len(receipt['before'])==39,'recorded full custody')
    need(len(receipt['checks'])==125 and all(x['actual']==x['expected']for x in receipt['checks']),'recorded predicates')
    names=['mixed-positive']+[x['name']for x in oracle['malformed_cases']]+oracle['additional_path_refusals'];commands=receipt['commands']
    need(len(commands)==32 and [r['name']for r in commands]==names,'exact command course')
    for i,r in enumerate(commands):
        streams={k:files['course/streams/'+r['name']+'.'+k]for k in ['stdout','stderr']}
        for k,b in streams.items():need(len(b)==r[k+'_bytes']and sha(b)==r[k+'_sha256'],'command stream '+r['name']+'/'+k)
        a=r['args'];need(a[1]=='-B'and a[2].endswith('/candidate/scripts/render_paired_comparison.py')and a[3]=='--input'and a[5]=='--input-sha256'and a[7]=='--output','producer command shape')
        need(r['exit']==(0 if i==0 else 2),'exit '+r['name'])
        if i==0:
            need(streams['stderr']==b''and len(a)==13 and a[9:]==['--baseline-label',oracle['labels']['baseline'],'--input-label',oracle['labels']['input']],'positive argv')
            out=parse(streams['stdout']);b=files['course/work/mixed14.html'];inp=files['fixtures/mixed14.json']
            need(out==dict(format='uma-sim.paired-html-receipt.v1',status='ok',input_sha256=sha(inp),input_bytes=len(inp),output_sha256=sha(b),output_bytes=len(b),pairs=14,output=a[8]),'positive complete receipt')
        else:
            need(streams['stdout']==b''and len(a)==9,'refusal stdout/argv');err=parse(streams['stderr'])
            need(set(err)=={'status','error'}and err['status']=='error'and isinstance(err['error'],str)and bool(err['error']),'typed error')
        fixture='mixed14.json'if i==0 or r['name']in oracle['additional_path_refusals']else oracle['malformed_cases'][i-1]['file']
        need(a[6]==('0'*64 if r['name']=='wrong-digest'else sha(files['fixtures/'+fixture])),'exact supplied digest')
        need(a[4].endswith('/'+('input-link.json'if r['name']=='symlink-input'else fixture)),'input path binding')
    got_mixed=receiver.reconcile(files['course/work/mixed14.html'],files['fixtures/mixed14.json'],oracle['labels'])
    got_saved=receiver.reconcile(files['author/saved-sixteen-pairs.html'],files['fixtures/saved16.json'],dict(baseline='Default (saved study)',input='Local bot (saved study)'))
    canonical=lambda x:json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':'))
    need(canonical(got_mixed)==canonical(parse(files['course/mixed-static.json'])),'complete mixed static report')
    need(canonical(got_saved)==canonical(parse(files['course/saved16-static.json'])),'complete saved static report')
    saved=parse(files['author/saved-sixteen-launch.json']);need(saved['exit']==0 and saved['before']==saved['after'],'author saved generation/custody')
    for k in ['stdout','stderr']:need(sha(files['author/saved-sixteen.'+k])==saved[k+'_sha256'],'saved generation stream')
    need(files['author/saved-sixteen.stderr']==b''and parse(files['author/saved-sixteen.stdout'])==saved['renderer_receipt'],'saved complete receipt')
    need(saved['renderer_receipt']['input_sha256']==sha(files['fixtures/saved16.json'])and saved['renderer_receipt']['output_sha256']==sha(files['author/saved-sixteen-pairs.html']),'saved bytes')
    a=saved['args'];need(len(a)==13 and a[1]=='-B'and a[3:]==['--input','/dev/shm/estate-product-596cb2f61c57/uma-produced/existing-study/raw/paired-comparison.stdout','--input-sha256',sha(files['fixtures/saved16.json']),'--output','/dev/shm/estate-product-596cb2f61c57/uma-paired-report/evidence/saved-sixteen-pairs.html','--baseline-label','Default (saved study)','--input-label','Local bot (saved study)'],'saved exact invocation')
    maintained=parse(files['author/maintained-receipt.json']);need(maintained['exit']==0 and maintained['methods']==8 and maintained['source_before']==maintained['source_after'],'maintained result/custody')
    for k in ['stdout','stderr']:need(sha(files['author/maintained.'+k])==maintained[k+'_sha256'],'maintained raw hash')
    log=files['author/maintained.stderr'].decode();need(log.count(' ... ok')==8 and 'Ran 8 tests' in log and log.endswith('OK\n')and files['author/maintained.stdout']==b'','maintained count/log')
    boundary=parse(files['browser-boundary.json']);need(boundary['result']=='UNQUALIFIED'and boundary['stopped']and not any(boundary[k]for k in ['workarounds_attempted','report_loaded','interactive_sort_filter_search','browser_download','disabled_javascript','visual_viewport_review']),'browser boundary')
    return dict(schema='uma-html-offline-reconciliation/1',result='PASS',packet_sha256=PACKET_SHA,payload_count=len(files),source_sha256=SOURCE_SHA,maintained_methods=8,actual_consumer_calls=32,positive_calls=1,refusals=31,recorded_predicates=125,raw_consumer_streams=64,recorded_protected_identities=39,saved16_author_generation_received_once=True,complete_static_reports_equal=True,mixed_html_sha256=got_mixed['html_sha256'],saved16_html_sha256=got_saved['html_sha256'],embedded_json_exact=True,full_static_tables_and_summaries=True,static_order_oracle_recomputed=True,browser='UNQUALIFIED: security policy refused local-file navigation before report load; no browser behavior was observed.',execution='Pure offline evidence reconciliation; no renderer, browser, simulator or Rust process launched.')

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('packet',type=Path);ap.add_argument('--receiver',type=Path,default=Path(__file__).with_name('receive_html.py'));ap.add_argument('--output',type=Path,required=True);args=ap.parse_args()
    need(not args.output.exists()and not args.output.is_symlink(),'output must be new')
    result=review(args.packet,args.receiver)
    args.output.open('x').write(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(result,sort_keys=True))
if __name__=='__main__':main()

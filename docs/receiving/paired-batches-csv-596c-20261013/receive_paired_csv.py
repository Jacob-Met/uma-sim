# SPDX-License-Identifier: GPL-3.0-only
"""Independent, original-first receiving for UMA complete paired CSV export.

Frozen against issue159's output contract before inspecting its implementation.
Uses an existing executable; never builds, installs, starts an API or simulates.
Each mode owns a new evidence directory. Inputs and the original are immutable.
"""
import argparse, csv, datetime, hashlib, io, json, os, pathlib, platform
import subprocess, sys, time

COLS = ['scenario', 'trainee', 'seed', 'baseline_score', 'input_score',
        'score_delta', 'baseline_grade', 'input_grade']
ORIGINAL_SHA = 'ef521867576794efb43121a8530c616df864320f3c7775cdf731c3d6d4b5ef4e'
ORIGINAL_SOURCE = '18d6cf1b9f1e6f7bfd947e9c04cac467bca5988a'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def snap_file(path):
    data = path.read_bytes()
    return {'bytes': len(data), 'sha256': digest(data), 'mtime_ns': path.stat().st_mtime_ns}

def snap_tree(root):
    return {p.relative_to(root).as_posix(): snap_file(p)
            for p in sorted(root.rglob('*')) if p.is_file()}

def fixture_data():
    originals, inputs = [], []
    seeds = [-(2**63), -10, -2, 0, 1, 2, 10, 99, 2**63-1]
    for i, seed in enumerate(seeds):
        before, after = 120 + i, 120 + i + (i % 3 - 1) * 7
        if i == 0:
            before, after = -(2**31), 2**31-1
        if i == 8:
            before, after = 2**31-1, -(2**31)
        common = dict(scenario='ura', trainee='Special Week', seed=seed)
        originals.append(dict(common, grade='F', score=before))
        inputs.append(dict(common, grade='A', score=after))
    literals = [
        ('U,R"A\r\n\t', '雪, "馬"\n第二', '\t F\r\n"原",', '新,"G"\r\n'),
        ('=1+1', '+literal', '@grade', '-literal'),
        (' ura ', ' Same ', ' F ', ' A '),
        ('ura', 'Same', 'F', 'A'),
        ('é', '馬', 'ß', '秀'),
    ]
    for i, (scenario, trainee, old_grade, new_grade) in enumerate(literals):
        common = dict(scenario=scenario, trainee=trainee, seed=100+i)
        originals.append(dict(common, grade=old_grade, score=200+i))
        inputs.append(dict(common, grade=new_grade, score=199+i))
    def encode(rows):
        return ('\n'.join(json.dumps(r, ensure_ascii=False, separators=(',', ':'))
                          for r in rows) + '\n').encode('utf-8')
    files = {
        'baseline.jsonl': encode(originals[4:]+originals[:4]),
        'input.jsonl': encode(list(reversed(inputs))),
        'baseline-reordered.jsonl': encode(list(reversed(originals))),
        'input-reordered.jsonl': encode(inputs[7:]+inputs[:7]),
        'duplicate-input.jsonl': encode(inputs+[inputs[-1]]),
        'duplicate-baseline.jsonl': encode(originals+[originals[-1]]),
        'late-invalid.jsonl': encode(inputs)+b'{"seed":\n',
        'unmatched-input.jsonl': encode(inputs[:-1]),
        'unmatched-baseline.jsonl': encode(originals[:-1]),
        'empty.jsonl': b'\n\t\r\n',
        'seed-overflow.jsonl': encode([dict(inputs[0], seed=2**63)]),
        'score-overflow.jsonl': encode([dict(inputs[0], score=2**31)]),
        '.uma-sim/session.json': b'authored unrelated session sentinel\n',
    }
    before_by_id = {(r['scenario'], r['trainee'], r['seed']): r for r in originals}
    rows = []
    for row in sorted(inputs, key=lambda r: (r['scenario'], r['trainee'], r['seed'])):
        old = before_by_id[(row['scenario'], row['trainee'], row['seed'])]
        rows.append(dict(scenario=row['scenario'], trainee=row['trainee'], seed=row['seed'],
                         baseline_score=old['score'], input_score=row['score'],
                         score_delta=row['score']-old['score'],
                         baseline_grade=old['grade'], input_grade=row['grade']))
    # Contract oracle is written before reading the Rust candidate. The parser
    # below independently checks field recovery, record count and exact numbers.
    text = ','.join(COLS)+'\r\n'
    for row in rows:
        values=[]
        for col in COLS:
            value=row[col]
            values.append('"'+value.replace('"','""')+'"' if isinstance(value,str) else str(value))
        text += ','.join(values)+'\r\n'
    return files, rows, text.encode('utf-8')

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--root', type=pathlib.Path, required=True)
    p.add_argument('--exe', type=pathlib.Path, required=True)
    p.add_argument('--expected-exe-sha', required=True)
    p.add_argument('--source', type=pathlib.Path, required=True)
    p.add_argument('--mode', choices=['original','candidate'], required=True)
    args=p.parse_args()
    root=args.root.resolve(); exe=args.exe.resolve(); source=args.source.resolve()
    if digest(exe.read_bytes())!=args.expected_exe_sha:
        raise RuntimeError('Executable pin mismatch')
    if args.mode=='original' and args.expected_exe_sha!=ORIGINAL_SHA:
        raise RuntimeError('Original executable differs from previously received release')
    manifest=json.loads((source/'SOURCE-MANIFEST.json').read_bytes())
    if manifest['source_commit']!=ORIGINAL_SOURCE or len(manifest['source_files'])!=226:
        raise RuntimeError('Original source manifest mismatch')
    for row in manifest['source_files']:
        data=(source/row['path']).read_bytes()
        blob=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
        if digest(data)!=row['sha256'] or len(data)!=row['bytes'] or blob!=row['git_blob_sha']:
            raise RuntimeError('Original source changed: '+row['path'])
    files, expected_rows, expected_csv=fixture_data()
    work=root/'work'; output=root/args.mode
    if args.mode=='original':
        work.mkdir()
        for name,data in files.items():
            dest=work/name; dest.parent.mkdir(parents=True,exist_ok=True)
            with dest.open('xb') as f: f.write(data)
    else:
        if set(snap_tree(work))!=set(files): raise RuntimeError('Fixture inventory changed')
        for name,data in files.items():
            if (work/name).read_bytes()!=data: raise RuntimeError('Fixture changed: '+name)
    output.mkdir()
    (output/'expected.csv').write_bytes(expected_csv)
    (output/'expected-pairs.json').write_text(json.dumps(expected_rows,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    before={'work':snap_tree(work),'source':snap_tree(source),'exe':snap_file(exe)}
    env=os.environ.copy(); env['UMA_REPO_ROOT']=str(work/'absent-repo-root')
    env['UMA_RACE_MODEL']='invalid-unused-setting'
    checks=[]; runs=[]; errors=[]
    receipt={'format':'uma-sim.paired-csv.native-receiving.v1','mode':args.mode,
             'utc_start':datetime.datetime.now(datetime.timezone.utc).isoformat(),
             'host':platform.node(),'platform':platform.platform(),'python':sys.version,
             'exe':str(exe),'original_source':ORIGINAL_SOURCE,'source':str(source),
             'runner_sha256':digest(pathlib.Path(__file__).read_bytes()),
             'expected_csv_sha256':digest(expected_csv),'expected_pairs':expected_rows,
             'before':before,'runs':runs,'checks':checks,'errors':errors}
    def check(name,actual,expected):
        ok=actual==expected
        checks.append({'name':name,'actual':actual,'expected':expected,'pass':ok})
        if not ok: raise RuntimeError('Contract mismatch: '+name)
    def run(name,argv):
        dest=output/name;dest.mkdir()
        started=time.monotonic()
        with (dest/'stdout.bin').open('xb') as stdout, (dest/'stderr.bin').open('xb') as stderr:
            child=subprocess.Popen([str(exe),'compare-batches']+argv,cwd=work,env=env,stdout=stdout,stderr=stderr)
            try: code=child.wait(timeout=30)
            except subprocess.TimeoutExpired:
                child.kill();child.wait();raise
        out=(dest/'stdout.bin').read_bytes();err=(dest/'stderr.bin').read_bytes()
        runs.append({'name':name,'argv':argv,'cwd':str(work),'pid':child.pid,'exit':code,
                     'elapsed_seconds':time.monotonic()-started,'stdout_sha256':digest(out),
                     'stderr_sha256':digest(err),'stdout_bytes':len(out),'stderr_bytes':len(err)})
        print(name+' exit='+str(code),flush=True)
        return code,out,err
    def compare_original(name,code,out,err):
        original=root/'original'/name
        check(name+' old stdout exact',out.hex(),(original/'stdout.bin').read_bytes().hex())
        check(name+' old stderr exact',err.hex(),(original/'stderr.bin').read_bytes().hex())
        old=json.loads((root/'original'/'receipt.json').read_bytes())
        check(name+' old exit exact',code,next(r['exit'] for r in old['runs'] if r['name']==name))
    def verify_csv(name,out):
        check(name+' full bytes',out.hex(),expected_csv.hex())
        table=list(csv.reader(io.StringIO(out.decode('utf-8'),newline=''),strict=True))
        check(name+' header',table[0],COLS)
        check(name+' all rows',len(table)-1,len(expected_rows))
        check(name+' literal and decimal fields',table[1:],[[str(row[c]) for c in COLS] for row in expected_rows])
        check(name+' no BOM',out.startswith(b'\xef\xbb\xbf'),False)
    try:
        for reordered in [False,True]:
            suffix='-reordered' if reordered else ''
            flags=['--input=input'+suffix+'.jsonl','--baseline=baseline'+suffix+'.jsonl']
            for fmt in ['json','text','csv']:
                name='valid'+suffix+'-'+fmt
                code,out,err=run(name,flags+['--format='+fmt])
                if fmt=='csv' and args.mode=='original':
                    check(name+' original refused',code,1);check(name+' no partial CSV',out,b'')
                    check(name+' format diagnostic',b'--format must be text or json' in err,True)
                else:
                    check(name+' success',code,0);check(name+' clean diagnostics',err,b'')
                    if fmt=='json':
                        doc=json.loads(out)
                        check(name+' all exact pairs',doc['comparisons'],expected_rows)
                        check(name+' count',doc['summary']['count'],14)
                        check(name+' direction',doc['direction'],'input_minus_baseline')
                    if fmt=='text':
                        check(name+' top-ten label',b'up to 10' in out,True)
                    if fmt=='csv': verify_csv(name,out)
                    if args.mode=='candidate' and fmt!='csv': compare_original(name,code,out,err)
        code,out,err=run('default-text',['--input=input.jsonl','--baseline=baseline.jsonl'])
        check('default exit',code,0)
        check('default text exact',out.hex(),(output/'valid-text'/'stdout.bin').read_bytes().hex())
        check('default stderr',err,b'')
        if args.mode=='candidate':compare_original('default-text',code,out,err)
        errors_to_try=[
          ('duplicate-input','duplicate-input.jsonl','baseline.jsonl','duplicate career'),
          ('duplicate-baseline','input.jsonl','duplicate-baseline.jsonl','duplicate career'),
          ('late-invalid','late-invalid.jsonl','baseline.jsonl','invalid record'),
          ('unmatched-input','unmatched-input.jsonl','baseline.jsonl','has no input partner'),
          ('unmatched-baseline','input.jsonl','unmatched-baseline.jsonl','has no baseline partner'),
          ('empty','empty.jsonl','baseline.jsonl','no career records'),
          ('seed-overflow','seed-overflow.jsonl','baseline.jsonl','invalid record'),
          ('score-overflow','score-overflow.jsonl','baseline.jsonl','invalid record'),
        ]
        for label,inp,base,marker in errors_to_try:
            for fmt in (['json','text'] if args.mode=='original' else ['json','text','csv']):
                name=label+'-'+fmt
                code,out,err=run(name,['--input='+inp,'--baseline='+base,'--format='+fmt])
                check(name+' refusal',code,1);check(name+' no primary output',out,b'')
                check(name+' exact failure class',marker.encode() in err,True)
                if args.mode=='candidate':
                    if fmt!='csv':compare_original(name,code,out,err)
                    else:check(name+' same semantic refusal',err.hex(),(root/'original'/(label+'-json')/'stderr.bin').read_bytes().hex())
    except Exception as exc:
        errors.append({'type':type(exc).__name__,'message':str(exc)})
    finally:
        after={'work':snap_tree(work),'source':snap_tree(source),'exe':snap_file(exe)}
        receipt['after']=after;receipt['all_inputs_source_exe_preserved']=before==after
        receipt['no_simulator_root_created']=not (work/'absent-repo-root').exists()
        receipt['utc_end']=datetime.datetime.now(datetime.timezone.utc).isoformat()
        receipt['success']=not errors and before==after and receipt['no_simulator_root_created'] and all(c['pass'] for c in checks)
        # Bytes-valued zero-output checks use hex so receipt encoding is unambiguous.
        def json_default(value):
            if isinstance(value,bytes):return {'hex':value.hex()}
            raise TypeError(type(value).__name__)
        with (output/'receipt.json').open('x',encoding='utf-8',newline='\n') as f:
            json.dump(receipt,f,ensure_ascii=False,indent=2,default=json_default);f.write('\n')
        print(json.dumps({'success':receipt['success'],'checks':len(checks),'runs':len(runs),'errors':errors},ensure_ascii=True),flush=True)
    return 0 if receipt['success'] else 1

if __name__=='__main__':
    raise SystemExit(main())

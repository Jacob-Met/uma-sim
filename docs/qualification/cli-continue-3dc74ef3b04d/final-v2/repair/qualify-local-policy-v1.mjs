import fs from 'node:fs';
import cp from 'node:child_process';
import crypto from 'node:crypto';
const phase=process.argv[2];
if(!['baseline','candidate'].includes(phase)) throw new Error('explicit baseline or candidate phase required');
const source='/home/jacob/uma-continue-3dc74ef3b04d-local-policy';
const proof='/home/jacob/uma-continue-3dc74ef3b04d-proof';
const stage=proof+'/local-policy-v1/'+phase;
fs.mkdirSync(stage,{recursive:true});
const receiptPath=stage+'/receipt.json';
if(fs.existsSync(receiptPath)) throw new Error('phase already has receipt');
const tmp=stage+'/tmp';fs.mkdirSync(tmp,{recursive:true});
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const check=(v,m)=>{if(!v)throw new Error(m);};
const original=JSON.parse(fs.readFileSync(proof+'/scoped-v2/candidate-scoped-native-receipt.json','utf8'));
const manifestPath='/home/jacob/uma-continue-3dc74ef3b04d-intake/component-source-manifest.json';
const manifestBytes=fs.readFileSync(manifestPath);
check(sha(manifestBytes)==='196bcda1792f2e5d05a4e6f9ad7eb59cd49288dcebf0597fdb8a46df1d768520','manifest pin');
const manifest=JSON.parse(manifestBytes);
const receipt={schema:'hamon.uma_continue.local_policy_native.v1',phase,started:new Date().toISOString(),source,method_sha256:sha(fs.readFileSync(new URL(import.meta.url))),records:[],ok:false};
const write=()=>fs.writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n');
const env={...process.env,TMPDIR:tmp,CARGO_MANIFEST_DIR:source+'/uma-sim-core',CARGO_PKG_VERSION:'0.1.0'};
delete env.UMA_POLICY_CMD;delete env.UMA_RACE_MODEL;delete env.LLVM_PROFILE_FILE;
const run=(label,program,args,extraEnv={})=>{
 const start=new Date().toISOString();
 const r=cp.spawnSync(program,args,{cwd:source,env:{...env,...extraEnv},encoding:'utf8',timeout:180000,maxBuffer:4*1024*1024});
 fs.writeFileSync(stage+'/'+label+'.stdout',r.stdout??'');fs.writeFileSync(stage+'/'+label+'.stderr',r.stderr??'');
 const rec={label,program,args,started:start,finished:new Date().toISOString(),status:r.status,signal:r.signal,error:r.error?.message??null,stdout_sha256:sha(r.stdout??''),stderr_sha256:sha(r.stderr??'')};
 receipt.records.push(rec);write();return {...rec,stdout:r.stdout??'',stderr:r.stderr??''};
};
try {
 const st=fs.statfsSync(proof);receipt.free_bytes=st.bavail*st.bsize;receipt.required_bytes=209715200;
 check(receipt.free_bytes>=receipt.required_bytes,'current scoped-link capacity admission');
 receipt.libraries=original.libraries.map(l=>{const b=fs.readFileSync(l.path);check(sha(b)===l.sha256,'library '+l.name);return {...l,bytes:b.length};});
 receipt.unchanged_component_files=0;
 for(const f of manifest.files){if(f.path==='uma-sim-core/src/bin/uma-sim.rs')continue;check(sha(fs.readFileSync(source+'/'+f.path))===f.sha256,'unchanged component '+f.path);receipt.unchanged_component_files++;}
 const productPaths=['uma-sim-core/src/bin/uma-sim.rs','uma-sim-core/src/bin/uma_sim/continue_run.rs','uma-sim-core/tests/cli_continue.rs','docs/CLI_CONTINUE.md'];
 receipt.source_files=productPaths.map(path=>({path,sha256:sha(fs.readFileSync(source+'/'+path))}));
 receipt.baseline_binary={...original.binary};check(sha(fs.readFileSync(original.binary.path))===original.binary.sha256,'frozen original binary');
 const baseArgs=['--edition=2021','-C','debuginfo=0','-L','dependency='+proof+'/target/debug/deps',...original.libraries.flatMap(l=>['--extern',l.name+'='+l.path])];
 const testSource=source+'/uma-sim-core/tests/cli_continue.rs';
 if(phase==='baseline'){
  const testBin=stage+'/original-continue-tests';
  const compile=run('compile-regression','/usr/bin/rustc',[...baseArgs,'--test',testSource,'-o',testBin],{'CARGO_BIN_EXE_uma-sim':original.binary.path});
  check(compile.status===0,'baseline regression compile');
  const result=run('inherited-policy-regression',testBin,['--exact','inherited_external_policy_command_cannot_override_local_continuation','--nocapture','--test-threads=1']);
  check(result.status===101&&(result.stdout+result.stderr).includes('local --policy=bot invoked the inherited external policy'),'expected original bot marker failure');
  receipt.expected_failure_observed=true;receipt.product_accepted=false;receipt.ok=true;
 }else{
  const binary=stage+'/candidate-uma-sim';
  check(run('compile-cli','/usr/bin/rustc',[...baseArgs,source+'/uma-sim-core/src/bin/uma-sim.rs','-o',binary]).status===0,'candidate CLI compile');
  receipt.binary={path:binary,bytes:fs.statSync(binary).size,sha256:sha(fs.readFileSync(binary))};
  for(const [label,path] of [['continue',testSource],['writer',source+'/uma-sim-core/tests/cli_session_persistence.rs']]){
   const testBin=stage+'/'+label+'-tests';
   check(run('compile-'+label,'/usr/bin/rustc',[...baseArgs,'--test',path,'-o',testBin],{'CARGO_BIN_EXE_uma-sim':binary}).status===0,label+' compile');
   const result=run(label+'-tests',testBin,['--nocapture','--test-threads=1']);
   check(result.status===0,label+' native tests');
   receipt[label+'_test_result']=result.stdout.match(/test result:.*$/m)?.[0]??null;
  }
  receipt.ok=true;
 }
 for(const f of receipt.source_files)check(sha(fs.readFileSync(source+'/'+f.path))===f.sha256,'source changed during gate '+f.path);
 for(const l of receipt.libraries)check(sha(fs.readFileSync(l.path))===l.sha256,'library changed during gate '+l.name);
 receipt.finished=new Date().toISOString();write();console.log(JSON.stringify({phase,ok:receipt.ok,receipt:receiptPath,sha256:sha(fs.readFileSync(receiptPath)),binary:receipt.binary,continue:receipt.continue_test_result,writer:receipt.writer_test_result,records:receipt.records.map(r=>({label:r.label,status:r.status}))}));
} catch(error){receipt.error=error.message;receipt.finished=new Date().toISOString();write();console.error(error.stack);process.exitCode=1;}

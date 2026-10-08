// Independent receiving expectations derived from original source 8d74413d7dbc3caf1745779c92d56b6eb7feea42 before reviewing the candidate patch for issue113.
// Revision2 corrects only the normal configured-symlink expectation to original fs::write behavior; failure and unrelated-file controls remain unchanged.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2), options = {};
for (let i=0;i<args.length;i+=2) {
  const key=args[i], value=args[i+1];
  assert.ok(['--baseline','--candidate','--source','--session-sha256','--cli-sha256','--out'].includes(key) && value && !options[key], 'one explicit value per supported option');
  options[key]=value;
}
assert.equal(Object.keys(options).length,6,'Usage: node receive-cli-persistence.mjs --baseline EXACT_BINARY --candidate EXACT_BINARY --source CANDIDATE_SOURCE --session-sha256 HASH --cli-sha256 HASH --out NEW_REVIEWER_DIRECTORY');
const output=path.resolve(options['--out']);
assert.ok(output.startsWith('/dev/shm/integration-69570d292200-cli-persistence-'),'exclusive reviewer tmpfs namespace');
const binary={baseline:fs.realpathSync(options['--baseline']),candidate:fs.realpathSync(options['--candidate'])};
assert.notEqual(binary.baseline,binary.candidate,'separate retained native binaries');
const source=fs.realpathSync(options['--source']);
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const gitBlob=b=>crypto.createHash('sha1').update('blob '+b.length+'\0').update(b).digest('hex');
const sourcePins={};
for (const [relative,expected] of [
  ['uma-sim-core/src/session.rs',options['--session-sha256']],
  ['uma-sim-core/src/bin/uma-sim.rs',options['--cli-sha256']],
]) {
  const bytes=fs.readFileSync(path.join(source,relative));
  assert.equal(sha(bytes),expected,relative+' candidate source pin');
  sourcePins[relative]={bytes:bytes.length,sha256:sha(bytes),gitBlob:gitBlob(bytes)};
}
const binaryPins=Object.fromEntries(Object.entries(binary).map(([label,name])=>[label,{path:name,bytes:fs.statSync(name).size,sha256:sha(fs.readFileSync(name))}]));
const scriptBytes=fs.readFileSync(fileURLToPath(import.meta.url));
const shimSource=path.join(path.dirname(fileURLToPath(import.meta.url)),'fault-inject.c');
const shimBytes=fs.readFileSync(shimSource);
fs.mkdirSync(output);
const shim=path.join(output,'fault-inject.so');
const compiler=spawnSync('cc',['-shared','-fPIC','-O2','-Wall','-Wextra','-Werror','-o',shim,shimSource,'-ldl'],{encoding:'utf8',timeout:30000,env:{...process.env,TMPDIR:output}});
fs.writeFileSync(path.join(output,'shim-compile.stdout'),compiler.stdout??'');
fs.writeFileSync(path.join(output,'shim-compile.stderr'),compiler.stderr??'');
assert.ifError(compiler.error); assert.equal(compiler.status,0,compiler.stderr);
const env={...process.env};
for(const key of ['UMA_POLICY_CMD','LLVM_PROFILE_FILE','LD_PRELOAD','HAMON_REVIEW_PHASE','HAMON_REVIEW_DIR','HAMON_REVIEW_TARGET'])delete env[key];
const common=['--scenario=ura','--dialogue=off','--race-model=stub','--deck=10001'];
const initialArgs=['start','--seed=42',...common];
const commands={
  start:['start','--seed=73',...common],
  step:['step','rest'],
  fast:['fast','--seed=73',...common,'--policy=default'],
  deck:['deck','place','10001','speed'],
};
const cases=[], failures=[];
function invoke(label,cwd,argv,phase) {
  const childEnv={...env};
  if(phase && phase!=='limit'){
    childEnv.LD_PRELOAD=shim;
    childEnv.HAMON_REVIEW_PHASE=phase;
    childEnv.HAMON_REVIEW_DIR=path.join(cwd,'.uma-sim');
    childEnv.HAMON_REVIEW_TARGET=path.join(cwd,'.uma-sim/session.json');
  }
  const executable=phase==='limit'?'/bin/bash':binary[label];
  const argvNative=phase==='limit'?['-c','trap "" XFSZ\nulimit -f 1\nexec "$@"','uma-independent-size-limit',binary[label],...argv]:argv;
  const result=spawnSync(executable,argvNative,{cwd,env:childEnv,encoding:'utf8',timeout:30000,maxBuffer:8*1024*1024});
  assert.ifError(result.error); assert.equal(result.signal,null,'native child must finish rather than be signaled');
  return {exit:result.status,stdout:result.stdout,stderr:result.stderr};
}
function saveLogs(directory,prefix,result) {
  fs.writeFileSync(path.join(directory,prefix+'.stdout'),result.stdout);
  fs.writeFileSync(path.join(directory,prefix+'.stderr'),result.stderr);
}
function attempt(name,body) {
  try{body();}catch(error){failures.push({name,message:error.message,stack:error.stack});}
}
const seedDirectory=path.join(output,'original-readable-snapshot');
fs.mkdirSync(seedDirectory);
const seeded=invoke('baseline',seedDirectory,initialArgs);
saveLogs(seedDirectory,'start',seeded);
assert.equal(seeded.exit,0,seeded.stderr);assert.equal(seeded.stderr,'');
const initial=fs.readFileSync(path.join(seedDirectory,'.uma-sim/session.json'));
assert.ok(initial.length>1024,'prior valid snapshot must exceed the child write limit');
JSON.parse(initial);
fs.writeFileSync(path.join(output,'prior-session.json'),initial);
const initialState=invoke('baseline',seedDirectory,['state']);
assert.equal(initialState.exit,0);assert.ok(!initialState.stdout.includes('No session.'));
saveLogs(seedDirectory,'state',initialState);
function fixture(name,kind='regular'){
  const directory=path.join(output,name);fs.mkdirSync(directory);
  const saveDirectory=path.join(directory,'.uma-sim');
  if(kind==='obstruction'){
    fs.writeFileSync(saveDirectory,'independent parent obstruction sentinel\n');
    return {directory,saveDirectory,save:path.join(saveDirectory,'session.json'),kind};
  }
  fs.mkdirSync(saveDirectory);
  const save=path.join(saveDirectory,'session.json');
  fs.writeFileSync(save,initial);
  fs.writeFileSync(path.join(saveDirectory,'unrelated.txt'),'unrelated sibling sentinel\n');
  fs.writeFileSync(path.join(saveDirectory,'other.tmp'),'unrelated temporary-file sentinel\n');
  fs.writeFileSync(path.join(saveDirectory,'other-career.json'),initial);
  fs.symlinkSync('unrelated.txt',path.join(saveDirectory,'unrelated-link'));
  if(kind==='symlink'){
    fs.unlinkSync(save);fs.symlinkSync('other-career.json',save);
  }
  if(kind==='hardlink')fs.linkSync(save,path.join(saveDirectory,'hardlinked-prior.json'));
  return {directory,saveDirectory,save,kind};
}
function inventory(directory){
  return Object.fromEntries(fs.readdirSync(directory).sort().map(name=>{
    const full=path.join(directory,name),st=fs.lstatSync(full);
    if(st.isSymbolicLink())return [name,{kind:'symlink',target:fs.readlinkSync(full)}];
    assert.ok(st.isFile(),name+' unexpected nonfile in private session directory');
    const bytes=fs.readFileSync(full);
    return [name,{kind:'regular',bytes:bytes.length,sha256:sha(bytes)}];
  }));
}
function receipt(name,label,command,phase,fixture,run,before,after,reloaded){
  const entry={name,binary:label,args:commands[command],phase:phase??'normal',fixture:fixture.kind,exit:run.exit,
    stdoutBytes:Buffer.byteLength(run.stdout),stdoutSha256:sha(Buffer.from(run.stdout)),stderr:run.stderr,
    priorBytes:initial.length,priorSha256:sha(initial),afterBytes:after?.length??null,afterSha256:after?sha(after):null,
    priorPreserved:after?initial.equals(after):null,afterJson:after?(()=>{try{JSON.parse(after);return true;}catch{return false;}})():null,
    beforeEntries:before,afterEntries:fixture.kind==='obstruction'?null:inventory(fixture.saveDirectory),
    reloadExit:reloaded?.exit??null,reloadStdout:reloaded?.stdout??null};
  cases.push(entry);
  saveLogs(fixture.directory,'command',run);
  if(reloaded)saveLogs(fixture.directory,'reload',reloaded);
  return entry;
}
function assertSiblings(before,after,extras=[]){
  const expected=Object.keys(before).filter(n=>n!=='session.json').sort();
  const actual=Object.keys(after).filter(n=>n!=='session.json'&&!extras.includes(n)).sort();
  assert.deepEqual(actual,expected,'no temporary remnants or missing unrelated files');
  for(const name of expected)assert.deepEqual(after[name],before[name],name+' unrelated sibling preserved');
}
const healthy={baseline:{},candidate:{}};
for(const label of ['baseline','candidate'])for(const command of Object.keys(commands))attempt(label+'-healthy-'+command,()=>{
  const name=label+'-healthy-'+command,f=fixture(name),before=inventory(f.saveDirectory);
  const result=invoke(label,f.directory,commands[command]);
  const after=fs.readFileSync(f.save),reloaded=invoke(label,f.directory,['state']);
  receipt(name,label,command,null,f,result,before,after,reloaded);
  assert.equal(result.exit,0,result.stderr);assert.equal(result.stderr,'');assert.ok(result.stdout.length>0);
  assert.equal(reloaded.exit,0);assert.equal(reloaded.stderr,'');assert.ok(!reloaded.stdout.includes('No session.'));
  const snapshot=JSON.parse(after);
  assertSiblings(before,inventory(f.saveDirectory));
  if(command==='start'||command==='fast')assert.equal(snapshot.meta.seed,73,'new seed is actually reloaded');
  if(command==='step')assert.ok(!after.equals(initial),'step publishes changed snapshot');
  if(command==='deck')assert.equal(snapshot.state.deck.slots.find(x=>x.supportId==='10001'||x.support_id==='10001').assignedFacility??snapshot.state.deck.slots.find(x=>x.supportId==='10001'||x.support_id==='10001').assigned_facility,'speed','placed card is persisted');
  healthy[label][command]={snapshot,reload:reloaded.stdout,stdout:result.stdout.replace(/elapsed=\d+ms/g,'elapsed=<measured>ms')};
});
for(const command of Object.keys(commands))attempt('healthy-source-parity-'+command,()=>{
  assert.deepEqual(healthy.candidate[command],healthy.baseline[command],'normal snapshot, RNG, reloaded state and output preserve original semantics');
});
for(const label of ['baseline','candidate'])for(const command of Object.keys(commands))attempt(label+'-limited-'+command,()=>{
  const name=label+'-limited-'+command,f=fixture(name),before=inventory(f.saveDirectory);
  const result=invoke(label,f.directory,commands[command],'limit');
  const after=fs.readFileSync(f.save),reloaded=invoke(label,f.directory,['state']);
  const row=receipt(name,label,command,'limit',f,result,before,after,reloaded);
  assert.match(result.stderr,/Failed to save session:/);
  assertSiblings(before,inventory(f.saveDirectory));
  if(label==='baseline'){
    assert.equal(result.exit,0,'original caller false-success witness');assert.ok(result.stdout.length>0);
    assert.equal(row.priorPreserved,false);assert.equal(after.length,1024);assert.equal(row.afterJson,false);
  }else{
    assert.notEqual(result.exit,0);assert.equal(result.stdout,'','no success-shaped output after failed persistence');
    assert.deepEqual(after,initial,'prior valid bytes survive partial-write error');
    assert.deepEqual(reloaded,initialState,'native load still returns prior career');
    assert.deepEqual(inventory(f.saveDirectory),before,'failed attempt leaves exact directory inventory');
  }
});
for(const phase of ['sync','rename'])attempt('candidate-'+phase+'-failure',()=>{
  const name='candidate-'+phase+'-failure',f=fixture(name),before=inventory(f.saveDirectory);
  const result=invoke('candidate',f.directory,commands.step,phase);
  const after=fs.readFileSync(f.save),reloaded=invoke('candidate',f.directory,['state']);
  receipt(name,'candidate','step',phase,f,result,before,after,reloaded);
  assert.match(result.stderr,new RegExp('HAMON_INDEPENDENT_FAULT '+phase+'(?: fsync| fdatasync)? bytes=([1-9][0-9]+)'));
  assert.match(result.stderr,/Failed to save session:/);assert.notEqual(result.exit,0);assert.equal(result.stdout,'');
  assert.deepEqual(after,initial,'prior valid bytes survive '+phase+' error');
  assert.deepEqual(reloaded,initialState,'native load survives '+phase+' error');
  assert.deepEqual(inventory(f.saveDirectory),before,'failed attempt cleans only its private temporary file');
});
for(const command of ['start','fast'])attempt('candidate-obstructed-'+command,()=>{
  const name='candidate-obstructed-'+command,f=fixture(name,'obstruction'),before=fs.readFileSync(f.saveDirectory);
  const result=invoke('candidate',f.directory,commands[command]);
  receipt(name,'candidate',command,'parent-obstruction',f,result,null,null,null);
  assert.notEqual(result.exit,0);assert.match(result.stderr,/Failed to save session:/);assert.equal(result.stdout,'');
  assert.deepEqual(fs.readFileSync(f.saveDirectory),before);
});
// The explicit symlink target is the configured session destination, not an unrelated sibling.
// Original fs::write followed that target; retain this success behavior while replacing its bytes atomically.
for(const kind of ['symlink','hardlink'])attempt('candidate-'+kind+'-alias-preservation',()=>{
  const name='candidate-'+kind+'-alias-preservation',f=fixture(name,kind),before=inventory(f.saveDirectory);
  const result=invoke('candidate',f.directory,commands.step);
  const after=fs.readFileSync(f.save),reloaded=invoke('candidate',f.directory,['state']);
  receipt(name,'candidate','step',null,f,result,before,after,reloaded);
  assert.equal(result.exit,0,result.stderr);assert.equal(result.stderr,'');
  const current=inventory(f.saveDirectory);
  if(kind==='symlink'){
    const beforeUnrelated={...before},afterUnrelated={...current};
    delete beforeUnrelated['other-career.json'];delete afterUnrelated['other-career.json'];
    assertSiblings(beforeUnrelated,afterUnrelated);
    assert.deepEqual(current['session.json'],before['session.json'],'configured session symlink remains unchanged');
    assert.deepEqual(JSON.parse(fs.readFileSync(path.join(f.saveDirectory,'other-career.json'))),healthy.candidate.step.snapshot,'resolved target contains the new saved career');
  }else{
    assertSiblings(before,current);
    assert.ok(fs.lstatSync(f.save).isFile()&&!fs.lstatSync(f.save).isSymbolicLink(),'successful session publication is regular file');
    assert.deepEqual(fs.readFileSync(path.join(f.saveDirectory,'hardlinked-prior.json')),initial,'another hardlink retains the previous bytes');
  }
  assert.deepEqual(JSON.parse(after),healthy.candidate.step.snapshot);
  assert.equal(reloaded.stdout,healthy.candidate.step.reload);
});
attempt('candidate-symlink-failed-write',()=>{
  const name='candidate-symlink-failed-write',f=fixture(name,'symlink'),before=inventory(f.saveDirectory);
  const result=invoke('candidate',f.directory,commands.step,'limit');
  const after=fs.readFileSync(f.save),reloaded=invoke('candidate',f.directory,['state']);
  receipt(name,'candidate','step','limit',f,result,before,after,reloaded);
  assert.notEqual(result.exit,0);assert.equal(result.stdout,'');assert.match(result.stderr,/Failed to save session:/);
  assert.deepEqual(after,initial);assert.deepEqual(reloaded,initialState);
  assert.deepEqual(inventory(f.saveDirectory),before,'failed preparation preserves prior symlink and all alias bytes');
});
attempt('candidate-exclusive-name-collision',()=>{
  const name='candidate-exclusive-name-collision',f=fixture(name),before=inventory(f.saveDirectory);
  const result=invoke('candidate',f.directory,commands.step,'collision');
  const after=fs.readFileSync(f.save),reloaded=invoke('candidate',f.directory,['state']);
  receipt(name,'candidate','step','collision',f,result,before,after,reloaded);
  const match=result.stderr.match(/^HAMON_INDEPENDENT_COLLISION (.+)$/m);
  assert.ok(match,'exclusive temporary creation collision was actually injected');
  const collided=match[1];
  assert.equal(path.dirname(collided),f.saveDirectory,'injected file is only in this reviewer fixture');
  assert.notEqual(collided,f.save,'existing destination is never opened for exclusive temporary allocation');
  assert.equal(fs.readFileSync(collided,'utf8'),'independent temporary-name collision sentinel\n','another file occupying an attempted name is not overwritten or removed');
  assertSiblings(before,inventory(f.saveDirectory),[path.basename(collided)]);
  if(result.exit===0){
    assert.deepEqual(JSON.parse(after),healthy.candidate.step.snapshot);
    assert.equal(reloaded.stdout,healthy.candidate.step.reload);
    assert.ok(result.stdout.length>0,'successful retry retains normal output');
  }else{
    assert.match(result.stderr,/Failed to save session:/);assert.equal(result.stdout,'');
    assert.deepEqual(after,initial);assert.deepEqual(reloaded,initialState);
  }
});
const sourceUnchanged=Object.entries(sourcePins).every(([relative,pin])=>sha(fs.readFileSync(path.join(source,relative)))===pin.sha256);
const binariesUnchanged=Object.entries(binaryPins).every(([label,pin])=>sha(fs.readFileSync(binary[label]))===pin.sha256);
assert.ok(sourceUnchanged&&binariesUnchanged,'read-only source and executables remain unchanged');
const record={recordedAt:new Date().toISOString(),reviewer:'integration_lane',runtime:process.version,platform:process.platform,
  source,sourcePins,binaryPins,sourceUnchanged,binariesUnchanged,
  program:{bytes:scriptBytes.length,sha256:sha(scriptBytes)},faultShim:{bytes:shimBytes.length,sha256:sha(shimBytes),compiledSha256:sha(fs.readFileSync(shim))},
  priorSnapshot:{bytes:initial.length,sha256:sha(initial)},cases,failures,result:failures.length?'FAIL':'PASS',
  boundary:'Reviewer-authored public native CLI receiver; original/current source and candidate binary provenance recorded separately by native builder; child-only RLIMIT and private-directory interposition; no source edits, installs, other-owner writes, command serialization or power-loss durability claim.'};
fs.writeFileSync(path.join(output,'receipt.json'),JSON.stringify(record,null,2)+'\n');
console.log(JSON.stringify(record,null,2));
process.exitCode=failures.length?1:0;

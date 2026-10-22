import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';

const root='/dev/shm/production-69570d292200/session-persistence';
const binary='/home/jacob/estate-production-2983fe20/baseline-target/debug/uma-sim';
const output=path.join(root,'baseline-v1');
fs.mkdirSync(output);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const binaryBefore=hash(fs.readFileSync(binary));
const env={...process.env}; delete env.UMA_POLICY_CMD; delete env.LLVM_PROFILE_FILE;
const receipts=[];
function run(name,args,obstruct=false,limited=false){
  const cwd=path.join(output,name);
  fs.mkdirSync(cwd);
  if(obstruct)fs.writeFileSync(path.join(cwd,'.uma-sim'),'private obstruction sentinel\n');
  const setupArgs=['start','--seed=42','--scenario=ura','--dialogue=off','--race-model=stub','--deck=10001'];
  let before=null;
  if(args[0]==='step'||args[0]==='deck'){
    const start=spawnSync(binary,setupArgs,{cwd,env,encoding:'utf8',timeout:30000});
    if(start.status!==0)throw new Error('Healthy fixture failed: '+start.stderr);
    before=fs.readFileSync(path.join(cwd,'.uma-sim/session.json'));
    JSON.parse(before);
    fs.writeFileSync(path.join(cwd,'prior-session.json'),before);
  }
  const command=limited?'/bin/bash':binary;
  const argv=limited?['-c','trap "" XFSZ\nulimit -f 1\nexec "$@"','uma-session-size-limit',binary,...args]:args;
  const process=spawnSync(command,argv,{cwd,env,encoding:'utf8',timeout:30000});
  if(process.error)throw process.error;
  fs.writeFileSync(path.join(cwd,'stdout.log'),process.stdout);
  fs.writeFileSync(path.join(cwd,'stderr.log'),process.stderr);
  let after=null,afterJson=null;
  const save=path.join(cwd,'.uma-sim/session.json');
  if(fs.existsSync(save)){after=fs.readFileSync(save);try{JSON.parse(after);afterJson=true;}catch{afterJson=false;}}
  const receipt={name,args,limited,obstructed:obstruct,exit:process.status,signal:process.signal,
    stdoutBytes:Buffer.byteLength(process.stdout),stderr:process.stderr,
    priorBytes:before?.length,afterBytes:after?.length,priorSha256:before&&hash(before),
    afterSha256:after&&hash(after),priorPreserved:before&&after&&before.equals(after),afterJson,
    obstructionPreserved:obstruct?fs.readFileSync(path.join(cwd,'.uma-sim'),'utf8')==='private obstruction sentinel\n':null};
  receipts.push(receipt);
}
run('healthy-start',['start','--seed=42','--scenario=ura','--dialogue=off','--race-model=stub']);
run('obstructed-start',['start','--seed=42','--scenario=ura','--dialogue=off','--race-model=stub'],true);
run('obstructed-fast',['fast','--seed=42','--scenario=ura','--dialogue=off','--race-model=stub','--policy=default'],true);
run('limited-step',['step','rest'],false,true);
run('limited-deck',['deck','place','10001','speed'],false,true);
const record={recordedAt:new Date().toISOString(),runtime:process.version,binary,binarySha256:binaryBefore,
 binaryUnchanged:hash(fs.readFileSync(binary))===binaryBefore,
 provenance:{retainedBaseline:'6245d48132df3f07aaa314f5faa9cb31536449b1',currentSource:'8d74413d7dbc3caf1745779c92d56b6eb7feea42',sessionAndAffectedCallerSpansByteIdentical:true,entireCurrentMainBuild:false},
 receipts};
fs.writeFileSync(path.join(output,'receipt.json'),JSON.stringify(record,null,2)+'\n');
console.log(JSON.stringify(record,null,2));

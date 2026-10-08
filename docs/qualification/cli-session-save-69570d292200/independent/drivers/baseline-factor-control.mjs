import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
const root='/dev/shm/integration-69570d292200-cli-factor-control-20261008T2013';
const receiving='/dev/shm/integration-69570d292200-cli-persistence-native-v2-20261008T2005';
const binary='/dev/shm/production-69570d292200/session-persistence/baseline-uma-sim';
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const expected='bb17d29839a7485718a1fffafd9a916a4fe5f1077ef8038ae2b10de1e8376a9c';
assert.equal(sha(binary),expected);
fs.mkdirSync(root);
const original=JSON.parse(fs.readFileSync(path.join(receiving,'baseline-healthy-fast/.uma-sim/session.json'),'utf8'));
const candidate=JSON.parse(fs.readFileSync(path.join(receiving,'candidate-healthy-fast/.uma-sim/session.json'),'utf8'));
function diff(a,b,at='$',out=[]){if(a===b)return out;if(a&&b&&typeof a==='object'&&typeof b==='object'){for(const key of [...new Set([...Object.keys(a),...Object.keys(b)])].sort())diff(a[key],b[key],at+'.'+key,out);}else out.push({path:at,original:a,replay:b});return out;}
const env={...process.env};for(const key of ['LD_PRELOAD','HAMON_REVIEW_PHASE','HAMON_REVIEW_DIR','HAMON_REVIEW_TARGET','UMA_POLICY_CMD','LLVM_PROFILE_FILE'])delete env[key];
const args=['fast','--seed=73','--scenario=ura','--dialogue=off','--race-model=stub','--deck=10001','--policy=default'];
const observed=new Set([JSON.stringify(original.state.generatedSparks)]);
const cases=[];
for(let i=1;i<=3;i++){
 const cwd=path.join(root,'baseline-replay-'+i);fs.mkdirSync(cwd);
 const result=spawnSync(binary,args,{cwd,env,encoding:'utf8',timeout:30000});
 fs.writeFileSync(path.join(cwd,'command.stdout'),result.stdout??'');fs.writeFileSync(path.join(cwd,'command.stderr'),result.stderr??'');
 assert.ifError(result.error);assert.equal(result.signal,null);assert.equal(result.status,0,result.stderr);
 const file=path.join(cwd,'.uma-sim/session.json'),snapshot=JSON.parse(fs.readFileSync(file,'utf8'));
 observed.add(JSON.stringify(snapshot.state.generatedSparks));
 cases.push({name:'baseline-replay-'+i,exit:result.status,stdout:result.stdout,stderr:result.stderr,snapshotSha256:sha(file),sparks:snapshot.state.generatedSparks,versusOriginalBaseline:diff(original,snapshot),versusCandidate:diff(candidate,snapshot)});
 if(observed.size>=2)break;
}
assert.equal(sha(binary),expected);
const receipt={recordedAt:new Date().toISOString(),reviewerRequestedControl:'At most three fresh unchanged-baseline fast commands; stop on two distinct baseline spark results; retain original v2 FAIL.',baselineBinary:{path:binary,sha256:expected,unchanged:true},command:args,originalBaselineSparks:original.state.generatedSparks,originalCandidateSparks:candidate.state.generatedSparks,distinctBaselineSparkResults:observed.size,baselineVariationObserved:observed.size>=2,cases};
fs.writeFileSync(path.join(root,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify(receipt));

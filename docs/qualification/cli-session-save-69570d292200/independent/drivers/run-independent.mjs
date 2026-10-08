import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
const root='/dev/shm/production-69570d292200/session-persistence';
const scripts='/dev/shm/integration-69570d292200-cli-persistence-inputs-v2';
const output='/dev/shm/integration-69570d292200-cli-persistence-native-v2-20261008T2005';
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
for(const [file,expected] of [
 [path.join(root,'baseline-uma-sim'),'bb17d29839a7485718a1fffafd9a916a4fe5f1077ef8038ae2b10de1e8376a9c'],
 [path.join(root,'candidate-uma-sim'),'ab4bbcaf06e55234abe688ec50cce7e8307e155fdc2a1dacbde769c28f61ff03'],
 [path.join(scripts,'receive-cli-persistence.mjs'),'54ba104a4253c3dd3ffe45c707d3ba215cd874d0ebe826ae5bba25f4df383b5e'],
 [path.join(scripts,'fault-inject.c'),'68b9b4cf5f0629f942d93b69c14703f25db0b874aa0ba612c68162aa33e43585']
])if(hash(file)!==expected)throw new Error('Receiving input mismatch: '+file);
const args=[path.join(scripts,'receive-cli-persistence.mjs'),'--baseline',path.join(root,'baseline-uma-sim'),'--candidate',path.join(root,'candidate-uma-sim'),'--source',path.join(root,'source'),'--session-sha256','95653ec2bc94c22598443c9e2e1c9d0301f2ec0183f0856d8c5baaa88098d637','--cli-sha256','b6acab8c62d9dfee3779f9f1b282427062a21a6030bf8dfac68654bdf2a119ed','--out',output];
const log=path.join(root,'independent-v2.log'),fd=fs.openSync(log,'wx');
const startedAt=new Date().toISOString();
const result=spawnSync('node',args,{stdio:['ignore',fd,fd],timeout:600000});
fs.fsyncSync(fd);fs.closeSync(fd);
const receipt={startedAt,completedAt:new Date().toISOString(),command:['node',...args],status:result.status,signal:result.signal,error:result.error?.message??null,output,log:{path:log,sha256:hash(log)}};
fs.writeFileSync(path.join(root,'independent-v2-execution.json'),JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify(receipt));
if(fs.existsSync(path.join(output,'receipt.json'))){const record=JSON.parse(fs.readFileSync(path.join(output,'receipt.json'),'utf8'));console.log(JSON.stringify({result:record.result,cases:record.cases.length,failures:record.failures,sourceUnchanged:record.sourceUnchanged,binariesUnchanged:record.binariesUnchanged}));}else console.log(fs.readFileSync(log,'utf8').slice(-2000));

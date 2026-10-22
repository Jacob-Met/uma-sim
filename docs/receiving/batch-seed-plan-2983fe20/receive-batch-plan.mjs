import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const [binary, source, evidence] = process.argv.slice(2);
if (!binary || !source || !evidence) throw Error('binary source evidence required');
fs.mkdirSync(evidence, {recursive:true});
const hash = b => createHash('sha256').update(b).digest('hex');
const env = {...process.env, UMA_REPO_ROOT:source};
delete env.UMA_POLICY_CMD; delete env.UMA_RACE_MODEL;
const cases = [
 ['malformed_count',['--seed=7','--count=oops']],
 ['empty_count',['--seed=7','--count=']],
 ['zero_count',['--seed=7','--count=0']],
 ['negative_count',['--seed=7','--count=-3']],
 ['range_overflow',['--seed=9223372036854775807','--count=2']],
 ['valid_consecutive',['--seed=7','--count=2']],
 ['explicit_precedence',['--seed=9223372036854775807','--count=oops','--seeds=10,5,10']],
 ['first_count_precedence',['--seed=7','--count=2','--count=oops']],
 ['maximum_endpoint',['--seed=9223372036854775806','--count=2']],
 ['minimum_seed',['--seed=-9223372036854775808','--count=1']]
];
const results=[];
for (const [name, flags] of cases) {
 const cwd=path.join(evidence,name);fs.mkdirSync(cwd,{recursive:true});
 const run=args=>spawnSync(binary,[...args,'--race-model=stub','--dialogue=off','--speed=100'],{cwd,env,encoding:'utf8',timeout:90000,maxBuffer:1024*1024});
 const start=run(['start','--seed=7']);
 if(start.status!==0)throw Error('start fixture failed: '+start.stderr);
 const session=path.join(cwd,'.uma-sim/session.json'), before=fs.readFileSync(session);
 const out=path.join(cwd,'batch.jsonl'), sentinel=Buffer.from('AUTHORED OUTPUT MUST SURVIVE REFUSED INPUT\n');
 fs.writeFileSync(out,sentinel);
 const got=run(['batch',...flags,'--output='+out]);
 const after=fs.readFileSync(out);
 const rowSeeds=after.toString().trim().split('\n').filter(Boolean).map(line=>line.match(/"seed"\s*:\s*(-?\d+)/)?.[1] ?? null);
 const entry={name,flags,status:got.status,signal:got.signal,error:got.error?.message??null,stdout:got.stdout,stderr:got.stderr,outputPreserved:after.equals(sentinel),outputBeforeSha256:hash(sentinel),outputAfterSha256:hash(after),outputBytes:after.length,rowCount:rowSeeds.length,rowSeeds,sessionPreserved:fs.readFileSync(session).equals(before),sessionSha256:hash(before)};
 results.push(entry); console.log(JSON.stringify({name,status:entry.status,outputPreserved:entry.outputPreserved,rowCount:entry.rowCount,sessionPreserved:entry.sessionPreserved}));
}
const receipt={at:new Date().toISOString(),binary,binarySha256:hash(fs.readFileSync(binary)),source,sourceHead:spawnSync('git',['-C',source,'rev-parse','HEAD'],{encoding:'utf8'}).stdout.trim(),results};
fs.writeFileSync(path.join(evidence,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const root='/dev/shm/production-69570d292200/session-persistence';
const source=path.join(root,'source');
const digest=b=>crypto.createHash('sha256').update(b).digest('hex');
const blob=b=>crypto.createHash('sha1').update('blob '+b.length+'\0').update(b).digest('hex');
const prior=JSON.parse(fs.readFileSync(path.join(root,'baseline-rust-receipt.json'),'utf8'));
if(prior.binary?.sha256!=='bb17d29839a7485718a1fffafd9a916a4fe5f1077ef8038ae2b10de1e8376a9c')throw new Error('Unexpected baseline');
if(!fs.readFileSync(path.join(root,'baseline-rust.log'),'utf8').includes('4 passed; 6 failed;'))throw new Error('Negative control incomplete');
const manifest=JSON.parse(fs.readFileSync(path.join(root,'current-source.json'),'utf8'));
for(const f of manifest.files)if(blob(fs.readFileSync(path.join(source,f.path)))!==f.blob)throw new Error('Baseline source changed: '+f.path);
const pins=[{"path":"uma-sim-core/src/bin/uma-sim.rs","gitBlob":"f54c2c50cdae8b78ed75c0eec89a0cdca9f36a37"},{"path":"uma-sim-core/src/session.rs","gitBlob":"8298731be4ee70a6338cc5d708c7c5f2f4549749"},{"path":"uma-sim-core/tests/cli_session_persistence.rs","gitBlob":"ac3a6e11a293f153db49016c756555ad4fffdcd2"}];
for(const [from,to] of [['candidate-session.rs','uma-sim-core/src/session.rs'],['candidate-cli.rs','uma-sim-core/src/bin/uma-sim.rs']])fs.copyFileSync(path.join(root,from),path.join(source,to));
for(const p of pins){const bytes=fs.readFileSync(path.join(source,p.path));if(blob(bytes)!==p.gitBlob)throw new Error('Candidate source mismatch: '+p.path);p.sha256=digest(bytes);p.bytes=bytes.length;}
const changes=[];
for(const f of manifest.files){const actual=blob(fs.readFileSync(path.join(source,f.path)));if(actual!==f.blob)changes.push({path:f.path,before:f.blob,after:actual});}
if(changes.length!==2)throw new Error('Unexpected tracked change count');
fs.mkdirSync(path.join(source,'packages/uma-sim-ui/dist'),{recursive:true});
const record={recordedAt:new Date().toISOString(),baselineCommit:manifest.receipt.commit,candidateCommit:'acbe58aeface82d7e3cac1134c6d7126a910bb15',verifiedBaselineLeaves:manifest.files.length,changes,pins,buildOnlyPreparation:'Empty packages/uma-sim-ui/dist directory prevents perpetual build.rs invalidation in this non-embed-ui subset; no generated or embedded UI.',baselineBinary:prior.binary};
fs.writeFileSync(path.join(root,'candidate-source.json'),JSON.stringify(record,null,2)+'\n');console.log(JSON.stringify(record));

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
const root='/dev/shm/production-69570d292200/session-persistence';
const peer='/home/jacob/estate-production-2983fe20/uma-batch';
const commit='8d74413d7dbc3caf1745779c92d56b6eb7feea42';
const source=path.join(root,'source');
const env={...process.env,GIT_OPTIONAL_LOCKS:'0'};
const git=(...args)=>execFileSync('git',['-C',peer,...args],{env,maxBuffer:64*1024*1024});
try{const partial=git('config','--get','extensions.partialclone').toString().trim();if(partial)throw new Error('Do not fetch through another owner repository');}catch(e){if(e.status!==1)throw e;}
git('cat-file','-e',commit+'^{commit}');
const selected=['Cargo.toml','Cargo.lock','rustfmt.toml','LICENSE','uma-sim-core','uma-race-core','research','knowledge','content_packs'];
const raw=git('ls-tree','-rz','-l',commit,'--',...selected).toString();
const leaves=raw.split('\0').filter(Boolean).map(line=>{
 const m=/^(\d+) blob ([0-9a-f]+)\s+(\d+)\t(.+)$/s.exec(line);
 if(!m||!['100644','100755'].includes(m[1]))throw new Error('Unsupported source leaf');
 return {mode:m[1],blob:m[2],bytes:Number(m[3]),path:m[4]};
});
const sourceBytes=leaves.reduce((n,f)=>n+f.bytes,0);
const capacity=fs.statfsSync('/dev/shm');const available=capacity.bavail*capacity.bsize;
if(available<3*1024**3||sourceBytes>100*1024**2)throw new Error('Capacity admission refused');
if(fs.existsSync(source))throw new Error('Source destination exists; do not overwrite');
fs.mkdirSync(source);
const archive=git('archive','--format=tar',commit,...selected);
const unpack=spawnSync('/usr/bin/tar',['-xf','-','-C',source],{input:archive,encoding:'utf8'});
if(unpack.status!==0)throw new Error(unpack.stderr);
const blob=bytes=>crypto.createHash('sha1').update('blob '+bytes.length+'\0').update(bytes).digest('hex');
for(const f of leaves){if(blob(fs.readFileSync(path.join(source,f.path)))!==f.blob)throw new Error('Source mismatch: '+f.path);}
const receipt={recordedAt:new Date().toISOString(),source,commit,leaves:leaves.length,sourceBytes,availableBefore:available,sourceVerified:true,readOnlyObjectSource:peer};
fs.writeFileSync(path.join(root,'current-source.json'),JSON.stringify({receipt,files:leaves},null,2)+'\n');
console.log(JSON.stringify(receipt));

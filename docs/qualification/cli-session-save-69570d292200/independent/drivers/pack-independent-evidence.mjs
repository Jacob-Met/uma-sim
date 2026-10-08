import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const root='/dev/shm/production-69570d292200/session-persistence';
const v2='/dev/shm/integration-69570d292200-cli-persistence-native-v2-20261008T2005';
const control='/dev/shm/integration-69570d292200-cli-factor-control-20261008T2013';
const guard='/dev/shm/integration-69570d292200-cli-target-guard-20261008T2013';
for(const p of [path.join(v2,'receipt.json'),path.join(control,'receipt.json'),path.join(guard,'receiving.json')])JSON.parse(fs.readFileSync(p,'utf8'));
const files=[],manifest=[];
const digest=b=>crypto.createHash('sha256').update(b).digest('hex');
function scan(directory,prefix){for(const entry of fs.readdirSync(directory,{withFileTypes:true})){
 const absolute=path.join(directory,entry.name),relative=prefix+'/'+entry.name,st=fs.lstatSync(absolute);
 if(st.isDirectory())scan(absolute,relative);
 else if(st.isSymbolicLink())manifest.push({path:relative,kind:'symlink',target:fs.readlinkSync(absolute)});
 else if(st.isFile()){
  const bytes=fs.readFileSync(absolute),sha256=digest(bytes);
  if(entry.name==='fault-inject.so'){manifest.push({path:relative,kind:'reproducible compiled helper, not committed',bytes:bytes.length,sha256});continue;}
  const content=bytes.toString('utf8');if(!Buffer.from(content).equals(bytes))throw new Error('Unexpected binary evidence '+relative);
  manifest.push({path:relative,kind:'file',bytes:bytes.length,sha256});
  files.push({path:relative,content});
 }else throw new Error('Unexpected filesystem evidence kind');
}}
scan(v2,'independent/v2');scan(control,'independent/factor-control');scan(guard,'independent/target-guard');
for(const [from,to] of [['independent-v2-execution.json','independent/v2-execution.json'],['independent-v2.log','independent/v2-console.log']]){
 const bytes=fs.readFileSync(path.join(root,from));files.push({path:to,content:bytes.toString('utf8')});manifest.push({path:to,kind:'file',bytes:bytes.length,sha256:digest(bytes)});
}
files.push({path:'independent/filesystem-manifest.json',content:JSON.stringify({recordedAt:new Date().toISOString(),boundary:'Exact native regular text files; symlink destinations retained in this manifest; reproducible compiled helper described by hash. No native binaries copied.',entries:manifest},null,2)+'\n'});
fs.writeFileSync(path.join(root,'independent-packet.json'),JSON.stringify(files)+'\n');console.log(JSON.stringify({files:files.length,entries:manifest.length,textBytes:files.reduce((n,f)=>n+Buffer.byteLength(f.content),0),packetBytes:fs.statSync(path.join(root,'independent-packet.json')).size}));

#!/usr/bin/env node
/** Install, inspect, or roll back one generated UMA training page in an isolated route. */
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const MARKER='<!-- hamon-demo:uma-training -->';
const SCHEMA='hamon.uma_training.static_install.v1';
const MAX_BYTES=8*1024*1024;
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const fail=(code,message)=>{throw Object.assign(new Error(message),{code});};
const exists=p=>{try{return fs.lstatSync(p);}catch(e){if(e.code==='ENOENT')return null;throw e;}};
const inside=(p,dir)=>{const r=path.relative(dir,p);return r===''||(!r.startsWith('..'+path.sep)&&r!=='..'&&!path.isAbsolute(r));};

function canonicalTarget(value){
  const p=path.resolve(value);
  if(path.basename(p)!=='training-demo')fail('WRONG_ROUTE','Target directory must be named training-demo.');
  const target=path.join(fs.realpathSync(path.dirname(p)),'training-demo');
  const st=exists(target);
  if(st&&(!st.isDirectory()||st.isSymbolicLink()))fail('UNSAFE_TARGET','Target must be a real directory, not a link.');
  return target;
}

function canonicalReceipt(value,target){
  const p=path.resolve(value),out=path.join(fs.realpathSync(path.dirname(p)),path.basename(p));
  if(inside(out,target))fail('PUBLIC_RECEIPT','Receipt must be outside the route; it holds the previous page bytes.');
  return out;
}

function fileSnapshot(file,{required=false,marked=true}={}){
  const st=exists(file);
  if(!st){if(required)fail('MISSING_FILE','Required file is absent: '+file);return {exists:false};}
  if(!st.isFile()||st.isSymbolicLink())fail('UNSAFE_FILE','Expected a regular file, not a link: '+file);
  if(st.size>MAX_BYTES)fail('OVERSIZED_FILE','Page exceeds the 8 MiB receiving limit.');
  const bytes=fs.readFileSync(file);
  if(marked&&!bytes.includes(Buffer.from(MARKER)))fail('FOREIGN_PAGE','Page lacks the UMA training application marker.');
  return {exists:true,bytes:bytes.length,sha256:hash(bytes),mode:st.mode&0o777,base64:bytes.toString('base64')};
}

function validateSnapshot(s){
  if(!s||typeof s.exists!=='boolean')fail('BAD_RECEIPT','Invalid snapshot.');
  if(!s.exists)return;
  if(!Number.isInteger(s.bytes)||s.bytes<0||s.bytes>MAX_BYTES||!Number.isInteger(s.mode)||s.mode<0||s.mode>0o777||
     typeof s.base64!=='string'||!/^[a-f0-9]{64}$/.test(s.sha256))fail('BAD_RECEIPT','Invalid snapshot fields.');
  const b=Buffer.from(s.base64,'base64');
  if(b.toString('base64')!==s.base64||b.length!==s.bytes||hash(b)!==s.sha256||!b.includes(Buffer.from(MARKER)))
    fail('BAD_RECEIPT','Snapshot bytes, application marker, or digest disagree.');
}

const same=(a,b)=>a.exists===b.exists&&(!a.exists||(a.bytes===b.bytes&&a.sha256===b.sha256&&
  (process.platform==='win32'||a.mode===b.mode)));

function inspect(receipt){
  const actual=fileSnapshot(path.join(receipt.target,'index.html'),{marked:false});
  const installed=same(actual,receipt.installed),original=same(actual,receipt.original);
  return {state:installed?'installed':original?'original':'changed',matches_installed:installed,matches_original:original,
    actual:actual.exists?{exists:true,bytes:actual.bytes,sha256:actual.sha256,mode:actual.mode}:{exists:false}};
}

function loadReceipt(value){
  const file=path.resolve(value),st=exists(file);
  if(!st||!st.isFile()||st.isSymbolicLink()||st.size>3*MAX_BYTES)fail('BAD_RECEIPT','Receipt must be a bounded regular file.');
  const receipt=JSON.parse(fs.readFileSync(file,'utf8'));
  if(receipt.schema!==SCHEMA||typeof receipt.target!=='string')fail('BAD_RECEIPT','Unknown receipt schema or target.');
  if(canonicalTarget(receipt.target)!==receipt.target||canonicalReceipt(file,receipt.target)!==receipt.receipt)
    fail('BAD_RECEIPT','Receipt paths no longer bind to this isolated route and receipt file.');
  validateSnapshot(receipt.original);validateSnapshot(receipt.installed);
  if(!receipt.installed.exists)fail('BAD_RECEIPT','Installed snapshot must contain a page.');
  return receipt;
}

function replacePage(target,bytes,mode,expected){
  const file=path.join(target,'index.html'),tmp=path.join(target,'.training-demo-'+crypto.randomBytes(12).toString('hex')+'.tmp');
  let created=false;
  try{
    const fd=fs.openSync(tmp,'wx',0o600);created=true;
    try{fs.writeFileSync(fd,bytes);fs.fchmodSync(fd,mode);fs.fsyncSync(fd);}finally{fs.closeSync(fd);}
    if(!same(fileSnapshot(file,{marked:false}),expected))fail('TARGET_CHANGED','Target changed before replacement; no page was replaced.');
    fs.renameSync(tmp,file);created=false;
  }finally{if(created)fs.unlinkSync(tmp);}
}

function install(args){
  const target=canonicalTarget(args.target),receiptFile=canonicalReceipt(args.receipt,target);
  if(exists(receiptFile))fail('RECEIPT_EXISTS','Use a new receipt path for each install; prior receipts are immutable.');
  const source=path.resolve(args.source),installed=fileSnapshot(source,{required:true});
  if(!/^[a-f0-9]{64}$/.test(args.sha256)||installed.sha256!==args.sha256)fail('SOURCE_HASH_MISMATCH','Source differs from the supplied artifact SHA-256.');
  const original=fileSnapshot(path.join(target,'index.html'));
  installed.mode=0o644;
  const receipt={schema:SCHEMA,created_at:new Date().toISOString(),target,receipt:receiptFile,
    source,original,installed,
    scope:'Only index.html; receipt is private, no other route files or live service configuration are changed.',
    concurrency:'Use an isolated target and serialize installs. Hash checks detect observed changes; no cross-process transaction is claimed.'};
  const fd=fs.openSync(receiptFile,'wx',0o600);
  try{fs.writeFileSync(fd,JSON.stringify(receipt,null,2)+'\n');fs.fsyncSync(fd);}finally{fs.closeSync(fd);}
  if(!exists(target))fs.mkdirSync(target,{mode:0o755});
  replacePage(target,Buffer.from(installed.base64,'base64'),installed.mode,original);
  const result=inspect(receipt);
  if(!result.matches_installed)fail('READBACK_FAILED','Installed bytes did not match; keep the receipt for inspection.');
  return {ok:true,action:'install',target,receipt:receiptFile,...result};
}

function rollback(receipt){
  let result=inspect(receipt);
  if(result.matches_original)return {ok:true,action:'rollback',effect:'none_original_already_present',target:receipt.target,...result};
  if(!result.matches_installed)fail('TARGET_CHANGED','Current page differs from both receipt snapshots; rollback refused.');
  if(receipt.original.exists){
    replacePage(receipt.target,Buffer.from(receipt.original.base64,'base64'),receipt.original.mode,receipt.installed);
  }else{
    const file=path.join(receipt.target,'index.html');
    if(!same(fileSnapshot(file,{marked:false}),receipt.installed))fail('TARGET_CHANGED','Target changed before removal; rollback refused.');
    fs.unlinkSync(file);
  }
  result=inspect(receipt);
  if(!result.matches_original)fail('READBACK_FAILED','Rollback bytes did not match the original snapshot.');
  return {ok:true,action:'rollback',effect:'original_restored',target:receipt.target,...result};
}

const HELP=`UMA-SIM training-turn artifact receiving (Node.js 18+)

  node receive.mjs install --source index.html --sha256 HEX --target /existing/parent/training-demo --receipt /private/existing/folder/install.json
  node receive.mjs readback --receipt /private/existing/folder/install.json
  node receive.mjs rollback --receipt /private/existing/folder/install.json

Both parent folders must already exist. Source and replaced page must carry:
${MARKER}
The immutable receipt holds exact before/after bytes and must stay outside the
served route. Rollback preserves every other file and leaves the route directory.
After rollback, reinstall with a new receipt path to restore the candidate.
Use an isolated target, serialize operations, and keep receipts private.
This helper does not publish, start a server, or alter another route.
`;

try{
  const [action,...rest]=process.argv.slice(2);
  if(!action||action==='--help'||action==='help'){process.stdout.write(HELP);}
  else{
    if(!['install','readback','rollback'].includes(action))fail('ARGUMENT','Unknown action.');
    const args={};const allowed=action==='install'?['source','sha256','target','receipt']:['receipt'];
    if(rest.length%2)fail('ARGUMENT','Options require values.');
    for(let i=0;i<rest.length;i+=2){const key=rest[i].replace(/^--/,'');
      if(!rest[i].startsWith('--')||!allowed.includes(key)||args[key]!==undefined||!rest[i+1])fail('ARGUMENT','Unknown, duplicate, or missing option.');
      args[key]=rest[i+1];}
    if(allowed.some(k=>args[k]===undefined))fail('ARGUMENT','Missing required option.');
    const result=action==='install'?install(args):action==='rollback'?rollback(loadReceipt(args.receipt)):
      {ok:true,action:'readback',...inspect(loadReceipt(args.receipt))};
    process.stdout.write(JSON.stringify(result)+'\n');
  }
}catch(e){process.stderr.write(JSON.stringify({ok:false,code:e.code||'RECEIVING_ERROR',message:e.message})+'\n');process.exitCode=2;}

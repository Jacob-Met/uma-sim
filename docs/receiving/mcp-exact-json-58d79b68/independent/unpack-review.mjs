import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
const here=path.dirname(fileURLToPath(import.meta.url));
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const archive=fs.readFileSync(path.join(here,'native-review.json.gz'));
assert.equal(archive.length,638323);
assert.equal(digest(archive),'19c8fd460cda9c03efd32b696ffa18286b67aea6f879dc7706334abf1dd7d316');
const packet=JSON.parse(gunzipSync(archive).toString('utf8'));
assert.equal(packet.schema,'uma_mcp.independent_packet.v1');
assert.equal(packet.files.length,297);
const entries=new Map();
for(const entry of packet.files){
 assert.equal(typeof entry.path,'string');
 assert.ok(entry.path.length&&!entry.path.includes('\\')&&!entry.path.includes('\0')&&!path.isAbsolute(entry.path));
 assert.ok(entry.path.split('/').every(part=>part!==''&&part!=='.'&&part!=='..'));
 assert.ok(!entries.has(entry.path),'Duplicate archive path');
 assert.ok(Number.isInteger(entry.mode)&&entry.mode>=0&&entry.mode<=0o777);
 assert.equal(typeof entry.content_base64,'string');
 entries.set(entry.path,{mode:entry.mode,bytes:Buffer.from(entry.content_base64,'base64')});
}
const manifestBytes=entries.get('manifest.json').bytes;
assert.equal(digest(manifestBytes),'b662798f8ed9dc4759ed16d7eb39d084bf3e58f7279a507ae043a25203764c3d');
assert.ok(fs.readFileSync(path.join(here,'manifest.json')).equals(manifestBytes));
const manifest=JSON.parse(manifestBytes.toString('utf8'));
assert.equal(manifest.files.length,296);
assert.deepEqual([...entries.keys()].sort(),[...manifest.files.map(x=>x.path),'manifest.json'].sort());
for(const expected of manifest.files){
 const entry=entries.get(expected.path);
 assert.equal(entry.bytes.length,expected.bytes,expected.path);
 assert.equal(entry.mode,expected.mode,expected.path);
 assert.equal(digest(entry.bytes),expected.sha256,expected.path);
 assert.equal(createHash('sha1').update(Buffer.from('blob '+entry.bytes.length+'\0')).update(entry.bytes).digest('hex'),expected.git_blob,expected.path);
}
const args=process.argv.slice(2);
assert.ok(args.length===0||(args.length===2&&args[0]==='--out'),'Use no arguments to verify, or --out NEW_DIRECTORY');
let destination=null;
if(args.length){
 destination=path.resolve(args[1]);assert.ok(!fs.existsSync(destination),'Destination already exists');
 fs.mkdirSync(destination,{recursive:true});
 for(const [relative,entry] of entries){
  const output=path.join(destination,relative);
  fs.mkdirSync(path.dirname(output),{recursive:true});
  fs.writeFileSync(output,entry.bytes,{flag:'wx',mode:entry.mode});fs.chmodSync(output,entry.mode);
  assert.ok(fs.readFileSync(output).equals(entry.bytes));assert.equal(fs.statSync(output).mode&0o777,entry.mode);
 }
}
console.log(JSON.stringify({verified_manifested_files:296,archive_files:297,archive_sha256:digest(archive),destination}));

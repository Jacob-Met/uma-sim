import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import {fileURLToPath} from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const blob = b => crypto.createHash('sha1').update(Buffer.from('blob '+b.length+'\0')).update(b).digest('hex');
const manifestBytes = fs.readFileSync(path.join(here,'manifest.json'));
const manifest = JSON.parse(manifestBytes);
const bundle = JSON.parse(zlib.gunzipSync(fs.readFileSync(path.join(here,'native-author.json.gz'))));
if (bundle.schema !== 'uma.mcp.exact-json.author-packet.v1' || bundle.manifestSha256 !== hash(manifestBytes)) throw Error('Manifest binding mismatch');
if (JSON.stringify(bundle.manifest) !== JSON.stringify(manifest) || bundle.files.length !== manifest.files.length) throw Error('Manifest content mismatch');
const seen = new Set();
const verified = [];
for (let i=0;i<bundle.files.length;i++) {
  const entry = bundle.files[i], meta = manifest.files[i];
  if (!entry.path || entry.path.includes('\\') || entry.path.includes('\0') || path.posix.isAbsolute(entry.path) || path.posix.normalize(entry.path) !== entry.path || entry.path.split('/').some(p => p==='..'||p==='.') || seen.has(entry.path)) throw Error('Unsafe or duplicate path');
  seen.add(entry.path);
  const {dataBase64,...actualMeta} = entry;
  if (JSON.stringify(actualMeta)!==JSON.stringify(meta) || !Number.isInteger(entry.mode) || (entry.mode & ~0o777)!==0) throw Error('Entry metadata mismatch');
  const bytes = Buffer.from(dataBase64,'base64');
  if (bytes.toString('base64')!==dataBase64 || bytes.length!==entry.bytes || hash(bytes)!==entry.sha256 || blob(bytes)!==entry.gitBlob) throw Error('Entry content mismatch: '+entry.path);
  verified.push([entry,bytes]);
}
if (process.argv.length>3) throw Error('Usage: node unpack-author.mjs [new-directory]');
const destination = process.argv[2] ? path.resolve(process.argv[2]) : null;
if (destination) {
  fs.mkdirSync(destination);
  for (const [entry,bytes] of verified) {
    const output = path.join(destination,entry.path);
    fs.mkdirSync(path.dirname(output),{recursive:true});
    fs.writeFileSync(output,bytes,{flag:'wx',mode:entry.mode});
    fs.chmodSync(output,entry.mode);
  }
}
console.log(JSON.stringify({verified:true,files:verified.length,bytes:verified.reduce((n,[e])=>n+e.bytes,0),manifestSha256:hash(manifestBytes),destination}));

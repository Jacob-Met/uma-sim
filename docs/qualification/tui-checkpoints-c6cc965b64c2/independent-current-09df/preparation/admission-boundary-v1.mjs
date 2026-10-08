import fs from "node:fs";
import path from "node:path";
import cp from "node:child_process";
import crypto from "node:crypto";
import http from "node:http";
import net from "node:net";
import assert from "node:assert/strict";
import {setTimeout as delay} from "node:timers/promises";
const root="/dev/shm/c6cc965b64c2-terminal-replacement-09df-v1";
const run=path.join(root,"admission-boundary-v1");
fs.mkdirSync(run);
const apiCwd=path.join(run,"api-cwd"),terminalCwd=path.join(run,"terminal-cwd"),artifacts=path.join(run,"artifacts");
for(const p of [apiCwd,terminalCwd,artifacts])fs.mkdirSync(p);
const manifest=JSON.parse(fs.readFileSync(path.join(root,"source-manifest.json"),"utf8"));
const candidate=path.join(root,"source/packages/uma-sim-cli/tui.js"),baseline=path.join(root,"baseline/packages/uma-sim-cli/tui.js");
const hash=b=>crypto.createHash("sha256").update(b).digest("hex"),hashFile=p=>hash(fs.readFileSync(p));
function ordered(x){if(Array.isArray(x))return x.map(ordered);if(x&&typeof x==="object")return Object.fromEntries(Object.keys(x).sort().map(k=>[k,ordered(x[k])]));return x;}
function lossless(raw){return JSON.stringify(ordered(JSON.parse(raw,(_k,v,c)=>typeof v==="number"?{"$numericToken":c.source}:v)));}
assert.equal(JSON.parse("9223372036854775807",(_k,_v,c)=>c.source),"9223372036854775807","numeric-token oracle requires native JSON source context");
function sourcePins(){
 for(const f of manifest.files){assert.equal(hashFile(path.join(manifest.source_repo,f.path)),f.sha256,"owner source "+f.path);assert.equal(hashFile(path.join(root,"source",f.path)),f.sha256,"received source "+f.path);}
 assert.equal(hashFile(baseline),manifest.baseline_tui.sha256);assert.equal(hashFile(manifest.binary.path),manifest.binary.sha256);
 return {owner_and_received_files:manifest.files.length,baseline_tui_exact:true,binary_exact:true};
}
const receipt={schema:"hamon.uma.terminal_durable.admission_boundary.v1",source_commit:manifest.source_commit,source_parent:manifest.source_parent,source_manifest_sha256:hashFile(path.join(root,"source-manifest.json")),binary:manifest.binary,node:process.version,platform:process.platform,arch:process.arch,source_before:sourcePins(),cases:[],clients:[],http_requests:[],native_processes:[],status:"running",qualified:false};
let phase="setup",apiProcess,apiBase,proxy,fault,held=[],serial=0,proxyBase;
const liveClients=new Set();
const save=()=>fs.writeFileSync(path.join(run,"receipt.json"),JSON.stringify(receipt,null,2)+"\n");
const fileArtifact=(name,data)=>{const p=path.join(artifacts,name);fs.writeFileSync(p,data);return{path:path.relative(run,p),bytes:Buffer.byteLength(data),sha256:hash(data)};};
const deadline=async(test,label,ms=8000)=>{const end=Date.now()+ms;while(!test()){if(Date.now()>end)throw Error("timeout: "+label);await delay(10);}};
async function port(){const s=net.createServer();await new Promise((r,j)=>{s.once("error",j);s.listen(0,"127.0.0.1",r);});const p=s.address().port;await new Promise(r=>s.close(r));return p;}
async function call(method,route,body,channel="oracle"){
 const r=await fetch(apiBase+route,{method,headers:body===undefined?undefined:{"Content-Type":"application/json"},body:body===undefined?undefined:JSON.stringify(body),signal:AbortSignal.timeout(8000)});
 const raw=await r.text(),json=JSON.parse(raw);receipt.http_requests.push({sequence:++serial,phase,channel,method,route,body,status:r.status,response_sha256:hash(raw),response_bytes:Buffer.byteLength(raw),error:json?.error});return{status:r.status,raw,json};
}
async function ok(method,route,body){const r=await call(method,route,body);assert.equal(r.status,200,method+" "+route+": "+r.raw);return r;}
async function startApi(){
 const p=await port(),env={...process.env,UMA_REPO_ROOT:manifest.source_repo};delete env.UMA_POLICY_CMD;
 const item={pid:null,port:p,cwd:apiCwd,binary:manifest.binary.path,args:[String(p)],environment:{UMA_REPO_ROOT:manifest.source_repo,UMA_POLICY_CMD:"omitted"},stdout:"",stderr:""};
 const child=cp.spawn(manifest.binary.path,[String(p)],{cwd:apiCwd,env,stdio:["ignore","pipe","pipe"]});item.pid=child.pid;
 child.stdout.setEncoding("utf8");child.stderr.setEncoding("utf8");child.stdout.on("data",s=>item.stdout+=s);child.stderr.on("data",s=>item.stderr+=s);child.once("exit",(code,signal)=>item.exit={code,signal});child.once("error",e=>item.error=e.message);
 receipt.native_processes.push(item);apiProcess={child,item};apiBase="http://127.0.0.1:"+p;
 const until=Date.now()+10000;let ready=false;while(Date.now()<until&&!ready){if(item.exit||item.error)throw Error("API startup: "+JSON.stringify(item));try{const r=await fetch(apiBase+"/v1/health",{signal:AbortSignal.timeout(400)});ready=r.ok&&(await r.json()).ok===true;}catch{}if(!ready)await delay(20);}
 assert(ready,"native API startup health");return item;
}
async function stopApi(){if(!apiProcess)return;const {child,item}=apiProcess;if(!item.exit){child.kill("SIGTERM");await deadline(()=>!!item.exit,"API stop",3000);}apiProcess=undefined;}
async function startProxy(){
 proxy=http.createServer(async(req,res)=>{
  const chunks=[];for await(const b of req)chunks.push(b);const raw=Buffer.concat(chunks).toString();
  const row={sequence:++serial,phase,channel:"tui",method:req.method,route:req.url,body:raw?JSON.parse(raw):undefined,request_raw:raw};receipt.http_requests.push(row);
  let selected;if(fault&&fault.match(row)){selected=fault;fault=undefined;row.injected_fault=selected.mode;}
  const forwarded=selected?.mode==="replace-request-body"?Buffer.from(selected.hex,"hex"):raw;if(selected?.mode==="replace-request-body")row.forwarded_request_hex=selected.hex;
  try{const u=await fetch(apiBase+req.url,{method:req.method,headers:raw?{"Content-Type":"application/json"}:undefined,body:forwarded||undefined,signal:AbortSignal.timeout(8000)});const body=await u.text();row.status=u.status;row.response_sha256=hash(body);row.response_bytes=Buffer.byteLength(body);if(u.status>=400)row.error=JSON.parse(body).error;
   if(selected){selected.observed={row,body,status:u.status};if(selected.mode==="drop-after-native-response"){res.destroy();return;}if(selected.mode==="hold-after-native-response"){held.push(res);return;}}
   res.writeHead(u.status,{"Content-Type":"application/json"});res.end(body);
  }catch(e){row.proxy_error=e.message;if(!res.destroyed){res.writeHead(502,{"Content-Type":"application/json"});res.end(JSON.stringify({error:"receiver upstream failure: "+e.message}));}}
 });
 await new Promise((r,j)=>{proxy.once("error",j);proxy.listen(0,"127.0.0.1",r);});return "http://127.0.0.1:"+proxy.address().port;
}
function launch(args,{old=false}={}){
 const record={number:receipt.clients.length+1,phase,args,script:old?baseline:candidate,cwd:terminalCwd,stdout:"",stderr:"",exit:null};
 const child=cp.spawn("/opt/homebrew/bin/node",[record.script,...args],{cwd:terminalCwd,env:{...process.env,UMA_SIM_API:proxyBase},stdio:["pipe","pipe","pipe"]});record.pid=child.pid;
 child.stdout.setEncoding("utf8");child.stderr.setEncoding("utf8");child.stdout.on("data",s=>record.stdout+=s);child.stderr.on("data",s=>record.stderr+=s);child.stdin.on("error",e=>record.stdin_error=e.code);child.once("error",e=>record.error=e.message);child.once("exit",(code,signal)=>{record.exit={code,signal};liveClients.delete(child);});
 receipt.clients.push(record);liveClients.add(child);const prompts=()=>record.stdout.split("\n> ").length-1;
 return{child,record,prompts,async ready(){await deadline(()=>prompts()>0||record.exit||record.error,"terminal ready");assert(prompts()>0,"expected prompt: "+record.stderr);},
 async command(text){const n=prompts(),o=record.stdout.length,e=record.stderr.length;child.stdin.write(text+"\n");await deadline(()=>prompts()>n||record.exit,"command "+text);assert(!record.exit,"terminal unexpectedly exited "+record.stderr);return{stdout:record.stdout.slice(o),stderr:record.stderr.slice(e)};},
 async finish(text="quit"){child.stdin.write(text+"\n");await deadline(()=>!!record.exit,"terminal quit");assert.equal(record.exit.code,0,record.stderr);},
 async exited(){await deadline(()=>!!record.exit||!!record.error,"terminal exit");assert(!record.error,record.error);return record;}};
}
async function group(name,fn){phase=name;const start=receipt.http_requests.length,t=Date.now();try{const details=await fn();receipt.cases.push({name,status:"pass",elapsed_ms:Date.now()-t,http_requests:receipt.http_requests.length-start,details});save();console.log(JSON.stringify({group:name,status:"pass",details}));}catch(error){receipt.cases.push({name,status:"failure",elapsed_ms:Date.now()-t,error:error.message,stack:error.stack});save();console.log(JSON.stringify({group:name,status:"failure",error:error.message}));throw error;}}
async function startSession(id,seed){return ok("POST","/v1/run/start",{session:id,seed,scenario:"ura",trainee:"Special Week",speed:"1",traceTelemetry:"true",traceRng:"true"});}
async function state(id){return ok("GET","/v1/run/state?session="+encodeURIComponent(id));}
async function sessions(){return (await ok("GET","/v1/sessions")).json;}
async function activate(id){return ok("POST","/v1/session/activate",{session:id});}
async function mainState(){const old=(await sessions()).active;await activate("");const r=await state("");if(old!=="")await activate(old);return r;}
async function advance(id,n){const before=await state(id);for(let i=0;i<n;i++)await ok("POST","/v1/run/auto",{session:id,policy:"bot"});const after=await state(id);assert.notEqual(lossless(after.raw),lossless(before.raw),"native advance must change actual state");return after;}
const library=path.join(apiCwd,".uma-sim/library"),cpPaths=name=>({snapshot:path.join(library,name+".snapshot.json"),meta:path.join(library,name+".meta.json")});
function cpBytes(name){return Object.fromEntries(Object.entries(cpPaths(name)).map(([k,p])=>[k,{raw:fs.readFileSync(p,"utf8"),sha256:hashFile(p)}]));}
function cpPins(name){return Object.fromEntries(Object.entries(cpBytes(name)).map(([k,v])=>[k,v.sha256]));}
const savedName="--"+"K".repeat(59)+".-_",primaryId="edge-"+"N".repeat(59),otherId="bystander";
let mainExpected,otherExpected,savedSnapshot,savedPins,primaryAfterSave,primaryAfterAuto,forkAId,forkBId,recoveredId;
async function invariant(){assert.equal(lossless((await mainState()).raw),lossless(mainExpected),"main state was modified");assert.equal(lossless((await state(otherId)).raw),lossless(otherExpected),"bystander state was modified");if(savedPins)assert.deepEqual(cpPins(savedName),savedPins,"original checkpoint was modified");}
function tuiRows(from=0){return receipt.http_requests.slice(from).filter(r=>r.channel==="tui");}
function proposedId(c){const m=c.record.stdout.match(/as session: (tui-[a-f0-9-]+)/);assert(m,"checkpoint proposed id missing");return m[1];}
function sameState(actual,expected,message){assert.equal(lossless(actual),lossless(expected),message);}
function cloneCheckpoint(name,{schema,malformedMeta=false,malformedSnapshot=false}={}){const b=cpBytes(savedName),p=cpPaths(name);let meta=b.meta.raw.replace(/("name"\s*:\s*)"[^"]*"/,'$1'+JSON.stringify(name));if(schema!==undefined)meta=meta.replace(/("snapshotSchema"\s*:\s*)\d+/,"$1"+schema);fs.writeFileSync(p.meta,malformedMeta?"{":meta);fs.writeFileSync(p.snapshot,malformedSnapshot?"{":b.snapshot.raw);return cpPins(name);}
function libraryPins(){return fs.existsSync(library)?Object.fromEntries(fs.readdirSync(library).sort().map(n=>[n,hashFile(path.join(library,n))])):{};}
function oneRejectedPost(rows,route,detail){
 const posts=rows.filter(r=>r.method==="POST");assert.equal(posts.length,1,"rejected request must not replay or reset");assert.equal(posts[0].route,route);assert.equal(posts[0].status,400);assert.equal(posts[0].error,detail);return posts[0];
}
function cleanKnownError(stderr,detail){assert(stderr.includes("HTTP 400: "+detail));assert(!stderr.includes("may have reached"));assert(!stderr.includes("retried automatically"));assert(!stderr.includes("Starting REST API"));}
try{
 await startApi();proxyBase=await startProxy();
 await startSession("","-9007199254740993");mainExpected=(await mainState()).raw;
 await startSession(primaryId,"-9223372036854775807");await advance(primaryId,7);
 await startSession(otherId,"9007199254740993");otherExpected=(await state(otherId)).raw;
 await ok("POST","/v1/library/save",{name:savedName,session:primaryId,overwrite:false});
 savedSnapshot=cpBytes(savedName).snapshot.raw;savedPins=cpPins(savedName);
 await group("nonobject-action-body-is-visible-and-next-explicit-action-applies-once",async()=>{
  const before=(await state(primaryId)).raw;await ok("POST","/v1/session/fork",{session:primaryId,id:"admission-action-oracle"});await ok("POST","/v1/run/auto",{session:"admission-action-oracle",policy:"bot"});const expected=(await state("admission-action-oracle")).raw;
  assert.notEqual(lossless(before),lossless(expected),"oracle action must make actual state progress");
  await activate(otherId);const sessionsBefore=(await sessions()).sessions.map(x=>x.id).sort(),disk=libraryPins();
  const at=receipt.http_requests.length,c=launch(["--session="+primaryId]);await c.ready();
  const f={mode:"replace-request-body",hex:Buffer.from("null").toString("hex"),match:r=>r.method==="POST"&&r.route==="/v1/run/auto"};fault=f;
  const rejected=await c.command("auto");cleanKnownError(rejected.stderr,"JSON request body must be an object");
  assert.equal(f.observed.status,400);const row=oneRejectedPost(tuiRows(at),"/v1/run/auto","JSON request body must be an object");assert.equal(row.body.session,primaryId);
  sameState((await state(primaryId)).raw,before,"rejected auto advanced selected state");assert.deepEqual((await sessions()).sessions.map(x=>x.id).sort(),sessionsBefore);assert.deepEqual(libraryPins(),disk);await invariant();
  const retryAt=receipt.http_requests.length;const explicit=await c.command("auto");assert.equal(explicit.stderr,"");await c.finish();const next=tuiRows(retryAt).filter(r=>r.method==="POST");assert.equal(next.length,1);assert.equal(next[0].status,200);assert.equal(next[0].body.session,primaryId);
  sameState((await state(primaryId)).raw,expected,"explicit recovery must apply exactly one native oracle action");await invariant();
  return{forwarded_body_utf8:"null",native_status:400,native_error:row.error,terminal_kept_prompt:true,rejected_mutation_count:0,no_implicit_replay_reset_or_new_session:true,subsequent_explicit_action_status:200,exact_independent_one_step_result:true,library_bytes_unchanged:true};
 });
 await group("malformed-save-body-is-visible-and-never-writes-default-checkpoint",async()=>{
  const before=(await state(primaryId)).raw,disk=libraryPins(),sessionsBefore=(await sessions()).sessions.map(x=>x.id).sort(),name="admission-explicit-save";
  await activate(otherId);const at=receipt.http_requests.length,c=launch(["--session="+primaryId]);await c.ready();
  const malformed=JSON.stringify({session:primaryId,name,overwrite:false}).slice(0,-1),f={mode:"replace-request-body",hex:Buffer.from(malformed).toString("hex"),match:r=>r.method==="POST"&&r.route==="/v1/library/save"};fault=f;
  const rejected=await c.command("save "+name);cleanKnownError(rejected.stderr,"invalid JSON request body");assert.equal(f.observed.status,400);
  const row=oneRejectedPost(tuiRows(at),"/v1/library/save","invalid JSON request body");assert.equal(row.body.session,primaryId);assert.equal(row.body.name,name);assert.equal(row.body.overwrite,false);
  assert.deepEqual(libraryPins(),disk,"rejected save must not generate a default-named checkpoint");sameState((await state(primaryId)).raw,before,"save rejection advanced selected state");assert.deepEqual((await sessions()).sessions.map(x=>x.id).sort(),sessionsBefore);await invariant();
  const retryAt=receipt.http_requests.length,explicit=await c.command("save "+name);assert.equal(explicit.stderr,"");await c.finish();const next=tuiRows(retryAt).filter(r=>r.method==="POST");assert.equal(next.length,1);assert.equal(next[0].status,200);assert.equal(next[0].body.name,name);assert.equal(next[0].body.overwrite,false);
  sameState(cpBytes(name).snapshot.raw,before,"explicit save must preserve exact selected state");sameState((await state(primaryId)).raw,before,"explicit save advanced state");await invariant();
  return{native_status:400,native_error:row.error,rejected_save_requests:1,rejected_new_files:0,terminal_kept_prompt:true,subsequent_explicit_create_only_save_status:200,exact_selected_snapshot_saved:true,main_bystander_and_original_checkpoint_unchanged:true};
 });
 await group("unreadable-fork-body-exits-without-session-or-fallback",async()=>{
  await activate(otherId);const registryBefore=lossless((await ok("GET","/v1/sessions")).raw),disk=libraryPins(),sourceBefore=(await state(primaryId)).raw,at=receipt.http_requests.length;
  const f={mode:"replace-request-body",hex:"ff",match:r=>r.method==="POST"&&r.route==="/v1/session/fork"};fault=f;
  const c=launch(["--checkpoint="+savedName]);await c.exited();assert.equal(c.record.exit.code,1);assert.equal(c.record.exit.signal,null);cleanKnownError(c.record.stderr,"could not read request body");assert.equal(c.prompts(),0);assert.equal(f.observed.status,400);
  const row=oneRejectedPost(tuiRows(at),"/v1/session/fork","could not read request body");assert.equal(row.body.checkpoint,savedName);assert(row.body.id.startsWith("tui-"));assert(!("session" in row.body));assert(!("replace" in row.body));
  assert.equal(lossless((await ok("GET","/v1/sessions")).raw),registryBefore,"rejected fork must leave registry and active session exact");assert.deepEqual(libraryPins(),disk);sameState((await state(primaryId)).raw,sourceBefore,"rejected fork altered source");await invariant();
  return{forwarded_request_hex:"ff",native_status:400,native_error:row.error,terminal_exit:1,new_sessions:0,fork_requests:1,no_start_load_replace_or_replay:true,checkpoint_and_session_bytes_unchanged:true};
 });
 receipt.source_after=sourcePins();receipt.status="passed";receipt.qualified=true;
}catch(error){receipt.status="failure";receipt.error=error.message;receipt.stack=error.stack;process.exitCode=1;
}finally{
 fault=undefined;for(const h of held)h.destroy();for(const child of liveClients)child.kill("SIGTERM");if(liveClients.size)await delay(50);await stopApi();if(proxy){proxy.closeAllConnections();await new Promise(r=>proxy.close(r));}
 receipt.finished_utc=new Date().toISOString();receipt.summary={groups:receipt.cases.length,passed:receipt.cases.filter(c=>c.status==="pass").length,failures:receipt.cases.filter(c=>c.status==="failure").length,terminal_processes:receipt.clients.length,native_api_processes:receipt.native_processes.length,http_requests:receipt.http_requests.length};save();console.log(JSON.stringify({status:receipt.status,qualified:receipt.qualified,summary:receipt.summary,receipt:path.join(run,"receipt.json"),error:receipt.error}));
}

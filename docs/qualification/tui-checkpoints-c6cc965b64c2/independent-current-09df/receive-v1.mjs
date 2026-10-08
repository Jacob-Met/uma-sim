import fs from "node:fs";
import path from "node:path";
import cp from "node:child_process";
import crypto from "node:crypto";
import http from "node:http";
import net from "node:net";
import assert from "node:assert/strict";
import {setTimeout as delay} from "node:timers/promises";
const root="/dev/shm/c6cc965b64c2-terminal-replacement-09df-v2";
const run=path.join(root,"run-v1");
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
const receipt={schema:"hamon.uma.terminal_durable.independent.v1",source_commit:manifest.source_commit,source_parent:manifest.source_parent,source_manifest_sha256:hashFile(path.join(root,"source-manifest.json")),binary:manifest.binary,node:process.version,platform:process.platform,arch:process.arch,source_before:sourcePins(),cases:[],clients:[],http_requests:[],native_processes:[],status:"running",qualified:false};
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
  try{const u=await fetch(apiBase+req.url,{method:req.method,headers:raw?{"Content-Type":"application/json"}:undefined,body:raw||undefined,signal:AbortSignal.timeout(8000)});const body=await u.text();row.status=u.status;row.response_sha256=hash(body);row.response_bytes=Buffer.byteLength(body);if(u.status>=400)row.error=JSON.parse(body).error;
   if(selected){selected.committed={row,body,status:u.status};if(selected.mode==="drop-after-native-response"){res.destroy();return;}if(selected.mode==="hold-after-native-response"){held.push(res);return;}}
   res.writeHead(u.status,{"Content-Type":"application/json"});res.end(body);
  }catch(e){row.proxy_error=e.message;if(!res.destroyed){res.writeHead(502,{"Content-Type":"application/json"});res.end(JSON.stringify({error:"receiver upstream failure: "+e.message}));}}
 });
 await new Promise((r,j)=>{proxy.once("error",j);proxy.listen(0,"127.0.0.1",r);});return "http://127.0.0.1:"+proxy.address().port;
}
function launch(args,{old=false}={}){
 const record={number:receipt.clients.length+1,phase,args,script:old?baseline:candidate,cwd:terminalCwd,stdout:"",stderr:"",exit:null};
 const child=cp.spawn(process.execPath,[record.script,...args],{cwd:terminalCwd,env:{...process.env,UMA_SIM_API:proxyBase},stdio:["pipe","pipe","pipe"]});record.pid=child.pid;
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
try{
 await startApi();proxyBase=await startProxy();
 await group("baseline-command-is-not-a-durable-save",async()=>{
  await startSession("baseline-receiving","123");await advance("baseline-receiving",4);
  const before=await state("baseline-receiving"),at=receipt.http_requests.length,c=launch(["--session=baseline-receiving"],{old:true});await c.ready();await c.command("save baseline-sentinel");await c.finish();
  const after=await state("baseline-receiving"),rows=tuiRows(at);assert.equal(rows.filter(r=>r.method==="POST"&&r.route==="/v1/library/save").length,0);assert.equal(rows.filter(r=>r.method==="POST"&&r.route==="/v1/run/action"&&r.body.action==="save baseline-sentinel").length,1);assert(!(await ok("GET","/v1/library")).json.entries.some(e=>e.name==="baseline-sentinel"));
  return{before:fileArtifact("baseline-before.json",before.raw),after:fileArtifact("baseline-after.json",after.raw),state_changed:lossless(before.raw)!==lossless(after.raw),baseline_post_went_to_action:true,no_checkpoint_created:true};
 });
 await startSession("","-9007199254740993");mainExpected=(await mainState()).raw;await startSession(primaryId,"-9223372036854775807");await advance(primaryId,7);await startSession(otherId,"9007199254740993");otherExpected=(await state(otherId)).raw;
 await group("64-byte-checkpoint-name-and-exact-i64-save",async()=>{
  assert.equal(savedName.length,64);assert.equal(primaryId.length,64);const before=await state(primaryId),at=receipt.http_requests.length,c=launch(["--session",primaryId]);await c.ready();await activate(otherId);const r=await c.command("save "+savedName);assert.match(r.stdout,/Saved checkpoint/);await c.finish();
  const row=tuiRows(at).filter(r=>r.method==="POST"&&r.route==="/v1/library/save");assert.equal(row.length,1);assert.deepEqual(row[0].body,{name:savedName,overwrite:false,session:primaryId});const files=cpBytes(savedName);sameState(files.snapshot.raw,before.raw,"checkpoint changed saved snapshot data");assert(files.snapshot.raw.includes("-9223372036854775807"),"exact i64 seed was not retained");
  savedSnapshot=files.snapshot.raw;savedPins=cpPins(savedName);primaryAfterSave=(await state(primaryId)).raw;sameState(primaryAfterSave,before.raw,"save advanced session");await invariant();return{name_length:savedName.length,session_length:primaryId.length,exact_seed:"-9223372036854775807",saved_snapshot:fileArtifact("checkpoint-original.snapshot.json",savedSnapshot),saved_meta:fileArtifact("checkpoint-original.meta.json",files.meta.raw),checkpoint_hashes:savedPins};
 });
 await group("duplicate-local-validation-and-read-only-listing",async()=>{
  const at=receipt.http_requests.length,c=launch(["--session="+primaryId]);await c.ready();await activate(otherId);const activeBefore=(await sessions()).active;assert.match((await c.command("save "+savedName)).stderr,/HTTP 409/);
  const badNames=["","..","../escape","界","X".repeat(65)];for(const n of badNames)assert.match((await c.command("save "+n)).stderr,/Use save NAME/);
  assert((await c.command("checkpoints")).stdout.includes(savedName));await c.finish();const l=launch(["--list-checkpoints"]);await l.exited();assert.equal(l.record.exit.code,0,l.record.stderr);assert(l.record.stdout.includes(savedName));const posts=tuiRows(at).filter(r=>r.method==="POST");assert.equal(posts.length,1);assert.equal(posts[0].route,"/v1/library/save");assert.equal(posts[0].status,409);assert.equal((await sessions()).active,activeBefore);sameState((await state(primaryId)).raw,primaryAfterSave,"save refusals/list advanced source");await invariant();
  return{invalid_names:badNames.length,native_conflict_status:409,only_post_is_explicit_duplicate_save:true,active_unchanged:true,checkpoint_bytes_unchanged:true};
 });
 await group("live-targeting-and-frozen-checkpoint-after-action",async()=>{
  const c=launch(["--session="+primaryId]);await c.ready();await activate(otherId);await c.command("auto");await c.finish();primaryAfterAuto=(await state(primaryId)).raw;assert.notEqual(lossless(primaryAfterAuto),lossless(primaryAfterSave),"auto did not exercise progress");assert((JSON.parse(primaryAfterAuto).rngCalls??0)>(JSON.parse(primaryAfterSave).rngCalls??0),"auto did not consume RNG");await invariant();
  return{before:fileArtifact("primary-before-auto.json",primaryAfterSave),after:fileArtifact("primary-after-auto.json",primaryAfterAuto),checkpoint_remains_old_snapshot:true};
 });
 await group("two-concurrent-checkpoint-forks-and-divergent-create-only-save",async()=>{
  const at=receipt.http_requests.length,a=launch(["--checkpoint="+savedName]),b=launch(["--checkpoint="+savedName]);await Promise.all([a.ready(),b.ready()]);forkAId=proposedId(a);forkBId=proposedId(b);assert.notEqual(forkAId,forkBId);sameState((await state(forkAId)).raw,savedSnapshot,"fork A initial data");sameState((await state(forkBId)).raw,savedSnapshot,"fork B initial data");
  await activate(otherId);await a.command("auto");await b.command("state");const aState=(await state(forkAId)).raw,bState=(await state(forkBId)).raw;assert.notEqual(lossless(aState),lossless(savedSnapshot));sameState(bState,savedSnapshot,"sibling mutated");await ok("POST","/v1/session/fork",{checkpoint:savedName,id:"independent-step-oracle"});await ok("POST","/v1/run/auto",{session:"independent-step-oracle",policy:"bot"});sameState(aState,(await state("independent-step-oracle")).raw,"one-step oracle disagrees");
  await activate(otherId);const raceName="race-save",outputs=await Promise.all([a.command("save "+raceName),b.command("save "+raceName)]);assert.equal(outputs.filter(r=>r.stdout.includes("Saved checkpoint")).length,1);assert.equal(outputs.filter(r=>r.stderr.includes("HTTP 409")).length,1);const winner=tuiRows(at).filter(r=>r.route==="/v1/library/save"&&r.body.name===raceName&&r.status===200);assert.equal(winner.length,1);sameState(cpBytes(raceName).snapshot.raw,winner[0].body.session===forkAId?aState:bState,"race mixed or overwrote checkpoint");
  await Promise.all([a.finish(),b.finish()]);const forks=tuiRows(at).filter(r=>r.method==="POST"&&r.route==="/v1/session/fork");assert.equal(forks.length,2);assert(forks.every(r=>r.body.checkpoint===savedName&&r.body.id&&!("session"in r.body)));sameState((await state(primaryId)).raw,primaryAfterAuto,"saved source career modified by fork");await invariant();
  return{forks:[forkAId,forkBId],exact_frozen_initial_data:true,one_real_bot_step_matches_independent_native_oracle:true,race_save_winner:winner[0].body.session,one_save_success_one_conflict:true};
 });
 await group("actual-api-restart-and-lost-live-session",async()=>{
  const before=cpPins(savedName);await stopApi();await startApi();assert.deepEqual(cpPins(savedName),before);assert.deepEqual((await sessions()).sessions,[]);const at=receipt.http_requests.length,l=launch(["--list-checkpoints"]);await l.exited();assert.equal(l.record.exit.code,0);assert(l.record.stdout.includes(savedName));const dead=launch(["--session="+primaryId]);await dead.exited();assert.equal(dead.record.exit.code,1);assert.match(dead.record.stderr,/HTTP 404/);assert(tuiRows(at).every(r=>r.method==="GET"));assert.deepEqual((await sessions()).sessions,[]);
  return{api_pids:receipt.native_processes.map(p=>p.pid),same_cwd:apiCwd,checkpoint_bytes_survive:true,live_sessions_do_not_survive:true,list_and_dead_resume_are_reads_only:true};
 });
 await startSession("","271828");mainExpected=(await mainState()).raw;await startSession(otherId,"17");otherExpected=(await state(otherId)).raw;
 await group("restart-fork-retains-full-state-and-two-step-rng-replay",async()=>{
  const c=launch(["--checkpoint",savedName]);await c.exited();assert.equal(c.record.exit.code,1);assert.match(c.record.stderr,/needs a saved name/);const d=launch(["--checkpoint="+savedName]);await d.ready();recoveredId=proposedId(d);sameState((await state(recoveredId)).raw,savedSnapshot,"restart load exact snapshot");await ok("POST","/v1/session/fork",{checkpoint:savedName,id:"restart-oracle"});
  const steps=[];for(let i=0;i<2;i++){await activate(otherId);const before=(await state(recoveredId)).raw;await d.command("auto");await ok("POST","/v1/run/auto",{session:"restart-oracle",policy:"bot"});const current=await state(recoveredId);sameState(current.raw,(await state("restart-oracle")).raw,"replayed step mismatch");assert.notEqual(lossless(current.raw),lossless(before),"replay step no-op");steps.push({turn:current.json.state.turn,rng_calls:current.json.rngCalls,snapshot:fileArtifact("restart-step-"+i+".json",current.raw)});}await d.finish();await invariant();
  return{recovered_id:recoveredId,equals_form_required_for_leading_dashes:true,steps,original_checkpoint_exact:true};
 });
 await group("default-new-career-preserves-main-and-named-careers",async()=>{
  const current=(await state(recoveredId)).raw,at=receipt.http_requests.length,c=launch([]);await c.ready();await c.finish();const starts=tuiRows(at).filter(r=>r.route==="/v1/run/start");assert.equal(starts.length,1);assert.match(starts[0].body.session,/^tui-[a-f0-9-]+$/);assert.notEqual(starts[0].body.session,recoveredId);sameState((await state(recoveredId)).raw,current,"new career reset existing named career");await invariant();return{new_id:starts[0].body.session,main_and_named_preserved:true};
 });
 await group("native-checkpoint-errors-never-create-fallback-careers",async()=>{
  const variants=[["missing-receiving",404],["bad-meta",500],["bad-snapshot",400],["future-schema",422]];cloneCheckpoint("bad-meta",{malformedMeta:true});cloneCheckpoint("bad-snapshot",{malformedSnapshot:true});cloneCheckpoint("future-schema",{schema:999});
  const beforeSessions=JSON.stringify((await sessions()).sessions),pins={};for(const [name]of variants.slice(1))pins[name]=cpPins(name);const results=[];for(const [name,status]of variants){const at=receipt.http_requests.length,c=launch(["--checkpoint="+name]);await c.exited();assert.equal(c.record.exit.code,1);assert(c.record.stderr.includes("HTTP "+status),c.record.stderr);const posts=tuiRows(at).filter(r=>r.method==="POST");assert.equal(posts.length,1);assert.equal(posts[0].route,"/v1/session/fork");assert.equal(posts[0].status,status);results.push({name,status});}
  assert.equal(JSON.stringify((await sessions()).sessions),beforeSessions);for(const [name,p]of Object.entries(pins))assert.deepEqual(cpPins(name),p);await invariant();return{native_statuses:results,no_replacement_start_or_new_session:true,malformed_inputs_retained:true};
 });
 await group("native-meta-write-failure-rolls-back-new-snapshot",async()=>{
  const name="meta-write-blocked",block=path.join(library,".tmp-"+apiProcess.item.pid+"-"+name+".meta.json");fs.mkdirSync(block);const before=(await state(recoveredId)).raw,c=launch(["--session="+recoveredId]);await c.ready();const r=await c.command("save "+name);assert.match(r.stderr,/HTTP 500/);await c.finish();assert(!fs.existsSync(cpPaths(name).snapshot));assert(!fs.existsSync(cpPaths(name).meta));assert(fs.statSync(block).isDirectory());sameState((await state(recoveredId)).raw,before,"failed save advanced selected session");await invariant();fs.rmdirSync(block);return{native_status:500,blocked_meta_temporary_path:block,first_written_snapshot_removed:true,no_partial_checkpoint:true,session_and_existing_checkpoint_unchanged:true};
 });
 await group("lost-save-response-retains-name-without-automatic-replay",async()=>{
  const name="lost-save",before=(await state(recoveredId)).raw,at=receipt.http_requests.length,c=launch(["--session="+recoveredId]);await c.ready();const f={mode:"drop-after-native-response",match:r=>r.route==="/v1/library/save"&&r.body.name===name};fault=f;const r=await c.command("save "+name);assert.equal(f.committed?.status,200);assert(r.stderr.includes("Checkpoint '"+name+"' may have been saved"));assert(r.stderr.includes("--session="+recoveredId));assert(r.stderr.includes("no action was retried automatically"));assert((await c.command("checkpoints")).stdout.includes(name));await c.finish();assert.equal(tuiRows(at).filter(r=>r.route==="/v1/library/save").length,1);sameState(cpBytes(name).snapshot.raw,before,"lost response checkpoint wrong");sameState((await state(recoveredId)).raw,before,"save advanced selected career");await invariant();return{native_commit_status:200,post_commit_transport_fault:true,save_requests:1,checkpoint_available_on_explicit_list:true,recovery_names_checkpoint_and_session:true};
 });
 await group("eof-during-held-save-retains-committed-checkpoint",async()=>{
  const name="eof-save",at=receipt.http_requests.length,c=launch(["--session="+recoveredId]);await c.ready();const before=(await state(recoveredId)).raw,f={mode:"hold-after-native-response",match:r=>r.route==="/v1/library/save"&&r.body.name===name};fault=f;c.child.stdin.write("save "+name+"\n");await deadline(()=>!!f.committed,"save commit before EOF");assert.equal(f.committed.status,200);c.child.stdin.end();await c.exited();assert.equal(c.record.exit.code,0);assert(c.record.stderr.includes("Checkpoint '"+name+"' may have been saved"));assert(c.record.stderr.includes("no action was retried automatically"));assert.equal(tuiRows(at).filter(r=>r.route==="/v1/library/save").length,1);sameState(cpBytes(name).snapshot.raw,before,"EOF committed snapshot wrong");for(const h of held)h.destroy();held=[];await invariant();return{exit_code:0,native_commit_status:200,save_requests:1,no_implicit_retry:true,checkpoint_retained:true};
 });
 await group("interrupt-after-native-fork-keeps-recoverable-proposed-id",async()=>{
  const at=receipt.http_requests.length,prior=new Set((await sessions()).sessions.map(s=>s.id)),f={mode:"hold-after-native-response",match:r=>r.route==="/v1/session/fork"};fault=f;const c=launch(["--checkpoint="+savedName]);await deadline(()=>!!f.committed,"fork commit before interrupt");const id=proposedId(c);assert.equal(f.committed.status,200);c.child.kill("SIGINT");await c.exited();assert.equal(c.record.exit.code,130);assert(c.record.stderr.includes("--session="+id));assert(c.record.stderr.includes("no action was retried automatically"));const created=(await sessions()).sessions.filter(s=>!prior.has(s.id));assert.deepEqual(created.map(s=>s.id),[id]);assert.equal(tuiRows(at).filter(r=>r.route==="/v1/session/fork").length,1);sameState((await state(id)).raw,savedSnapshot,"held fork snapshot wrong");for(const h of held)h.destroy();held=[];const resume=launch(["--session="+id]);await resume.ready();await resume.finish();sameState((await state(id)).raw,savedSnapshot,"explicit recovered session changed");await invariant();return{exit_code:130,recoverable_id:id,fork_requests:1,created_sessions:1,explicit_live_resume_succeeded:true};
 });
 await group("lost-action-response-applies-once-to-bound-session",async()=>{
  const source=(await state(recoveredId)).raw;await ok("POST","/v1/session/fork",{session:recoveredId,id:"loss-action-oracle"});await ok("POST","/v1/run/auto",{session:"loss-action-oracle",policy:"bot"});const expected=(await state("loss-action-oracle")).raw;assert.notEqual(lossless(source),lossless(expected));const at=receipt.http_requests.length,c=launch(["--session="+recoveredId]);await c.ready();await activate(otherId);const f={mode:"drop-after-native-response",match:r=>r.route==="/v1/run/auto"&&r.body.session===recoveredId};fault=f;const r=await c.command("auto");assert.equal(f.committed.status,200);assert(r.stderr.includes("no action was retried automatically"));await c.finish();assert.equal(tuiRows(at).filter(r=>r.route==="/v1/run/auto").length,1);sameState((await state(recoveredId)).raw,expected,"lost action applied zero or multiple times");await invariant();return{native_commit_status:200,auto_requests:1,exact_independent_one_step_result:true,active_bystander_unchanged:true};
 });
 await group("invalid-checkpoint-mode-combinations-make-no-http-request",async()=>{
  const inputs=[["--checkpoint="],["--checkpoint","--new"],["--checkpoint="+savedName,"--session="+primaryId],["--checkpoint=x","43"],["--checkpoint=x","--list-checkpoints"],["--checkpoint="+"Z".repeat(65)],["--checkpoint=../escape"],["--checkpoint=界"],["--list-checkpoints","--list-sessions"]];const at=receipt.http_requests.length;for(const args of inputs){const c=launch(args);await c.exited();assert.equal(c.record.exit.code,1);assert(c.record.stderr.trim().length>0);}assert.equal(tuiRows(at).length,0);await invariant();return{parser_cases:inputs.length,http_requests:0};
 });
 await group("all-terminal-routes-remain-explicit-and-never-reset",async()=>{
  const rows=receipt.http_requests.filter(r=>r.channel==="tui");let addressed=0;for(const r of rows){const u=new URL(r.route,"http://receiver.invalid");if(r.method==="GET"&&u.pathname.startsWith("/v1/run/")){const id=u.searchParams.get("session");assert(id&&/^[A-Za-z0-9_.-]{1,64}$/.test(id));addressed++;}if(r.method==="POST"&&(u.pathname.startsWith("/v1/run/")||u.pathname==="/v1/library/save")){assert(r.body.session&&/^[A-Za-z0-9_.-]{1,64}$/.test(r.body.session));addressed++;}if(u.pathname==="/v1/library/save")assert.equal(r.body.overwrite,false);assert(!["/v1/library/load","/v1/library/delete","/v1/session/activate","/v1/session/close"].includes(u.pathname),"destructive/implicit terminal route "+u.pathname);}
  return{terminal_requests:rows.length,explicitly_addressed_run_and_save_requests:addressed,every_save_is_create_only:true,no_load_activate_close_delete_route:true};
 });
 receipt.source_after=sourcePins();receipt.status="passed";receipt.qualified=true;
}catch(error){receipt.status="failure";receipt.error=error.message;receipt.stack=error.stack;process.exitCode=1;
}finally{
 fault=undefined;for(const h of held)h.destroy();for(const child of liveClients)child.kill("SIGTERM");if(liveClients.size)await delay(50);await stopApi();if(proxy){proxy.closeAllConnections();await new Promise(r=>proxy.close(r));}
 receipt.finished_utc=new Date().toISOString();receipt.summary={groups:receipt.cases.length,passed:receipt.cases.filter(c=>c.status==="pass").length,failures:receipt.cases.filter(c=>c.status==="failure").length,terminal_processes:receipt.clients.length,native_api_processes:receipt.native_processes.length,http_requests:receipt.http_requests.length};save();console.log(JSON.stringify({status:receipt.status,qualified:receipt.qualified,summary:receipt.summary,receipt:path.join(run,"receipt.json"),error:receipt.error}));
}

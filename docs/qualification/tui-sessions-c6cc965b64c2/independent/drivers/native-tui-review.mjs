import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import net from 'node:net';
import crypto from 'node:crypto';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {setTimeout as delay} from 'node:timers/promises';
import {fileURLToPath} from 'node:url';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const mode=process.argv[2];
assert.ok(['baseline','candidate'].includes(mode));
const entry=path.resolve(process.argv[3]??root+'/source/packages/uma-sim-cli/tui.js');
const out=path.resolve(process.argv[4]??root+'/results-'+mode+'-v1');
const binary=root+'/server/uma-sim-api';
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
assert.equal(sha(fs.readFileSync(binary)),'fdda7861a592987391e57b59d8dc0230b870b73268e9e3b02af0f330bffcab5e');
assert.equal(sha(fs.readFileSync(root+'/source/uma-sim-core/src/api.rs')),'3db7edc89536c1670c26c3a900de8654fde1b27d66c552d017c94dcc31e44f95');
fs.mkdirSync(out);
const receipt={schema:'hamon.uma_tui.independent_native_receiving.v1',reviewer:'estate-c6cc965b64c2/native_engine',mode,entry,source_sha256:sha(fs.readFileSync(entry)),server_binary:binary,server_sha256:sha(fs.readFileSync(binary)),server_source_commit:'f5f9b29393731d18aee2d66a31d89c315aa79c60',node:process.version,platform:process.platform,arch:process.arch,started_utc:new Date().toISOString(),results:[]};
if(mode==='candidate'){
  receipt.candidate_commit='379eee9707a224d14adb6c362d729b38f43dcf43';
  assert.equal(receipt.source_sha256,'39d36f1ebba8c06823e57f2a5c9d49ab6a2b00d8ff0d67ff7afb6f5c5f1d17bb');
}
const children=new Set();
async function until(check,description,ms=6000){
  const deadline=Date.now()+ms;
  while(!check()){
    if(Date.now()>deadline)throw Error('Timed out: '+description());
    await delay(8);
  }
}
async function freePort(){
  const s=net.createServer(); await new Promise((resolve,reject)=>{s.once('error',reject);s.listen(0,'127.0.0.1',resolve)});
  const port=s.address().port; await new Promise(resolve=>s.close(resolve));return port;
}
function tracked(exe,args,options){
  const child=spawn(exe,args,options);children.add(child);
  const record={pid:child.pid,exe,args,stdout:'',stderr:'',exit_code:null,signal:null,closed:false};
  child.stdout?.on('data',b=>{record.stdout+=b});child.stderr?.on('data',b=>{record.stderr+=b});
  child.on('error',e=>{record.spawn_error=e.message});
  child.on('close',(code,signal)=>{Object.assign(record,{exit_code:code,signal,closed:true});children.delete(child)});
  return{child,record};
}
async function stop(p){
  if(p.record.closed)return;
  p.child.kill('SIGTERM');
  try{await until(()=>p.record.closed,()=>`PID ${p.record.pid} shutdown`,1000)}catch{p.child.kill('SIGKILL');await until(()=>p.record.closed,()=>`PID ${p.record.pid} killed`,1000)}
}
function client(context,args){
  const p=tracked(process.execPath,[entry,...args],{cwd:context.cwd,env:{...process.env,UMA_SIM_API:context.proxyUrl},stdio:['pipe','pipe','pipe']});
  context.clients.push(p);
  return{...p,
    async prompt(n=1){await until(()=>p.record.closed||(p.record.stdout.match(/\n> /g)??[]).length>=n,()=>JSON.stringify(p.record));assert.equal(p.record.closed,false,JSON.stringify(p.record))},
    send(s){p.child.stdin.write(s+'\n')},
    async finish(){await until(()=>p.record.closed,()=>JSON.stringify(p.record));return p.record},
    async quit(){p.child.stdin.write('quit\n');return this.finish()},
  };
}
async function fixture(name){
  const dir=path.join(out,name);fs.mkdirSync(dir);const cwd=path.join(dir,'server-cwd');fs.mkdirSync(cwd);
  const port=await freePort(),url='http://127.0.0.1:'+port;
  const env={...process.env,UMA_REPO_ROOT:root+'/source'};delete env.UMA_POLICY_CMD;
  const native=tracked(binary,[String(port)],{cwd,env,stdio:['ignore','pipe','pipe']});
  const context={dir,cwd,url,native,clients:[],trace:[],controlTrace:[],artifacts:[],fault:undefined};
  async function direct(method,route,body){
    const response=await fetch(url+route,{method,body:body===undefined?undefined:JSON.stringify(body),headers:body===undefined?undefined:{'Content-Type':'application/json'},signal:AbortSignal.timeout(8000)});
    const raw=await response.text();
    context.controlTrace.push({method,route,body,status:response.status,response_sha256:sha(raw)});
    assert.equal(response.status,200,method+' '+route+': '+raw);
    return{raw,value:JSON.parse(raw)};
  }
  context.direct=direct;
  let ready=false;
  // The readiness loop polls actual health. It does not accept a
  // spawned process as proof that the API initialized its catalogs successfully.
  for(let attempt=0;attempt<100;attempt++){
    assert.equal(native.record.closed,false,JSON.stringify(native.record));
    try{const health=await fetch(url+'/v1/health',{signal:AbortSignal.timeout(250)});const value=await health.json();if(value.ok===true){assert.equal(value.repoRootPath,root+'/source');ready=true;break}}catch{}
    await delay(15);
  }
  assert.equal(ready,true,'native health never became ready');
  const proxy=http.createServer(async(req,res)=>{
    let raw='';for await(const chunk of req)raw+=chunk;
    const parsed=new URL(req.url,'http://localhost');
    const record={method:req.method,url:req.url,route:parsed.pathname,query:Object.fromEntries(parsed.searchParams),body:raw?JSON.parse(raw):undefined};context.trace.push(record);
    try{
      if(await context.fault?.('before',record,{req,res}))return;
      const response=await fetch(url+req.url,{method:req.method,body:['GET','HEAD'].includes(req.method)?undefined:raw,headers:raw?{'Content-Type':'application/json'}:undefined,signal:AbortSignal.timeout(10000)});
      const responseRaw=await response.text();record.status=response.status;record.response_sha256=sha(responseRaw);
      if(await context.fault?.('after',record,{req,res,responseRaw,status:response.status}))return;
      res.writeHead(response.status,{'Content-Type':'application/json',Connection:'close'});res.end(responseRaw);
    }catch(error){record.proxy_error=error.message;if(!res.destroyed){res.writeHead(502,{'Content-Type':'application/json'});res.end(JSON.stringify({error:'independent forwarding proxy: '+error.message}))}}
  });
  await new Promise(resolve=>proxy.listen(0,'127.0.0.1',resolve));
  context.proxyUrl='http://127.0.0.1:'+proxy.address().port;
  context.save=(name,raw)=>{const file=path.join(dir,name);fs.writeFileSync(file,raw,{flag:'wx'});context.artifacts.push({path:path.relative(out,file),bytes:Buffer.byteLength(raw),sha256:sha(raw)});return raw};
  context.read=async id=>direct('GET','/v1/run/state?session='+encodeURIComponent(id));
  context.main=async()=>{
    const active=(await direct('GET','/v1/sessions')).value.active;
    await direct('POST','/v1/session/activate',{session:''});
    const snapshot=await direct('GET','/v1/run/state');
    if(active!=='')await direct('POST','/v1/session/activate',{session:active});
    return snapshot;
  };
  context.close=async()=>{
    for(const p of context.clients)await stop(p);
    proxy.closeAllConnections();await new Promise(resolve=>proxy.close(resolve));await stop(native);
    fs.writeFileSync(path.join(dir,'native-process.json'),JSON.stringify(native.record,null,2)+'\n');
    fs.writeFileSync(path.join(dir,'clients.json'),JSON.stringify(context.clients.map(p=>p.record),null,2)+'\n');
    fs.writeFileSync(path.join(dir,'terminal-http.json'),JSON.stringify(context.trace,null,2)+'\n');
    fs.writeFileSync(path.join(dir,'control-http.json'),JSON.stringify(context.controlTrace,null,2)+'\n');
  };
  await direct('POST','/v1/run/start',{seed:'7301',scenario:'ura',speed:'3',traceTelemetry:'true'});
  for(let n=0;n<5;n++)await direct('POST','/v1/run/auto',{policy:'bot'});
  context.mainBefore=context.save('main-before.json',(await context.main()).raw);
  await direct('POST','/v1/run/start',{session:'saved-review',seed:'7302',scenario:'ura',traceTelemetry:'true'});
  for(let n=0;n<3;n++)await direct('POST','/v1/run/auto',{session:'saved-review',policy:'bot'});
  await direct('POST','/v1/run/start',{session:'other-review',seed:'7303',scenario:'ura',speed:'7'});
  context.otherBefore=context.save('other-before.json',(await context.read('other-review')).raw);
  context.savedBefore=context.save('saved-before.json',(await context.read('saved-review')).raw);
  return context;
}
const writes=f=>f.trace.filter(r=>r.method==='POST');
function assertSelected(f,id){
  const run=f.trace.filter(r=>r.route.startsWith('/v1/run/'));assert.ok(run.length>0);
  for(const r of run)assert.equal(r.method==='GET'?r.query.session:r.body?.session,id,JSON.stringify(r));
  assert.ok(id.length>0);
  assert.equal(f.trace.some(r=>r.route.startsWith('/v1/session/')),false);
}
async function unchangedMain(f){
  const raw=f.save('main-after.json',(await f.main()).raw);
  assert.equal(raw,f.mainBefore,'actual native main snapshot changed');
  return{sha256:sha(raw),full_snapshot_bytes:Buffer.byteLength(raw),unchanged:true};
}
async function test(name,body){
  let f;const started=performance.now();const result={name,pass:false};
  try{f=await fixture(name);result.evidence=await body(f);result.pass=true}catch(error){result.error=error.stack??String(error)}
  finally{if(f){result.artifacts=f.artifacts;await f.close()}else{for(const child of children)child.kill('SIGKILL');await until(()=>children.size===0,()=>`fixture cleanup left ${children.size} children`,1000)}result.elapsed_ms=performance.now()-started;receipt.results.push(result);receipt.passed=receipt.results.filter(r=>r.pass).length;receipt.failed=receipt.results.filter(r=>!r.pass).length;fs.writeFileSync(out+'/receipt.json',JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify({name,pass:result.pass,error:result.error,elapsed_ms:result.elapsed_ms}))}
}

await test('fresh-session-preserves-real-main',async f=>{
  const ids=[];
  for(const args of mode==='baseline'?[[]]:[[],['--new'],['+123','unity']]){
    const offset=f.trace.length,c=client(f,args);await c.prompt();
    const requests=f.trace.slice(offset),start=requests.find(r=>r.route==='/v1/run/start');assert.ok(start);
    const id=start.body?.session;
    const after=f.save('main-after-launch-'+ids.length+'.json',(await f.main()).raw);
    assert.equal(after,f.mainBefore,'actual native main snapshot changed during TUI launch');
    assert.match(id,/^tui-[a-f0-9-]{36}$/);assert.ok(!ids.includes(id));ids.push(id);
    assert.equal((await c.quit()).exit_code,0);
    for(const r of requests.filter(r=>r.route.startsWith('/v1/run/')))assert.equal(r.method==='GET'?r.query.session:r.body?.session,id);
    assert.equal((await f.read('saved-review')).raw,f.savedBefore);
    assert.equal((await f.read('other-review')).raw,f.otherBefore);
  }
  return{main:await unchangedMain(f),created_ids:ids};
});

if(mode==='candidate'){
  await test('resume-survives-active-churn-for-every-operation',async f=>{
    let switched=0;
    f.fault=async(phase,r)=>{if(phase==='before'&&r.route.startsWith('/v1/run/')){await f.direct('POST','/v1/session/activate',{session:switched++%2?'':'other-review'})}return false};
    const c=client(f,['--session=saved-review']);await c.prompt();
    assert.equal(writes(f).length,0);assert.equal((await f.read('saved-review')).raw,f.savedBefore);
    const choices=(await f.direct('GET','/v1/run/choices?session=saved-review')).value.choices;assert.ok(choices.length);
    for(const [index,command]of[choices[0].id,'auto','fast','state'].entries()){c.send(command);await c.prompt(index+2)}
    const after=f.save('selected-after.json',(await f.read('saved-review')).raw);
    assert.notEqual(after,f.savedBefore);assert.equal((await f.read('other-review')).raw,f.otherBefore);
    assert.deepEqual(writes(f).map(r=>r.route),['/v1/run/action','/v1/run/auto','/v1/run/fast']);assertSelected(f,'saved-review');
    assert.equal((await c.quit()).exit_code,0);
    return{main:await unchangedMain(f),active_switches:switched,selected_before_sha256:sha(f.savedBefore),selected_after_sha256:sha(after),complete:JSON.parse(after).state.careerComplete};
  });

  await test('parser-readonly-discovery-and-native-rejections',async f=>{
    const initial=(await f.direct('GET','/v1/sessions')).raw;
    const invalid=[['--session='],['--session',''],['--session= saved-review'],['--session=..'],['--session='+'a'.repeat(65)],['--session=saved-review','--new'],['--session=saved-review','42'],['--session=saved-review','42','ura'],['--session=saved-review','--list-sessions'],['--wat'],['9223372036854775808'],['-9223372036854775809'],['1e3'],['3.5']];
    for(const args of invalid){const offset=f.trace.length,c=client(f,args);const r=await c.finish();assert.equal(r.exit_code,1,JSON.stringify({args,r}));assert.equal(f.trace.length,offset,'parser rejection contacted HTTP');assert.ok(r.stderr.length)}
    const help=client(f,['--help']);assert.equal((await help.finish()).exit_code,0);assert.equal(f.trace.length,0);
    const list=client(f,['--list-sessions']);const listed=await list.finish();assert.equal(listed.exit_code,0);assert.match(listed.stdout,/\(main\)/);assert.match(listed.stdout,/saved-review/);assert.equal((await f.direct('GET','/v1/sessions')).raw,initial);
    const missing=client(f,['--session=unknown-native-career']);const missingResult=await missing.finish();assert.equal(missingResult.exit_code,1);assert.match(missingResult.stderr,/HTTP 404.*no such session/);
    const badScenario=client(f,['43','not-a-scenario']);const badResult=await badScenario.finish();assert.equal(badResult.exit_code,1);assert.match(badResult.stderr,/HTTP 400.*unknown scenario/);assert.equal(writes(f).length,1);assert.match(writes(f)[0].body.session,/^tui-/);
    assert.equal((await f.direct('GET','/v1/sessions')).raw,initial);
    return{main:await unchangedMain(f),invalid_argument_cases:invalid.length,listing_readonly:true,missing_resume_status:404,unknown_scenario_status:400};
  });

  await test('signed-i64-seeds-reach-native-engine-exactly',async f=>{
    const seeds=['9223372036854775807','-9223372036854775808','+0'];const outcomes=[];
    for(const seed of seeds){const offset=f.trace.length,c=client(f,[seed,'ura']);await c.prompt();const start=f.trace.slice(offset).find(r=>r.route==='/v1/run/start');assert.equal(start.body.seed,seed);const raw=(await f.read(start.body.session)).raw;const exactSeed=raw.match(/"seed"\s*:\s*(-?\d+)/)?.[1];assert.equal(BigInt(exactSeed),BigInt(seed));outcomes.push({input:seed,request_seed:start.body.seed,actual_seed_lexeme:exactSeed,snapshot_sha256:sha(raw)});assert.equal((await c.quit()).exit_code,0)}
    return{main:await unchangedMain(f),outcomes};
  });

  for(const fault of['disconnect','invalid-json'])await test('committed-action-'+fault+'-is-never-replayed',async f=>{
    await f.direct('POST','/v1/session/fork',{session:'saved-review',id:'one-step-reference'});
    const choice=(await f.direct('GET','/v1/run/choices?session=saved-review')).value.choices[0].id;
    await f.direct('POST','/v1/run/action',{session:'one-step-reference',action:choice});
    const expected=(await f.read('one-step-reference')).raw;
    f.fault=async(phase,r,{res})=>{if(phase==='after'&&r.route==='/v1/run/action'){if(fault==='disconnect')res.destroy();else{res.writeHead(200,{Connection:'close'});res.end('committed, response corrupted')}return true}return false};
    const c=client(f,['--session=saved-review']);await c.prompt();c.send(choice);await c.prompt(2);
    assert.match(c.record.stderr,/may have reached the server/);assert.match(c.record.stderr,/--session=saved-review/);
    assert.equal(writes(f).length,1);assertSelected(f,'saved-review');
    const actual=f.save('selected-after.json',(await f.read('saved-review')).raw);f.save('exact-one-step-reference.json',expected);assert.equal(actual,expected,'selected career differs from exactly one real native action');assert.notEqual(actual,f.savedBefore);
    assert.equal((await c.quit()).exit_code,0);
    return{main:await unchangedMain(f),action:choice,posts:1,exact_one_step_snapshot_sha256:sha(actual)};
  });

  await test('native-external-policy-error-preserves-session-and-recovers',async f=>{
    await f.direct('POST','/v1/run/start',{session:'external-setting',seed:'88',scenario:'ura',policy:'external'});
    const c=client(f,['--session=saved-review']);await c.prompt();c.send('fast');await c.prompt(2);
    assert.match(c.record.stderr,/HTTP 503.*external policy unavailable/);assert.equal((await f.read('saved-review')).raw,f.savedBefore);
    c.send('auto');await c.prompt(3);const after=f.save('selected-after.json',(await f.read('saved-review')).raw);assert.notEqual(after,f.savedBefore);assert.equal(writes(f).length,2);assert.equal(writes(f)[0].status,503);assert.equal(writes(f)[1].status,200);assertSelected(f,'saved-review');assert.equal((await c.quit()).exit_code,0);
    return{main:await unchangedMain(f),failed_fast_unchanged:true,explicit_bot_recovery:true};
  });

  await test('incomplete-read-is-visible-without-partial-state-or-mutation',async f=>{
    let failed=false;
    f.fault=async(phase,r,{res})=>{if(phase==='before'&&r.route==='/v1/run/text'&&!failed){failed=true;res.writeHead(503,{'Content-Type':'application/json',Connection:'close'});res.end(JSON.stringify({error:'receiving read fault'}));return true}return false};
    const c=client(f,['--session=saved-review']);await c.prompt();assert.match(c.record.stderr,/receiving read fault/);assert.doesNotMatch(c.record.stdout,/┌─ State/);c.send('state');await c.prompt(2);assert.match(c.record.stdout,/┌─ State/);assert.equal(writes(f).length,0);assertSelected(f,'saved-review');assert.equal((await f.read('saved-review')).raw,f.savedBefore);assert.equal((await c.quit()).exit_code,0);
    return{main:await unchangedMain(f),read_recovered:true,mutations:0};
  });

  await test('eof-during-initial-read-prevents-new-career',async f=>{
    let held=false;f.fault=async(phase,r)=>{if(phase==='after'&&r.route==='/v1/health'){held=true;return true}return false};
    const c=client(f,[]);await until(()=>held,()=>JSON.stringify(c.record));c.child.stdin.end();const r=await c.finish();assert.equal(r.exit_code,0);assert.equal(writes(f).length,0);
    return{main:await unchangedMain(f),exit_code:r.exit_code,posts:0};
  });

  for(const interrupt of['EOF','SIGINT'])await test(interrupt.toLowerCase()+'-during-committed-start-retains-resume-id',async f=>{
    let held;
    f.fault=async(phase,r)=>{if(phase==='after'&&r.route==='/v1/run/start'){held=r;return true}return false};
    const c=client(f,['--new']);await until(()=>held,()=>JSON.stringify(c.record));const actual=(await f.read(held.body.session)).raw;
    if(interrupt==='EOF')c.child.stdin.end();else c.child.kill('SIGINT');const result=await c.finish();assert.equal(result.exit_code,interrupt==='EOF'?0:130);assert.match(result.stderr,/may have reached the server/);assert.ok(result.stderr.includes('--session='+held.body.session));assert.equal(writes(f).length,1);assertSelected(f,held.body.session);assert.equal((await f.read(held.body.session)).raw,actual);
    return{main:await unchangedMain(f),exit_code:result.exit_code,retained_session:held.body.session,retained_snapshot_sha256:sha(actual),posts:1};
  });
}

receipt.finished_utc=new Date().toISOString();fs.writeFileSync(out+'/receipt.json',JSON.stringify(receipt,null,2)+'\n');
if(children.size){for(const child of children)child.kill('SIGKILL');throw Error('receiver left child processes running')}
console.log(JSON.stringify({mode,passed:receipt.passed,failed:receipt.failed,receipt:out+'/receipt.json'}));
if(receipt.failed)process.exitCode=1;

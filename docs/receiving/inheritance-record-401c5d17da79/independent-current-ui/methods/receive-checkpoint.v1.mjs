// SPDX-License-Identifier: MIT
// Independent checkpoint/decision receiving. Exact native API and exact compiled UI; no substitutes.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import http from 'node:http';
import net from 'node:net';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import assert from 'node:assert/strict';
import {CDP} from '/home/jacob/.commander-tp-1/hamon-work/estate-401c5d17da79/towerops-flight-register/methods/cdp.mjs';
const root='/tmp/uma-inheritance-checkpoint-receiving-401c5d17da79';
const owner='/home/jacob/.commander-tp-1/hamon-work/estate-401c5d17da79/uma-inheritance-205178f1';
const current='/tmp/uma-inheritance-current-8d74413-401c5d17da79';
const out=path.join(root,'run-v1'),privateCwd=path.join(out,'private-careers'),profile=path.join(out,'chromium-profile');
assert.equal(process.getuid(),1000);
assert.ok(fs.statfsSync(root).bavail*fs.statfsSync(root).bsize>256*1024*1024,'private receiver requires 256 MiB free');
fs.mkdirSync(out);fs.mkdirSync(privateCwd);fs.mkdirSync(profile);
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const write=(name,v)=>fs.writeFileSync(path.join(out,name),typeof v==='string'?v:JSON.stringify(v,null,2)+'\n');
const pause=ms=>new Promise(r=>setTimeout(r,ms));
const wait=async(fn,ms,label)=>{const end=Date.now()+ms;let error;while(Date.now()<end){try{const v=await fn();if(v)return v;}catch(e){error=e;}await pause(80);}throw Error(label+' timed out '+String(error||''));};
const report={schema:'uma.inheritance-checkpoint-independent.v1',ui_parent:'8d74413d7dbc3caf1745779c92d56b6eb7feea42',api_parent:'205178f1bf3bf5f34c04bbda88c3e4df2102f535',started_at:new Date().toISOString(),checks:[],failure:null};
const check=(name,actual,expected=true)=>{let pass=true;try{assert.deepEqual(actual,expected);}catch{pass=false;}report.checks.push({name,pass,...pass?{}:{actual,expected}});write('progress.json',report.checks);assert.ok(pass,name);};
const input=JSON.parse(fs.readFileSync(path.join(current,'evidence/receipt.json')));
const artifactPath=path.join(current,'evidence/ui-build-artifacts.json'),artifactRaw=fs.readFileSync(artifactPath);
assert.equal(sha(artifactRaw),'b05817c01ec100fdfc5527cb7cb4bc92ffb653a4b0bf1857972e60905675a473');
const compiled=JSON.parse(artifactRaw),assets=new Map(compiled.files.map(f=>{const b=Buffer.from(f.base64,'base64');assert.equal(sha(b),f.sha256);assert.equal(b.length,f.bytes);return [f.key,b];}));
const binary=path.join(owner,'native-api-target/debug/uma-sim-api');
const borrowed={[binary]:'3b210a6d7f5885b54b281f3d7dca658a71d695a836346efd7daf1cd52b7d2c23',[artifactPath]:sha(artifactRaw),[path.join(current,'evidence/receipt.json')]:sha(fs.readFileSync(path.join(current,'evidence/receipt.json'))),'/home/jacob/.commander-tp-1/hamon-work/estate-401c5d17da79/towerops-flight-register/methods/cdp.mjs':'be20b757f9cd7b3b85bcc701b589d61ad338123be70d75acb914eda529af5639'};
for(const [p,pin] of Object.entries(input.source))borrowed[path.join(current,'source',p)]=pin.sha256;
for(const p of ['uma-sim-core/src/state.rs','uma-sim-core/src/legacy.rs','uma-sim-core/src/api.rs','uma-sim-core/src/engine.rs','uma-sim-core/src/snapshot.rs'])borrowed[path.join(owner,'native-api-source',p)]=sha(fs.readFileSync(path.join(owner,'native-api-source',p)));
const changed=()=>Object.entries(borrowed).filter(([p,h])=>sha(fs.readFileSync(p))!==h).map(([p])=>p);
assert.deepEqual(changed(),[]);
report.inputs={source:input,assets:compiled.files.map(({base64,...f})=>f),borrowed,method_sha256:sha(fs.readFileSync(new URL(import.meta.url)))};
write('INPUTS.json',report.inputs);
const ephemeral=()=>new Promise((resolve,reject)=>{const s=net.createServer();s.on('error',reject);s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>resolve(p));});});
function disk(){const result={};function visit(p){if(!fs.existsSync(p))return;for(const e of fs.readdirSync(p,{withFileTypes:true})){const f=path.join(p,e.name);if(e.isDirectory())visit(f);else if(e.isFile())result[path.relative(privateCwd,f)]=sha(fs.readFileSync(f));else throw Error('unexpected private saved-file type');}}visit(privateCwd);return result;}
let apiChild,browser,server,c,apiBase;const browserTraffic=[],directTraffic=[],errors=[],external=[];
async function api(route,body){const res=await fetch(apiBase+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});const raw=await res.text();directTraffic.push({route,method:body===undefined?'GET':'POST',body,status:res.status,response_sha256:sha(raw)});assert.ok(res.ok,route+' '+res.status+' '+raw);return JSON.parse(raw);}
try{
 const apiPort=await ephemeral();apiBase='http://127.0.0.1:'+apiPort;
 const log=fs.openSync(path.join(out,'native-api.log'),'wx');apiChild=spawn(binary,[String(apiPort)],{cwd:privateCwd,env:{...process.env,UMA_REPO_ROOT:path.join(owner,'native-api-source')},stdio:['ignore',log,log]});fs.closeSync(log);
 const health=await wait(()=>api('/v1/health'),15000,'private native API');
 check('The actual native API uses the pinned205 catalog source',health.repoRootPath,path.join(owner,'native-api-source'));
 const factors=(await api('/v1/catalog/factors')).items,blue=factors.find(f=>f.kind.toLowerCase()==='blue');assert.ok(blue);
 const initial=await api('/v1/run/start',{seed:'112',scenario:'ura',trainee:'Special Week',raceModel:'stub',dialogue:'choices',legacyFactors:blue.id+'@2,'+blue.id+'@1',parentNames:'Native parent A,Native parent B',session:'origin',label:'Checkpoint origin'});
 const progression=[];let pending=initial;
 for(let n=0;n<40&&!(pending.state.phase==='EVENT'&&pending.state.pendingEventOptions.length);n++){
  const choices=(await api('/v1/run/choices?session=origin')).choices;assert.ok(choices.length,'bounded origin must have a native action');
  const action=choices.some(x=>x.id==='rest')?'rest':choices[0].id;
  progression.push({turn:pending.state.turn,phase:pending.state.phase,action});
  await api('/v1/run/action',{session:'origin',action});pending=await api('/v1/run/state?session=origin');
 }
 check('Fixture reaches a genuine native pending event through bounded normal actions',pending.state.awaitingChoice&&pending.state.phase==='EVENT'&&pending.state.pendingEventOptions.length>0);
 write('origin-initial.json',initial);write('origin-actions.json',progression);write('origin-pending.json',pending);
 const partial=structuredClone(pending);
 for(const meta of [partial.meta,partial.state.meta]){
  meta.legacyTree={parentA:{uma:'Restored <Parent> & record',blue:{factorId:blue.id,stars:0}},gpA1:{uma:'Name-only grandparent'},parentB:{blue:{factorId:blue.id,stars:4},white:{factorId:'unknown:<checkpoint>&',stars:-1}}};
  delete meta.compatibilityScore;
 }
 delete partial.state.legacy.blueStartBonuses;delete partial.state.legacy.inspirationEventsDone;
 write('imported-partial.json',partial);
 await api('/v1/library/import',{snapshot:partial,name:'pending-partial'});
 await api('/v1/library/load',{name:'pending-partial',session:'pending-ui',label:'Pending checkpoint'});
 await api('/v1/library/load',{name:'pending-partial',session:'pending-control',label:'Pending control'});
 const restored=await api('/v1/run/state?session=pending-ui'),control=await api('/v1/run/state?session=pending-control');
 check('The imported checkpoint restores identical UI and native control snapshots',restored,control);
 const decision=s=>({turn:s.state.turn,phase:s.state.phase,awaitingChoice:s.state.awaitingChoice,title:s.state.pendingEventTitle,options:s.state.pendingEventOptions,rngSeed:s.rngSeed,rngCalls:s.rngCalls,rngState:s.rngState});
 check('Partial inheritance import preserves the exact pending decision and RNG',decision(restored),decision(pending));
 const positions=['parentA','gpA1','gpA2','parentB','gpB1','gpB2'];
 check('Native normalization is recorded before any missing-field display claim',Object.keys(restored.state.meta.legacyTree).sort(),[...positions].sort());
 check('Native omitted-bonus/count defaults stay explicit',{compatibility:restored.state.meta.compatibilityScore,starting:restored.state.legacy.blueStartBonuses,inspirations:restored.state.legacy.inspirationEventsDone},{compatibility:0,starting:{},inspirations:0});
 write('restored-normalized.json',restored);
 await api('/v1/run/start',{seed:'114',scenario:'ura',trainee:'Special Week',raceModel:'stub',session:'parking',label:'Receiver parking'});
 const savedBefore=disk();write('saved-before.json',savedBefore);assert.ok(Object.keys(savedBefore).length);
 server=http.createServer(async(req,res)=>{try{
  const u=new URL(req.url,'http://localhost');
  if(u.pathname.startsWith('/v1/')){const chunks=[];for await(const x of req)chunks.push(x);const b=Buffer.concat(chunks);const entry={method:req.method,path:req.url,body:b.toString()};browserTraffic.push(entry);const upstream=await fetch(apiBase+req.url,{method:req.method,headers:b.length?{'Content-Type':'application/json'}:{},body:b.length?b:undefined});const raw=Buffer.from(await upstream.arrayBuffer());entry.status=upstream.status;entry.response_sha256=sha(raw);res.writeHead(upstream.status,{'Content-Type':upstream.headers.get('content-type')||'application/json'});res.end(raw);return;}
  const key=u.pathname.slice(1).replace(/\/$/,'/index.html');const b=assets.get(key);if(!b){res.writeHead(404);res.end('not found');return;}res.writeHead(200,{'Content-Type':key.endsWith('.js')?'text/javascript':key.endsWith('.css')?'text/css':'text/html'});res.end(b);
 }catch(e){res.writeHead(500);res.end(String(e));}});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 let stdout='',stderr='';browser=spawn('/snap/chromium/current/usr/lib/chromium-browser/chrome',['--headless=new','--no-sandbox','--disable-dev-shm-usage','--disable-background-networking','--disable-component-update','--disable-default-apps','--disable-extensions','--disable-sync','--no-first-run','--no-default-browser-check','--disk-cache-size=8388608','--remote-debugging-address=127.0.0.1','--remote-debugging-port=0','--user-data-dir='+profile,'about:blank'],{stdio:['ignore','pipe','pipe']});
 browser.stdout.on('data',b=>{stdout+=b;});browser.stderr.on('data',b=>{stderr+=b;});
 const endpoint=await wait(()=>{if(browser.exitCode!==null)throw Error('owned browser exited '+browser.exitCode);return stderr.match(/DevTools listening on (ws:\/\/\S+)/)?.[1];},30000,'owned browser');
 const target=await(await fetch('http://127.0.0.1:'+new URL(endpoint).port+'/json/new?about:blank',{method:'PUT'})).json();c=await CDP.connect(target.webSocketDebuggerUrl);
 c.on('Runtime.exceptionThrown',p=>errors.push(p));c.on('Fetch.requestPaused',async p=>{if(p.request.url.startsWith('http://127.0.0.1:')||p.request.url.startsWith('data:')||p.request.url.startsWith('blob:'))await c.send('Fetch.continueRequest',{requestId:p.requestId});else{external.push(p.request.url);await c.send('Fetch.failRequest',{requestId:p.requestId,errorReason:'BlockedByClient'});}});
 await c.send('Runtime.enable');await c.send('Page.enable');await c.send('Fetch.enable',{patterns:[{urlPattern:'*'}]});await c.send('Emulation.setDeviceMetricsOverride',{width:1344,height:1000,deviceScaleFactor:1,mobile:false});
 report.browser=await c.send('Browser.getVersion');
 await c.send('Page.navigate',{url:'http://127.0.0.1:'+server.address().port+'/candidate/'});
 await c.wait('document.readyState==="complete" && !!document.querySelector(".inheritance-record")');
 async function settle(){await c.wait('!document.querySelector(".busy-overlay")');await c.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');}
 async function clickExpr(expr){const xy=await c.evaluate('(()=>{const e='+expr+';if(!e)throw Error("control missing");e.scrollIntoView({block:"center"});const r=e.getBoundingClientRect();return{x:r.x+r.width/2,y:r.y+r.height/2};})()');await c.send('Input.dispatchMouseEvent',{type:'mousePressed',button:'left',clickCount:1,...xy});await c.send('Input.dispatchMouseEvent',{type:'mouseReleased',button:'left',clickCount:1,...xy});await settle();}
 const button=label=>'[...document.querySelectorAll("button")].find(e=>e.textContent.trim()==='+JSON.stringify(label)+')';
 await clickExpr(button('Library & lab'));await c.wait('[...document.querySelectorAll("tr")].some(e=>e.textContent.includes("Pending checkpoint"))');
 await clickExpr('[...document.querySelectorAll("tr")].find(e=>e.textContent.includes("Pending checkpoint")).querySelector("button")');await clickExpr(button('Run'));await c.wait('!!document.querySelector(".modal-backdrop .event")');
 check('Restored pending event is presented through the actual modal without auto-choice',await c.evaluate('({title:document.querySelector(".modal>p").textContent,options:[...document.querySelectorAll(".modal button.event")].map(e=>e.textContent)})'),{title:restored.state.pendingEventTitle,options:restored.state.pendingEventOptions});
 const heldState=await api('/v1/run/state?session=pending-ui'),heldChoices=await api('/v1/run/choices?session=pending-ui'),heldSessions=await api('/v1/sessions'),trafficBefore=browserTraffic.length;
 await c.screenshot(path.join(out,'restored-pending-event.png'));
 await c.evaluate('document.querySelector(".inheritance-record").textContent');
 check('Passive panel rendering behind the existing modal sends no request',browserTraffic.length,trafficBefore);
 check('Pending render leaves exact session decision data unchanged',await api('/v1/run/state?session=pending-ui'),heldState);
 check('Pending render leaves native choices and session list unchanged',{choices:await api('/v1/run/choices?session=pending-ui'),sessions:await api('/v1/sessions')},{choices:heldChoices,sessions:heldSessions});
 check('Pending render leaves saved checkpoint bytes unchanged',disk(),savedBefore);
 const action=heldChoices.choices[0]?.id;assert.match(action,/^event_\d+$/);const index=Number(action.slice(6));
 await api('/v1/run/action',{session:'pending-control',action});const expected=await api('/v1/run/state?session=pending-control');
 const actionStart=browserTraffic.length;await clickExpr('document.querySelectorAll(".modal button.event")['+index+']');await c.wait('!document.querySelector(".modal-backdrop")');await settle();
 const actual=await api('/v1/run/state?session=pending-ui');write('after-ui-choice.json',actual);write('after-control-choice.json',expected);
 check('One deliberate browser choice matches the exact private native control snapshot',actual,expected);
 check('The UI sent exactly one native event action',browserTraffic.slice(actionStart).filter(x=>x.method==='POST'&&x.path.startsWith('/v1/run/action')).map(x=>JSON.parse(x.body).action),[action]);
 const afterChoiceHeld=await api('/v1/run/state?session=pending-ui'),choiceHeld=await api('/v1/run/choices?session=pending-ui'),sessionHeld=await api('/v1/sessions'),readStart=browserTraffic.length;
 await clickExpr('document.querySelectorAll(".inheritance-record summary")[0]');await clickExpr('document.querySelectorAll(".inheritance-record summary")[1]');
 const observed=await c.evaluate('({positions:[...document.querySelectorAll("[data-ancestor]")].map(e=>({key:e.dataset.ancestor,name:e.querySelector(".inheritance-name")?.textContent,sparks:[...e.querySelectorAll("[data-factor-id]")].map(s=>({id:s.dataset.factorId,stars:s.querySelector(".inheritance-spark-label span").textContent}))})),bonuses:[...document.querySelectorAll(".inheritance-bonuses")].map(e=>({title:e.querySelector("h3").textContent,rows:[...e.querySelectorAll("dl>div")].map(x=>[x.querySelector("dt").textContent,x.querySelector("dd").textContent]),text:e.textContent})),text:document.querySelector(".inheritance-record").textContent})');
 write('restored-panel.json',observed);
 const expectedPositions=positions.map(key=>{const node=actual.state.meta.legacyTree[key];return {key,name:node.uma===''?'No name recorded':node.uma,sparks:['blue','pink','white','green','race'].filter(kind=>node[kind].factorId!=='').map(kind=>({id:node[kind].factorId,stars:String(node[kind].stars)+'★ recorded'}))};});
 check('The restored partial record displays the actual normalized native lineage',observed.positions,expectedPositions);
 check('Missing optional starting bonuses are displayed as returned empty values',observed.bonuses.find(x=>x.title==='Starting-stat bonuses').rows,[]);
 check('Reading the restored record introduces no new API traffic',browserTraffic.length,readStart);
 check('Record disclosure preserves exact state, choices and active sessions',{state:await api('/v1/run/state?session=pending-ui'),choices:await api('/v1/run/choices?session=pending-ui'),sessions:await api('/v1/sessions')},{state:afterChoiceHeld,choices:choiceHeld,sessions:sessionHeld});
 check('Deliberate decision and later reading preserve saved source checkpoint bytes',disk(),savedBefore);
 await c.evaluate('document.querySelector(".inheritance-record").scrollIntoView({block:"start"})');await c.screenshot(path.join(out,'restored-record-after-choice.png'));
 check('Current UI and native checkpoint journey has no runtime errors or external requests',{errors,external},{errors:[],external:[]});
 report.normalization_limit='The imported JSON omitted some lineage and optional bonus/count fields. The real205 decoder expanded them before serving the browser. This receives the normalized API record and genuine pending event, not absent-field UI handling.';
 report.browser_stdout=stdout;report.browser_stderr=stderr;
}catch(e){report.failure={name:e.name,message:e.message,stack:e.stack};if(c){try{write('failure-dom.txt',await c.evaluate('document.documentElement.outerHTML'));await c.screenshot(path.join(out,'failure.png'));}catch{}}}
finally{
 report.borrowed_changed=changed();report.page_errors=errors;report.external_requests=external;
 write('browser-api-traffic.json',browserTraffic);write('direct-api-traffic.json',directTraffic);
 if(c){try{await c.send('Browser.close');}catch{}c.socket.close();}
 if(browser&&browser.exitCode===null){await Promise.race([once(browser,'exit'),pause(2000)]);if(browser.exitCode===null)browser.kill('SIGTERM');}
 if(server)await new Promise(r=>server.close(r));
 if(apiChild&&apiChild.exitCode===null){apiChild.kill('SIGTERM');await Promise.race([once(apiChild,'exit'),pause(2000)]);}
 report.processes={api_pid:apiChild?.pid,api_exit:apiChild?.exitCode,api_signal:apiChild?.signalCode,browser_pid:browser?.pid,browser_exit:browser?.exitCode,browser_signal:browser?.signalCode,privateCwd,profile};
 report.completed_at=new Date().toISOString();report.passed=report.checks.filter(x=>x.pass).length;report.failed=report.checks.filter(x=>!x.pass).length;report.status=!report.failure&&!report.failed&&!report.borrowed_changed.length?'passed':'failed';
 write('receipt.json',report);console.log(JSON.stringify({status:report.status,passed:report.passed,failed:report.failed,failure:report.failure,borrowed_changed:report.borrowed_changed,receipt:path.join(out,'receipt.json')}));if(report.status!=='passed')process.exitCode=1;
}

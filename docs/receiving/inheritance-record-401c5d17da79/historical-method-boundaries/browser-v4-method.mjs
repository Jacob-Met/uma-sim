import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {isDeepStrictEqual} from 'node:util';
import crypto from 'node:crypto';
import http from 'node:http';
import net from 'node:net';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
const root=process.argv[2];
if(!root || !root.endsWith('/uma-inheritance-205178f1')) throw Error('explicit owned source root required');
const receiving=path.join(root,'receiving-native-v4');
fs.mkdirSync(receiving,{recursive:false});
const privateCwd=path.join(receiving,'private-careers');fs.mkdirSync(privateCwd);
const profilesRoot='/home/jacob/uma-inheritance-profiles-401c5d17da79';fs.mkdirSync(profilesRoot,{recursive:true,mode:0o700});
const profile=fs.mkdtempSync(path.join(profilesRoot,'inheritance-'));
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const write=(name,v)=>fs.writeFileSync(path.join(receiving,name),typeof v==='string'?v:JSON.stringify(v,null,2)+'\n');
const pause=ms=>new Promise(r=>setTimeout(r,ms));
const deadline=async(fn,ms,label)=>{const end=Date.now()+ms;let last;while(Date.now()<end){try{const v=await fn();if(v)return v;}catch(e){last=e;}await pause(100);}throw Error(label+' timed out '+String(last||''));};
const files=JSON.parse(fs.readFileSync(path.join(root,'ui-build-artifacts.json')));
for(const f of files.files){const b=Buffer.from(f.base64,'base64');if(b.length!==f.bytes||sha(b)!==f.sha256)throw Error('artifact hash mismatch '+f.key);}
const artefacts=new Map(files.files.map(f=>[f.key,Buffer.from(f.base64,'base64')]));
const sourcePaths=['src/components/inheritanceView.ts','src/components/InheritancePanel.tsx','src/components/inheritance-record.css','src/App.tsx','src/api/types.ts','test/inheritance-view.test.mjs','package.json'];
const source=sourcePaths.map(p=>{const b=fs.readFileSync(path.join(root,'candidate/packages/uma-sim-ui',p));return{path:'packages/uma-sim-ui/'+p,bytes:b.length,sha256:sha(b),gitBlob:crypto.createHash('sha1').update('blob '+b.length+'\0').update(b).digest('hex')};});
write('source-input.json',{canonical:'205178f1bf3bf5f34c04bbda88c3e4df2102f535',files:source,builds:{baseline:files.baseline,candidate:files.candidate}});
let apiProcess,browser,proxy,ws;const apiTraffic=[],runtimeErrors=[],checks=[];let outputPort;
function check(name,actual,expected=true){const ok=isDeepStrictEqual(actual,expected);checks.push({name,pass:ok,actual,expected});write('checks-progress.json',checks);if(!ok)throw Error('failed predicate: '+name);}
const ephemeral=()=>new Promise((resolve,reject)=>{const s=net.createServer();s.on('error',reject);s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>resolve(p));});});
let cdpId=0;const pending=new Map();
function cdp(method,params={}){return new Promise((resolve,reject)=>{const id=++cdpId;const timer=setTimeout(()=>{pending.delete(id);reject(Error('CDP timeout '+method));},15000);pending.set(id,{resolve,reject,timer});ws.send(JSON.stringify({id,method,params}));});}
async function ev(expression){const r=await cdp('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value;}
async function clickExpr(expr){const rect=await ev('(()=>{const e='+expr+';if(!e)throw Error("element missing");e.scrollIntoView({block:"center"});const r=e.getBoundingClientRect();return{x:r.x+r.width/2,y:r.y+r.height/2};})()');await cdp('Input.dispatchMouseEvent',{type:'mousePressed',button:'left',clickCount:1,...rect});await cdp('Input.dispatchMouseEvent',{type:'mouseReleased',button:'left',clickCount:1,...rect});}
const button=text=>'[...document.querySelectorAll("button")].find(e=>e.textContent.trim()==='+JSON.stringify(text)+')';
async function quiet(){await deadline(()=>ev('!document.querySelector(".busy-overlay")'),15000,'UI idle');await pause(150);}
async function tab(text){await clickExpr(button(text));await quiet();}
async function activate(label){await tab('Library & lab');await deadline(()=>ev('[...document.querySelectorAll("tr")].some(r=>r.textContent.includes('+JSON.stringify(label)+'))'),10000,'session rows');await clickExpr('[...document.querySelectorAll("tr")].find(r=>r.textContent.includes('+JSON.stringify(label)+')).querySelector("button")');await quiet();await tab('Run');await deadline(()=>ev('!!document.querySelector(".inheritance-record")'),10000,'inheritance panel');}
async function screenshot(name){const v=await cdp('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});fs.writeFileSync(path.join(receiving,name),Buffer.from(v.data,'base64'));}
async function navigate(variant){await cdp('Page.navigate',{url:'http://127.0.0.1:'+outputPort+'/'+variant+'/'});await deadline(()=>ev('document.readyState==="complete"&&!!document.querySelector("h1")&&[...document.querySelectorAll("option")].length>0'),15000,'app ready');await quiet();}
async function treeDisk(){const out={};function walk(p){if(!fs.existsSync(p))return;for(const e of fs.readdirSync(p,{withFileTypes:true})){const f=path.join(p,e.name);if(e.isDirectory())walk(f);else if(e.isFile())out[path.relative(privateCwd,f)]=sha(fs.readFileSync(f));}}walk(privateCwd);return out;}
try{
 const stat=fs.statfsSync(root);if(stat.bavail*stat.bsize<128*1024*1024)throw Error('insufficient private receiver capacity');
 const apiPort=await ephemeral();const apiBase='http://127.0.0.1:'+apiPort;
 const apiLog=fs.openSync(path.join(receiving,'native-api.log'),'wx');
 apiProcess=spawn(path.join(root,'native-api-target/debug/uma-sim-api'),[String(apiPort)],{cwd:privateCwd,env:{...process.env,UMA_REPO_ROOT:path.join(root,'native-api-source')},stdio:['ignore',apiLog,apiLog]});
 fs.closeSync(apiLog);
 async function api(route,body){const res=await fetch(apiBase+route,{method:body===undefined?'GET':'POST',headers:body===undefined?{}:{'Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});const text=await res.text();if(!res.ok)throw Error(route+' '+res.status+' '+text);return JSON.parse(text);}
 const health=await deadline(()=>api('/v1/health'),15000,'native API readiness');write('api-health.json',health);
 check('native catalog root matches exact owned source',health.repoRootPath,path.join(root,'native-api-source'));
 const factors=(await api('/v1/catalog/factors')).items;write('factor-catalog.json',factors);
 const blue=factors.find(x=>x.kind.toLowerCase()==='blue'),pink=factors.find(x=>x.kind.toLowerCase()==='pink'),white=factors.find(x=>x.kind.toLowerCase()==='white');
 if(!blue||!pink)throw Error('native catalogs missing factors');
 const blank=()=>({uma:'',blue:{factorId:'',stars:1},pink:{factorId:'',stars:1},white:{factorId:'',stars:1},green:{factorId:'',stars:1},race:{factorId:'',stars:1}});
 const positions=['parentA','gpA1','gpA2','parentB','gpB1','gpB2'];const tree=Object.fromEntries(positions.map((k,i)=>[k,{...blank(),uma:['Silence Suzuka','Mejiro McQueen','Gold Ship','Tokai Teio','Symboli Rudolf','Air Groove'][i],blue:{factorId:blue.id,stars:i===0?0:i===1?4:2}}]));
 tree.parentA.pink={factorId:pink.id,stars:2};
 tree.gpB2.white={factorId:'unknown:<record-only>&'+('long-record-'.repeat(7)),stars:-1};
 if(white)tree.parentB.white={factorId:white.id,stars:1};
 const structuredRequest={seed:'112',scenario:'ura',trainee:'Special Week',raceModel:'stub',dialogue:'choices',legacyTree:tree,legacyFactors:'flat-entry-kept-literal@9',compatibilityScore:67,session:'record-tree',label:'Record tree'};
 const flatRequest={seed:'113',scenario:'ura',trainee:'Special Week',raceModel:'stub',dialogue:'choices',legacyFactors:blue.id+'@3,'+blue.id+'@1,unknown:<flat-only>@9',parentNames:'Flat parent one,Flat parent two',session:'record-flat',label:'Record flat'};
 const sTree=await api('/v1/run/start',structuredRequest);write('structured-request.json',structuredRequest);write('structured-start.json',sTree);
 const sFlat=await api('/v1/run/start',flatRequest);write('flat-request.json',flatRequest);write('flat-start.json',sFlat);
 await api('/v1/run/start',{seed:'114',scenario:'ura',trainee:'Special Week',raceModel:'stub',session:'record-empty',label:'Record empty'});
 await api('/v1/run/start',{seed:'115',scenario:'ura',trainee:'Special Week',raceModel:'stub',session:'parking',label:'Parking'});
 await api('/v1/library/save',{name:'record-tree-saved',session:'record-tree'});
 await api('/v1/library/save',{name:'record-flat-saved',session:'record-flat'});
 const savedBefore=await treeDisk();write('saved-before.json',savedBefore);
 check('native start retains original tree stars rather than editor normalization',positions.map(k=>sTree.state.meta.legacyTree[k].blue.stars),[0,4,2,2,2,2]);
 check('native flat career omits structured tree',Object.hasOwn(sFlat.state.meta,'legacyTree'),false);
 check('native structured retained duplicate factors',sTree.state.legacy.factorIds.filter(v=>v===blue.id).length,6);
 // Serve exact production build and forward every /v1 call to the private native API unchanged.
 proxy=http.createServer(async(req,res)=>{try{const u=new URL(req.url,'http://localhost');if(u.pathname.startsWith('/v1/')){const chunks=[];for await(const c of req)chunks.push(c);const body=Buffer.concat(chunks);apiTraffic.push({method:req.method,path:req.url,body:body.toString()});const up=await fetch(apiBase+req.url,{method:req.method,headers:body.length?{'Content-Type':'application/json'}:{},body:body.length?body:undefined});res.writeHead(up.status,{'Content-Type':up.headers.get('content-type')||'application/json'});res.end(Buffer.from(await up.arrayBuffer()));return;}const key=u.pathname.slice(1).replace(/\/$/,'/index.html');const b=artefacts.get(key);if(!b){res.writeHead(404);res.end('not found');return;}res.writeHead(200,{'Content-Type':key.endsWith('.js')?'text/javascript':key.endsWith('.css')?'text/css':'text/html'});res.end(b);}catch(e){res.writeHead(500);res.end(String(e));}});
 await new Promise(r=>proxy.listen(0,'127.0.0.1',r));outputPort=proxy.address().port;
 let browserStderr='',browserStdout='';
 browser=spawn('/snap/chromium/current/usr/lib/chromium-browser/chrome',['--headless=new','--no-sandbox','--disable-dev-shm-usage','--disable-background-networking','--disable-component-update','--disable-default-apps','--disable-extensions','--disable-sync','--no-first-run','--no-default-browser-check','--remote-debugging-address=127.0.0.1','--remote-debugging-port=0','--user-data-dir='+profile,'about:blank'],{stdio:['ignore','pipe','pipe']});
 browser.stdout.on('data',b=>{browserStdout+=b;write('chromium.stdout',browserStdout)});browser.stderr.on('data',b=>{browserStderr+=b;write('chromium.log',browserStderr)});
 const endpoint=await deadline(()=>{if(browser.exitCode!==null)throw Error('Chromium exited '+browser.exitCode);return browserStderr.match(/DevTools listening on (ws:\/\/\S+)/)?.[1];},30000,'Chromium private debugging endpoint');
 const browserPort=new URL(endpoint).port;
 const target=await (await fetch('http://127.0.0.1:'+browserPort+'/json/new?about:blank',{method:'PUT'})).json();
 ws=new WebSocket(target.webSocketDebuggerUrl);await once(ws,'open');ws.onmessage=e=>{const m=JSON.parse(e.data);if(m.id&&pending.has(m.id)){const p=pending.get(m.id);clearTimeout(p.timer);pending.delete(m.id);m.error?p.reject(Error(JSON.stringify(m.error))):p.resolve(m.result);}else if(m.method==='Runtime.exceptionThrown')runtimeErrors.push(m.params);};
 await cdp('Page.enable');await cdp('Runtime.enable');await cdp('Emulation.setDeviceMetricsOverride',{width:1344,height:960,deviceScaleFactor:1,mobile:false});
 await navigate('baseline');await tab('Library & lab');await deadline(()=>ev('[...document.querySelectorAll("tr")].some(r=>r.textContent.includes("Record tree"))'),10000,'baseline session');await clickExpr('[...document.querySelectorAll("tr")].find(r=>r.textContent.includes("Record tree")).querySelector("button")');await quiet();await tab('Run');
 check('baseline running career has no inheritance inspection',await ev('document.querySelectorAll(".inheritance-record").length'),0);
 await screenshot('baseline-running.png');
 await api('/v1/session/activate',{session:'parking'});
 await navigate('candidate');await activate('Record tree');
 const panel=()=>ev('document.querySelector(".inheritance-record").innerText');
 check('new panel sits directly after aptitude',await ev('document.querySelector(".inheritance-record").previousElementSibling.textContent.includes("Aptitude")'));
 // Keyboard toggle is sent through Chromium, not a DOM click or an API action.
 const beforeRead=await api('/v1/run/state?session=record-tree'), trafficBefore=apiTraffic.length;
 await cdp('Page.bringToFront'); await ev('document.querySelector(".inheritance-record summary").focus()'); write('keyboard-focus-before.json',await ev('({tag:document.activeElement?.tagName,text:document.activeElement?.textContent,focused:document.hasFocus(),open:document.querySelector(".inheritance-record details").open})'));
 await cdp('Input.dispatchKeyEvent',{type:'keyDown',key:'Enter',code:'Enter',windowsVirtualKeyCode:13,text:'\r',unmodifiedText:'\r'});await cdp('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});
 await pause(100); check('keyboard Enter opens lineage disclosure',await ev('document.querySelector(".inheritance-record details").open'));
 await clickExpr('document.querySelectorAll(".inheritance-record summary")[1]');
 const text=await panel();write('structured-rendered.txt',text);
 check('all six explicit positions are rendered',await ev('[...document.querySelectorAll("[data-ancestor]")].map(e=>e.dataset.ancestor)'),positions);
 check('recorded zero and out-of-range stars remain literal',text.includes('0★ recorded')&&text.includes('4★ recorded')&&text.includes('-1★ recorded'));
 check('duplicate tree factors remain separate occurrences',await ev('[...document.querySelectorAll("[data-factor-id]")].filter(e=>e.dataset.factorId==='+JSON.stringify(blue.id)+').length'),6);
 check('known factor catalog label is visible',text.includes(blue.name));
 check('unknown markup-like factor is literal text',text.includes(tree.gpB2.white.factorId)&&await ev('document.querySelector(".inheritance-record").querySelectorAll("script,img,iframe").length')===0);
 check('recorded compatibility appears without calculated grade',text.includes('Recorded compatibility\n67'));
 const observedBonus=await ev('[...document.querySelectorAll(".inheritance-bonuses")].map(e=>({title:e.querySelector("h3").textContent,rows:[...e.querySelectorAll("dl>div")].map(r=>[r.querySelector("dt").textContent,r.querySelector("dd").textContent])}))');
 const labels={speed:'Speed',stamina:'Stamina',power:'Power',guts:'Guts',wit:'Wit'};
 const normalized=Object.fromEntries(observedBonus.map(x=>[x.title,Object.fromEntries(x.rows)]));
 check('cap values are native retained amounts',normalized['Stat-cap additions'],Object.fromEntries(Object.entries(sTree.state.legacy.sparkCaps).map(([k,v])=>[labels[k]||k,String(v)])));
 check('starting values are native retained amounts',normalized['Starting-stat bonuses'],Object.fromEntries(Object.entries(sTree.state.legacy.blueStartBonuses).map(([k,v])=>[labels[k]||k,String(v)])));
 check('candidate skills are not labelled as learned',text.includes('Inherited skill candidates')&&text.includes('Skills panel shows skills actually learned.'));
 await ev('document.querySelector(".inheritance-record").scrollIntoView({block:"start"})');await screenshot('candidate-desktop.png');
 await cdp('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:false});await ev('document.querySelector(".inheritance-record").scrollIntoView({block:"start"})');await pause(100);await screenshot('candidate-390.png');
 const geometry=await ev('(()=>{const p=document.querySelector(".inheritance-record"),r=p.getBoundingClientRect();return{viewport:innerWidth,left:r.left,right:r.right,width:r.width,client:p.clientWidth,scroll:p.scrollWidth,overflow:[...p.querySelectorAll("*")].filter(e=>{const b=e.getBoundingClientRect();return b.width&& (b.left<r.left-1||b.right>r.right+1)}).map(e=>({tag:e.tagName,text:e.textContent.slice(0,80)}))}})()');write('390-geometry.json',geometry);
 check('panel and long recorded identifiers fit 390px',geometry.left>=0&&geometry.right<=390&&geometry.scroll<=geometry.client+1&&geometry.overflow.length===0);
 await cdp('Input.dispatchKeyEvent',{type:'keyDown',key:'Tab',code:'Tab',windowsVirtualKeyCode:9});await cdp('Input.dispatchKeyEvent',{type:'keyUp',key:'Tab',code:'Tab',windowsVirtualKeyCode:9});
 check('reading and toggling trigger zero additional API requests',apiTraffic.length,trafficBefore);
 check('reading preserves exact native session snapshot',await api('/v1/run/state?session=record-tree'),beforeRead);
 check('reading preserves all saved file hashes',await treeDisk(),savedBefore);
 await cdp('Emulation.setDeviceMetricsOverride',{width:1344,height:960,deviceScaleFactor:1,mobile:false});
 // Continue through the real controls; one native initial choice and a later action.
 const choices=await api('/v1/run/choices?session=record-tree');write('structured-choices.json',choices);
 const first=choices.choices[0];if(!first)throw Error('no native choice');
 const candidateButtons=await ev('[...document.querySelectorAll("button")].map(e=>e.textContent)');write('choice-buttons.json',candidateButtons);
 await clickExpr('[...document.querySelectorAll(".choice-panel button,.choices button,button")].find(e=>e.textContent.includes('+JSON.stringify(first.label)+'))');await quiet();
 const continued=await api('/v1/run/state?session=record-tree');write('structured-after-choice.json',continued);
 check('completion wording mirrors the native state after continuation',await ev('[...document.querySelectorAll(".inheritance-overview>div")].find(e=>e.querySelector("dt").textContent==="Initial inheritance choice").querySelector("dd").textContent'),continued.state.legacy.inheritanceComplete?'Completed':'Not completed');
 check('continuing retains exact recorded tree',continued.state.meta.legacyTree,sTree.state.meta.legacyTree);
 const next=await api('/v1/run/choices?session=record-tree');
 if(next.choices.length){await clickExpr('[...document.querySelectorAll("button")].find(e=>e.textContent.includes('+JSON.stringify(next.choices[0].label)+'))');await quiet();}
 const progressed=await api('/v1/run/state?session=record-tree');write('structured-progressed.json',progressed);
 check('real continuation evolves career state',JSON.stringify(progressed)!==JSON.stringify(continued));
 check('progressed lineage remains the recorded native tree',progressed.state.meta.legacyTree,sTree.state.meta.legacyTree);
 await activate('Record flat');
 await clickExpr('document.querySelector(".inheritance-record summary")');await clickExpr('document.querySelectorAll(".inheritance-record summary")[1]');
 const flatText=await panel();write('flat-rendered.txt',flatText);
 check('switching career replaces structured rows with explicit flat-only state',await ev('document.querySelectorAll("[data-ancestor]").length')===0&&flatText.includes('No structured lineage is recorded.'));
 check('flat factors are displayed literally without inferred stars',sFlat.state.meta.legacyFactors.every(x=>flatText.includes(x))&&!flatText.includes('★ recorded'));
 check('changed career replaces parent names without stale tree names',flatText.includes('Flat parent one')&&!flatText.includes('Silence Suzuka'));
 await screenshot('flat-record.png');
 await activate('Record empty');await clickExpr('document.querySelector(".inheritance-record summary")');await clickExpr('document.querySelectorAll(".inheritance-record summary")[1]');
 const emptyText=await panel();write('empty-rendered.txt',emptyText);
 check('recorded empty inheritance remains distinct from missing',emptyText.includes('No parents recorded')&&emptyText.includes('No factors recorded.')&&emptyText.includes('No flat entries recorded.'));
 check('private saved checkpoints remain unchanged after progression and switching',await treeDisk(),savedBefore);
 check('no browser runtime exceptions',runtimeErrors.length,0);
 write('api-traffic.json',apiTraffic);write('runtime-errors.json',runtimeErrors);
 write('receipt.json',{pass:true,checks,canonical:'205178f1bf3bf5f34c04bbda88c3e4df2102f535',source,apiBinarySha256:sha(fs.readFileSync(path.join(root,'native-api-target/debug/uma-sim-api'))),browser:(await cdp('Browser.getVersion')),profile,privateCwd,realNativeApi:true,realBrowserInput:true,noMockedApi:true,fixtureBoundary:'Native current schema structured, flat-only and empty records. Missing/partial fields are covered by separately pinned view-helper tests, not by this native runtime.',savedHashesUnchanged:true});
 console.log(JSON.stringify({pass:true,checks:checks.length,receiving}));
}catch(e){write('failure.json',{error:e.stack,checks,apiTraffic,runtimeErrors});console.error(e.stack);process.exitCode=1;}
finally{try{if(ws?.readyState===1)await cdp('Browser.close');ws?.close();}catch{};if(proxy)await new Promise(r=>proxy.close(r));for(const p of [browser,apiProcess])if(p&&p.exitCode===null){p.kill('SIGTERM');}write('process-custody.json',{ownApiPid:apiProcess?.pid,ownBrowserPid:browser?.pid,privateCwd,profile,ended:new Date().toISOString()});}

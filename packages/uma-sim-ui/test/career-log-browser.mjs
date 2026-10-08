// Optional receiving test: built React UI, the real Rust API, Node22+, Chromium.
// All careers/checkpoints, ports and browser state belong to this test process.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import http from 'node:http';
import crypto from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(process.argv[2] ?? fileURLToPath(new URL('../../../', import.meta.url)));
const baseline = process.argv.includes('--baseline');
const chromium = process.env.UMA_LOG_CHROMIUM ?? 'chromium';
const binary = process.env.UMA_LOG_API_BINARY ?? path.join(root, 'target/debug/uma-sim-api');
const evidence = process.env.UMA_LOG_EVIDENCE_DIR ?? await fs.mkdtemp(path.join(os.tmpdir(), 'uma-log-evidence-'));
await fs.mkdir(evidence, { recursive: true });
const stateDir = await fs.mkdtemp(path.join(process.env.UMA_LOG_STATE_ROOT ?? os.tmpdir(), 'uma-log-state-'));
const profile = await fs.mkdtemp(path.join(process.env.UMA_LOG_PROFILE_ROOT ?? os.tmpdir(), 'hamon-uma-log-e652aa-'));
const files = ['packages/uma-sim-ui/src/App.tsx', 'packages/uma-sim-ui/src/components/LogPanel.tsx',
  'packages/uma-sim-ui/src/components/logView.ts', 'packages/uma-sim-ui/src/components/logPanel.css',
  'packages/uma-sim-ui/src/state/runStore.ts', 'packages/uma-sim-ui/src/api/client.ts', 'uma-sim-core/src/api.rs'];
async function hashes() {
  const result = {};
  for (const name of files) {
    try { result[name] = crypto.createHash('sha256').update(await fs.readFile(path.join(root, name))).digest('hex'); }
    catch (error) { if (!baseline || error.code !== 'ENOENT') throw error; }
  }
  return result;
}
const sourceBefore = await hashes();
const checks = [], exceptions = [], requests = [];
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const pending = new Map();
let browser, apiProcess, socket, server, nextId = 0, receipt;

async function freePort() {
  const reservation = http.createServer();
  await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
  const port = reservation.address().port;
  await new Promise(resolve => reservation.close(resolve));
  return port;
}
const apiPort = await freePort();
const apiUrl = `http://127.0.0.1:${apiPort}`;
async function api(route, body) {
  const response = await fetch(apiUrl + route, { method: body ? 'POST' : 'GET',
    headers: body ? { 'content-type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined, signal: AbortSignal.timeout(15000) });
  const value = await response.json();
  assert.ok(response.ok, `${route}: ${JSON.stringify(value)}`);
  return value;
}
function send(method, params = {}, sessionId) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(Error(`CDP timeout ${method}`)); }, 15000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
  });
}
async function evaluate(session, expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, session);
  if (response.exceptionDetails) throw Error(response.exceptionDetails.exception?.description ?? response.exceptionDetails.text);
  return response.result.value;
}
async function waitFor(session, expression) {
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    if (await evaluate(session, expression)) return;
    await delay(60);
  }
  throw Error(`Browser condition not reached: ${expression}\n${await evaluate(session, 'document.body.innerText.slice(0, 2000)')}`);
}
async function clickElement(session, expression) {
  let point, last;
  for (let attempt=0; attempt<40; attempt++) {
    await evaluate(session, `(${expression})?.scrollIntoView({block:'center',behavior:'instant'})`);
    await delay(50);
    last = await evaluate(session, `(() => { const el = ${expression};
      if (!el || el.disabled) return {ready:false,reason:'missing or disabled'};
      const r=el.getBoundingClientRect(),x=r.x+r.width/2,y=r.y+r.height/2;
      const hit=document.elementFromPoint(x,y);
      return {ready:el.contains(hit),x,y,hit:hit?.outerHTML.slice(0,250)}; })()`);
    if(last.ready){point={x:last.x,y:last.y};break;}
  }
  assert.ok(point,`Receiving control did not become clickable: ${expression} ${JSON.stringify(last)}`);
  await send('Input.dispatchMouseEvent', {type:'mousePressed',button:'left',clickCount:1,...point}, session);
  await send('Input.dispatchMouseEvent', {type:'mouseReleased',button:'left',clickCount:1,...point}, session);
  await delay(100);
}
const button = name => `[...document.querySelectorAll('button')].find(el => el.textContent.trim() === ${JSON.stringify(name)})`;
async function settle(session) {
  await waitFor(session, '!document.querySelector(".busy-overlay") && !document.querySelector(".banner.error")');
}
async function step(session) {
  await settle(session);
  const modal = await evaluate(session, '!!document.querySelector(".modal button")');
  await clickElement(session, modal ? 'document.querySelector(".modal button")' : button('Auto step'));
  await settle(session);
}
async function dismissEvent(session) {
  for (let i = 0; i < 8 && await evaluate(session, '!!document.querySelector(".modal button")'); i++) await step(session);
  assert.equal(await evaluate(session, '!!document.querySelector(".modal button")'), false);
}
async function search(session, query) {
  await clickElement(session, 'document.querySelector(".career-log input")');
  await send('Input.dispatchKeyEvent', {type:'rawKeyDown',key:'Control',code:'ControlLeft',windowsVirtualKeyCode:17,modifiers:2}, session);
  await send('Input.dispatchKeyEvent', {type:'rawKeyDown',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:2}, session);
  await send('Input.dispatchKeyEvent', {type:'keyUp',key:'a',code:'KeyA',windowsVirtualKeyCode:65,modifiers:2}, session);
  await send('Input.dispatchKeyEvent', {type:'keyUp',key:'Control',code:'ControlLeft',windowsVirtualKeyCode:17,modifiers:0}, session);
  assert.equal(await evaluate(session,`(() => {const el=document.querySelector('.career-log input');return document.activeElement===el && el.selectionStart===0 && el.selectionEnd===el.value.length})()`),true);
  await send('Input.insertText', {text:query}, session);
  await waitFor(session,`document.querySelector('.career-log input').value===${JSON.stringify(query)}`);
}
async function logState(session) {
  return evaluate(session, `(() => { const box=document.querySelector('.career-log-viewport'); return {
    rows:[...box.querySelectorAll('li')].map(el=>el.textContent),
    positions:[...box.querySelectorAll('li')].map(el=>el.value),
    marks:[...box.querySelectorAll('mark')].map(el=>el.textContent),
    top:box.scrollTop, height:box.clientHeight, scrollHeight:box.scrollHeight,
    overflow:box.scrollWidth>box.clientWidth+1,
    status:document.querySelector('.career-log-status').textContent,
    query:document.querySelector('.career-log input').value,
    following:document.querySelector('.career-log button[data-following]').getAttribute('data-following'),
    focus:document.activeElement===document.querySelector('.career-log input'),
    injectedElements:box.querySelectorAll('img,script,iframe').length,
    summary:document.querySelector('.career-log-summary pre').textContent
  }; })()`);
}
async function screenshot(session, name) {
  await evaluate(session, 'document.querySelector(".career-log")?.scrollIntoView({block:"start"})');
  const shot = await send('Page.captureScreenshot', {format:'png',captureBeyondViewport:false}, session);
  await fs.writeFile(path.join(evidence, name), Buffer.from(shot.data, 'base64'));
}

try {
  const dist = path.join(root, 'packages/uma-sim-ui/dist');
  server = http.createServer(async (request,response)=>{
    if (request.url.startsWith('/v1/')) {
      const upstream=http.request(apiUrl+request.url,{method:request.method,headers:request.headers},res=>{
        response.writeHead(res.statusCode,res.headers);res.pipe(response);
      });
      upstream.on('error',()=>{response.writeHead(502);response.end('Isolated API unavailable')});
      request.pipe(upstream); return;
    }
    try {
      const pathname=decodeURIComponent(new URL(request.url,'http://localhost').pathname);
      const file=path.resolve(dist,'.'+(pathname==='/'?'/index.html':pathname));
      if (!file.startsWith(dist+path.sep)) throw Error('outside dist');
      const content=await fs.readFile(file);
      const type={'.html':'text/html','.js':'text/javascript','.css':'text/css'}[path.extname(file)]??'application/octet-stream';
      response.writeHead(200,{'content-type':type,'cache-control':'no-store'});response.end(content);
    } catch {response.writeHead(404);response.end('No such test asset')}
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const url=`http://127.0.0.1:${server.address().port}/`;
  browser=spawn(chromium,['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check',
    '--disable-background-networking','--disable-component-update','--disable-sync','--disable-extensions',
    '--password-store=basic','--remote-debugging-address=127.0.0.1','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],
    {env:{...process.env,TMPDIR:stateDir},stdio:['ignore','ignore','pipe']});
  const endpoint=await new Promise((resolve,reject)=>{
    let output='';const timer=setTimeout(()=>reject(Error('Chromium start timeout: '+output.slice(-1000))),15000);
    browser.on('error',reject);browser.stderr.on('data',bytes=>{
      output+=bytes.toString();const found=output.match(/DevTools listening on (ws:\/\/127\.0\.0\.1:[^\s]+)/);
      if(found){clearTimeout(timer);resolve(found[1])}
    });
  });
  socket=new WebSocket(endpoint);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true})});
  socket.addEventListener('message',event=>{
    const message=JSON.parse(event.data);
    if(pending.has(message.id)){const task=pending.get(message.id);pending.delete(message.id);clearTimeout(task.timer);
      if(message.error)task.reject(Error(JSON.stringify(message.error)));else task.resolve(message.result)}
    if(message.method==='Runtime.exceptionThrown')exceptions.push(message.params.exceptionDetails.text);
    if(message.method==='Network.requestWillBeSent')requests.push(message.params.request.url);
  });
  // Start the browser before warming native catalogs to limit startup peaks.
  apiProcess = spawn(binary, [String(apiPort)], {cwd:stateDir,env:{...process.env,UMA_REPO_ROOT:root},stdio:['ignore','pipe','pipe']});
  let apiOutput = '';
  apiProcess.stdout.on('data', data=>{apiOutput+=data.toString()});
  apiProcess.stderr.on('data', data=>{apiOutput+=data.toString()});
  for (let i=0; i<100; i++) {
    try { await api('/v1/health'); break; } catch (error) {
      if (apiProcess.exitCode !== null || i===99) throw Error(`Isolated Rust API did not start: ${apiOutput}`);
      await delay(100);
    }
  }
  const {targetId}=await send('Target.createTarget',{url:'about:blank'});
  const {sessionId:session}=await send('Target.attachToTarget',{targetId,flatten:true});
  for(const method of ['Page.enable','Runtime.enable','Network.enable'])await send(method,{},session);
  await send('Emulation.setDeviceMetricsOverride',{width:1280,height:1000,deviceScaleFactor:1,mobile:false},session);
  await send('Page.navigate',{url},session);
  await waitFor(session, `${button('Start run')} && !document.querySelector('.busy-overlay')`);
  await clickElement(session,button('Start run'));
  await waitFor(session,'!!document.querySelector(".turn-layout")');
  await dismissEvent(session);
  for(let i=0;i<3;i++)await step(session);
  await dismissEvent(session);
  let snapshot=await api('/v1/run/state');
  const stateText=(await api('/v1/run/text')).text;
  assert.ok(snapshot.state.log.length>0);
  // RunSetup already loads catalog portrait URLs. The log itself must add no
  // external requests after that setup view has been replaced by the career.
  const logRequestStart=requests.length;

  if(baseline){
    const visible=await evaluate(session,'document.querySelector(".log").textContent');
    assert.equal(visible,stateText);
    assert.notEqual(visible,snapshot.state.log.join('\n'));
    assert.equal(await evaluate(session,'document.querySelector(".career-log input")'),null);
    receipt={status:'baseline-reproduced',retainedEntries:snapshot.state.log,visibleSummary:visible,
      defect:'Event log contains current-state rendering, not the retained career log'};
  }else{
    assert.deepEqual((await logState(session)).rows,snapshot.state.log);
    assert.equal((await logState(session)).summary,stateText);
    checks.push('Real UI career choices expose exact native retained history and keep state summary separate');
    for(let i=0;i<55 && snapshot.state.log.length<38;i++){
      await step(session);snapshot=await api('/v1/run/state');
      if(snapshot.state.careerComplete)break;
    }
    await dismissEvent(session);snapshot=await api('/v1/run/state');
    assert.equal(snapshot.state.careerComplete,false,'Need a continuing career to test live append behavior');
    let view=await logState(session);
    assert.deepEqual(view.rows,snapshot.state.log);
    assert.ok(view.scrollHeight>view.height+100,'Retained career must make a bounded scroll region');
    await search(session,'rAcE');view=await logState(session);
    const matching=snapshot.state.log.map((line,index)=>({line,index})).filter(x=>/race/i.test(x.line));
    assert.deepEqual(view.rows,matching.map(x=>x.line));
    assert.deepEqual(view.positions,matching.map(x=>x.index+1));
    assert.equal(view.following,'false');assert.equal(view.focus,true);
    assert.ok(view.marks.every(x=>/^race$/i.test(x)));
    await search(session,'this entry cannot exist');
    assert.equal((await logState(session)).rows.length,0);
    assert.match((await logState(session)).status,/0 of .*entries match/);
    checks.push('Real input preserves focus, literal search uses actual native rows and positions, no-match state is accurate');
    await clickElement(session,button('Clear search'));
    await evaluate(session,`document.querySelector('.career-log-viewport').scrollTop=48`);
    await delay(100);const paused=(await logState(session)).top;
    const before=snapshot.state.log;
    await step(session);await dismissEvent(session);snapshot=await api('/v1/run/state');
    view=await logState(session);
    assert.deepEqual(view.rows,snapshot.state.log);
    assert.deepEqual(snapshot.state.log.slice(0,before.length),before);
    assert.ok(snapshot.state.log.length>before.length);
    assert.equal(view.following,'false');assert.ok(Math.abs(view.top-paused)<2);
    await send('Emulation.setDeviceMetricsOverride',{width:1040,height:1000,deviceScaleFactor:1,mobile:false},session);
    await delay(150);view=await logState(session);
    assert.equal(view.following,'false');assert.ok(Math.abs(view.top-paused)<2);
    await send('Emulation.setDeviceMetricsOverride',{width:1280,height:1000,deviceScaleFactor:1,mobile:false},session);
    await delay(150);
    checks.push('A real native career update appends visible events without moving the paused reading position');
    await clickElement(session,button('Follow latest'));view=await logState(session);
    assert.equal(view.following,'true');assert.ok(view.scrollHeight-view.height-view.top<2);
    await screenshot(session,'desktop.png');
    await send('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true},session);
    await delay(150);view=await logState(session);
    assert.equal(view.overflow,false);
    assert.equal(view.following,'true');assert.ok(view.scrollHeight-view.height-view.top<2,'Resize must keep a following reader at the latest entry');
    await screenshot(session,'mobile.png');
    checks.push('Follow latest reaches the newest entry at desktop and mobile widths without horizontal log overflow');

    await send('Emulation.setDeviceMetricsOverride',{width:1280,height:1000,deviceScaleFactor:1,mobile:false},session);
    await delay(100);
    const fixture=structuredClone(snapshot);
    fixture.state.log=['<img src=x onerror=alert(1)>','İrace RACE','ΟΣ ος οσ','skill[123].(x)+',
      'ウマ娘 🏇 ウマ娘','Rested','Rested',...Array.from({length:35},(_,i)=>`Retained fixture ${i} ${'very-long-event-id-'.repeat(12)}`)];
    await api('/v1/library/import',{name:'log-reader-fixture',snapshot:fixture});
    await api('/v1/library/load',{name:'log-reader-fixture'});
    await clickElement(session,button('Pause following'));
    await evaluate(session,`document.querySelector('.career-log-viewport').scrollTop=90`);await delay(80);
    await step(session);await dismissEvent(session);view=await logState(session);
    assert.equal(view.top,0,'Replacement must reset a paused position rather than infer an append');
    assert.equal(view.injectedElements,0);
    assert.deepEqual(view.rows,(await api('/v1/run/state')).state.log);
    await search(session,'[123].(x)+');view=await logState(session);
    assert.deepEqual(view.rows,['skill[123].(x)+']);assert.deepEqual(view.marks,['[123].(x)+']);
    await search(session,'race');view=await logState(session);
    assert.ok(view.rows.includes('İrace RACE'),JSON.stringify(view));assert.deepEqual(view.marks.slice(0,2),['race','RACE']);
    await search(session,'<img');view=await logState(session);
    assert.equal(view.injectedElements,0);assert.deepEqual(view.marks,['<img']);
    await clickElement(session,button('Clear search & follow latest'));
    await send('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true},session);
    await delay(150);view=await logState(session);assert.equal(view.overflow,false);
    assert.ok(view.scrollHeight-view.height-view.top<2);
    checks.push('Native library-loaded replacement resets paused history; markup and Unicode remain text; long identifiers wrap on mobile');
    assert.deepEqual(exceptions,[]);
    const logExternalRequests=requests.slice(logRequestStart).filter(request=>new URL(request).origin!==new URL(url).origin);
    assert.deepEqual(logExternalRequests,[]);
    receipt={status:'pass',checks,actualBrowser:true,actualRustApi:true,coreOrSessionImplementationChanged:false,
      nativeSeed:snapshot.meta.seed,nativeTurn:snapshot.state.turn,viewports:[[1280,1000],[390,844]],
      logExternalRequests,setupMayLoadExistingCatalogPortraitUrls:true};
  }
  assert.deepEqual(await hashes(),sourceBefore);
  receipt={...receipt,node:process.version,browser:execFileSync(chromium,['--version'],{encoding:'utf8',timeout:15000}).trim(),
    sourceSha256:sourceBefore,apiBinarySha256:crypto.createHash('sha256').update(await fs.readFile(binary)).digest('hex'),
    persistentUserCareerTouched:false,publicDeploymentClaimed:false};
}finally{
  if(socket?.readyState===WebSocket.OPEN)await Promise.race([send('Browser.close').catch(()=>{}),delay(3000)]);
  socket?.close();
  for(const process of [browser,apiProcess])if(process&&process.exitCode===null){
    process.kill('SIGTERM');await Promise.race([new Promise(resolve=>process.once('exit',resolve)),delay(3000)]);
    if(process.exitCode===null)process.kill('SIGKILL');
  }
  if(server){server.closeAllConnections();await new Promise(resolve=>server.close(resolve))}
  await fs.rm(profile,{recursive:true,force:true,maxRetries:10,retryDelay:100});
  await fs.rm(stateDir,{recursive:true,force:true,maxRetries:5,retryDelay:100});
  for(const task of pending.values())clearTimeout(task.timer);
}
await fs.writeFile(path.join(evidence,baseline?'baseline-receipt.json':'browser-receipt.json'),JSON.stringify(receipt,null,2)+'\n');
console.log(JSON.stringify(receipt,null,2));

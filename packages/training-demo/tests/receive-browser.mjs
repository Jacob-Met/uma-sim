import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {fileURLToPath, pathToFileURL} from 'node:url';

// Independent DOM receiving: no app module imports or internal React state reads.
const argv=process.argv.slice(2),args={};
if(argv.length%2)throw Error('Options need values');
for(let i=0;i<argv.length;i+=2){
  if(!['--page','--fixture','--out','--playwright','--chromium','--tmp'].includes(argv[i])||args[argv[i].slice(2)])throw Error('Unknown or duplicate option');
  args[argv[i].slice(2)]=path.resolve(argv[i+1]);
}
for(const key of ['page','fixture','out','playwright','chromium','tmp'])if(!args[key])throw Error('Missing --'+key);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const pageBytes=fs.readFileSync(args.page),fixtureBytes=fs.readFileSync(args.fixture),fixture=JSON.parse(fixtureBytes);
assert.equal(hash(fixtureBytes),'906e6876f2485b012677ffda82295858ad5afda818c33b57a39fece9f992d9de');
assert.equal(fixture.source.commit,'0bc58cb83e89f07c6af3056dd7c37436906d994f');
assert.equal(fixture.checkpoints.length,9);
assert.equal(fixture.checkpoints.reduce((n,c)=>n+c.outcomes.length,0),63);
fs.mkdirSync(args.out,{mode:0o700});
fs.mkdirSync(args.tmp,{recursive:true,mode:0o700});
process.env.TMPDIR=args.tmp;
const require=createRequire(import.meta.url),{chromium}=require(args.playwright);
const url=pathToFileURL(args.page).href,checks=[],pageErrors=[],requests=[],blocked=[],screenshots=[];
let assertions=0,browser,context,page,fatal=null,browserVersion=null;
const eq=(a,b,label)=>{assertions++;assert.deepEqual(a,b,label);};
const ok=(v,label)=>{assertions++;assert.ok(v,label);};
const title=v=>v[0].toUpperCase()+v.slice(1).toLowerCase();
const conditions=s=>s.statuses.length?s.statuses.join(', '):'None';
const statKeys=['speed','stamina','power','guts','wit'];
const checkpoint=(seed,depth)=>fixture.checkpoints.find(c=>c.seed===String(seed)&&c.trainingActionsBefore===depth);
const selectedChoices=c=>c.choices.filter(x=>c.outcomes.some(y=>y.actionId===x.id));
async function check(name,fn){
  const before=assertions;
  try{const detail=await fn();checks.push({name,passed:true,assertions:assertions-before,...(detail?{detail}:{})});return true;}
  catch(e){checks.push({name,passed:false,assertions:assertions-before,error:e.message});return false;}
}
async function setup(ctx){
  const pg=await ctx.newPage();pg.setDefaultTimeout(5000);
  pg.on('pageerror',e=>pageErrors.push(e.message));
  pg.on('request',r=>requests.push({url:r.url(),method:r.method()}));
  await pg.route('**/*',route=>{
    const request=route.request();
    if(request.url().startsWith('file:'))return route.continue();
    blocked.push({url:request.url(),method:request.method()});return route.abort();
  });
  await pg.goto(url);return pg;
}
async function readDom(pg=page){
  return pg.evaluate(()=>{
    const q=s=>document.querySelector(s),all=s=>[...document.querySelectorAll(s)],text=e=>e?.textContent?.trim()??null;
    const status=q('[data-testid="status-panel"]');
    return {
      seed:q('select')?.value,depth:all('select')[1]?.value,
      action:q('[data-testid="outcome"]')?.getAttribute('data-action'),
      heading:text(q('#outcome-heading')),context:text(q('.result-context')),
      checkpoint:all('[data-testid="checkpoint-summary"] > span').map(e=>text(e)),
      statusLabel:text(status?.querySelector('.status-label')),
      meta:[...status.querySelectorAll('.meta-line')].map(e=>[text(e.querySelector('span')),text(e.querySelector('strong'))]),
      stats:[...status.querySelectorAll('.stat-row')].map(e=>[text(e.querySelector(':scope > span')),text(e.querySelector('strong'))]),
      choices:all('.choice-list button').map(e=>({text:text(e),shortcut:e.title,disabled:e.disabled})),
      preparation:all('[data-testid="preparation"] li code').map(text),
      provenance:text(q('.provenance')),
      deltas:all('[data-testid="changes"] tbody tr').map(e=>({key:e.getAttribute('data-measure'),cells:[...e.children].map(text),className:e.lastElementChild.className})),
      conditionChanges:all('[data-testid="condition-changes"] > div').map(e=>[text(e.querySelector('dt')),text(e.querySelector('dd'))]),
      native:text(q('[data-testid="native-response"]')),
      nextChoices:all('[data-testid="next-choices"] li').map(e=>({text:text(e),id:text(e.querySelector('code')),interactive:e.querySelectorAll('button,a,input,select').length})),
      nextText:text(q('.next-choices')),
      announcement:text(q('[role="status"]')),
      resetDisabled:q('.reset-button')?.disabled,
      replayCount:all('.replay-button').length,
      overflow:{scroll:document.documentElement.scrollWidth,viewport:innerWidth},
    };
  });
}
function assertStatus(dom,s,where){
  eq(dom.meta,[['Turn',String(s.turn)],['Date',`Y${s.date.year} M${s.date.month} ${s.date.half===1?'Early':'Late'}`],['Phase',s.phase],['Energy',`${s.energy}/${s.maxEnergy}`],['Mood',s.mood],['Fans',s.fans.toLocaleString('en-US')],['Skill pts',String(s.skillPoints)],...statKeys.map(k=>[k,`Lv ${s.facilityLevels[k]??1}`])],where+' status metadata and facilities');
  eq(dom.stats,statKeys.map(k=>[k,String(s.stats[k])]),where+' all five native stats');
}
function assertCheckpoint(dom,c){
  const s=c.snapshot.state;
  eq(dom.seed,c.seed,c.id+' selected seed');eq(dom.depth,String(c.trainingActionsBefore),c.id+' selected depth');
  eq(dom.checkpoint,[`Turn ${s.turn}`,`Energy ${s.energy}/${s.maxEnergy}`,`Mood ${title(s.mood)}`,`Conditions ${conditions(s)}`],c.id+' unchanged checkpoint summary');
  eq(dom.preparation,c.preparation,c.id+' exact native preparation lineage');
  ok(dom.provenance.includes(`${c.snapshot.rngCalls} RNG calls before the recorded choice.`),c.id+' checkpoint RNG calls');
  eq(dom.choices,selectedChoices(c).map((choice,i)=>{
    const key=choice.id.replace('train_',''),level=s.facilityLevels[key];
    return{text:`${i+1} ${choice.label}${choice.id.startsWith('train_')&&level!=null?` (Lv ${level})`:''}`,shortcut:`Shortcut ${i+1}`,disabled:false};
  }),c.id+' only seven qualified actions at baseline facility levels');
}
function assertBaseline(dom,c){
  assertCheckpoint(dom,c);assertStatus(dom,c.snapshot.state,c.id+' baseline');
  eq(dom.action,'',c.id+' no selected action');eq(dom.deltas,[],c.id+' no stale deltas');
  eq(dom.native,null,c.id+' no stale native response');eq(dom.statusLabel,'At the checkpoint');
  eq(dom.resetDisabled,true);eq(dom.replayCount,0);
}
function assertOutcome(dom,c,outcome){
  const before=c.snapshot.state,after=outcome.response.state.state;
  assertCheckpoint(dom,c);assertStatus(dom,after,c.id+'/'+outcome.actionId);
  eq(dom.action,outcome.actionId,'selected action identity');
  eq(dom.heading,c.choices.find(x=>x.id===outcome.actionId).label,'native choice label');
  eq(dom.context,`Seed ${c.seed} · turn ${before.turn} → ${after.turn} · ${after.phase}`,'seed, both turns and resulting phase');
  const measures=[...statKeys,'energy','skillPoints','fans'];
  eq(dom.deltas,measures.map(key=>{
    const a=statKeys.includes(key)?before.stats[key]:before[key],b=statKeys.includes(key)?after.stats[key]:after[key];
    return{key,cells:[key==='skillPoints'?'Skill points':title(key),String(a),String(b),b>a?'+'+(b-a):String(b-a)],className:b>a?'gain':b<a?'loss':'unchanged'};
  }),'all eight native before/after values and exact arithmetic');
  eq(dom.conditionChanges,[['Mood',`${title(before.mood)} → ${title(after.mood)}`],['Conditions',`${conditions(before)} → ${conditions(after)}`]],'native mood and injury changes');
  eq(dom.native,outcome.response.text,'unmodified full native response');
  eq(dom.nextChoices,outcome.response.choices.map(x=>({text:x.label+' '+x.id,id:x.id,interactive:0})),'complete informational next choices, including excluded actions');
  ok(dom.nextText.includes(`Career ended: ${outcome.response.careerEnded?'Yes':'No'} · RNG calls after: ${outcome.response.state.rngCalls}`),'native ending and RNG evidence');
  ok(dom.announcement.startsWith(`${dom.heading}: recorded outcome from seed ${c.seed}, turn ${before.turn}. Result turn ${after.turn}, energy ${after.energy}.`),'live region identifies actual outcome');
  eq(dom.statusLabel,'After the recorded action');eq(dom.resetDisabled,false);eq(dom.replayCount,1);
}
async function selectCheckpoint(c,pg=page){
  await pg.getByRole('combobox',{name:'Seed',exact:true}).selectOption(c.seed);
  await pg.getByRole('combobox',{name:'Checkpoint',exact:true}).selectOption(String(c.trainingActionsBefore));
  const reset=pg.getByRole('button',{name:'Reset comparison',exact:true});if(await reset.isEnabled())await reset.click();
}
async function choose(c,action,pg=page){
  const index=selectedChoices(c).findIndex(x=>x.id===action);ok(index>=0,'selected action exists in recorded fixture');
  await pg.locator('.choice-list button').nth(index).click();return readDom(pg);
}
try{
  browser=await chromium.launch({headless:true,executablePath:args.chromium,args:['--no-sandbox','--disable-dev-shm-usage']});
  browserVersion=browser.version();
  context=await browser.newContext({viewport:{width:1440,height:1000},locale:'en-US'});
  page=await setup(context);
  const initial=await check('offline artifact default and native provenance',async()=>{
    await page.getByRole('heading',{name:'Compare a training turn',exact:true}).waitFor();
    eq(await page.getByRole('combobox',{name:'Seed',exact:true}).count(),1);
    eq(await page.getByRole('combobox',{name:'Checkpoint',exact:true}).count(),1);
    eq(await page.getByRole('combobox',{name:'Seed',exact:true}).locator('option').evaluateAll(es=>es.map(e=>e.value)),fixture.seeds);
    eq(await page.getByRole('combobox',{name:'Checkpoint',exact:true}).locator('option').evaluateAll(es=>es.map(e=>e.value)),['0','3','6']);
    assertBaseline(await readDom(),checkpoint('7',0));
    ok(await page.locator('.mode-badge').isVisible());
    eq((await page.locator('.mode-badge').innerText()).trim(),'Unofficial · Recorded fixture');
    const footer=await page.locator('footer').textContent();
    ok(footer.includes('Three seeds, nine checkpoints, 63 native outcomes.'));
    ok(footer.includes('it does not run a new simulation or continue a full career.'));
    ok(footer.includes(fixture.source.commit));
    eq(await page.locator('a[href*="/tree/"]').getAttribute('href'),fixture.source.repository+'/tree/'+fixture.source.commit);
  });
  if(!initial)throw Error('Initial receiving precondition failed');
  for(const c of fixture.checkpoints){
    if(page.isClosed())throw Error('Browser page closed during finite outcome receiving');
    await check(c.id+' baseline and lineage',async()=>{await selectCheckpoint(c);assertBaseline(await readDom(),c);});
    for(const outcome of c.outcomes){
      await check(c.id+'/'+outcome.actionId+' exact rendered native outcome',async()=>assertOutcome(await choose(c,outcome.actionId),c,outcome));
    }
  }
  await check('same-checkpoint alternatives never chain and replay is exact',async()=>{
    const c=checkpoint('1',3);await selectCheckpoint(c);
    const rest=c.outcomes.find(x=>x.actionId==='rest'),training=c.outcomes.find(x=>x.actionId==='train_speed');
    assertOutcome(await choose(c,'rest'),c,rest);
    const first=await readDom();eq(first.meta.find(x=>x[0]==='Energy')[1],'90/100');eq(first.stats.find(x=>x[0]==='speed')[1],'115');
    await page.getByRole('button',{name:'Replay this recording',exact:true}).click();
    const replay=await readDom();assertOutcome(replay,c,rest);eq(replay.native,first.native);eq(replay.deltas,first.deltas);
    ok(replay.announcement.includes('Comparisons viewed at this checkpoint: 2.'));
    const trained=await choose(c,'train_speed');assertOutcome(trained,c,training);
    eq(trained.meta.find(x=>x[0]==='Energy')[1],'20/100');eq(trained.stats.find(x=>x[0]==='speed')[1],'127');
    ok(trained.announcement.includes('Comparisons viewed at this checkpoint: 3.'));
    eq(trained.deltas.find(x=>x.key==='energy').cells.slice(1),['40','20','-20']);
    await page.getByRole('button',{name:'Reset comparison',exact:true}).click();assertBaseline(await readDom(),c);
  });
  await check('seed and checkpoint changes clear result while preserving selected depth',async()=>{
    const c=checkpoint('1',6);await selectCheckpoint(c);await choose(c,'rest');
    await page.getByRole('combobox',{name:'Seed',exact:true}).selectOption('42');assertBaseline(await readDom(),checkpoint('42',6));
    await choose(checkpoint('42',6),'train_wit');
    await page.getByRole('combobox',{name:'Checkpoint',exact:true}).selectOption('0');assertBaseline(await readDom(),checkpoint('42',0));
  });
  await check('seven numeric shortcuts map to recorded actions and respect focus/modifiers',async()=>{
    const c=checkpoint('7',3);await selectCheckpoint(c);
    await page.locator('h1').click();
    for(let i=0;i<7;i++){
      await page.keyboard.press(String(i+1));assertOutcome(await readDom(),c,c.outcomes.find(x=>x.actionId===selectedChoices(c)[i].id));
    }
    const before=await readDom();
    for(const key of ['Control+1','Meta+2','Alt+3','Shift+4','8','0']){await page.keyboard.press(key);eq((await readDom()).action,before.action,'ignored '+key);}
    await choose(c,'train_speed');
    await page.getByRole('combobox',{name:'Seed',exact:true}).focus();
    await page.keyboard.press('7');eq((await readDom()).action,'train_speed','numeric shortcut ignores focused seed select without changing its selected value');
    await page.keyboard.press('Escape');
    await page.getByRole('combobox',{name:'Checkpoint',exact:true}).focus();
    await page.keyboard.press('2');eq((await readDom()).action,'train_speed','numeric shortcut ignores focused checkpoint select');
    await page.keyboard.press('Escape');
    await page.locator('h1').click();
    await page.keyboard.down('1');const once=await readDom();await page.keyboard.down('1');
    eq((await readDom()).announcement,once.announcement,'repeat keydown does not replay again');await page.keyboard.up('1');
  });
  await check('keyboard skip link and native response disclosure work',async()=>{
    await page.reload();await page.getByRole('heading',{name:'Compare a training turn',exact:true}).waitFor();
    await page.keyboard.press('Tab');eq(await page.evaluate(()=>document.activeElement.textContent),'Skip to choices');
    await page.keyboard.press('Enter');eq(new URL(page.url()).hash,'#recorded-choices');
    await page.keyboard.press('Tab');eq(await page.evaluate(()=>document.activeElement.title),'Shortcut 1');
    await page.keyboard.press('Enter');eq((await readDom()).action,'train_speed');
    const details=page.locator('.native-response');eq(await details.getAttribute('open'),'');
    await details.locator('summary').click();eq(await page.getByTestId('native-response').isVisible(),false);
    await details.locator('summary').click();eq(await page.getByTestId('native-response').isVisible(),true);
    const c=checkpoint('7',0);assertOutcome(await readDom(),c,c.outcomes.find(x=>x.actionId==='train_speed'));
  });
  await check('desktop layout and final rendered screenshot',async()=>{
    const c=checkpoint('1',3);await selectCheckpoint(c);await choose(c,'rest');
    const dom=await readDom();ok(dom.overflow.scroll<=dom.overflow.viewport,'desktop has no document overflow');
    const file=path.join(args.out,'desktop.png');await page.screenshot({path:file,fullPage:true});screenshots.push(file);
  });
  await check('reload discards comparisons and uses no application storage',async()=>{
    const storage=await page.evaluate(()=>({local:Object.keys(localStorage),session:Object.keys(sessionStorage)}));eq(storage,{local:[],session:[]});
    eq(await context.cookies(),[]);await page.goto(url);assertBaseline(await readDom(),checkpoint('7',0));
  });
  await context.close();context=null;
  await check('mobile 320px offline receiving includes low energy and injuries',async()=>{
    context=await browser.newContext({viewport:{width:320,height:780},locale:'en-US',isMobile:true,hasTouch:true});
    await context.setOffline(true);page=await setup(context);await page.getByRole('heading',{name:'Compare a training turn',exact:true}).waitFor();
    assertBaseline(await readDom(),checkpoint('7',0));
    for(const c of fixture.checkpoints.filter(x=>x.trainingActionsBefore===6)){
      await selectCheckpoint(c);assertBaseline(await readDom(),c);
      for(const action of ['train_speed','rest','recreation']){
        const dom=await choose(c,action);assertOutcome(dom,c,c.outcomes.find(x=>x.actionId===action));
        ok(dom.overflow.scroll<=dom.overflow.viewport,c.id+'/'+action+' no mobile document overflow');
      }
    }
    const c=fixture.checkpoints.find(x=>x.snapshot.state.statuses.length)||checkpoint('7',6);
    await selectCheckpoint(c);await choose(c,'rest');
    const file=path.join(args.out,'mobile.png');await page.screenshot({path:file,fullPage:true});screenshots.push(file);
    await context.close();context=null;
  });
  await check('JavaScript-disabled page explains the bounded offline demo',async()=>{
    context=await browser.newContext({javaScriptEnabled:false,viewport:{width:800,height:600}});page=await setup(context);
    ok(await page.getByRole('heading',{name:'Training turn · uma-sim',exact:true}).isVisible());
    const text=await page.locator('body').innerText();ok(text.includes('JavaScript is required for the seed, checkpoint, and action controls.'));
    eq(await page.getByRole('combobox').count(),0);eq(await page.getByRole('button').count(),0);
    await context.close();context=null;
  });
  await check('no external requests, runtime exceptions, or source mutation',async()=>{
    eq(blocked,[]);eq(requests.filter(r=>!r.url.startsWith('file:')),[]);eq(pageErrors,[]);
    eq(hash(fs.readFileSync(args.page)),hash(pageBytes));eq(hash(fs.readFileSync(args.fixture)),hash(fixtureBytes));
  });
}catch(e){fatal=e.stack||e.message;}
finally{if(context)await context.close().catch(()=>{});if(browser)await browser.close().catch(()=>{});}
const receipt={schema:'hamon.uma_training.browser_receiving.v1',at:new Date().toISOString(),node:process.version,
  page:{path:args.page,bytes:pageBytes.length,sha256:hash(pageBytes)},fixture:{path:args.fixture,bytes:fixtureBytes.length,sha256:hash(fixtureBytes),source:fixture.source},
  harness_sha256:hash(fs.readFileSync(fileURLToPath(import.meta.url))),browser:{executable:args.chromium,version:browserVersion,playwright:args.playwright,tmp:args.tmp},
  finite_native_scope:{seeds:fixture.seeds,checkpoints:9,outcomes:63},assertions,groups:checks.length,passed:checks.filter(x=>x.passed).length,failed:checks.filter(x=>!x.passed).length,
  checks,page_errors:pageErrors,requests,blocked_requests:blocked,screenshots:screenshots.map(file=>({path:file,bytes:fs.statSync(file).size,sha256:hash(fs.readFileSync(file))})),fatal,
  simulator_replay_owned_by_independent_engine_receiver:true,live_deployment:false};
fs.writeFileSync(path.join(args.out,'receipt.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});
process.stdout.write(JSON.stringify({receipt:path.join(args.out,'receipt.json'),page_sha256:receipt.page.sha256,assertions,groups:receipt.groups,passed:receipt.passed,failed:receipt.failed,fatal,failures:checks.filter(x=>!x.passed)})+'\n');
process.exitCode=receipt.failed||fatal?1:0;

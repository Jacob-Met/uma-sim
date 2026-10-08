import subprocess, pathlib, json, hashlib, datetime, os, time
R=pathlib.Path('/tmp/hamon-product-d98d06fb-20261008')
q=R/'receiving-copy-v2'; ui=q/'packages/uma-sim-ui'; e=R/'evidence'
sha=lambda b: hashlib.sha256(b).hexdigest()
def now(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def run(label,argv,cwd,env=None):
    start=now(); t=time.monotonic()
    p=subprocess.run(argv,cwd=cwd,env=env,capture_output=True)
    (e/(label+'.stdout.txt')).write_bytes(p.stdout)
    (e/(label+'.stderr.txt')).write_bytes(p.stderr)
    result={'start':start,'end':now(),'elapsed_seconds':time.monotonic()-t,'argv':argv,'cwd':str(cwd),'returncode':p.returncode,'stdout_sha256':sha(p.stdout),'stderr_sha256':sha(p.stderr)}
    (e/(label+'.execution.json')).write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result),flush=True)
    assert p.returncode==0, label
    return result
env=dict(os.environ); env['PATH']='/opt/homebrew/bin:'+env.get('PATH','')
run('copy-refinement-build',['/opt/homebrew/bin/npm','run','build'],ui,env)
tracked=ui/'tsconfig.tsbuildinfo'
base=subprocess.check_output(['git','show','HEAD:packages/uma-sim-ui/tsconfig.tsbuildinfo'],cwd=q)
before=tracked.read_bytes()
(e/'copy-refinement-generated-build-info.json').write_text(json.dumps({'path':str(tracked),'generated_sha256':sha(before),'head_sha256':sha(base),'action':'Restore only own build-generated tracked file'},indent=2)+'\n')
tracked.write_bytes(base)
changes=subprocess.check_output(['git','diff','--name-only'],cwd=q).decode().splitlines()
assert changes==['packages/uma-sim-ui/src/components/SkillsPanel.tsx'],changes
dist=ui/'dist'
files={str(p.relative_to(dist)):{'bytes':p.stat().st_size,'sha256':sha(p.read_bytes())} for p in sorted(dist.rglob('*')) if p.is_file()}
js='\n'.join(p.read_text() for p in dist.rglob('*.js'))
assert 'Name unavailable' in js and 'Name not in catalog' not in js
(e/'copy-refinement-built-ui.json').write_text(json.dumps({'built_at':now(),'source_path':str(q),'source_delta':'One text node only','files':files,'expected_text_present':True,'old_text_absent':True},indent=2)+'\n')
source=(q/'packages/uma-sim-ui/tests/receiving/skills-hints-receiver.mjs').read_text()
source=source.replace('import { chromium } from "playwright";','import { chromium } from "/tmp/hamon-product-d98d06fb-20261008/uma-sim/packages/uma-sim-ui/node_modules/playwright/index.mjs";')
source=source.replace('const repo = path.resolve(here, "../../../..");','const repo = "/tmp/hamon-product-d98d06fb-20261008/receiving-copy-v2";')
source=source.replace('let catalogMode = "native";','let catalogMode = "html";')
a=source.index('  if (!absent) {\n    const before')
b=source.index('  server = http.createServer',a)
source=source[:a]+source[b:]
a=source.index('  if (absent) {\n    assert.equal(await panel.count()')
b=source.index('  receipt.passed = true;',a)
focused='''  await panel.waitFor();
  await panel.getByRole("button", { name: "Retry names", exact: true }).waitFor();
  const expectedRows = snapshot.state.learnedSkillIds.length + Object.keys(snapshot.state.hintLevels).length;
  assert.equal(await panel.getByText("Name unavailable", { exact: true }).count(), expectedRows);
  assert.equal(await panel.getByText("Name not in catalog", { exact: true }).count(), 0);
  await panel.getByText(snapshot.state.learnedSkillIds[0], { exact: true }).waitFor();
  const before = await native("/v1/run/state");
  await panel.screenshot({ path: path.join(output, "name-unavailable-built-display.png") });
  const after = await native("/v1/run/state");
  assert.equal(after.text, before.text);
  assert.deepEqual(receipt.pageErrors, []);
  pass("Built application shows accurate name-unavailable row text during catalog failure", {
    expectedRows, actualText: "Name unavailable", oldTextAbsent: true,
    stateSha256: sha(before.text), globalStatusProvidesFailureDetail: true,
    scope: "One copy-refinement display control; no broader behavioral suite rerun"
  });
'''
source=source[:a]+focused+source[b:]
h=e/'copy-display-receiver.mjs'; h.write_text(source)
binary=R/'target/debug/uma-sim-api'
assert sha(binary.read_bytes())=='478a358f6d8687a3f69fd968b1ae26d829d48e8b5797bd866b352a6bd50aeeb1'
run('copy-refinement-display',['/opt/homebrew/bin/node',str(h),str(dist),str(e/'browser-copy-refinement-v2'),str(binary),'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'],q,env)
assert subprocess.check_output(['git','diff','--name-only'],cwd=q).decode().splitlines()==changes
print(json.dumps({'copy_refinement_qualified':True,'harness_sha256':sha(h.read_bytes()),'source_change_count':1}),flush=True)

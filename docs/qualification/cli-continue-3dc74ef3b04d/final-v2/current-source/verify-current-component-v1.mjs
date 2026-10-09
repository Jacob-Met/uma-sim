import fs from 'node:fs';
import crypto from 'node:crypto';
const root = '/home/jacob/uma-continue-3dc74ef3b04d-proof';
const manifestPath = '/home/jacob/uma-continue-3dc74ef3b04d-intake/component-source-manifest.json';
const treePath = root + '/current-main-tree-ac4a4c2.json';
const digest = b => crypto.createHash('sha256').update(b).digest('hex');
const assert = (v, m) => { if (!v) throw new Error(m); };
const mb = fs.readFileSync(manifestPath), tb = fs.readFileSync(treePath);
assert(digest(mb) === '196bcda1792f2e5d05a4e6f9ad7eb59cd49288dcebf0597fdb8a46df1d768520', 'component manifest pin');
const m = JSON.parse(mb), t = JSON.parse(tb);
assert(t.sha === 'db51a8dbbfc4a193929eef5b0ddcee19965f6b15' && t.commit === 'ac4a4c2d4b4159bea1012e66cb79e76b6bc25d56' && t.truncated === false, 'current source identity');
const byPath = new Map(t.tree.map(e => [e.path, e]));
assert(byPath.size === t.tree.length, 'unique tree paths');
const children = new Map([['', []]]);
for (const e of t.tree) {
  const slash = e.path.lastIndexOf('/'), parent = slash < 0 ? '' : e.path.slice(0, slash), name = e.path.slice(slash + 1);
  assert(parent === '' || byPath.get(parent)?.type === 'tree', 'represented parent ' + e.path);
  if (!children.has(parent)) children.set(parent, []);
  children.get(parent).push({...e, name});
  if (e.type === 'tree' && !children.has(e.path)) children.set(e.path, []);
}
const treeChecks = [];
for (const [path, entries] of children) {
  entries.sort((a,b) => Buffer.compare(Buffer.from(a.name + (a.type === 'tree' ? '/' : '')), Buffer.from(b.name + (b.type === 'tree' ? '/' : ''))));
  const body = Buffer.concat(entries.map(e => Buffer.concat([Buffer.from(e.mode.replace(/^0/, '') + ' ' + e.name + '\0'), Buffer.from(e.sha, 'hex')])));
  const sha = crypto.createHash('sha1').update(Buffer.from('tree ' + body.length + '\0')).update(body).digest('hex');
  const expected = path === '' ? t.sha : byPath.get(path).sha;
  assert(sha === expected, 'Git tree hash ' + path);
  treeChecks.push({path, sha, children:entries.length});
}
const differences = m.files.flatMap(f => {
  const e = byPath.get(f.path);
  return e && e.sha === f.git_blob && e.mode === f.mode ? [] : [{path:f.path, dependency_blob:f.git_blob, dependency_mode:f.mode, current_blob:e?.sha ?? null, current_mode:e?.mode ?? null}];
});
assert(m.files.length === 302 && differences.length === 4, 'component comparison count');
assert(JSON.stringify(differences.map(d=>d.path)) === JSON.stringify(['README.md','uma-sim-core/src/bin/uma-sim.rs','uma-sim-core/src/session.rs','uma-sim-core/tests/cli_session_persistence.rs']), 'only known writer dependency, caller guards, test, README differ');
const manifestPaths = new Set(m.files.map(f=>f.path));
const runtimeRoots = ['uma-sim-core/','uma-race-core/','knowledge/','research/'];
const extras = t.tree.filter(e=>e.type !== 'tree' && runtimeRoots.some(r=>e.path.startsWith(r)) && !manifestPaths.has(e.path));
assert(extras.length === 0, 'full current runtime/data closure represented');
const newPaths = ['uma-sim-core/src/bin/uma_sim/continue_run.rs','uma-sim-core/tests/cli_continue.rs','docs/CLI_CONTINUE.md'];
assert(newPaths.every(p=>!byPath.has(p)), 'new paths absent on current main');
const receipt = {
  schema:'hamon.uma_continue.root_current_source_compatibility.v1', at:new Date().toISOString(), status:'source_component_compatible_with_explicit_pr118_dependency',
  repository:t.repository,current_commit:t.commit,current_tree:t.sha,source_tree_entries:t.tree.length,reconstructed_git_trees:treeChecks.length,
  dependency_commit:m.dependency_commit,dependency_tree:m.dependency_tree,component_files:m.files.length,exact_component_matches:m.files.length-differences.length,differences,
  runtime_data_roots:runtimeRoots,additional_current_runtime_or_data_leaves:extras,new_paths_absent:newPaths,
  inputs:[{path:manifestPath,bytes:mb.length,sha256:digest(mb)},{path:treePath,bytes:tb.length,sha256:digest(tb)},{path:new URL(import.meta.url).pathname,sha256:digest(fs.readFileSync(new URL(import.meta.url)))}],
  tree_checks:treeChecks,
  disposition:['PR118 atomic session writer and its four existing CLI error exits remain an explicit unmerged dependency.','Preserve current README and all current unrelated source/documents when composing.','The source match permits reuse of exact compiled dependency evidence; it is not a new whole-repository build, repository merge, installed adoption or acceptance of the pending local-policy repair.','Private55e3 setup/resume/batch and d98 target inspection require independent shared-CLI composition; their private native deltas were not included in this canonical-main comparison.']
};
const out = root + '/ROOT-CURRENT-SOURCE-COMPATIBILITY-v1.json';
fs.writeFileSync(out, JSON.stringify(receipt,null,2) + '\n', {flag:'wx'});
console.log(JSON.stringify({status:receipt.status,path:out,sha256:digest(fs.readFileSync(out)),component_files:302,exact_matches:298,reconstructed_git_trees:treeChecks.length,additional_runtime_leaves:0}));

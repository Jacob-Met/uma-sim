import assert from 'node:assert/strict';
import fs from 'node:fs';
import crypto from 'node:crypto';
const root='D:/Hamon/worktrees/uma-sim-discovery-0378a7b6';
const out='D:/Hamon/worktrees/uma-sim-career-controls-0378a7b6-proof';
process.env.UMA_SIM_TEST_REPO_ROOT=root;
process.env.UMA_SIM_TEST_OUTPUT_DIR=out+'/baseline';
process.env.UMA_SIM_TEST_API_BIN=out+'/target/debug/uma-sim-api.exe';
process.env.UMA_SIM_TEST_RECEIVING_COMMIT='0ad4bd4d2ca0a133fe91080b6b7818efbaedb2fa';
const {actualServer}=await import('file:///'+root+'/packages/uma-sim-mcp/tests/helpers/native-api.mjs');
const {paramsError}=await import('file:///'+root+'/packages/uma-sim-mcp/tests/helpers/mcp-client.mjs');
const cleanup=[];const t={after:f=>cleanup.push(f)};
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const receipt={at:new Date().toISOString(),source:process.env.UMA_SIM_TEST_RECEIVING_COMMIT,binarySha256:sha(fs.readFileSync(process.env.UMA_SIM_TEST_API_BIN)),mcpSha256:sha(fs.readFileSync(root+'/packages/uma-sim-mcp/mcp-stdio.js')),checks:[],catalogs:[],outcome:'started'};
try{
const {client,direct,state,cwd}=await actualServer(t);receipt.cwd=cwd;
const listing=await client.request('tools/list');const names=listing.result.tools.map(t=>t.name);
assert(!names.includes('sim_catalog')&&!names.includes('sim_style'));
paramsError(await client.call('sim_catalog',{kind:'trainees'}));
paramsError(await client.call('sim_style',{style:'front'}));
receipt.checks.push('Both intended tools absent from actual stdio discovery and actual calls refused');
const sessionsBefore=(await direct('GET','/v1/sessions')).data;
for(const kind of ['scenarios','trainees','supports','factors','skills']){const r=await direct('GET','/v1/catalog/'+kind);assert.equal(r.status,200,r.text);assert(Array.isArray(r.data.items));receipt.catalogs.push({kind,count:r.data.items.length,sha256:sha(Buffer.from(r.text))});}
assert.deepEqual((await direct('GET','/v1/sessions')).data,sessionsBefore);
receipt.checks.push('All five existing native catalog routes return item lists without creating a session');
assert.equal((await direct('POST','/v1/run/start',{seed:42,session:'source',raceModel:'stub'})).status,200);
const original=await state('source');assert.equal((await direct('POST','/v1/session/fork',{session:'source',id:'strategy'})).status,200);
const styled=await direct('POST','/v1/run/style',{session:'strategy',style:'front'});assert.equal(styled.status,200,styled.text);
const expected=structuredClone(original);expected.state.preferredRunningStyle='front';assert.deepEqual(styled.data,expected);assert.deepEqual(await state('source'),original);
paramsError(await client.call('sim_style',{session:'strategy',style:'pace'}));assert.deepEqual(await state('strategy'),expected);
receipt.checks.push('Existing native style endpoint changes only fork preferredRunningStyle; missing MCP control cannot perform that action');
receipt.outcome='pass';
}catch(error){receipt.outcome='fail';receipt.error=String(error.stack??error);process.exitCode=1;}finally{for(const f of cleanup.reverse())await f();fs.writeFileSync(out+'/baseline-receipt.json',JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify(receipt));}

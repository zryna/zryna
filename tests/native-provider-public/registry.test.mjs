import assert from 'node:assert/strict';
import test from 'node:test';
import {mkdtempSync,mkdirSync,copyFileSync,writeFileSync,readFileSync,rmSync,symlinkSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname,join} from 'node:path';
import {loadRegistry,validateRegistry,repositoryRoot,acceptanceStatus,sha256} from './registry.mjs';
import {bundleInventory} from './cli-parity.mjs';

const registry=loadRegistry();
function clone(){return structuredClone(registry);}
function temporary(t){const path=mkdtempSync(join(tmpdir(),'zryna-public-native-registry-'));t.after(()=>rmSync(path,{recursive:true,force:true}));return path;}
function retainedFixtureRoot(t){const root=temporary(t);for(const p of [...registry.registries,...registry.syntaxCorpusAuthority,registry.m3BuildOracle,registry.m1BuildOracle,...JSON.parse(readFileSync(join(repositoryRoot,registry.m1BuildOracle.path))).owners,registry.currentM3BuildAuthority.owner,...registry.currentM3BuildAuthority.cases.map(c=>c.snapshot),...registry.sources.map(c=>c.source)]){const target=join(root,p.path);mkdirSync(dirname(target),{recursive:true});copyFileSync(join(repositoryRoot,p.path),target);}return root;}

test('frozen current sources include97 M3 roots and genuine unchanged69-root oracle',()=>{
 assert.equal(registry.sources.length,114);assert.equal(registry.sources.filter(c=>c.protocol===4).length,97);
 assert.equal(registry.cliBuildCases.filter(c=>c.profile==='data-ownership-v1'&&c.disposition==='accepted').length,35);
 assert.equal(registry.cliBuildCases.filter(c=>c.profile==='data-ownership-v1'&&c.disposition==='rejected').length,36);
 assert.equal(registry.historicalH4.blockedCaseIds.length,30);
});
test('missing, duplicated and extra source identities cannot shrink the corpus',()=>{
 for(const mutate of [r=>r.sources.pop(),r=>{r.sources[1]=r.sources[0];},r=>r.sources.push(r.sources[0])]){const changed=clone();mutate(changed);assert.throws(()=>validateRegistry(changed));}
});
test('whole source byte changes fail before executing a compiler',t=>{
 const root=retainedFixtureRoot(t),pin=registry.sources[0].source;writeFileSync(join(root,pin.path),Buffer.concat([readFileSync(join(root,pin.path)),Buffer.from('\n')]));assert.throws(()=>validateRegistry(registry,root),/assert|input|size|false/i);
});
test('unregistered source fixtures fail complete physical census',t=>{
 const root=retainedFixtureRoot(t);writeFileSync(join(root,'tests/m3-fixtures/unregistered.zry'),'');assert.throws(()=>validateRegistry(registry,root));
});
test('coherently rehashed alteredM3 oracle cannot replace independent frozen decisions',t=>{
 const root=retainedFixtureRoot(t),r=clone(),path=join(root,r.m3BuildOracle.path),oracle=JSON.parse(readFileSync(path));oracle.fixtures[0].disposition='accepted';const bytes=Buffer.from(JSON.stringify(oracle));writeFileSync(path,bytes);r.m3BuildOracle.bytes=bytes.length;r.m3BuildOracle.sha256=sha256(bytes);assert.throws(()=>validateRegistry(r,root));
});
test('not-run required lanes remain incomplete and omitted lanes reject',()=>{
 const lanes=Object.fromEntries(registry.requiredAcceptanceLanes.map(id=>[id,{status:'not-run'}]));assert.equal(acceptanceStatus(registry,lanes),'incomplete');delete lanes['ordinary-installed-no-node'];assert.throws(()=>acceptanceStatus(registry,lanes));
});
test('zero-case or skipped lanes cannot mint complete acceptance',()=>{
 for(const entry of [{status:'passed',executed:0,failed:0,skipped:0},{status:'passed',executed:1,failed:0,skipped:1},{status:'passed',executed:1,failed:1,skipped:0}]){const lanes=Object.fromEntries(registry.requiredAcceptanceLanes.map(id=>[id,entry]));assert.throws(()=>acceptanceStatus(registry,lanes));}
});
test('complete bundle inventory includes nested bytes and rejects links',t=>{
 const root=temporary(t);mkdirSync(join(root,'nested'));writeFileSync(join(root,'nested','artifact.wasm'),Buffer.from([0,97,115,109]));writeFileSync(join(root,'manifest.json'),'{}');assert.deepEqual(bundleInventory(root).map(p=>p.path),['manifest.json','nested/artifact.wasm']);
 if(process.platform!=='win32'){symlinkSync('manifest.json',join(root,'alias'));assert.throws(()=>bundleInventory(root));}
});

test('current clone-prefix BUILD decisions retain separate original owner authority',()=>{
 assert.equal(registry.cliBuildCases.length,88);assert.equal(registry.currentM3BuildAuthority.cases.length,2);
 const changed=clone();changed.cliBuildCases.find(c=>c.id==='m3-current-build:clone-prefix-aggregate').disposition='rejected';assert.throws(()=>validateRegistry(changed));
});

test('current M1 oracle preserves original owner multiplicity and all diagnostic fields',()=>{
 const row=registry.cliBuildCases.find(c=>c.id==='m1-build:gatedBooleanSource');assert.deepEqual(row.diagnosticCodes,['ZRYNA-I1006','ZRYNA-I1006','ZRYNA-I1006']);assert.equal(row.diagnostics[2].primary.span.start,53);assert.equal(row.diagnostics[2].primary.span.end,58);
 for(const mutate of [r=>r.cliBuildCases.find(c=>c.id===row.id).diagnosticCodes.pop(),r=>r.cliBuildCases.find(c=>c.id===row.id).diagnostics[0].message='different owner diagnostic']){const changed=clone();mutate(changed);assert.throws(()=>validateRegistry(changed));}
});

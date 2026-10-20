import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync,lstatSync,realpathSync,readdirSync} from 'node:fs';
import {resolve,relative,isAbsolute,sep} from 'node:path';
import {fileURLToPath} from 'node:url';

export const repositoryRoot=resolve(fileURLToPath(new URL('../../',import.meta.url)));
export const registrySha256='1bddc89817fad90a5be64968833ddecfc7d0c8cdec69812a5c111c8087abf291';
export const sha256=bytes=>createHash('sha256').update(bytes).digest('hex');
export const canonical=value=>Buffer.from(JSON.stringify(sort(value))+'\n');
function sort(value){return Array.isArray(value)?value.map(sort):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(k=>[k,sort(value[k])])):value;}
export function exact(value,keys,label){assert(value&&typeof value==='object'&&!Array.isArray(value),label);assert.deepEqual(Object.keys(value).sort(),[...keys].sort(),label);}
export function portable(path){assert(typeof path==='string'&&path.length<=1024&&!path.includes('\\')&&!isAbsolute(path)&&path.split('/').every(p=>p&&!['.','..'].includes(p)),'portable relative path');return path;}
export function readPinned(root,pin){
 exact(pin,['path','bytes','sha256'],'input pin');portable(pin.path);assert(Number.isSafeInteger(pin.bytes)&&pin.bytes>0&&pin.bytes<=2*1024*1024&&/^[a-f0-9]{64}$/.test(pin.sha256));
 const path=resolve(root,pin.path);assert(!relative(root,path).startsWith('..'+sep)&&realpathSync(path)===path,'ordinary input path');
 const before=lstatSync(path);assert(before.isFile()&&before.nlink===1&&!before.isSymbolicLink()&&before.size===pin.bytes);
 const bytes=readFileSync(path),after=lstatSync(path);for(const k of ['dev','ino','mode','size','mtimeMs','ctimeMs'])assert.equal(after[k],before[k],'stable input');
 assert.equal(bytes.length,pin.bytes);assert.equal(sha256(bytes),pin.sha256,'frozen input hash');return bytes;
}
function sourceCensus(root,path){
 const result=[];for(const entry of readdirSync(resolve(root,path),{withFileTypes:true})){assert(!entry.isSymbolicLink(),'linked fixture');const p=path+'/'+entry.name;if(entry.isDirectory())result.push(...sourceCensus(root,p));else if(entry.name.endsWith('.zry')){assert(entry.isFile());result.push(p);}}
 return result.sort();
}
export function validateRegistry(registry,root=repositoryRoot){
 exact(registry,['format','version','sourceBaseline','counts','registries','syntaxCorpusAuthority','m3BuildOracle','m3OracleOrigin','currentM3BuildAuthority','historicalH4','additionalCurrentSources','sources','cliBuildCases','requiredAcceptanceLanes','supportedHosts','scope'],'registry');
 assert.equal(registry.format,'zryna.native-provider-public-registry.v1');assert.equal(registry.version,1);
 assert.deepEqual(registry.counts,{syntaxSources:114,m1Sources:3,m2Sources:14,m3Sources:97,cliBuildRoots:88,m3HistoricalAccepted:33,m3HistoricalRejected:36,m3CurrentAccepted:2});
 assert.deepEqual(registry.supportedHosts,['linux-x64','win32-x64']);
 assert.deepEqual(registry.requiredAcceptanceLanes,['worker-snapshot-parity','cli-build-parity','verified-ir-parity','diagnostic-boundaries','runtime-and-cleanup-parity','ordinary-installed-no-node','reproducible-installation','linux-windows-full-m0-m4-ci']);
 assert.equal(registry.registries.length,3);assert.equal(registry.syntaxCorpusAuthority.length,2);
 for(const pin of [...registry.registries,...registry.syntaxCorpusAuthority])readPinned(root,pin);
 const oracle=JSON.parse(readPinned(root,registry.m3BuildOracle));assert.equal(registry.m3BuildOracle.sha256,'1326226f6c1bfa8122564d6507d70df4c6acf75060fb1a650b02c36ed562155e');
 assert.deepEqual(oracle.counts,{accepted:33,rejected:36,diagnostics:67});assert.equal(oracle.fixtures.length,69);
 assert.equal(registry.sources.length,114);assert.equal(new Set(registry.sources.map(c=>c.id)).size,114);assert.equal(new Set(registry.sources.map(c=>c.source.path)).size,114);
 for(const [protocol,count]of [[2,3],[3,14],[4,97]])assert.equal(registry.sources.filter(c=>c.protocol===protocol).length,count);
 for(const item of registry.sources){exact(item,['id','protocol','profile','source','syntaxVerification'],'source case');assert([2,3,4].includes(item.protocol));assert.equal(item.profile,{2:'i32-v1',3:'control-flow-v1',4:'data-ownership-v1'}[item.protocol]);assert(['verify','known-hostile-Y4002'].includes(item.syntaxVerification));readPinned(root,item.source);}
 assert.deepEqual(registry.sources.filter(c=>c.protocol===4).map(c=>c.source.path).sort(),sourceCensus(root,'tests/m3-fixtures'));
 assert.deepEqual(registry.sources.filter(c=>c.protocol===3).map(c=>c.source.path).sort(),sourceCensus(root,'tests/m2-fixtures'));
 assert.equal(registry.sources.filter(c=>c.syntaxVerification==='known-hostile-Y4002').length,1);
 assert.equal(registry.cliBuildCases.length,88);assert.equal(new Set(registry.cliBuildCases.map(c=>c.id)).size,88);
 for(const item of registry.cliBuildCases){assert(registry.sources.some(s=>s.source.path===item.entrypoint));assert(['accepted','rejected'].includes(item.disposition));assert(Array.isArray(item.diagnosticCodes));assert(item.disposition==='accepted'?item.diagnosticCodes.length===0:item.diagnosticCodes.length>0);}
 for(const fixture of oracle.fixtures){const row=registry.cliBuildCases.find(c=>c.profile==='data-ownership-v1'&&c.entrypoint===fixture.source);assert(row);assert.equal(row.disposition,fixture.disposition);assert.deepEqual(row.diagnostics,fixture.diagnostics);for(const source of fixture.sources){const pin=registry.sources.find(c=>c.source.path===source.path)?.source;assert(pin&&pin.sha256===source.sha256,'historical oracle input is unchanged');}}
 const current=registry.currentM3BuildAuthority;exact(current,['owner','cases','rationale'],'current independent BUILD authority');readPinned(root,current.owner);assert.equal(current.owner.sha256,'9f10187089547fd45f54c1fee713c8ec6ff67a59bf5b3609b3c4ec31018193f2');assert.equal(current.cases.length,2);
 for(const [index,kind]of ['aggregate','vector'].entries()){const authority=current.cases[index];exact(authority,['buildCase','test','snapshot'],'current clone-prefix authority');assert.equal(authority.test,'ordinary_'+kind+'_clone_uses_actual_'+kind+'_prefix_before_source_cleanup');readPinned(root,authority.snapshot);assert.equal(authority.buildCase.id,'m3-current-build:clone-prefix-'+kind);assert.equal(authority.buildCase.entrypoint,'tests/m3-fixtures/clone-prefix-'+kind+'.zry');assert.equal(authority.buildCase.disposition,'accepted');assert.deepEqual(authority.buildCase.diagnostics,[]);assert.deepEqual(authority.buildCase.diagnosticCodes,[]);assert.deepEqual(registry.cliBuildCases.find(c=>c.id===authority.buildCase.id),authority.buildCase);}
 assert.equal(registry.historicalH4.blockedCaseIds.length,30);assert.equal(new Set(registry.historicalH4.blockedCaseIds).size,30);assert.equal(registry.historicalH4.total,220);assert.equal(registry.historicalH4.passed,190);
 return registry;
}
export function loadRegistry(root=repositoryRoot){const raw=readFileSync(resolve(root,'tests/native-provider-public/registry.json'));assert.equal(sha256(raw),registrySha256,'closed registry bytes');return validateRegistry(JSON.parse(raw),root);}

// A scoped runner cannot claim issue closure by omitting unexecuted lanes.
export function acceptanceStatus(registry,lanes){
 assert.deepEqual(Object.keys(lanes).sort(),[...registry.requiredAcceptanceLanes].sort(),'complete required lane census');
 for(const result of Object.values(lanes)){assert(result&&['passed','failed','not-run'].includes(result.status));if(result.status==='passed')assert(Number.isSafeInteger(result.executed)&&result.executed>0&&result.failed===0&&result.skipped===0,'genuine nonempty passing lane');}
 return Object.values(lanes).every(r=>r.status==='passed')?'complete':'incomplete';
}

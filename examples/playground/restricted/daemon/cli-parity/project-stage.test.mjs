// Inert host-fs fixtures map the fixed namespace and synthetic UID/tmpfs metadata; no runtime credit.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { findOperation } from './plan.mjs';
const { stage } = createRequire(import.meta.url)('./project-stage.cjs');
const request = () => { const op = findOperation('build-addition'); return { version: 1, operationId: op.id, files: structuredClone(op.projectFiles) }; };
function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'cli-stage-fixture-')); t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const target = value => value === '/work/project' ? root : value.replace('/work/project/', root + '/');
  const metadata = value => Object.assign(Object.create(Object.getPrototypeOf(value)), value, { uid: 65534, gid: 65534 });
  const api = { constants: fs.constants, statfsSync: () => ({ type: 0x01021994 }),
    lstatSync: p => metadata(fs.lstatSync(target(p))), fstatSync: fd => metadata(fs.fstatSync(fd)),
    readdirSync: p => fs.readdirSync(target(p)), mkdirSync: (p,o) => fs.mkdirSync(target(p),o),
    openSync: (p,f,m) => fs.openSync(target(p),f,m), closeSync: fs.closeSync,
    readSync: fs.readSync, writeSync: fs.writeSync };
  return { root, api };
}
test('trusted staging writes only three opaque source files with complete hash/mode readback and handles short writes', t => {
  const { root, api } = fixture(t); const write = api.writeSync;
  api.writeSync = (fd,b,o,n) => write(fd,b,o,Math.min(n,5));
  const result = stage(request(), api);
  assert.equal(result.before.empty, true); assert.equal(result.sourceStaged, true); assert.equal(result.after.files.length, 3);
  for (const file of request().files) { assert.deepEqual(fs.readFileSync(path.join(root,file.path)), Buffer.from(file.bodyBase64,'base64')); assert.equal(fs.statSync(path.join(root,file.path)).mode & 0o7777,0o600); }
  assert.equal(fs.statSync(path.join(root,'src')).mode & 0o7777,0o700);
});
test('source changes, selectors and duplicate inventory are rejected before filesystem effects', () => {
  for (const change of [r => r.files[0].bodyBase64='ZXZpbA==',r => r.files[0].path='../outside',r => r.files.push(r.files[0]),r => r.extra='selector']) {
    const value=request();change(value);assert.throws(() => stage(value, new Proxy({}, { get(){throw new Error('unexpected filesystem effect');} })), /CLI-PROJECT-STAGE-REJECTED/);
  }
});
test('actual-root conditions remain mandatory: non-tmpfs, root identity and occupied roots fail', t => {
  for (const kind of ['filesystem','uid','mode','occupied']) {
    const { root, api }=fixture(t);
    if(kind==='filesystem') api.statfsSync=()=>({type:0x794c7630});
    if(kind==='uid') { const stat=api.lstatSync;api.lstatSync=p=>Object.assign(stat(p),{uid:0}); }
    if(kind==='mode') fs.chmodSync(root,0o555);
    if(kind==='occupied') fs.writeFileSync(path.join(root,'sentinel'),'original');
    assert.throws(()=>stage(request(),api),/CLI-PROJECT-STAGE-REJECTED/);
    assert.equal(fs.existsSync(path.join(root,'src')),false);
  }
});
test('O_EXCL and exact inventory prevent symlink overwrite and hidden staged files', t => {
  for(const kind of ['symlink','hidden']) {
    const {root,api}=fixture(t);const mkdir=api.mkdirSync;
    api.mkdirSync=(p,o)=>{mkdir(p,o);if(kind==='symlink')fs.symlinkSync(path.join(root,'sentinel'),path.join(root,'zryna.lock.json'));else fs.writeFileSync(path.join(root,'hidden'),'unexpected');};
    fs.writeFileSync(path.join(root,'sentinel'),'original');
    // Keep the root empty admission proof, then introduce the hostile fixture during the first mkdir.
    const list=api.readdirSync;let first=true;api.readdirSync=p=>{if(first){first=false;return [];}return list(p);};
    assert.throws(()=>stage(request(),api));assert.equal(fs.readFileSync(path.join(root,'sentinel'),'utf8'),'original');
  }
});

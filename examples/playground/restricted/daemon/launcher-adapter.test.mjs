import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createDaemonBridge } from './bridge.mjs';
import { createLauncherHost } from './launcher-adapter.mjs';
import { fixture, frame, sourceRequest, startUpload } from '../../../../tests/playground-fixtures/service.mjs';

const pin = path => ({ path, bytes: statSync(path).size, ownerUid: statSync(path).uid,
  sha256: createHash('sha256').update(readFileSync(path)).digest('hex') });
const supported = process.platform === 'linux' && process.arch === 'x64';

test('actual service and subprocess bridge adapt bounded observation JSON; undispatched failures retain reuse',
  { skip: !supported }, async t => {
    // This inert helper proves transport composition only, not compilation, isolation or browser execution.
    const root = mkdtempSync(join(tmpdir(), 'playground-source-adapter-'));
    const artifact = await frame(sourceRequest);
    const observed = { revision: sourceRequest.revision, logical: 'value',
      componentSha256: artifact.metadata.identity.componentSha256, value: 42 };
    const helper = join(root, 'inert-transport-helper');
    const script = `#!${process.execPath}\nlet input=Buffer.alloc(0);process.stdin.on('data',chunk=>{` +
      'input=Buffer.concat([input,chunk]);if(input.length<4)return;const size=input.readUInt32LE(0);' +
      'if(input.length<4+size)return;process.stdin.removeAllListeners("data");' +
      `const bytes=process.argv.includes('evaluate')?Buffer.from(${JSON.stringify(JSON.stringify(observed))}):` +
      `Buffer.from(${JSON.stringify(Buffer.from(artifact.bytes).toString('base64'))},'base64');` +
      'process.stdout.write(bytes,()=>process.exit(0));});\n';
    writeFileSync(helper, script, { mode: 0o700 });
    const approval = join(root, 'inert-approval.json'); writeFileSync(approval, '{}', { mode: 0o600 });
    const bridge = createDaemonBridge({ compiler: pin(helper), approval: pin(approval) });
    const host = createLauncherHost(bridge);
    const f = await fixture(t, host, { evaluationMode: 'supervised-host' });
    try {
      const malformed = await f.send('/compile', { ...sourceRequest, argv: ['forbidden'] });
      assert.equal(malformed.status, 400);
      assert.equal(bridge.idle(), true, 'malformed submission had no helper');
      const upload = await startUpload(t, f, '/compile', sourceRequest);
      assert.deepEqual((await f.send('/cancel', { cancelled: true })).json(), { finished: true });
      upload.complete();
      assert.equal((await upload.pending).status, 400);
      assert.equal(bridge.idle(), true, 'cancelled partial request had no helper');
      const compiled = await f.send('/compile', sourceRequest);
      assert.equal(compiled.status, 200);
      const compileToken = compiled.headers['x-zryna-job'];
      await assert.rejects(host.finish('f'.repeat(64), true), /JOB/);
      assert.equal(bridge.idle(), false, 'foreign token cannot acknowledge current owner');
      assert.equal((await f.send('/finish', { token: compileToken, cancelled: false })).status, 200);
      const selection = { revision: sourceRequest.revision, sourceSha256: artifact.metadata.sourceSha256,
        componentSha256: observed.componentSha256, logical: 'value', args: [] };
      const evaluated = await f.send('/evaluate', selection);
      assert.equal(evaluated.status, 200);
      assert.deepEqual(evaluated.json().observation, observed, 'real bridge parsed its actual process stdout');
      assert.equal((await f.send('/finish', { token: evaluated.json().token, cancelled: false })).status, 200);
      assert.equal(bridge.idle(), true);
    } finally {
      await host.close(); rmSync(root, { recursive: true });
    }
  });

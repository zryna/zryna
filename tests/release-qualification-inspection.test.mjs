import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { inspectReleaseQualification } from '../scripts/distribution-release/inspect-release-qualification.mjs';
import { createQualificationFixture } from './release-qualification-fixture.mjs';

test('inspects dependencies with one bound immutable native tool and rejects path leakage', async (t) => {
  const fixture = await createQualificationFixture();
  const parent = mkdtempSync(join(tmpdir(), 'zryna-qualification-inspection-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const sourceRoot = join(parent, 'source');
  const workRoot = join(parent, 'work');
  mkdirSync(sourceRoot);
  mkdirSync(workRoot);
  const path = join(parent, 'readelf');
  const executable = join(workRoot, 'zryna');
  writeFileSync(path, 'inspector');
  writeFileSync(executable, Buffer.alloc(64, 7));
  const record = { ...fixture.input.nativeTools[0], name: 'inspector', size: 9,
    sha256: sha256(Buffer.from('inspector')) };
  fixture.input.nativeTools.push(record);
  fixture.input.nativeTools.sort((left, right) => left.name.localeCompare(right.name));
  const binding = Buffer.from(`${canonicalBounded(fixture.input)}\n`);
  const cli = Buffer.alloc(64, 7);
  const observedTools = { nativeTools: fixture.input.nativeTools, paths: { inspector: path }, executable };
  const inspection = inspectReleaseQualification({ binding, cli, sourceRoot, workRoot, observedTools,
    spawn(command, args, options) {
      assert.equal(command, path);
      assert.deepEqual(args, ['-d', '--wide', executable]);
      assert.deepEqual(options.env, {});
      return { status: 0, signal: null,
        stdout: ' 0x0000000000000001 (NEEDED) Shared library: [libc.so.6]\n', stderr: '' };
    } });
  assert.deepEqual(JSON.parse(inspection).checks.dynamicDependencies, ['libc.so.6']);
  assert.throws(() => inspectReleaseQualification({ binding,
    cli: Buffer.concat([Buffer.alloc(64, 7), Buffer.from(sourceRoot)]), sourceRoot, workRoot,
    observedTools, spawn: () => ({ status: 0, signal: null,
      stdout: 'Shared library: [libc.so.6]\n', stderr: '' }) }), /contains a controlled root/);
});

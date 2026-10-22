// Offline source contracts only. No published CLI, provider or generated source executes here.
import assert from 'node:assert/strict';
import test from 'node:test';
import { createParityPlan, fixedCorpus, projectFiles, findOperation, parityPlanSha256, release } from './plan.mjs';
import { sha256 } from './records.mjs';
import { parseCanonical, digest } from '../../../../../scripts/package-release/canonical.mjs';

test('fixed 19 operation plan has exact scalar coverage and no arbitrary execution selector', () => {
  const plan = createParityPlan();
  assert.equal(plan.operations.length, 19);
  assert.equal(plan.operations.filter(value => value.kind === 'build').length, 9);
  assert.equal(plan.operations.filter(value => value.kind === 'run').length, 10);
  assert.equal(new Set(plan.operations.map(value => value.id)).size, 19);
  assert.equal(plan.executionApproved, false); assert.equal(plan.browserComponentProfileInOldCli, false);
  assert.equal(plan.fullInstalledInventoryRequired, true); assert.equal(plan.originalArchiveModesRequired, true);
  assert.equal(release.archive.files, 349);
  for (const item of plan.operations) {
    assert.equal(item.executable, '/opt/zryna-release/bin/zryna'); assert.equal(item.cwd, '/work/project');
    assert.deepEqual(item.argv.slice(0, 8), [item.kind, 'src/main.zry', '--target', 'javascript',
      '--project-root', '/work/project', '--name', 'parity']);
    assert.ok(!item.argv.some(value => ['--node', '--root', '--profile', '--grant-file', 'native', 'all'].includes(value)));
    assert.equal(item.argv.at(-1), '--json'); assert.equal(item.stdin, 'none');
    assert.equal(item.activeMs, 10000); assert.equal(item.cleanupMs, 2000);
    assert.equal(item.sourceSha256, sha256(Buffer.from(fixedCorpus.find(value => value.id === item.corpusId).source)));
    assert.ok(Object.isFrozen(item.argv));
  }
  assert.match(parityPlanSha256(), /^[a-f0-9]{64}$/);
  assert.throws(() => findOperation('run-../../arbitrary'), /CLI-PARITY/);
  assert.throws(() => projectFiles('export function attacker(): i32 { return 7; }'), /CLI-PARITY/);
  assert.deepEqual(plan.operations.filter(value => value.kind === 'run').map(value => value.expected.value),
    [42, -2147483648, 2147483647, 0, -5, 42, 0, 42, 42, 0]);
});
test('generated project manifest and frozen lock match the independent package-v1 canonical/domain contracts', () => {
  for (const entry of fixedCorpus) {
    const files = new Map(projectFiles(entry.source).map(file => [file.path, file]));
    assert.deepEqual([...files.keys()], ['src/main.zry', 'zryna.lock.json', 'zryna.package.json']);
    const bytes = name => Buffer.from(files.get(name).bodyBase64, 'base64');
    const manifest = parseCanonical(bytes('zryna.package.json')); const lock = parseCanonical(bytes('zryna.lock.json'));
    assert.deepEqual(manifest.source, { kind: 'local', locator: 'project', revision: '' });
    assert.deepEqual(manifest.compatibility, { compiler: '0.2.3', profile: 'i32-v1', targets: ['javascript'] });
    assert.deepEqual(manifest.dependencies, []); assert.equal(manifest.files[0].sha256, sha256(bytes('src/main.zry')));
    assert.equal(manifest.files[0].size, bytes('src/main.zry').length);
    assert.equal(lock.root, digest('manifest', manifest));
    assert.deepEqual(lock.packages, [{ id: lock.root, sourceSha256: digest('source-files', manifest.files), dependencies: [] }]);
    assert.deepEqual(lock.compatibility, manifest.compatibility);
  }
});

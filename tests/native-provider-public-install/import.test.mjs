import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

const producer = new URL('../../scripts/distribution/produce-native-installation.mjs', import.meta.url).href;
const loader = `export async function resolve(specifier, context, nextResolve) {
  if (!specifier.startsWith('node:') && !specifier.startsWith('.')
    && !specifier.startsWith('/') && !specifier.startsWith('file:') && !specifier.startsWith('data:')) {
    throw new Error('native installation bare package denied: ' + specifier);
  }
  return nextResolve(specifier, context);
}`;

test('real producer import succeeds with all bare packages denied, including actual Ajv control', () => {
  const program = `import assert from 'node:assert/strict';
    await assert.rejects(import('ajv'), /native installation bare package denied: ajv/);
    const producer = await import(${JSON.stringify(producer)});
    assert.equal(typeof producer.produceNativeInstallation, 'function');
    process.stdout.write('native-producer-import-without-packages-passed\\n');`;
  const observed = spawnSync(process.execPath, ['--no-warnings', '--experimental-loader',
    'data:text/javascript,' + encodeURIComponent(loader), '--input-type=module', '--eval', program],
  { encoding: 'utf8', timeout: 30_000, maxBuffer: 1024 * 1024,
    env: { ...process.env, NODE_OPTIONS: '' } });
  assert.equal(observed.error, undefined); assert.equal(observed.signal, null);
  assert.equal(observed.status, 0, observed.stderr);
  assert.equal(observed.stdout, 'native-producer-import-without-packages-passed\n');
  assert.equal(observed.stderr, '');
});

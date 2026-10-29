import assert from 'node:assert/strict';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import test from 'node:test';
import { loadFuzzRegistry, REGISTRY, ROOT } from '../scripts/stability-gates/fuzz-registry.mjs';

function copyAuthority(t) {
  const root = mkdtempSync(resolve(tmpdir(), 'zryna-fuzz-registry-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const { registry } = loadFuzzRegistry();
  const legacy = JSON.parse(readFileSync(resolve(ROOT, 'tests/stability-gates-v1.json'), 'utf8'));
  const paths = [REGISTRY, 'tests/stability-gates-v1.json', ...registry.sourcePins.map(pin => pin.path),
    ...legacy.contracts.map(contract => contract.authority)];
  for (const path of paths) {
    mkdirSync(dirname(resolve(root, path)), { recursive: true });
    copyFileSync(resolve(ROOT, path), resolve(root, path));
  }
  return root;
}

test('frozen authority rejects changed corpus, seed, argv, budgets and unknown fields', t => {
  const root = copyAuthority(t);
  loadFuzzRegistry(root);
  const path = resolve(root, REGISTRY);
  const original = readFileSync(path);
  for (const mutate of [
    value => { value.campaigns[0].corpus.reverse(); },
    value => { value.campaigns[0].seed = '0x1'; },
    value => { value.campaigns[0].cases -= 1; },
    value => { value.campaigns[0].args.push('--ignored'); },
    value => { value.fullOwner.args.pop(); },
    value => { value.waiver = true; },
  ]) {
    const value = JSON.parse(original);
    mutate(value);
    writeFileSync(path, JSON.stringify(value));
    assert.throws(() => loadFuzzRegistry(root));
  }
  writeFileSync(path, original);
  const material = loadFuzzRegistry(root).registry.sourcePins[0].path;
  writeFileSync(resolve(root, material), 'substituted generator');
  assert.throws(() => loadFuzzRegistry(root));
});

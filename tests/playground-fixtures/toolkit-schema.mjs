import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { validateToolkitEnvelope } from '../../examples/playground/restricted/toolkit-schema.mjs';

// Synthetic publication metadata checks schema/policy only, never signatures or executable admission.
const digest = value => createHash('sha256').update(value).digest('hex');
const sourceCommit = 'a'.repeat(40);
const sourceTree = 'b'.repeat(40);
const archiveSha256 = 'c'.repeat(64);
const nestedCompiler = { version: '0.2.3', envelopeSha256: 'd'.repeat(64),
  archiveSha256: 'e'.repeat(64), sourceCommit: 'f'.repeat(40) };
const browser = { version: '153.0.8010.12',
  archiveSha256: '8aac35011c18f6e2d10696154af89a5728ac2ddd6dc6fad24ffdf243c3fcfd5a',
  inventorySha256: '110d5111c1a885596b2dba4fe5d04e570d26e1c57454d0a902f1115ce818635a' };
const captured = {
  'releases/compiler-envelope.json': [nestedCompiler.envelopeSha256, 262144],
  'releases/compiler-envelope.sigstore.json': [digest('synthetic:compiler-bundle'), 1048576],
  'releases/zryna-0.2.3-x86_64-unknown-linux-gnu.tar.gz': [nestedCompiler.archiveSha256, 268435456],
  'releases/chrome-linux64.zip': [browser.archiveSha256, 268435456],
  'releases/chrome-linux64.inventory.json': [browser.inventorySha256, 8388608],
};
const requiredGates = ['linux-contract', 'windows-contract'];
const root = 'zryna-playground-0.1.0-x86_64-unknown-linux-gnu';

function sorted(value) {
  if (Array.isArray(value)) return value.map(sorted);
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort()
    .map(key => [key, sorted(value[key])]));
  return value;
}
const encode = value => Buffer.from(JSON.stringify(sorted(value)) + '\n');
const file = (path, mode = 0o644) => ({ path, mode, bytes: 17, sha256: digest(`synthetic:${path}`) });

function fixture() {
  const files = [file('bin/zryna-playground-compiler', 0o755), file('policy.json'),
    file('resources/loader.js'), file('resources/browser.wit'), file('resources/wit-sources.json'),
    file('runtime/node/bin/node', 0o755),
    ...Object.entries(captured).map(([path, [sha256]]) => ({ ...file(path), sha256 })),
    ...requiredGates.map(name => file(`evidence/${name}.json`))].sort((a, b) => a.path < b.path ? -1 : 1);
  const document = { format: 'zryna.playground-toolkit.v1', version: 1, productVersion: '0.1.0',
    target: 'x86_64-unknown-linux-gnu',
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/tags/playground-v0.1.0',
      commit: sourceCommit, tree: sourceTree, sourceDateEpoch: 1_700_000_000 },
    signing: { issuer: 'https://token.actions.githubusercontent.com', certificateIdentity:
      'https://github.com/zryna/zryna/.github/workflows/playground-release.yml@refs/tags/playground-v0.1.0' },
    archive: { path: root + '.tar.gz', bytes: 1024, sha256: archiveSha256 },
    nestedCompiler: structuredClone(nestedCompiler),
    browser: structuredClone(browser),
    files, materials: ['bin/zryna-playground-compiler', 'runtime/node/bin/node'].map((path, index) => {
      const descriptor = files.find(value => value.path === path);
      return { path, mount: index === 0 ? '/app/compiler' : '/materials/runtime/node/bin/node',
        bytes: descriptor.bytes, sha256: descriptor.sha256, executable: true };
    }), gates: requiredGates.map(name => ({ name, sourceCommit, status: 'passed',
      receiptPath: `evidence/${name}.json`, receiptSha256: files.find(value =>
        value.path === `evidence/${name}.json`).sha256 })) };
  const policy = { version: 1, sourceCommit, sourceTree, envelopeSha256: digest(encode(document)),
    archiveSha256, nestedCompiler: structuredClone(nestedCompiler), requiredGates: [...requiredGates] };
  return { document, policy };
}

function reseal(document, policy) {
  const input = encode(document);
  return { input, policy: { ...policy, envelopeSha256: digest(input) } };
}

function rejects(mutate, pattern) {
  const { document, policy } = fixture();
  mutate(document, policy);
  const sealed = reseal(document, policy);
  // Recomputing the independently expected envelope digest cannot waive fixed policy checks.
  assert.throws(() => validateToolkitEnvelope(sealed.input, sealed.policy), pattern);
}

function syncMaterial(document, path) {
  const descriptor = document.files.find(value => value.path === path);
  for (const material of document.materials.filter(value => value.path === path)) {
    material.bytes = descriptor.bytes;
    material.sha256 = descriptor.sha256;
    material.executable = descriptor.mode === 0o755;
  }
}

export { digest, sourceCommit, sourceTree, archiveSha256, nestedCompiler, browser, captured, requiredGates, root, sorted, encode, file, fixture, reseal, rejects, syncMaterial };

import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { requireSecurityRustWorkspace } from '../scripts/distribution/rust-security-materials.mjs';
import { checkSecurityRustClosure } from '../scripts/distribution/rust-materials.mjs';

const inputs = ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json'];
function workspace() {
  const root = mkdtempSync(join(tmpdir(), 'zryna-security-rust-inputs-'));
  for (const file of inputs) {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), readFileSync(new URL(`../${file}`, import.meta.url)));
  }
  return root;
}

test('security workspace guard accepts exact current inputs and rejects each drift before any tool spawn', () => {
  const root = workspace();
  try {
    requireSecurityRustWorkspace(root);
    for (const file of inputs) {
      const original = readFileSync(join(root, file));
      let spawned = 0;
      writeFileSync(join(root, file), Buffer.concat([original, Buffer.from('\n')]));
      assert.throws(() => checkSecurityRustClosure({ cwd: root,
        spawn() { spawned++; throw new Error('unexpected compiler process'); } }), /exact workspace inputs/);
      assert.equal(spawned, 0);
      writeFileSync(join(root, file), original); requireSecurityRustWorkspace(root);
    }
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('current package script chooses the explicit security closure audit', () => {
  const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url)));
  assert.equal(pkg.scripts['qualification:rust-closure'],
    'node scripts/distribution/rust-materials.mjs --check-security-patch-qualification-closure');
});

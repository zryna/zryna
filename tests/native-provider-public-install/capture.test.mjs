import test from 'node:test';
import assert from 'node:assert/strict';
import {
  linkSync, lstatSync, mkdirSync, mkdtempSync, realpathSync, readFileSync, rmSync, unlinkSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  captureProducerExecutable, verifyProducerExecutableCapture,
} from '../../scripts/distribution/native-producer-capture.mjs';
import { bytes } from '../../scripts/distribution/canonical.mjs';
import { ordinaryBytes } from './commands.mjs';

function fixture(action) {
  const root = realpathSync.native(mkdtempSync(join(tmpdir(), 'zryna-producer-capture-')));
  const deps = join(root, 'deps'), captures = join(root, 'captures');
  mkdirSync(deps); mkdirSync(captures);
  const suffix = process.platform === 'win32' ? '.exe' : '';
  const source = join(root, 'zryna' + suffix), alias = join(deps, 'zryna-0123456789abcdef' + suffix);
  writeFileSync(source, 'bounded test executable bytes', { mode: 0o700 }); linkSync(source, alias);
  const options = { source, destination: join(captures, 'compiler'), aliasDirectory: deps, kind: 'cargo-cli' };
  try { action({ root, source, alias, deps, captures, suffix, options }); }
  finally { rmSync(root, { recursive: true }); }
}

test('actual Cargo-style hardlink capture explains both names and keeps ordinary input rejection', () => {
  fixture(({ source, options }) => {
    assert.equal(lstatSync(source).nlink, 2);
    assert.throws(() => ordinaryBytes(source), /bounded ordinary input/);
    const proof = captureProducerExecutable(options);
    assert.equal(proof.aliases.length, 2); assert.equal(proof.source.nlink, 2);
    assert.equal(proof.captureNlink, 1); assert.equal(proof.copyCountedAsRebuild, false);
    assert(readFileSync(proof.capture.path).equals(readFileSync(source)));
    verifyProducerExecutableCapture(proof);
    verifyProducerExecutableCapture(JSON.parse(bytes(proof)));
    assert.throws(() => captureProducerExecutable(options), /EEXIST/);
  });
});

test('producer capture rejects hidden or unapproved aliases instead of waiving link count', () => {
  fixture(({ root, source, options }) => {
    linkSync(source, join(root, 'hidden-alias'));
    assert.throws(() => captureProducerExecutable(options), /not fully explained/);
  });
  fixture(({ deps, source, options }) => {
    linkSync(source, join(deps, 'unapproved-helper'));
    assert.throws(() => captureProducerExecutable(options), /unapproved producer hardlink/);
  });
});

test('retained producer proof rejects mutated source, alias replacement and capture aliases', () => {
  for (const mutation of [
    ({ source }) => writeFileSync(source, 'mutated test executable bytes'),
    ({ alias }) => { unlinkSync(alias); writeFileSync(alias, 'replacement bytes'); },
    ({ root, proof }) => linkSync(proof.capture.path, join(root, 'capture-alias')),
  ]) fixture(context => {
    const proof = captureProducerExecutable(context.options); mutation({ ...context, proof });
    assert.throws(() => verifyProducerExecutableCapture(proof));
  });
});

test('Rustup multicall capture admits only the finite actual same-inode shim closure', () => {
  fixture(({ root, captures, suffix }) => {
    const bin = join(root, 'bin'); mkdirSync(bin);
    const source = join(bin, 'rustup' + suffix); writeFileSync(source, 'rustup unit fixture bytes');
    linkSync(source, join(bin, 'cargo' + suffix)); linkSync(source, join(bin, 'rustc' + suffix));
    const proof = captureProducerExecutable({ source, destination: join(captures, 'rustup' + suffix),
      aliasDirectory: bin, kind: 'rustup-shim' });
    assert.equal(proof.aliases.length, 3); assert.equal(proof.captureNlink, 1);
    verifyProducerExecutableCapture(proof);
    linkSync(source, join(bin, 'arbitrary-executable' + suffix));
    assert.throws(() => verifyProducerExecutableCapture(proof));
  });
});

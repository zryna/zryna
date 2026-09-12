import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  auditQualificationCargoCache, provisionQualificationCargoHome,
} from '../scripts/distribution-release/provision-qualification-cargo.mjs';

test('fetches into one fresh Cargo home and binds every cached crate to captured bytes', (t) => {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-qualification-cargo-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const sourceRoot = join(parent, 'source');
  const workRoot = join(parent, 'work');
  const windows = process.platform === 'win32';
  const cargo = join(parent, windows ? 'cargo.exe' : 'cargo');
  const rustc = join(parent, windows ? 'rustc.exe' : 'rustc');
  const target = windows ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu';
  mkdirSync(sourceRoot);
  writeFileSync(cargo, 'cargo executable');
  writeFileSync(rustc, 'rustc executable');
  const archive = Buffer.from('authenticated crate archive');
  const environment = {
    ZRYNA_CARGO_PATH: cargo,
    ZRYNA_CARGO_SHA256: sha256(Buffer.from('cargo executable')),
    ZRYNA_RUSTC_PATH: rustc,
    ZRYNA_QUALIFICATION_FETCH_PATH: parent,
    SOURCE_DATE_EPOCH: '1789081200',
    ...(windows ? { SystemRoot: 'C:\\Windows' } : {}),
  };
  let called = false;
  const provisioned = provisionQualificationCargoHome({
    sourceRoot, workRoot, target, environment,
    spawn(executable, args, options) {
      called = true;
      assert.equal(executable, cargo);
      assert.deepEqual(args, ['fetch', '--locked', '--target', target]);
      assert.equal(options.env.CARGO_NET_OFFLINE, 'false');
      const cache = join(options.env.CARGO_HOME, 'registry', 'cache',
        'index.crates.io-1949cf8c6b5b557f');
      mkdirSync(cache, { recursive: true });
      writeFileSync(join(cache, 'example-1.2.3.crate'), archive);
      return { status: 0, signal: null, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    },
  });
  assert.equal(called, true);
  const captures = [{ identity: 'example-1.2.3', archive }];
  auditQualificationCargoCache(provisioned.cargoHome, captures);
  writeFileSync(join(provisioned.cargoHome, 'registry', 'cache',
    'index.crates.io-1949cf8c6b5b557f', 'example-1.2.3.crate'), 'changed');
  assert.throws(() => auditQualificationCargoCache(provisioned.cargoHome, captures),
    /Cargo material bytes differ/);
});

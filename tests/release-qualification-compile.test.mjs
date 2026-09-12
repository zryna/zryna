import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { compileReleaseQualification } from '../scripts/distribution-release/compile-release-qualification.mjs';
import { createQualificationFixture } from './release-qualification-fixture.mjs';

function roots(t) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-qualification-compile-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const sourceRoot = join(parent, 'source');
  const workRoot = join(parent, 'work');
  const cargoHome = join(workRoot, 'cargo-home');
  mkdirSync(sourceRoot);
  mkdirSync(workRoot);
  mkdirSync(cargoHome);
  return { parent, sourceRoot, workRoot, cargoHome };
}

function observed(input, parent) {
  const paths = {};
  const records = new Map([...input.toolchains, ...input.nativeTools]
    .map((record) => [record.name, record]));
  for (const [name, data] of [['cargo', Buffer.from('cargo')], ['rustc', Buffer.from('rustc')],
    ['linker', Buffer.from('linker')]]) {
    const path = join(parent, name);
    writeFileSync(path, data);
    paths[name] = path;
    records.get(name).size = data.length;
    records.get(name).sha256 = sha256(data);
  }
  return { toolchains: input.toolchains, nativeTools: input.nativeTools, paths };
}

test('materializes only bound compile tokens and runs one absolute Cargo command', async (t) => {
  const fixture = await createQualificationFixture();
  const paths = roots(t);
  const tools = observed(fixture.input, paths.parent);
  const binding = Buffer.from(`${canonicalBounded(fixture.input)}\n`);
  let called = false;
  const result = compileReleaseQualification({
    binding, ...paths, observedTools: tools,
    spawn(executable, args, options) {
      called = true;
      assert.equal(executable, tools.paths.cargo);
      assert.deepEqual(args, fixture.input.compile.argv.slice(1));
      assert.equal(options.shell, false);
      assert.equal(options.env.SOURCE_DATE_EPOCH, '1789081200');
      assert(!Object.values(options.env).some((value) => /@[A-Za-z0-9_-]+@/.test(value)));
      const output = join(paths.workRoot, 'target', fixture.input.target.triple, 'release');
      mkdirSync(output, { recursive: true });
      writeFileSync(join(output, 'zryna'), Buffer.alloc(64, 7));
      return { status: 0, signal: null, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    },
  });
  assert.equal(called, true);
  assert.equal(result.cli.length, 64);
  assert.match(result.environment.CARGO_ENCODED_RUSTFLAGS, /-Cdebuginfo=0/);
});

test('rejects tool drift before invoking Cargo', async (t) => {
  const fixture = await createQualificationFixture();
  const paths = roots(t);
  const tools = observed(fixture.input, paths.parent);
  tools.toolchains[0].sha256 = 'f'.repeat(64);
  const binding = Buffer.from(`${canonicalBounded(fixture.input)}\n`);
  assert.throws(() => compileReleaseQualification({
    binding, ...paths, observedTools: tools, spawn: assert.fail,
  }), /executable bytes differ/);
});

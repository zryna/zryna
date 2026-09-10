import assert from 'node:assert/strict';
import {
  mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { compareReleaseBuilds } from '../scripts/distribution-release/compare-release-builds.mjs';

const TARGET = 'x86_64-unknown-linux-gnu';
const BASE = `zryna-0.2.0-${TARGET}`;
const FILES = {
  archive: `${BASE}.tar.gz`,
  buildReceipt: `${BASE}.build-receipt.json`,
  sbom: `${BASE}.spdx.json`,
};

function source() {
  return {
    repository: 'https://github.com/zryna/zryna',
    ref: 'refs/tags/v0.2.0',
    tagObject: 'a'.repeat(40),
    commit: 'b'.repeat(40),
    tree: 'c'.repeat(40),
    sourceDateEpoch: 1_789_081_200,
  };
}

function build(root, replica, mutate = () => {}) {
  const bytes = {
    archive: Buffer.from('archive bytes'),
    buildReceipt: Buffer.from('{"receipt":true}\n'),
    sbom: Buffer.from('{"spdxVersion":"SPDX-2.3"}\n'),
  };
  const value = {
    format: 'zryna.release-build-result.v1',
    version: '0.2.0',
    target: TARGET,
    replica,
    source: source(),
    recipe: { format: 'zryna.distribution-recipe.v1', sha256: 'd'.repeat(64) },
    artifacts: Object.fromEntries(Object.entries(FILES).map(([key, path]) => [key, {
      path, size: bytes[key].length, sha256: sha256(bytes[key]),
    }])),
  };
  mutate({ bytes, value });
  for (const [key, path] of Object.entries(FILES)) writeFileSync(join(root, path), bytes[key]);
  writeFileSync(join(root, 'build-result.json'), `${canonicalBounded(value)}\n`);
  return value;
}

function roots(t) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-release-reproduction-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const firstRoot = join(parent, 'first');
  const secondRoot = join(parent, 'second');
  const outputRoot = join(parent, 'output');
  mkdirSync(firstRoot);
  mkdirSync(secondRoot);
  return { firstRoot, secondRoot, outputRoot };
}

test('compares every deterministic byte and emits one canonical reproduced subject', (t) => {
  const paths = roots(t);
  const first = build(paths.firstRoot, 1);
  build(paths.secondRoot, 2);
  const result = compareReleaseBuilds({ ...paths, target: TARGET });
  assert.equal(result.comparison, 'byte-identical');
  assert.equal(result.artifacts.archive.sha256, first.artifacts.archive.sha256);
  assert.notEqual(result.builds[0].manifestSha256, result.builds[1].manifestSha256);
  const output = readFileSync(join(paths.outputRoot, 'reproduction.json'), 'utf8');
  assert.equal(output, `${canonicalBounded(result)}\n`);
  for (const [key, path] of Object.entries(FILES)) {
    assert.deepEqual(readFileSync(join(paths.outputRoot, path)),
      readFileSync(join(paths.firstRoot, first.artifacts[key].path)));
  }
});

test('rejects differing bytes even when a replica repeats the admitted descriptor', (t) => {
  const paths = roots(t);
  build(paths.firstRoot, 1);
  build(paths.secondRoot, 2, ({ bytes, value }) => {
    bytes.archive = Buffer.from('forged archive');
    value.artifacts.archive.size = Buffer.byteLength('archive bytes');
    value.artifacts.archive.sha256 = sha256('archive bytes');
  });
  assert.throws(() => compareReleaseBuilds({ ...paths, target: TARGET }),
    /R406-RELEASE-FILE:.*replica (sizes|bytes) differ/);
});

test('rejects identity drift, replica drift, extra entries, and reused roots', (t) => {
  const identity = roots(t);
  build(identity.firstRoot, 1);
  build(identity.secondRoot, 2, ({ value }) => { value.source.tree = 'e'.repeat(40); });
  assert.throws(() => compareReleaseBuilds({ ...identity, target: TARGET }),
    /R406-REPRODUCTION: build result identities differ/);

  const replica = roots(t);
  build(replica.firstRoot, 1);
  build(replica.secondRoot, 1);
  assert.throws(() => compareReleaseBuilds({ ...replica, target: TARGET }),
    /R406-REPRODUCTION: build result target or replica differs/);

  const extra = roots(t);
  build(extra.firstRoot, 1);
  build(extra.secondRoot, 2);
  writeFileSync(join(extra.firstRoot, 'unexpected.txt'), 'extra');
  assert.throws(() => compareReleaseBuilds({ ...extra, target: TARGET }),
    /R406-RELEASE-FILE: release file inventory differs/);

  assert.throws(() => compareReleaseBuilds({
    firstRoot: extra.firstRoot,
    secondRoot: extra.firstRoot,
    outputRoot: extra.outputRoot,
    target: TARGET,
  }), /R406-REPRODUCTION: exact distinct absolute roots/);
});

test('rejects noncanonical manifests before reading artifact bytes', (t) => {
  const paths = roots(t);
  const value = build(paths.firstRoot, 1);
  build(paths.secondRoot, 2);
  writeFileSync(join(paths.firstRoot, 'build-result.json'), JSON.stringify(value));
  assert.throws(() => compareReleaseBuilds({ ...paths, target: TARGET }), /R406-CANONICAL:/);
});

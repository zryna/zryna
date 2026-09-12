import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { createReleaseTagReceipt } from '../scripts/distribution-release/create-release-tag-receipt.mjs';
import {
  validateReleaseTagReceipt,
  validateReleaseTagReceiptText,
} from '../scripts/distribution-release/validate-release-tag-receipt.mjs';

const TAG_OBJECT = 'a'.repeat(40);
const COMMIT = 'b'.repeat(40);
const TREE = 'c'.repeat(40);
const WORKFLOW_OBJECT = 'd'.repeat(40);
const WORKFLOW = Buffer.from('name: Release\non:\n  push:\n    tags: [v0.2.0]\n');
const ROOT = resolve('tag-receipt-source');

function fixture() {
  return {
    format: 'zryna.release-tag-receipt.v1',
    version: '0.2.0',
    source: {
      repository: 'https://github.com/zryna/zryna',
      ref: 'refs/tags/v0.2.0',
      tagType: 'annotated',
      tagObject: TAG_OBJECT,
      commit: COMMIT,
      tree: TREE,
      sourceDateEpoch: 1_789_081_200,
    },
    workflow: {
      path: '.github/workflows/release.yml',
      size: WORKFLOW.length,
      sha256: sha256(WORKFLOW),
    },
  };
}

function environment() {
  return {
    GITHUB_EVENT_NAME: 'push',
    GITHUB_REPOSITORY: 'zryna/zryna',
    GITHUB_REF: 'refs/tags/v0.2.0',
    GITHUB_REF_TYPE: 'tag',
    GITHUB_REF_NAME: 'v0.2.0',
    GITHUB_REF_PROTECTED: 'true',
    GITHUB_SHA: COMMIT,
    GITHUB_WORKFLOW_SHA: COMMIT,
    GITHUB_WORKFLOW_REF:
      'zryna/zryna/.github/workflows/release.yml@refs/tags/v0.2.0',
    ZRYNA_SOURCE_ROOT: ROOT,
  };
}

function responses() {
  const tag = Buffer.from([
    `object ${COMMIT}`,
    'type commit',
    'tag v0.2.0',
    'tagger Release Maintainer <release@example.invalid> 1789081200 +0000',
    '',
    'Zryna 0.2.0',
    '',
  ].join('\n'));
  return new Map([
    ['rev-parse\0--show-toplevel', `${ROOT}\n`],
    ['status\0--porcelain=v1\0--untracked-files=all\0--ignored=matching', ''],
    ['show-ref\0--verify\0--hash\0refs/tags/v0.2.0', `${TAG_OBJECT}\n`],
    [`cat-file\0-t\0${TAG_OBJECT}`, 'tag\n'],
    [`cat-file\0-s\0${TAG_OBJECT}`, `${tag.length}\n`],
    [`cat-file\0tag\0${TAG_OBJECT}`, tag],
    [`cat-file\0-t\0${COMMIT}`, 'commit\n'],
    ['rev-parse\0refs/tags/v0.2.0^{commit}', `${COMMIT}\n`],
    ['rev-parse\0HEAD', `${COMMIT}\n`],
    [`show\0-s\0--format=%T\0${COMMIT}`, `${TREE}\n`],
    [`show\0-s\0--format=%ct\0${COMMIT}`, '1789081200\n'],
    [`ls-tree\0-z\0--full-tree\0${COMMIT}\0--\0.github/workflows/release.yml`,
      Buffer.from(`100644 blob ${WORKFLOW_OBJECT}\t.github/workflows/release.yml\0`)],
    [`cat-file\0-s\0${WORKFLOW_OBJECT}`, `${WORKFLOW.length}\n`],
    [`cat-file\0blob\0${WORKFLOW_OBJECT}`, WORKFLOW],
  ]);
}

function producer(overrides = {}) {
  const output = responses();
  for (const [key, value] of Object.entries(overrides)) output.set(key, value);
  const seen = [];
  const spawn = (executable, args, options) => {
    assert.equal(executable, 'git');
    assert.equal(options.cwd, ROOT);
    assert.equal(options.shell, false);
    assert.equal(options.maxBuffer, 256 * 1024 + 1);
    const key = args.join('\0');
    seen.push(key);
    assert(output.has(key), `unexpected command: ${key}`);
    const value = output.get(key);
    return {
      status: 0,
      stdout: options.encoding === null
        ? Buffer.from(value)
        : Buffer.isBuffer(value) ? value.toString('utf8') : value,
    };
  };
  return { receipt: createReleaseTagReceipt({ environment: environment(), spawn }), seen };
}

test('accepts one canonical protected annotated-tag receipt', () => {
  const value = fixture();
  const text = `${canonical(value)}\n`;
  assert.equal(validateReleaseTagReceipt(value), value);
  assert.deepEqual(validateReleaseTagReceiptText(text), value);
  const { receipt, seen } = producer();
  assert.deepEqual(receipt, value);
  assert(seen.includes(`cat-file\0blob\0${WORKFLOW_OBJECT}`));
});

test('rejects version, ref, tag-object, workflow, and nondeterministic drift', () => {
  for (const [code, mutate] of [
    ['R406-TAG-SOURCE', (value) => { value.version = '0.2.1'; }],
    ['R406-TAG-SOURCE', (value) => { value.source.ref = 'refs/tags/v0.2.1'; }],
    ['R406-TAG-SOURCE', (value) => { value.source.tagObject = value.source.commit; }],
    ['R406-TAG-SCHEMA', (value) => { value.workflow.path = 'release.yml'; }],
    ['R406-TAG-SCHEMA', (value) => { value.runId = '123'; }],
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateReleaseTagReceipt(value), new RegExp(`${code}:`));
  }
  assert.throws(() => validateReleaseTagReceiptText(JSON.stringify(fixture())),
    /R406-CANONICAL:/);
});

test('producer rejects lightweight, nested, dirty, and non-ordinary workflow inputs', () => {
  const light = responses();
  light.set(`cat-file\0-t\0${TAG_OBJECT}`, 'commit\n');
  assert.throws(() => createReleaseTagReceipt({
    environment: environment(),
    spawn: mock(light),
  }), /release reference must identify one annotated tag object/);

  const nested = responses();
  const tag = Buffer.from([
    `object ${'e'.repeat(40)}`, 'type tag', 'tag v0.2.0',
    'tagger Release Maintainer <release@example.invalid> 1789081200 +0000', '', 'nested', '',
  ].join('\n'));
  nested.set(`cat-file\0-s\0${TAG_OBJECT}`, `${tag.length}\n`);
  nested.set(`cat-file\0tag\0${TAG_OBJECT}`, tag);
  assert.throws(() => createReleaseTagReceipt({
    environment: environment(),
    spawn: mock(nested),
  }), /does not directly identify the expected release commit and name/);

  const dirty = responses();
  dirty.set('status\0--porcelain=v1\0--untracked-files=all\0--ignored=matching', '?? target/\n');
  assert.throws(() => createReleaseTagReceipt({
    environment: environment(),
    spawn: mock(dirty),
  }), /checkout contains tracked, untracked, or ignored files/);

  const executable = responses();
  executable.set(
    `ls-tree\0-z\0--full-tree\0${COMMIT}\0--\0.github/workflows/release.yml`,
    Buffer.from(`100755 blob ${WORKFLOW_OBJECT}\t.github/workflows/release.yml\0`),
  );
  assert.throws(() => createReleaseTagReceipt({
    environment: environment(),
    spawn: mock(executable),
  }), /workflow must be one ordinary non-executable Git blob/);
});

test('producer rejects workflow context, object-size, and read-size drift', () => {
  const wrongContext = environment();
  wrongContext.GITHUB_SHA = 'e'.repeat(40);
  assert.throws(() => createReleaseTagReceipt({ environment: wrongContext, spawn: assert.fail }),
    /exact protected tag workflow context is required/);

  const oversize = responses();
  oversize.set(`cat-file\0-s\0${WORKFLOW_OBJECT}`, `${256 * 1024 + 1}\n`);
  assert.throws(() => createReleaseTagReceipt({
    environment: environment(),
    spawn: mock(oversize),
  }), /tagged workflow blob exceeds the receipt budget/);

  const changed = responses();
  changed.set(`cat-file\0-s\0${WORKFLOW_OBJECT}`, `${WORKFLOW.length + 1}\n`);
  assert.throws(() => createReleaseTagReceipt({
    environment: environment(),
    spawn: mock(changed),
  }), /tagged workflow size changed while reading/);

  const swapped = responses();
  let refReads = 0;
  const swapRef = (executable, args, options) => {
    assert.equal(executable, 'git');
    const key = args.join('\0');
    let value = swapped.get(key);
    assert.notEqual(value, undefined, `unexpected command: ${args.join(' ')}`);
    if (key === 'show-ref\0--verify\0--hash\0refs/tags/v0.2.0' && refReads++ === 1) {
      value = `${'e'.repeat(40)}\n`;
    }
    return {
      status: 0,
      stdout: options.encoding === null
        ? Buffer.from(value)
        : Buffer.isBuffer(value) ? value.toString('utf8') : value,
    };
  };
  assert.throws(() => createReleaseTagReceipt({
    environment: environment(),
    spawn: swapRef,
  }), /release tag reference changed while producing the receipt/);
});

function mock(output) {
  return (executable, args, options) => {
    assert.equal(executable, 'git');
    const value = output.get(args.join('\0'));
    assert.notEqual(value, undefined, `unexpected command: ${args.join(' ')}`);
    return {
      status: 0,
      stdout: options.encoding === null
        ? Buffer.from(value)
        : Buffer.isBuffer(value) ? value.toString('utf8') : value,
    };
  };
}

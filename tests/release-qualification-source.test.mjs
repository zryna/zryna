import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { createReleaseQualificationSource } from '../scripts/distribution-release/create-release-qualification-source.mjs';
import {
  validateReleaseQualificationSource,
  validateReleaseQualificationSourceText,
} from '../scripts/distribution-release/validate-release-qualification-source.mjs';

const COMMIT = 'b'.repeat(40);
const TREE = 'c'.repeat(40);
const WORKFLOW_OBJECT = 'd'.repeat(40);
const WORKFLOW = Buffer.from('name: Qualify release candidate\non: workflow_dispatch\n');
const ROOT = resolve('qualification-source');

function fixture() {
  return {
    format: 'zryna.release-qualification-source.v1',
    status: 'provisional-candidate',
    productionAdmission: 'forbidden',
    versionCandidate: '0.2.0',
    source: {
      repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: COMMIT, tree: TREE, sourceDateEpoch: 1_789_081_200,
    },
    workflow: {
      path: '.github/workflows/release-qualification.yml',
      size: WORKFLOW.length,
      sha256: sha256(WORKFLOW),
    },
  };
}

function environment() {
  return {
    GITHUB_EVENT_NAME: 'workflow_dispatch', GITHUB_REPOSITORY: 'zryna/zryna',
    GITHUB_REF: 'refs/heads/main', GITHUB_REF_TYPE: 'branch', GITHUB_REF_NAME: 'main',
    GITHUB_REF_PROTECTED: 'true', GITHUB_SHA: COMMIT, GITHUB_WORKFLOW_SHA: COMMIT,
    GITHUB_WORKFLOW_REF:
      'zryna/zryna/.github/workflows/release-qualification.yml@refs/heads/main',
    ZRYNA_SOURCE_ROOT: ROOT,
  };
}

function responses() {
  return new Map([
    ['rev-parse\0--show-toplevel', `${ROOT}\n`],
    ['status\0--porcelain=v1\0--untracked-files=all\0--ignored=matching', ''],
    ['rev-parse\0HEAD', `${COMMIT}\n`],
    [`cat-file\0-t\0${COMMIT}`, 'commit\n'],
    [`show\0-s\0--format=%T\0${COMMIT}`, `${TREE}\n`],
    [`show\0-s\0--format=%ct\0${COMMIT}`, '1789081200\n'],
    [`ls-tree\0-z\0--full-tree\0${COMMIT}\0--\0.github/workflows/release-qualification.yml`,
      Buffer.from(`100644 blob ${WORKFLOW_OBJECT}\t.github/workflows/release-qualification.yml\0`)],
    [`cat-file\0-s\0${WORKFLOW_OBJECT}`, `${WORKFLOW.length}\n`],
    [`cat-file\0blob\0${WORKFLOW_OBJECT}`, WORKFLOW],
  ]);
}

function mock(values, mutate = (value) => value) {
  return (executable, args, options) => {
    assert.equal(executable, 'git');
    const key = args.join('\0');
    const value = mutate(values.get(key), key);
    assert.notEqual(value, undefined, `unexpected command: ${key}`);
    return { status: 0, stdout: options.encoding === null
      ? Buffer.from(value) : Buffer.isBuffer(value) ? value.toString('utf8') : value };
  };
}

test('accepts and produces one truthful protected-main qualification source receipt', () => {
  const value = fixture();
  assert.equal(validateReleaseQualificationSource(value), value);
  assert.deepEqual(validateReleaseQualificationSourceText(`${canonical(value)}\n`), value);
  assert.deepEqual(createReleaseQualificationSource({
    environment: environment(), spawn: mock(responses()),
  }), value);
});

test('rejects production-tag identity, open fields, dirty source, and executable workflow', () => {
  for (const mutate of [
    (value) => { value.source.ref = 'refs/tags/v0.2.0'; },
    (value) => { value.status = 'release'; },
    (value) => { value.tagObject = 'a'.repeat(40); },
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateReleaseQualificationSource(value),
      /R406-QUALIFICATION-SOURCE-SCHEMA:/);
  }
  const dirty = responses();
  dirty.set('status\0--porcelain=v1\0--untracked-files=all\0--ignored=matching', '?? output\n');
  assert.throws(() => createReleaseQualificationSource({
    environment: environment(), spawn: mock(dirty),
  }), /checkout contains tracked, untracked, or ignored files/);
  const executable = responses();
  executable.set(
    `ls-tree\0-z\0--full-tree\0${COMMIT}\0--\0.github/workflows/release-qualification.yml`,
    Buffer.from(`100755 blob ${WORKFLOW_OBJECT}\t.github/workflows/release-qualification.yml\0`),
  );
  assert.throws(() => createReleaseQualificationSource({
    environment: environment(), spawn: mock(executable),
  }), /workflow must be one ordinary non-executable Git blob/);
});

test('rejects unprotected, non-main, wrong-workflow, or changing candidate context', () => {
  for (const change of [
    ['GITHUB_REF_PROTECTED', 'false'],
    ['GITHUB_REF', 'refs/heads/feature'],
    ['GITHUB_WORKFLOW_SHA', 'e'.repeat(40)],
  ]) {
    const env = environment();
    env[change[0]] = change[1];
    assert.throws(() => createReleaseQualificationSource({ environment: env, spawn: assert.fail }),
      /exact protected main qualification workflow context/);
  }
  let headReads = 0;
  assert.throws(() => createReleaseQualificationSource({
    environment: environment(),
    spawn: mock(responses(), (value, key) => key === 'rev-parse\0HEAD' && headReads++ === 1
      ? `${'e'.repeat(40)}\n` : value),
  }), /checkout changed while producing the receipt/);
});

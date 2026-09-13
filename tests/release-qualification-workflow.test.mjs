import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { parseDocument } from 'yaml';
import { qualificationHostEnvironment }
  from '../scripts/distribution-release/observe-qualification-tools.mjs';

const path = resolve(import.meta.dirname, '../.github/workflows/release-qualification.yml');
const text = readFileSync(path, 'utf8');
const document = parseDocument(text);
assert.deepEqual(document.errors, []);
const workflow = document.toJS();
const pin = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/;

test('qualification workflow is manual, non-cancellable, and read-only', () => {
  assert.deepEqual(workflow.on, { workflow_dispatch: null });
  assert.deepEqual(workflow.permissions, { contents: 'read' });
  assert.deepEqual(Object.keys(workflow.jobs), ['admit', 'build', 'compare']);
  assert.equal(workflow.concurrency['cancel-in-progress'], false);
  for (const job of Object.values(workflow.jobs)) {
    assert.notEqual(job.permissions?.contents, 'write');
    assert.equal(job.permissions?.['id-token'], undefined);
    assert.equal(job.permissions?.attestations, undefined);
    assert.equal(job.environment, undefined);
  }
  assert.doesNotMatch(text, /gh\s+release|refs\/tags|attest|id-token:\s*write/i);
});

test('qualification workflow pins actions and closes the replica topology', () => {
  for (const job of Object.values(workflow.jobs)) {
    for (const step of job.steps) if (step.uses) assert.match(step.uses, pin, step.uses);
  }
  assert.equal(workflow.jobs.build.needs, 'admit');
  assert.equal(workflow.jobs.compare.needs, 'build');
  assert.deepEqual(workflow.jobs.build.strategy.matrix.include.map(({ target, replica }) =>
    `${target}:${replica}`), [
    'x86_64-unknown-linux-gnu:1', 'x86_64-unknown-linux-gnu:2',
    'x86_64-pc-windows-msvc:1', 'x86_64-pc-windows-msvc:2',
  ]);
  assert.match(text, /run-release-qualification\.mjs/);
  assert.match(text, /compare-release-qualifications\.mjs/);
  assert.match(text, /compression-level: 0/);
  assert.match(text, /retention-days: 1/);
  const windows = workflow.jobs.build.steps.find(
    ({ name }) => name === 'Bind exact Windows tools and developer environment');
  assert.match(windows.run, /ZRYNA_QUALIFICATION_FETCH_PATH=/);
  assert.match(windows.run, /ZRYNA_QUALIFICATION_PATH=/);
  const fetchPath = /"ZRYNA_QUALIFICATION_FETCH_PATH=([^"\r\n]+)"/.exec(windows.run);
  const compilePath = /"ZRYNA_QUALIFICATION_PATH=([^"\r\n]+)"/.exec(windows.run);
  assert.equal(fetchPath[1], compilePath[1]);
});

function windowsDeveloperEnvironment() {
  const msvc = String.raw`C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.44.35207`;
  const kits = String.raw`C:\Program Files (x86)\Windows Kits`;
  return {
    INCLUDE: [
      `${msvc}\\include`, `${msvc}\\ATLMFC\\include`,
      String.raw`C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\VS\include`,
      `${kits}\\10\\include\\10.0.26100.0\\ucrt`,
      `${kits}\\10\\include\\10.0.26100.0\\um`,
      `${kits}\\10\\include\\10.0.26100.0\\shared`,
      `${kits}\\10\\include\\10.0.26100.0\\winrt`,
      `${kits}\\10\\include\\10.0.26100.0\\cppwinrt`,
      `${kits}\\NETFXSDK\\4.8\\include\\um`,
    ].join(';'),
    LIB: [
      `${msvc}\\ATLMFC\\lib\\x64`, `${msvc}\\lib\\x64`,
      `${kits}\\NETFXSDK\\4.8\\lib\\um\\x64`,
      `${kits}\\10\\lib\\10.0.26100.0\\ucrt\\x64`,
      `${kits}\\10\\lib\\10.0.26100.0\\um\\x64`,
    ].join(';'),
    LIBPATH: [
      `${msvc}\\ATLMFC\\lib\\x64`, `${msvc}\\lib\\x64`,
      `${msvc}\\lib\\x86\\store\\references`,
      `${kits}\\10\\UnionMetadata\\10.0.26100.0`,
      `${kits}\\10\\References\\10.0.26100.0`,
      String.raw`C:\Windows\Microsoft.NET\Framework64\v4.0.30319`,
    ].join(';'),
    ZRYNA_QUALIFICATION_PATH: [
      `${msvc}\\bin\\Hostx64\\x64`, String.raw`C:\Windows\System32`, String.raw`C:\Windows`,
    ].join(';'),
    SystemRoot: String.raw`C:\Windows`,
  };
}

test('qualification binds bounded expected Windows developer path identities', () => {
  const environment = windowsDeveloperEnvironment();
  assert(environment.INCLUDE.length > 512 && environment.INCLUDE.length <= 1024);
  assert.deepEqual(qualificationHostEnvironment('x86_64-pc-windows-msvc', environment), {
    INCLUDE: environment.INCLUDE,
    LIB: environment.LIB,
    LIBPATH: environment.LIBPATH,
    PATH: environment.ZRYNA_QUALIFICATION_PATH,
    SystemRoot: environment.SystemRoot,
  });
  assert.deepEqual(qualificationHostEnvironment('x86_64-unknown-linux-gnu', environment), {});

  const unexpected = windowsDeveloperEnvironment();
  unexpected.INCLUDE = unexpected.INCLUDE.replace(/[^;]+$/, String.raw`C:\temp\include`);
  assert.throws(() => qualificationHostEnvironment('x86_64-pc-windows-msvc', unexpected),
    /Windows INCLUDE environment differs/);
  const traversal = windowsDeveloperEnvironment();
  traversal.LIB = traversal.LIB.replace('ATLMFC\\lib', 'ATLMFC\\unused\\..\\lib');
  assert.throws(() => qualificationHostEnvironment('x86_64-pc-windows-msvc', traversal),
    /Windows LIB environment differs/);
  const control = windowsDeveloperEnvironment();
  control.LIB += '\n';
  assert.throws(() => qualificationHostEnvironment('x86_64-pc-windows-msvc', control),
    /Windows LIB environment differs/);
  for (const systemRoot of [String.raw`C:\temp\..\Windows`, 'C:/Windows']) {
    const noncanonical = windowsDeveloperEnvironment();
    noncanonical.SystemRoot = systemRoot;
    assert.throws(() => qualificationHostEnvironment('x86_64-pc-windows-msvc', noncanonical),
      /Windows SystemRoot environment differs/);
  }
  const unbounded = windowsDeveloperEnvironment();
  unbounded.INCLUDE = unbounded.INCLUDE.replace('14.44.35207', `14.${'4'.repeat(1024)}`);
  assert.throws(() => qualificationHostEnvironment('x86_64-pc-windows-msvc', unbounded),
    /Windows INCLUDE environment differs/);
});

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { cleanupBrowserRuntime, requireExecutedProof, requireExactCandidate } from '../scripts/scalar-host/run-conformance.mjs';
import { inventory, parseCanonical } from '../scripts/m4/registry-files.mjs';

const name = 'fixture::real_browser';
const success = `test ${name} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 9 filtered out;\n`;
const result = stdout => ({ status: 0, signal: null, stdout });

test('accepts an actual exact test receipt including Windows line endings', () => {
  assert.deepEqual(requireExecutedProof(result(success.replaceAll('\n', '\r\n')), name),
    { test: name, passed: 1, failed: 0, ignored: 0 });
});

test('rejects a successful process that listed, filtered out or ignored the fixture', () => {
  for (const text of ['', `test ${name} ... ignored\ntest result: ok. 0 passed; 0 failed; 1 ignored;`,
    'test result: ok. 0 passed; 0 failed; 0 ignored;', success.replace(name, 'unrelated')]) {
    assert.throws(() => requireExecutedProof(result(text), name));
  }
});

test('a printed pass cannot hide another summary, a failing exit, timeout or signal', () => {
  assert.throws(() => requireExecutedProof(result(`${success}test result: ok. 0 passed; 0 failed; 0 ignored;`), name));
  for (const failure of [{ status: 101 }, { error: new Error('timeout') }, { signal: 'SIGTERM' }]) {
    assert.throws(() => requireExecutedProof({ ...result(success), ...failure }, name));
  }
});

test('rejects changed commits, tracked changes and untracked files in exact-head evidence', () => {
  const commit = 'a'.repeat(40);
  assert.equal(requireExactCandidate(result(`${commit}\r\n`), result(''), commit), commit);
  for (const [identity, state] of [[result('b'.repeat(40)), result('')],
    [result(`${commit}\n`), result(' M fixture.mjs\n')],
    [result(`${commit}\n`), result('?? extra.mjs\n')], [result('not-a-commit'), result('')]]) {
    assert.throws(() => requireExactCandidate(identity, state, commit));
  }
});

test('candidate inspection cannot succeed through a failing process, timeout or signal', () => {
  const identity = result('a'.repeat(40));
  for (const failure of [{ status: 1 }, { error: new Error('timeout') }, { signal: 'SIGTERM' }]) {
    assert.throws(() => requireExactCandidate({ ...identity, ...failure }, result('')));
    assert.throws(() => requireExactCandidate(identity, { ...result(''), ...failure }));
  }
});

function runtimeFixture(t) {
  const root = mkdtempSync(path.join(realpathSync(os.tmpdir()), 'zryna-browser-retention-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const evidence = path.join(root, 'browser');
  mkdirSync(evidence);
  for (const name of ['config', 'data']) mkdirSync(path.join(evidence, name));
  const crash = path.join(evidence, 'config', 'google-chrome-for-testing', 'Crash Reports');
  mkdirSync(crash, { recursive: true });
  writeFileSync(path.join(crash, 'settings.dat'), 'actual transient path shape');
  writeFileSync(path.join(evidence, 'conformance.json'), '{"diagnostic":"preserve"}\n');
  writeFileSync(path.join(evidence, 'component.stdout.log'), 'actual proof bytes\n');
  const victim = path.join(root, 'outside-victim');
  mkdirSync(victim);
  writeFileSync(path.join(victim, 'sentinel'), 'outside bytes');
  return { root, evidence, victim };
}

test('successful owned runtime cleanup leaves exact receipt/log bytes and a portable upload inventory', t => {
  const { root, evidence, victim } = runtimeFixture(t);
  const receipt = readFileSync(path.join(evidence, 'conformance.json'));
  const proof = readFileSync(path.join(evidence, 'component.stdout.log'));
  assert.throws(() => inventory(root, 'browser'), /unsafe portable path/);
  cleanupBrowserRuntime(evidence, true);
  assert.deepEqual(inventory(root, 'browser'), ['browser/component.stdout.log', 'browser/conformance.json']);
  assert.deepEqual(readFileSync(path.join(evidence, 'conformance.json')), receipt);
  assert.deepEqual(readFileSync(path.join(evidence, 'component.stdout.log')), proof);
  assert.equal(readFileSync(path.join(victim, 'sentinel'), 'utf8'), 'outside bytes');
  assert(!existsSync(path.join(evidence, 'config')));
  assert(!existsSync(path.join(evidence, 'data')));
});

test('unsuccessful or incomplete browser execution preserves transient diagnostics', t => {
  const { evidence } = runtimeFixture(t);
  const diagnostic = path.join(evidence, 'config', 'google-chrome-for-testing', 'Crash Reports', 'settings.dat');
  const before = readFileSync(diagnostic);
  cleanupBrowserRuntime(evidence);
  cleanupBrowserRuntime(evidence, false);
  cleanupBrowserRuntime(evidence, 1);
  cleanupBrowserRuntime(evidence, 'success');
  assert.deepEqual(readFileSync(diagnostic), before);
  assert(existsSync(path.join(evidence, 'data')));
});

test('linked owned root rejects before either tree is removed and preserves its outside target', t => {
  const { evidence, victim } = runtimeFixture(t);
  rmSync(path.join(evidence, 'data'), { recursive: true });
  symlinkSync(victim, path.join(evidence, 'data'), process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => cleanupBrowserRuntime(evidence, true), /linked or non-directory browser runtime root/);
  assert(existsSync(path.join(evidence, 'config', 'google-chrome-for-testing', 'Crash Reports', 'settings.dat')));
  assert.equal(readFileSync(path.join(victim, 'sentinel'), 'utf8'), 'outside bytes');
});

test('regular-file and FIFO runtime roots reject without removing diagnostics', t => {
  const { evidence } = runtimeFixture(t);
  const data = path.join(evidence, 'data');
  rmSync(data, { recursive: true });
  writeFileSync(data, 'not a directory');
  assert.throws(() => cleanupBrowserRuntime(evidence, true), /non-directory browser runtime root/);
  assert(existsSync(path.join(evidence, 'config')));
  if (process.platform !== 'win32') {
    rmSync(data);
    const fifo = spawnSync('mkfifo', [data], { shell: false, encoding: 'utf8' });
    assert.equal(fifo.status, 0, fifo.stderr);
    assert.throws(() => cleanupBrowserRuntime(evidence, true), /non-directory browser runtime root/);
    assert(existsSync(path.join(evidence, 'config')));
  }
});

test('linked evidence ancestry rejects cleanup without touching either owned tree', t => {
  const { root, evidence } = runtimeFixture(t);
  const alias = path.join(root, 'alias');
  symlinkSync(evidence, alias, process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => cleanupBrowserRuntime(alias, true), /browser evidence ancestor/);
  assert(existsSync(path.join(evidence, 'config')));
  assert(existsSync(path.join(evidence, 'data')));
});

test('nested runtime link is removed without following it into an outside victim', t => {
  const { evidence, victim } = runtimeFixture(t);
  symlinkSync(victim, path.join(evidence, 'config', 'outside-link'), process.platform === 'win32' ? 'junction' : 'dir');
  cleanupBrowserRuntime(evidence, true);
  assert.equal(readFileSync(path.join(victim, 'sentinel'), 'utf8'), 'outside bytes');
});

test('cleanup leaves other evidence names for the unchanged strict inventory and canonical guards', t => {
  const { root, evidence } = runtimeFixture(t);
  writeFileSync(path.join(evidence, 'unexpected extra.json'), '{}\n');
  writeFileSync(path.join(evidence, 'unexpected.log'), 'retained hostile extra');
  cleanupBrowserRuntime(evidence, true);
  assert(existsSync(path.join(evidence, 'unexpected.log')));
  assert.throws(() => inventory(root, 'browser'), /unsafe portable path/);
  assert.throws(() => parseCanonical(Buffer.from('{\r\n  "extra": true\r\n}\r\n')), /not canonical JSON/);
});

test('Python receipt producer emits native canonical UTF-8 LF bytes despite Windows text-newline translation', t => {
  const { evidence } = runtimeFixture(t);
  const output = path.join(evidence, 'acquisition.json');
  const producer = path.resolve(import.meta.dirname, '../scripts/scalar-host/acquire-browser.py');
  const script = [
    'import sys, pathlib, importlib.util',
    'sys.dont_write_bytecode = True',
    'spec = importlib.util.spec_from_file_location("acquisition", sys.argv[1])',
    'module = importlib.util.module_from_spec(spec)',
    'spec.loader.exec_module(module)',
    'def windows_text_write(path, text, *args, **kwargs):',
    '    return path.write_bytes(text.replace("\\n", "\\r\\n").encode("utf-8"))',
    'pathlib.Path.write_text = windows_text_write',
    'module.write_receipt(pathlib.Path(sys.argv[2]), {"state": "fixture", "notices": ["caf\\u00e9"], "archiveBytes": 123})',
  ].join('\n');
  const result = spawnSync(process.platform === 'win32' ? 'python' : 'python3', ['-c', script, producer, output],
    { shell: false, encoding: 'utf8', timeout: 30_000 });
  assert.equal(result.status, 0, result.stderr);
  const bytes = readFileSync(output);
  assert.deepEqual(parseCanonical(bytes), { state: 'fixture', notices: ['caf\u00e9'], archiveBytes: 123 });
  assert(!bytes.includes(13), 'physical bytes contain no CRLF translation');
  assert(bytes.subarray(-1).equals(Buffer.from('\n')));
});

import assert from 'node:assert/strict';
import test from 'node:test';
import os from 'node:os';
import path from 'node:path';
import { existsSync, mkdtempSync, mkdirSync, readFileSync, realpathSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { digest, parseCanonical } from '../scripts/m4/registry-files.mjs';
import { retained, exact, MAX_LOG, processResult } from '../scripts/m4/evidence-input.mjs';
import { rustAccounting, requireNodeTests } from '../scripts/m4/evidence-counts.mjs';
import { m1Accounting, m2RetainedAccounting, m2RetainedProof, foundationStages } from '../scripts/m4/evidence-foundations.mjs';
import { sourceAccounting, sourceCommands } from '../scripts/m4/evidence-source.mjs';
import { browserArgv, browserProofs } from '../scripts/m4/evidence-browser.mjs';
import { installedNames, readInstalledArchitectureReceipt } from '../scripts/m4/evidence-installed.mjs';
import { bytes as distributionBytes } from '../scripts/distribution/canonical.mjs';
import { ClosedEvidenceStage, retainedOutputFiles, stageM4Evidence } from '../scripts/m4/collect-output.mjs';
import { positive, invocationArgv } from '../scripts/m4/installed-cases.mjs';
import { installedByteCases, installedByteSource, validateInstalledByteSource, validateInstalledByteDenial } from '../scripts/m4/installed-byte-boundary.mjs';
import { prepareByteSources, observedByteSource, verifyByteOutcome } from './installed-command-qualification/byte-boundary.mjs';
import { producedJson, validateInstalledObservation } from '../scripts/m4/installed-observation.mjs';
import { composeM4Evidence, selectM4Gates, requireM4ResultInventory, resolveM4Argv, normalizeM4Host, loadM4Evidence, validateM4Evidence } from '../scripts/m4/evidence.mjs';
import { loadAndValidateM4Conformance, workspaceRoot, expectedRegistrySha256 } from '../scripts/check-m4-conformance.mjs';
import { resolvePnpmScript } from '../scripts/m4/observe-materials.mjs';

const selector = 'public_boundary::exact_limit_then_first_extra';
const actual = `running 1 test\ntest ${selector} ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.02s\n`;
test('exact Rust proof accounts named nonignored row independently of summary flags', () => {
  assert.deepEqual(rustAccounting(actual, selector), { passed: 1, ignored: 0, names: [selector] });
});
for (const [name, output] of [
  ['zero rows', 'running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.01s\n'],
  ['ignored selected proof', actual.replace('... ok', '... ignored').replace('1 passed; 0 failed; 0 ignored', '0 passed; 0 failed; 1 ignored')],
  ['wrong selected name', actual.replace(selector, 'other_proof')],
  ['duplicate named row', actual.replace('\n\ntest result:', `\ntest ${selector} ... ok\n\ntest result:`)],
  ['extra summary', actual + actual],
  ['summary only', actual.slice(actual.indexOf('test result:'))],
  ['row only', actual.slice(0, actual.indexOf('test result:'))],
  ['failed named proof', actual.replace('... ok', '... FAILED')],
  ['pass counter inflated', actual.replace('1 passed', '2 passed')],
  ['measured benchmark', actual.replace('0 measured', '1 measured')],
  ['wrong running count', actual.replace('running 1 test', 'running 0 tests')],
]) test(`hostile Rust evidence rejects ${name}`, () => assert.throws(() => rustAccounting(output, selector)));

test('full-suite accounting retains executed and ignored outcomes separately', () => {
  const output = `${actual}running 2 tests\ntest another ... ok\ntest expensive ... ignored, Linux only\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s\n`;
  assert.equal(rustAccounting(output).passed, 2);
  assert.equal(rustAccounting(output).ignored, 1);
});
function nodeRow(number, name, { depth = 0, type = 'test', skip = false, children = '', childCount = 0 } = {}) {
  const pad = '    '.repeat(depth);
  const plan = children ? `${pad}    1..${childCount}\n` : '';
  return `${pad}# Subtest: ${name}\n${children}${plan}${pad}ok ${number} - ${name}${skip ? ' # SKIP unsupported host' : ''}\n${pad}  ---\n${pad}  duration_ms: 0.01\n${pad}  type: '${type}'\n${pad}  ...\n`;
}
function nodeReport(rows, plan, { tests, suites = 0, pass, skipped = 0 }) {
  return `TAP version 13\n${rows}1..${plan}\n# tests ${tests}\n# suites ${suites}\n# pass ${pass}\n# fail 0\n# cancelled 0\n# skipped ${skipped}\n# todo 0\n# duration_ms 1.00\n`;
}
const tap = nodeReport(nodeRow(1, 'positive boundary') + nodeRow(2, 'hostile rejection')
  + nodeRow(3, 'unsupported host', { skip: true }), 3, { tests: 3, pass: 2, skipped: 1 });
test('Node TAP evidence accounts actual named rows, suites and explicit skips', () => {
  assert.deepEqual(requireNodeTests(tap), { tests: 3, suites: 0, pass: 2, fail: 0, cancelled: 0, skipped: 1, todo: 0 });
});
const nested = nodeReport(nodeRow(1, 'outer suite', { type: 'suite', childCount: 2,
  children: nodeRow(1, 'positive nested leaf', { depth: 1 }) + nodeRow(2, 'inner suite', { depth: 1, type: 'suite', childCount: 1,
    children: nodeRow(1, 'unsupported nested leaf', { depth: 2, skip: true }) }) })
  + nodeRow(2, 'standalone leaf'), 2, { tests: 3, suites: 2, pass: 2, skipped: 1 });
test('Node TAP nested suite summaries never substitute for genuine passing tests', () => {
  assert.deepEqual(requireNodeTests(nested), { tests: 3, suites: 2, pass: 2, fail: 0, cancelled: 0, skipped: 1, todo: 0 });
  const parent = nodeReport(nodeRow(1, 'parent test', { childCount: 2,
    children: nodeRow(1, 'passing child', { depth: 1 }) + nodeRow(2, 'skipped child', { depth: 1, skip: true }) }),
  1, { tests: 3, pass: 2, skipped: 1 });
  assert.equal(requireNodeTests(parent).tests, 3); // Node22 includes the typed parent test.
  const empty = nodeReport(nodeRow(1, 'skipped suite', { type: 'suite', skip: true }) + nodeRow(2, 'real leaf'),
    2, { tests: 1, suites: 1, pass: 1 });
  assert.deepEqual(requireNodeTests(empty), { tests: 1, suites: 1, pass: 1, fail: 0, cancelled: 0, skipped: 0, todo: 0 });
  const repeated = nodeReport(nodeRow(1, 'parameterized name') + nodeRow(2, 'parameterized name'), 2, { tests: 2, pass: 2 });
  assert.equal(requireNodeTests(repeated).pass, 2); // Sibling indices identify real repetitions.
});
for (const [name, output, error] of [
  ['inflated counters', tap.replace('# tests 3', '# tests 101').replace('# pass 2', '# pass 100'), /named test\/suite\/outcome counter accounting/],
  ['single row with 100 passes', nodeReport(nodeRow(1, 'single'), 1, { tests: 100, pass: 100 }), /named test\/suite\/outcome counter accounting/],
  ['suite summaries counted as tests', nested.replace('# tests 3', '# tests 5').replace('# pass 2', '# pass 4'), /named test\/suite\/outcome counter accounting/],
  ['missing declared leaf', tap.replace(nodeRow(3, 'unsupported host', { skip: true }), ''), /plan\/row mismatch/],
  ['duplicate leaf index', tap.replace('1..3\n', nodeRow(3, 'unsupported host', { skip: true }) + '1..3\n'), /duplicate, missing or reordered sibling result/],
  ['wrong named outcome', tap.replace('ok 1 - positive boundary', 'ok 1 - unrelated'), /named result\/start hierarchy mismatch/],
  ['out-of-order sibling', tap.replace('ok 2 - hostile rejection', 'ok 4 - hostile rejection'), /duplicate, missing or reordered sibling result/],
  ['wrong nested indentation', nested.replace('        ok 1 - unsupported', '    ok 1 - unsupported'), /named result\/start hierarchy mismatch/],
  ['borrowed leaf under skip', nested.replace('ok 1 - outer suite', 'ok 1 - outer suite # SKIP'), /skipped Node TAP parent borrowed executed children/],
  ['missing nested plan', nested.replace('        1..1\n', ''), /nested plan\/row mismatch/],
  ['duplicated root plan', tap.replace('1..3\n', '1..3\n1..3\n'), /duplicate or misplaced plan/],
  ['missing type', tap.replace("  type: 'test'\n", ''), /complete Node TAP result diagnostics/],
  ['unknown type', tap.replace("type: 'test'", "type: 'unknown'"), /explicit Node TAP test\/suite type/],
  ['missing result diagnostics', tap.replace('  ...\n', ''), /complete Node TAP result diagnostics/],
  ['missing root plan', tap.replace('1..3\n', ''), /counters precede completed hierarchy/],
  ['truncated open test', tap.replace('ok 3 - unsupported host # SKIP unsupported host\n  ---\n  duration_ms: 0.01\n  type: \'test\'\n  ...\n', ''), /duplicate or misplaced plan/],
  ['failed result', tap.replace('ok 1 - positive', 'not ok 1 - positive'), /failed or TODO named Node TAP result/],
  ['TODO result', tap.replace('# SKIP unsupported host', '# TODO unsupported host'), /failed or TODO named Node TAP result/],
  ['zero tests', tap.replace('# tests 3', '# tests 0').replace('# pass 2', '# pass 0').replace('# skipped 1', '# skipped 0'), /named test\/suite\/outcome counter accounting/],
  ['failed counter', tap.replace('# fail 0', '# fail 1'), /named test\/suite\/outcome counter accounting/],
  ['cancelled counter', tap.replace('# cancelled 0', '# cancelled 1'), /named test\/suite\/outcome counter accounting/],
  ['TODO counter', tap.replace('# todo 0', '# todo 1'), /named test\/suite\/outcome counter accounting/],
  ['duplicate counter', `${tap}# pass 2\n`, /single complete ordered Node TAP counter inventory/],
  ['truncated counter', tap.replace('# todo 0\n', ''), /single complete ordered Node TAP counter inventory/],
  ['duplicated TAP report', tap + tap, /single initial Node TAP version required/],
  ['missing TAP version', tap.replace('TAP version 13\n', ''), /execution outside complete report/],
  ['truncated counter value', tap.replace('# skipped 1', '# skipped'), /malformed or truncated Node TAP structure/],
  ['missing reported outcome', tap.replace(' # SKIP unsupported host', ''), /named test\/suite\/outcome counter accounting/],
]) test(`hostile Node TAP evidence rejects ${name}`, () => assert.throws(() => requireNodeTests(output), error));
test('all skipped leaves cannot borrow passing parent or suite status', () => {
  const skipped = nodeReport(nodeRow(1, 'skip-only leaf', { skip: true }), 1, { tests: 1, pass: 0, skipped: 1 });
  assert.throws(() => requireNodeTests(skipped), /actual passing named Node leaf tests absent/);
  const parent = nodeReport(nodeRow(1, 'parent test', { childCount: 1,
    children: nodeRow(1, 'skip-only child', { depth: 1, skip: true }) }), 1, { tests: 2, pass: 1, skipped: 1 });
  assert.throws(() => requireNodeTests(parent), /actual passing named Node leaf tests absent/);
});
test('unsupported spec output cannot supply Node execution authority', () => {
  const spec = '▶ suite\n  ✔ genuine leaf (0.01ms)\n✔ suite (0.02ms)\nℹ tests 1\nℹ suites 1\nℹ pass 1\nℹ fail 0\nℹ cancelled 0\nℹ skipped 0\nℹ todo 0\n';
  assert.throws(() => requireNodeTests(spec), /unsupported Node reporter/);
});

function temporary(run) {
  const root = mkdtempSync(path.join(os.tmpdir(), 'm4-hostile-evidence-'));
  try { return run(root); } finally { rmSync(root, { recursive: true, force: true }); }
}
function file(root, relative, content) {
  const bytes = Buffer.from(content);
  mkdirSync(path.dirname(path.join(root, relative)), { recursive: true });
  writeFileSync(path.join(root, relative), bytes);
  return { path: relative, bytes: bytes.length, sha256: digest(bytes) };
}
test('retained logs reject substituted bytes, missing data, traversal and incorrect sizes', () => temporary(root => {
  const descriptor = file(root, 'proof.log', actual);
  assert.deepEqual(retained(descriptor, root), Buffer.from(actual));
  assert.throws(() => retained({ ...descriptor, path: '../proof.log' }, root));
  assert.throws(() => retained({ ...descriptor, bytes: descriptor.bytes - 1 }, root));
  assert.throws(() => retained({ ...descriptor, sha256: '0'.repeat(64) }, root));
  assert.throws(() => retained({ ...descriptor, path: 'missing.log' }, root));
  assert.throws(() => retained({ ...descriptor, bytes: MAX_LOG + 1 }, root));
  writeFileSync(path.join(root, descriptor.path), `${actual}extra\n`);
  assert.throws(() => retained(descriptor, root));
}));
test('no-follow receipt input rejects a linked file and linked ancestor', { skip: process.platform === 'win32' }, () => temporary(root => {
  const descriptor = file(root, 'original/proof.log', actual);
  symlinkSync(path.join(root, descriptor.path), path.join(root, 'linked.log'));
  assert.throws(() => retained({ ...descriptor, path: 'linked.log' }, root));
  symlinkSync(path.join(root, 'original'), path.join(root, 'linked-directory'));
  assert.throws(() => retained({ ...descriptor, path: 'linked-directory/proof.log' }, root));
}));
for (const raw of ['{"passed":true,"passed":false}\n', '{"passed":true}\n{}\n', '\ufeff{"passed":true}\n', '{"passed":true}']) {
  test('noncanonical or duplicate raw receipt bytes reject', () => assert.throws(() => parseCanonical(Buffer.from(raw))));
}
test('closed receipt shape forbids supplied passed, eligible and skip authority', () => {
  for (const authority of ['passed', 'eligible', 'skip', 'waivers']) assert.throws(() => exact({ source: {}, [authority]: true }, ['source'], 'receipt'));
});
test('actual process success rejects timeout, nonzero status and truncated buffers', () => temporary(root => {
  const base = { id: 'actual', argv: ['cargo', 'test'], launchArgv: ['/tool/cargo', 'test'], exitCode: 0,
    signal: null, error: null, elapsedMs: 1, stdout: file(root, 'logs/actual.stdout.log', actual),
    stderr: file(root, 'logs/actual.stderr.log', ''), attachments: [] };
  processResult(base, root, 'logs/actual');
  for (const change of [{ exitCode: 1 }, { signal: 'SIGTERM' }, { error: 'ENOBUFS' }, { error: 'ETIMEDOUT' }]) {
    assert.throws(() => processResult({ ...base, ...change }, root, 'logs/actual'));
  }
}));
test('actual before and after Git logs bind candidate/tree and clean state', () => temporary(root => {
  const source = { commit: '1'.repeat(40), tree: '2'.repeat(40) };
  const states = Object.fromEntries(['before', 'after'].map(phase => [phase,
    Object.fromEntries(Object.entries(sourceCommands).map(([field, argv]) => [field,
      { argv, exitCode: 0, signal: null, error: null,
        stdout: file(root, `source/${phase}-${field}.stdout.log`, field === 'state' ? '' : `${source[field]}\n`),
        stderr: file(root, `source/${phase}-${field}.stderr.log`, '') }]))]));
  sourceAccounting(states, source, root);
  const changed = structuredClone(states);
  changed.after.state.stdout = file(root, 'source/after-state.stdout.log', ' M source.rs\n');
  assert.throws(() => sourceAccounting(changed, source, root));
  const stale = structuredClone(states);
  stale.before.commit.argv = ['git', 'rev-parse', 'HEAD~1'];
  assert.throws(() => sourceAccounting(stale, source, root));
}));
test('browser inventory independently contains public component, sealed corpus and full transport', () => {
  assert.deepEqual(browserProofs.map(proof => proof.label), ['component', 'scalar', 'transport']);
  assert.equal(browserArgv(browserProofs[2]).at(-3), '--ignored');
  assert(browserProofs[2].selector.endsWith('retained_esm_transport_accepts_exact_ceiling_and_rejects_first_extra'));
});
test('installed inventory preserves the original 29 identities and adds both actual authored byte cases', () => {
  assert.equal(installedNames.length, 31);
  assert.equal(new Set(installedNames).size, 31);
  assert.equal(installedNames[23], 'existing-output-preserved');
  assert.deepEqual(installedNames.slice(24, 26), ['component-bytes-exact', 'component-bytes-first-extra']);
  assert.equal(installedNames.at(-1), 'restored-installation');
  assert.equal(positive.size, 9);
  const report = { project: '/ordinary/project', installation: '/external/relocated-install' };
  for (const name of installedNames.slice(24, 26)) {
    assert.deepEqual(invocationArgv(name, report, path.posix), ['run', installedByteCases[name].file,
      '--target', 'wasi-command', '--profile', 'command-h1-v1', '--export', 'main', '--json',
      '--name', name, '--project-root', report.project]);
  }
});
test('installed byte source observations retain the two fixed authored sources and reject mutations', () => temporary(project => {
  prepareByteSources(workspaceRoot, project);
  for (const [name, fixture] of Object.entries(installedByteCases)) {
    const bytes = readFileSync(path.join(project, fixture.file));
    const source = observedByteSource(name, workspaceRoot, project);
    assert.deepEqual(source, { path: fixture.file, bytes: 87161, sha256: fixture.sha256 });
    validateInstalledByteSource({ name, source }, bytes, workspaceRoot);
    for (const change of [{ bytes: 1048576 }, { sha256: '0'.repeat(64) }, { path: '../exact.zry' }, { padding: true }]) {
      assert.throws(() => validateInstalledByteSource({ name, source: { ...source, ...change } }, bytes, workspaceRoot));
    }
    const altered = Buffer.from(bytes); altered[80] ^= 1;
    assert.throws(() => installedByteSource(name, altered, workspaceRoot));
    writeFileSync(path.join(project, fixture.file), altered);
    assert.throws(() => observedByteSource(name, workspaceRoot, project));
  }
  assert.throws(() => prepareByteSources(workspaceRoot, project), /EEXIST/);
}));
function installedByteFailure() {
  return { version: 1, command: 'run', ok: false, manifest: null, results: [], diagnostics: [
    { code: 'ZRYNA-W4102', severity: 'error', primary: { kind: 'global' },
      message: 'command component exceeds 1048576 bytes', guidance: 'reduce the verified command within its component budget' },
  ] };
}
test('installed byte producer requires exact component bytes and unchanged actual source observations', () => temporary(project => {
  prepareByteSources(workspaceRoot, project);
  const name = 'component-bytes-exact';
  const source = observedByteSource(name, workspaceRoot, project);
  const row = { name };
  verifyByteOutcome(row, source, source, Buffer.alloc(1048576));
  assert.deepEqual(row.source, source);
  for (const bytes of [1048575, 1048577]) assert.throws(() => verifyByteOutcome({ name }, source, source, Buffer.alloc(bytes)));
  assert.throws(() => verifyByteOutcome({ name }, source, { ...source, sha256: '0'.repeat(64) }, Buffer.alloc(1048576)));
}));
test('installed first-extra proof rejects fabricated execution, wrong diagnostics, exit codes and bundle publication', () => {
  const row = { name: 'component-bytes-first-extra', exitCode: 3, bundleExists: false };
  validateInstalledByteDenial(row, installedByteFailure());
  for (const change of [{ exitCode: 0 }, { exitCode: 5 }, { bundleExists: true }, { name: 'component-bytes-exact' }]) {
    assert.throws(() => validateInstalledByteDenial({ ...row, ...change }, installedByteFailure()));
  }
  for (const change of [{ execution: null }, { execution: { teardown: 'confirmed' } }, { ok: true },
    { manifest: 'bundle.json' }, { results: [{ kind: 'run-returned' }] }, { diagnostics: [] }]) {
    assert.throws(() => validateInstalledByteDenial(row, { ...installedByteFailure(), ...change }));
  }
  for (const change of [{ code: 'ZRYNA-C4220' }, { message: 'component too large' }, { primary: { kind: 'source' } }]) {
    const failure = installedByteFailure(); failure.diagnostics[0] = { ...failure.diagnostics[0], ...change };
    assert.throws(() => validateInstalledByteDenial(row, failure));
  }
  const source = { path: 'extra.zry', bytes: 87161, sha256: installedByteCases[row.name].sha256 };
  verifyByteOutcome({ ...row, stdout: JSON.stringify(installedByteFailure()) }, source, source);
});
test('lane selection preserves foundation argv and forbids quick/skip lanes', () => {
  const registry = { gates: [
    { id: 'm0-full', argv: ['pnpm', 'm0:check'], hosts: ['linux', 'windows'] },
    { id: 'm1-full', argv: ['pnpm', 'm1:check'], hosts: ['linux', 'windows'] },
    { id: 'm2-full', argv: ['pnpm', 'm2:check'], hosts: ['linux', 'windows'] },
    { id: 'm3-full', argv: ['pnpm', 'm3:check'], hosts: ['linux', 'windows'] },
    { id: 'pinned-browser-three-proof', hosts: ['linux', 'windows'] },
    { id: 'installed-qualification', hosts: ['linux', 'windows'] },
  ] };
  assert.deepEqual(selectM4Gates(registry, 'core', 'linux').slice(0, 4).map(gate => gate.argv),
    [['pnpm', 'm0:check'], ['pnpm', 'm1:check'], ['pnpm', 'm2:check'], ['pnpm', 'm3:check']]);
  assert.deepEqual(selectM4Gates(registry, 'browser', 'windows').map(gate => gate.id), ['pinned-browser-three-proof']);
  assert.throws(() => selectM4Gates(registry, 'quick', 'linux'));
  assert.throws(() => selectM4Gates(registry, 'core', 'darwin'));
});
test('result inventory rejects substitution, duplicate, missing and reordered gates against a valid inventory', () => {
  const gates = [{ id: 'm0-full' }, { id: 'm1-full' }, { id: 'm2-full' }, { id: 'm3-full' }];
  requireM4ResultInventory(structuredClone(gates), gates);
  for (const results of [[...gates, { id: 'extra' }], gates.slice(1), [...gates].reverse(),
    [{ id: 'm0-full' }, { id: 'm0-full' }, ...gates.slice(2)], [{ id: 'quick' }, ...gates.slice(1)]]) {
    assert.throws(() => requireM4ResultInventory(results, gates), /complete fixed ordered gate inventory/);
  }
});
test('caller-supplied authenticated flags cannot fabricate six-way composition', () => {
  const fake = ['linux', 'windows'].flatMap(os => ['core', 'browser', 'installed'].map(lane =>
    ({ host: { os }, lane, executionAuthenticated: true, source: {}, registrySha256: '0'.repeat(64) })));
  assert.throws(() => composeM4Evidence({}, fake), /unvalidated/);
});
test('both exact frozen browser and installed directory placeholders resolve across hosts', () => {
  const browser = { id: 'pinned-browser-three-proof', argv: ['node', 'scripts/scalar-host/run-conformance.mjs',
    '$ABSOLUTE_CREATE_ONLY_BROWSER_EVIDENCE_DIRECTORY'] };
  const installed = { id: 'installed-qualification', argv: ['node', 'tests/installed-command-qualification/run.mjs',
    '$EXTERNAL_CREATE_ONLY_EVIDENCE_DIRECTORY'] };
  assert.equal(resolveM4Argv(browser, '/outside/evidence', 'linux').at(-1), '/outside/evidence/browser');
  assert.equal(resolveM4Argv(installed, 'C:\\runner\\evidence', 'windows').at(-1), 'C:\\runner\\evidence\\installed');
  assert.throws(() => resolveM4Argv({ ...browser, id: 'installed-qualification' }, '/outside/evidence', 'linux'));
  assert.throws(() => resolveM4Argv(browser, 'relative', 'linux'));
  assert.throws(() => resolveM4Argv({ id: 'other', argv: ['node', '$ARBITRARY'] }, '/outside/evidence', 'linux'));
});
const m1Names = ['m1_registry_freezes_the_supported_slice', 'm1_javascript_and_webassembly_match_every_portable_fixture',
  'm1_invalid_source_is_target_independent_and_publishes_nothing', 'm1_bool_host_normalization_matches_without_enabling_bool_source',
  'm1_linux_three_target_results_and_artifacts_match_every_fixture'];
function m1Output(names) {
  return `running ${names.length} tests\n${names.map(name => `test ${name} ... ok\n`).join('')}test result: ok. ${names.length} passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.01s\n`;
}
test('full M1 requires all actual current-host cases and rejects unrelated or omitted target proof', () => {
  const linux = m1Output(m1Names);
  assert.equal(m1Accounting(linux, 'linux').passed, 5);
  const windows = m1Output([...m1Names.slice(0, 4), 'm1_windows_native_and_all_runs_are_rejected_without_a_bundle']);
  assert.equal(m1Accounting(windows, 'windows').passed, 5);
  for (const output of [m1Output(m1Names.slice(0, 4)), linux.replace(m1Names[4], 'm1_unrelated'), windows,
    m1Output([...m1Names, 'm1_unrelated']), linux.replace(m1Names[4], m1Names[0])]) {
    assert.throws(() => m1Accounting(output, 'linux'), /current-host M1 named passing inventory/);
  }
  assert.throws(() => m1Accounting(linux + actual, 'linux'), /current-host M1 named passing inventory/);
  assert.throws(() => m1Accounting(linux.replace(`test ${m1Names[4]} ... ok`, `test ${m1Names[4]} ... ignored`)
    .replace('5 passed; 0 failed; 0 ignored', '4 passed; 0 failed; 1 ignored'), 'linux'), /current-host M1 named passing inventory/);
});
test('Linux-only M2 retained identity preserves Windows zero execution as source-cfg exclusion', () => {
  const linux = actual.replace(selector, m2RetainedProof);
  const windows = 'running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 460 filtered out; finished in 0.00s\n';
  assert.equal(m2RetainedAccounting(linux, 'linux').excluded, false);
  assert.deepEqual(m2RetainedAccounting(windows, 'windows'), { passed: 0, ignored: 0, names: [], excluded: true });
  for (const output of [linux, windows + windows, windows.replace('460 filtered', '0 filtered'), windows.replace('0 passed', '1 passed')]) {
    assert.throws(() => m2RetainedAccounting(output, 'windows'), /exact Windows M2 source-cfg exclusion block required/);
  }
  assert.throws(() => m2RetainedAccounting(windows, 'linux'), /missing actual named Rust rows or summaries/);
  assert.throws(() => m2RetainedAccounting(actual, 'linux'), /exact named Rust proof/);
});
test('registry Windows win32 host maps explicitly to authenticated windows receipt OS', () => {
  assert.equal(normalizeM4Host('win32'), 'windows');
  assert.equal(normalizeM4Host('windows'), 'windows');
  assert.equal(normalizeM4Host('linux'), 'linux');
  assert.throws(() => normalizeM4Host('darwin'));
});
test('foundation stage boundaries preserve numbered order and exact host completion', () => {
  const output = `[m2 1/3] first\n${actual}[m2 2/3] second\n${actual}[m2 3/3] third\n${actual}M2 conformance passed: 3 ordered gates.\n`;
  const contract = { family: 'm2', expected: ['first', 'second', 'third'], completion: 'M2 conformance passed: 3 ordered gates.' };
  assert.deepEqual(foundationStages(output, contract).map(stage => stage.id), contract.expected);
  for (const hostile of [
    output.replace('[m2 2/3] second', '[m2 2/3] first'),
    output.replace('[m2 2/3] second', '[m2 3/3] second'),
    output.replace('[m2 3/3] third', '[m2 3/2] third'),
    output.replace('[m2 2/3] second', 'second'),
    output + 'M2 conformance passed: 3 ordered gates.\n',
    output + actual,
    output.replace('[m2 2/3] second', '[m2 2/3] second\n[m2 2/3] second'),
  ]) assert.throws(() => foundationStages(hostile, contract));
  const linux = '[M0 1/1] rust-tests\n' + actual + 'M0 conformance passed on linux: 18 commands, 13 registered proof suites.\n';
  const m0 = { family: 'M0', expected: ['rust-tests'], completion: 'M0 conformance passed on windows: 18 commands, 13 registered proof suites.' };
  assert.throws(() => foundationStages(linux, m0), /current-host/);
});
test('M3 requires explicit authorized Windows exclusion markers in their frozen stage positions', () => {
  const suffix = ': unsupported on Windows; Linux evidence required by aggregate';
  const output = `[m3] portable\n${actual}[m3] linux-required${suffix}\nM3 full gate passed on win32/x64.\n`;
  const contract = { family: 'm3', expected: ['portable', 'linux-required'], exclusions: ['linux-required'],
    completion: 'M3 full gate passed on win32/x64.' };
  const stages = foundationStages(output, contract);
  assert.equal(stages[1].excluded, true);
  assert.equal(stages[1].output.trim(), '');
  assert.throws(() => foundationStages(output, { ...contract, exclusions: [] }));
  assert.throws(() => foundationStages(output.replace('portable', `portable${suffix}`), contract));
  assert.throws(() => foundationStages(output.replace('[m3] portable', '[m3] linux-required'), contract));
  assert.throws(() => foundationStages(output.replace('win32/x64', 'linux/x64'), contract));
});
test('stage-local accounting cannot borrow a passing Rust footer from a later required stage', () => {
  const output = `[m2 1/3] contract\nempty\n[m2 2/3] rust-first\nrunning 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n[m2 3/3] rust-second\n${actual}M2 conformance passed: 3 ordered gates.\n`;
  const stages = foundationStages(output, { family: 'm2', expected: ['contract', 'rust-first', 'rust-second'], completion: 'M2 conformance passed: 3 ordered gates.' });
  assert.throws(() => rustAccounting(stages[1].output));
  assert.equal(rustAccounting(stages[2].output).passed, 1);
  assert.throws(() => rustAccounting(actual.replace('running 1 test', 'running 9 tests')));
});
test('produced observation bytes reject duplicate fields and unsupported serialization without authority claims', () => {
  assert.deepEqual(producedJson(Buffer.from('{"kind":"run-returned","runReturn":"ok"}')), { kind: 'run-returned', runReturn: 'ok' });
  for (const value of ['{"kind":"run-returned","kind":"runtime-trap"}', '{"kind":"run-returned"}\n{}',
    '{ "kind": "run-returned" }', '{"kind":"run-returned"}\n']) {
    assert.throws(() => producedJson(Buffer.from(value)));
  }
  assert.throws(() => producedJson(Buffer.from([0xff])));
  assert.throws(() => validateInstalledObservation({ schema: 'zryna.wasi-command-manifest.v1', runtimeAuthority: true }, {}, {}), /closed keys/);
});

function architectureFixture() {
  return { format: 'zryna.source-build-receipt.v1',
    source: { repository: 'https://github.com/zryna/zryna', commit: '1'.repeat(40), tree: '2'.repeat(40) },
    command: ['cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json'],
    toolchain: { channel: '1.97.1', cargoVersion: 'cargo 1.97.1 (c980f4866 2026-06-30)', cargoSha256: '3'.repeat(64),
      rustcVersion: 'rustc 1.97.1 (8bab26f4f 2026-07-14)', rustcSha256: '4'.repeat(64) },
    inputs: ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json']
      .map(logicalPath => ({ logicalPath, size: 1, sha256: '5'.repeat(64) })), report: { diagnostics: [] } };
}
test('installed architecture reads the original typed distribution wire without rewriting its bytes or digest', () => temporary(root => {
  const document = architectureFixture();
  const bytes = distributionBytes(document);
  const descriptor = file(root, 'installed/architecture-receipt.json', bytes);
  assert.throws(() => parseCanonical(bytes), /not canonical JSON/); // Counterfactual prior M4 reader.
  assert.deepEqual(readInstalledArchitectureReceipt(descriptor, root), document);
  assert(readFileSync(path.join(root, descriptor.path)).equals(bytes));
  assert.equal(descriptor.sha256, digest(bytes));
  assert.throws(() => readInstalledArchitectureReceipt({ ...descriptor, sha256: '0'.repeat(64) }, root), /retained digest/);
}));
test('installed architecture native wire rejects duplicates, alternate trivia, malformed UTF-8, oversized and invalid schema records', () => temporary(root => {
  const base = architectureFixture();
  const wire = distributionBytes(base);
  const duplicate = Buffer.from(wire.toString().replace('"format":', '"format":"duplicate","format":'));
  const badDocuments = [
    { ...base, authenticated: true }, { ...base, report: { diagnostics: [{ severity: 'error' }] } },
    { ...base, command: ['cargo', 'check'] }, { ...base, source: { ...base.source, commit: 'main' } },
    { ...base, toolchain: { ...base.toolchain, channel: '1.97.0' } }, { ...base, inputs: [...base.inputs].reverse() },
  ];
  const hostile = [duplicate, Buffer.from(`${JSON.stringify(base, null, 2)}\n`), Buffer.concat([wire, wire]),
    Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), wire]),
    Buffer.from([0xff]), Buffer.alloc(256 * 1024 + 1), ...badDocuments.map(distributionBytes)];
  for (const [index, bytes] of hostile.entries()) {
    const descriptor = file(root, `installed/${index}.json`, bytes);
    assert.throws(() => readInstalledArchitectureReceipt(descriptor, root));
  }
}));

function stagedFixture(descriptors = []) {
  const receipt = { tools: descriptors.map(versionLog => ({ versionLog })), sourceState: {}, results: [] };
  return { receipt, bytes: Buffer.from(`${JSON.stringify(receipt, null, 2)}\n`) };
}
function stagePlan(descriptors) {
  const fixture = stagedFixture(descriptors);
  return retainedOutputFiles(fixture.receipt, fixture.bytes);
}
const stageDescriptor = (name, bytes = 1) => ({ path: name, bytes, sha256: '1'.repeat(64) });
test('closed artifact plan includes only referenced files and the original receipt last within the unchanged ZIP count', () => {
  const selected = [stageDescriptor('logs/actual.stdout.log'), stageDescriptor('observations/node.version.log')];
  const files = stagePlan(selected);
  assert.deepEqual(files.slice(0, -1), selected);
  assert.equal(files.at(-1).path, 'receipt.json');
  assert.equal(stagePlan(Array.from({ length: 255 }, (_, index) => stageDescriptor(`logs/${index}.log`))).length, 256);
  assert.throws(() => stagePlan(Array.from({ length: 256 }, (_, index) => stageDescriptor(`logs/${index}.log`))), /entry count/);
});
test('closed artifact plan rejects path aliases, duplicate identities, parent-file conflicts and unsupported records', () => {
  for (const names of [['logs/a', 'logs/a'], ['logs/A', 'logs/a'], ['receipt.json'], ['RECEIPT.JSON'],
    ['logs', 'logs/a'], ['logs/A', 'logs/a/nested'], ['../outside'], ['logs/../a'], ['logs\\a'], ['/absolute']]) {
    assert.throws(() => stagePlan(names.map(name => stageDescriptor(name))));
  }
  for (const changed of [{ ...stageDescriptor('a'), bytes: -1 }, { ...stageDescriptor('a'), bytes: 1.5 },
    { ...stageDescriptor('a'), sha256: 'wrong' }, { ...stageDescriptor('a'), authority: true }]) {
    assert.throws(() => stagePlan([changed]));
  }
  const fixture = stagedFixture();
  assert.throws(() => retainedOutputFiles(fixture.receipt, Buffer.from(JSON.stringify(fixture.receipt))), /canonical/);
  assert.throws(() => retainedOutputFiles({ ...fixture.receipt, passed: true }, fixture.bytes), /receipt value identity/);
});
test('closed artifact plan requires consistent directory prefix spelling across portable files', () => {
  assert.equal(stagePlan(['logs/A/x', 'logs/A/y', 'logs/B/z'].map(name => stageDescriptor(name))).length, 4);
  for (const names of [['logs/A/x', 'logs/a/y'], ['Logs/a', 'logs/B/c'], ['logs/A/X/a', 'logs/A/x/b']]) {
    assert.throws(() => stagePlan(names.map(name => stageDescriptor(name))), /directory prefix case alias/);
  }
  assert.throws(() => stagePlan(['logs/A', 'logs/a/x'].map(name => stageDescriptor(name))), /parent file conflict/);
});
test('closed artifact plan preserves per-file and aggregate byte ceilings including the original receipt', () => {
  const maximum = 256 * 1024 * 1024;
  assert.throws(() => stagePlan([stageDescriptor('too-large', maximum + 1)]), /file byte bound/);
  assert.throws(() => stagePlan([stageDescriptor('a', maximum), stageDescriptor('b', maximum), stageDescriptor('c', maximum)]), /aggregate byte bound/);
  assert.equal(stagePlan([stageDescriptor('a', maximum), stageDescriptor('b', maximum)]).length, 3);
});
test('closed artifact plan reserves exact conservative ZIP32 overhead and rejects its first extra byte', () => {
  const maximum = 256 * 1024 * 1024;
  const descriptors = [stageDescriptor('a', maximum), stageDescriptor('b', maximum), stageDescriptor('c', maximum - 4096)];
  const fixture = stagedFixture(descriptors);
  const files = retainedOutputFiles(fixture.receipt, fixture.bytes);
  const overhead = 22 + files.reduce((sum, item) => sum + 30 + 46 + 16 + 2 * Buffer.byteLength(item.path, 'utf8'), 0);
  descriptors[2].bytes = 768 * 1024 * 1024 - 2 * maximum - fixture.bytes.length - overhead;
  assert.equal(stagedFixture(descriptors).bytes.length, fixture.bytes.length);
  assert.equal(stagePlan(descriptors).length, 4);
  descriptors[2].bytes++;
  assert.throws(() => stagePlan(descriptors), /ZIP byte bound including supported record overhead/);
});
test('closed stage copies byte-identical selected files create-only without copying native installation/project trees', () => temporary(parent => {
  const root = path.join(parent, 'original');
  mkdirSync(root);
  const descriptor = file(root, 'logs/actual.log', 'actual original log\n');
  file(root, 'installed/private/project.zry', 'private original project');
  const stage = new ClosedEvidenceStage(root);
  try {
    stage.create(); stage.write(descriptor, readFileSync(path.join(root, descriptor.path)));
    assert.equal(readFileSync(path.join(stage.target, descriptor.path), 'utf8'), 'actual original log\n');
    assert(!existsSync(path.join(stage.target, 'installed')));
    assert.throws(() => stage.write(descriptor, readFileSync(path.join(root, descriptor.path))), /EEXIST/);
    assert.throws(() => new ClosedEvidenceStage(root), /already exists/);
  } finally { stage.close(); }
}));
test('closed stage rejects changed source ancestors and linked output directories without redirecting writes', () => temporary(parent => {
  const root = path.join(parent, 'original');
  mkdirSync(root);
  const descriptor = { path: 'logs/actual.log', bytes: 1, sha256: digest(Buffer.from('a')) };
  const stage = new ClosedEvidenceStage(root);
  try {
    stage.create();
    const outside = path.join(parent, 'outside'); mkdirSync(outside);
    symlinkSync(outside, path.join(stage.target, 'logs'), 'junction');
    assert.throws(() => stage.write(descriptor, Buffer.from('a')));
    assert(!existsSync(path.join(outside, 'actual.log')));
    renameSync(root, `${root}-moved`); mkdirSync(root);
    assert.throws(() => stage.unchanged(), /identity changed/);
  } finally { stage.close(); }
}));
test('closed stage rejects descriptor substitutions, traversal and changed retained subdirectories', () => temporary(parent => {
  const root = path.join(parent, 'original'); mkdirSync(root);
  const descriptor = { path: 'logs/first.log', bytes: 1, sha256: digest(Buffer.from('a')) };
  const stage = new ClosedEvidenceStage(root);
  try {
    stage.create();
    assert.throws(() => stage.write(descriptor, Buffer.from('b')), /digest identity/);
    assert.throws(() => stage.write({ ...descriptor, bytes: 2 }, Buffer.from('a')), /bytes identity/);
    assert.throws(() => stage.write({ ...descriptor, path: '../escape' }, Buffer.from('a')), /unsafe portable path/);
    stage.write(descriptor, Buffer.from('a'));
    renameSync(path.join(stage.target, 'logs'), path.join(stage.target, 'old-logs'));
    mkdirSync(path.join(stage.target, 'logs'));
    assert.throws(() => stage.write({ ...descriptor, path: 'logs/next.log' }, Buffer.from('a')), /identity changed/);
    assert(!existsSync(path.join(stage.target, 'logs/next.log')));
  } finally { stage.close(); }
}));
test('closed stage rejects a linked source ancestor and noncanonical evidence path before creation', () => temporary(parent => {
  const root = path.join(parent, 'original'); mkdirSync(root);
  const linked = path.join(parent, 'linked'); symlinkSync(root, linked, 'junction');
  assert.throws(() => new ClosedEvidenceStage(linked), /non-linked directory/);
  assert.throws(() => new ClosedEvidenceStage(`${root}${path.sep}..${path.sep}original`), /canonical absolute/);
  assert(!existsSync(`${linked}-retained`));
}));
test('full native evidence validation is mandatory before any retained stage output is created', async () => {
  const parent = mkdtempSync(path.join(os.tmpdir(), 'm4-stage-native-'));
  try {
    const root = path.join(parent, 'original'); mkdirSync(root);
    const base = envelopeFixture(root);
    file(root, 'receipt.json', `${JSON.stringify(base.receipt, null, 2)}\n`);
    const registry = loadAndValidateM4Conformance();
    await assert.rejects(stageM4Evidence(base.receipt, { root, sourceRoot: workspaceRoot, registry, expected: base.expected }),
      /complete fixed ordered gate inventory/);
    assert(!existsSync(`${root}-retained`));
  } finally { rmSync(parent, { recursive: true, force: true }); }
});
test('receipt loader rejects noncanonical, duplicate, oversize and malformed UTF-8 inputs before composition', async () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'm4-raw-receipt-'));
  try {
    for (const [index, bytes] of [Buffer.from('{"format":"a","format":"b"}\n'),
      Buffer.from('{"format":"a"}\n{}\n'), Buffer.from([0xff, 0xfe]), Buffer.alloc(2 * 1024 * 1024 + 1)].entries()) {
      const filename = path.join(root, `${index}.json`);
      writeFileSync(filename, bytes);
      await assert.rejects(loadM4Evidence(filename, {}));
    }
  } finally { rmSync(root, { recursive: true, force: true }); }
});

function envelopeFixture(root) {
  const source = { commit: '1'.repeat(40), tree: '2'.repeat(40), ref: 'refs/heads/review-candidate' };
  const sourceState = Object.fromEntries(['before', 'after'].map(phase => [phase,
    Object.fromEntries(Object.entries(sourceCommands).map(([field, argv]) => [field,
      { argv, exitCode: 0, signal: null, error: null,
        stdout: file(root, `source/${phase}-${field}.stdout.log`, field === 'state' ? '' : `${source[field]}\n`),
        stderr: file(root, `source/${phase}-${field}.stderr.log`, '') }]))]));
  const tools = ['node', 'rustc', 'cargo', 'pnpm'].map(name => {
    const version = name === 'node' ? '22.22.1' : name === 'pnpm' ? '11.18.0' : '1.97.1';
    const executable = { path: `/observed/${name}`, bytes: 1, sha256: '3'.repeat(64) };
    return { name, version, executable, versionArgv: name === 'pnpm' ? ['/observed/node', executable.path, '--version'] : [executable.path, '--version'],
      versionLog: file(root, `observations/${name}.version.log`, name === 'node' ? `v${version}\n`
        : name === 'pnpm' ? `${version}\n` : `${name} ${version} (observed)\n`) };
  });
  const host = { os: 'linux', architecture: 'x86_64', distribution: 'ubuntu', version: '24.04',
    kernel: 'observed-kernel', runnerImage: 'ubuntu24', runnerImageVersion: 'observed-image-version' };
  const materials = ['typescript-wrapper', 'typescript-implementation', 'cargo-lock', 'runtime-material-binding']
    .map(name => ({ name, path: `/observed/${name}`, bytes: 1, sha256: '4'.repeat(64) }));
  const workflow = { repository: 'zryna/zryna', runId: 1, attempt: 1, jobName: 'M4 core (ubuntu-latest)', jobKey: 'm4',
    workflowPath: '.github/workflows/ci.yml', workflowSha256: '5'.repeat(64), event: 'pull_request',
    headCommit: '6'.repeat(40), executionCommit: '6'.repeat(40), checkoutCommit: source.commit };
  const evidenceDirectory = '/observed/m4-envelope';
  const expected = { source, registrySha256: expectedRegistrySha256, evidenceDirectory, host, lane: 'core', workflow, tools, materials };
  const receipt = { format: 'zryna.m4-evidence.v1', registrySha256: expectedRegistrySha256, evidenceDirectory,
    source: { ...source, cleanBefore: true, cleanAfter: true }, sourceState, host, lane: 'core', workflow, tools, materials, results: [] };
  return { expected, receipt };
}
test('hostile complete envelopes cannot change external candidate, image, workflow attempt, tools or gate inventory', async () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'm4-envelope-'));
  try {
    // The fixture is synthetic rejection input and never supplies a conformance certificate.
    const registry = loadAndValidateM4Conformance();
    const base = envelopeFixture(root);
    const invoke = receipt => validateM4Evidence(receipt, { root, sourceRoot: workspaceRoot, registry, expected: base.expected });
    await assert.rejects(invoke(base.receipt), /complete fixed ordered gate inventory/);
    for (const [change, error] of [
      [receipt => { receipt.passed = true; }, /receipt closed keys/],
      [receipt => { receipt.eligible = true; }, /receipt closed keys/],
      [receipt => { receipt.registrySha256 = '0'.repeat(64); }, /exact frozen receipt registry binding/],
      [receipt => { receipt.source.commit = '7'.repeat(40); }, /external candidate source binding/],
      [receipt => { receipt.source.tree = '8'.repeat(40); }, /external candidate source binding/],
      [receipt => { receipt.source.ref = 'refs/heads/main'; }, /external candidate source binding/],
      [receipt => { receipt.source.cleanAfter = false; }, /clean source after execution/],
      [receipt => { receipt.host.runnerImageVersion = 'other-image'; }, /externally authenticated actual host/],
      [receipt => { receipt.host.os = 'windows'; }, /externally authenticated actual host/],
      [receipt => { receipt.workflow.attempt = 2; }, /externally authenticated workflow/],
      [receipt => { receipt.workflow.checkoutCommit = receipt.workflow.headCommit; }, /actual candidate checkout/],
      [receipt => { receipt.workflow.executionCommit = '9'.repeat(40); }, /externally authenticated workflow/],
      [receipt => { receipt.tools[0].executable.sha256 = '0'.repeat(64); }, /externally bound tool observations/],
      [receipt => { receipt.materials = []; }, /externally bound material observations/],
    ]) {
      const receipt = structuredClone(base.receipt);
      change(receipt);
      await assert.rejects(invoke(receipt), error);
    }
  } finally { rmSync(root, { recursive: true, force: true }); }
});

function pnpmLayout(root) {
  // Synthetic discovery fixture shaped by the retained actual action self-update; no execution claim.
  const home = path.join(root, 'node_modules/.bin');
  const packageRoot = path.join(home, 'store/v11/links/pnpm/11.18.0/package/node_modules/pnpm');
  const packageAlias = path.join(home, 'global/v11/unique-update/node_modules/pnpm');
  mkdirSync(path.join(packageRoot, 'bin'), { recursive: true });
  mkdirSync(path.dirname(packageAlias), { recursive: true });
  symlinkSync(packageRoot, packageAlias, 'junction');
  const manifest = path.join(packageRoot, 'package.json');
  writeFileSync(manifest, JSON.stringify({ name: 'pnpm', version: '11.18.0', bin: { pnpm: 'bin/pnpm.mjs' } }));
  const cjs = path.join(packageRoot, 'bin/pnpm.cjs'), mjs = path.join(packageRoot, 'bin/pnpm.mjs');
  writeFileSync(cjs, "#!/usr/bin/env node\n// CommonJS shim for compatibility with older Corepack versions\nimport('./pnpm.mjs')\n");
  writeFileSync(mjs, 'throw new Error("discovery must never execute this fixture");\n');
  const target = path.join(packageAlias, 'bin/pnpm.mjs');
  const launcher = path.join(home, 'pnpm');
  const marker = `# cmd-shim-target=${target.replaceAll('\\', '/')}\n`;
  writeFileSync(launcher, `#!/bin/sh\nexit 99\n${marker}`);
  return { home, manifest, cjs, mjs, target, launcher, marker };
}
test('pnpm direct MJS and compatibility CJS launchers preserve the independently pinned CJS identity', () => temporary(root => {
  const layout = pnpmLayout(root);
  for (const launcher of [layout.mjs, layout.cjs]) assert.equal(resolvePnpmScript(launcher), realpathSync(layout.cjs));
  assert.equal(digest(readFileSync(layout.cjs)), '67b035e322203961795e8e34ca63a08c37a4386eda94107fb3d28f3246d882ad');
}));
test('pnpm managed Linux global/store layout resolves the unique target without invoking its shell launcher', () => temporary(root => {
  const layout = pnpmLayout(root);
  assert.equal(resolvePnpmScript(layout.launcher), realpathSync(layout.cjs));
}));
test('pnpm Windows cmd family reads the generated extensionless sidecar and retains direct CJS invocation', () => temporary(root => {
  const layout = pnpmLayout(root), cmd = path.join(layout.home, 'pnpm.cmd');
  writeFileSync(cmd, '@SETLOCAL\r\n@node "%~dp0\\global\\v11\\unique-update\\node_modules\\pnpm\\bin\\pnpm.mjs" %*\r\n');
  assert.equal(resolvePnpmScript(cmd), realpathSync(layout.cjs));
}));
for (const [name, mutate, error] of [
  ['duplicate metadata', layout => writeFileSync(layout.launcher, layout.marker + layout.marker), /one absolute/],
  ['relative metadata', layout => writeFileSync(layout.launcher, '# cmd-shim-target=relative/pnpm/bin/pnpm.mjs\n'), /one absolute/],
  ['unknown metadata', layout => writeFileSync(layout.launcher, '# unknown-target=pnpm.mjs\n'), /one absolute/],
  ['malformed metadata', layout => writeFileSync(layout.launcher, 'exec cmd-shim-target=pnpm.mjs\n'), /malformed/],
  ['oversize metadata', layout => writeFileSync(layout.launcher, Buffer.alloc(64 * 1024 + 1)), /byte limit/],
  ['unsupported JS target', layout => writeFileSync(layout.launcher, `# cmd-shim-target=${path.join(layout.home, 'other.mjs')}\n`), /unsupported direct/],
  ['wrong package name', layout => writeFileSync(layout.manifest, '{"name":"other","version":"11.18.0","bin":{"pnpm":"bin/pnpm.mjs"}}'), /exact pinned/],
  ['wrong package version', layout => writeFileSync(layout.manifest, '{"name":"pnpm","version":"11.19.0","bin":{"pnpm":"bin/pnpm.mjs"}}'), /exact pinned/],
  ['wrong package bin', layout => writeFileSync(layout.manifest, '{"name":"pnpm","version":"11.18.0","bin":{"pnpm":"bin/other.mjs"}}'), /exact pinned/],
  ['missing CJS sibling', layout => rmSync(layout.cjs), /ENOENT/],
  ['missing MJS sibling', layout => { writeFileSync(layout.launcher, `# cmd-shim-target=${layout.cjs.replaceAll('\\', '/')}\n`); rmSync(layout.mjs); }, /ENOENT/],
  ['nonregular CJS sibling', layout => { rmSync(layout.cjs); mkdirSync(layout.cjs); }, /regular direct/],
  ['nonregular MJS sibling', layout => { rmSync(layout.mjs); mkdirSync(layout.mjs); }, /regular direct/],
]) test(`pnpm discovery rejects ${name} before any launcher execution`, () => temporary(root => {
  const layout = pnpmLayout(root); mutate(layout); assert.throws(() => resolvePnpmScript(layout.launcher), error);
}));

import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash } from 'node:crypto';
import { browserByteBoundaryAccounting } from '../scripts/m4/evidence-browser-byte-boundary.mjs';

// Explicit protocol fixture, not a producer summary or hosted execution receipt.
const record = {
  schema: 'zryna.browser-byte-boundary-observation.v1',
  sourceSha256: 'ef631a1b99fbf55bff698e50c0bd336f1ba32b1ead1f44c9db2f047e3534ecc8',
  firstExtraSourceSha256: '4a7955dc78e5a66ded0b11671914993e4ac20c151f1909c4db0399a74dc1e306',
  sourceBytes: 309364, firstExtraSourceBytes: 309365, functions: 1863,
  world: 'zryna:capability-profiles/browser@0.1.0',
  witSourceSha256: '72e017162d5080960589e749c8b5a4f662e5f604bf317e7f7adcd085246b3a24',
  coreSha256: '5877a25cc493884b22885d1e1a3a68c2d171ce80ee4ae0e4cd7e1e6e48590b8a',
  interfaceSha256: '3557a78a99ec1cf774ee360c879f0484de5592b2d76bbb59fec8616015969c05',
  componentBytes: 1048576,
  componentSha256: '128b13f47a9195a2e927bf6cdea34588a7194656ad8b265b8e346c8c43d93c97',
  firstExtraComponentBytes: 1048577,
  firstExtraComponentSha256: 'c8345ef73d90facd7f49cbe520588e146c830cd034430504e448a1c29dd90cd6',
  factory: { exactAccepted: true, replayIdentical: true, firstExtraExitCode: 4,
    firstExtraRejection: 'ZRYNA-W4021', firstExtraBundleAbsent: true, ordinaryRecovery: true },
  node: { executedExports: 1863, wideReturns: 106, narrowReturns: 1757,
    firstExtraRejection: 'ZRYNA-B3991', ordinaryRecovery: 42 },
  browser: { executedExports: 1863, wideReturns: 106, narrowReturns: 1757,
    firstExtraRejection: 'ZRYNA-B3991', ordinaryRecovery: 42 },
  cleanup: { browserConfirmed: true, childExitCode: 0 },
};
const marker = 'browserByteBoundary=';
const selector = 'browser_component_pinned_real_browser_and_node_agree';
const row = `test ${selector} ... ok`;
const footer = 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 120 filtered out; finished in 1.25s';
const output = value => `running 1 test\n${marker}${JSON.stringify(value)}\n${row}\n\n${footer}\n`;

test('browser byte accounting requires the complete frontend, Node, Chrome and cleanup observation', () => {
  assert.deepEqual(browserByteBoundaryAccounting(output(record)), record);
});
test('browser source hashes independently bind 1863 ordinary functions and the next source literal', () => {
  for (const [wide, bytesKey, shaKey] of [[106, 'sourceBytes', 'sourceSha256'],
    [107, 'firstExtraSourceBytes', 'firstExtraSourceSha256']]) {
    let source = '';
    for (let index = 0; index < 1863; index++) {
      const name = `f${index.toString(16).padStart(4, '0')}${'x'.repeat(123)}`;
      assert.equal(name.length, 128);
      source += `export function ${name}(): i32 { return ${index < wide ? 64 : 0}; }\n`;
    }
    assert.equal(Buffer.byteLength(source), record[bytesKey]);
    assert.equal(createHash('sha256').update(source).digest('hex'), record[shaKey]);
  }
});
for (const key of Object.keys(record).filter(key => !['factory', 'node', 'browser', 'cleanup'].includes(key))) {
  test(`browser accounting rejects changed source/factory identity ${key}`, () => {
    const changed = structuredClone(record);
    changed[key] = typeof changed[key] === 'number' ? changed[key] + 1 : 'substituted';
    assert.throws(() => browserByteBoundaryAccounting(output(changed)));
  });
}
for (const section of ['factory', 'node', 'browser', 'cleanup']) {
  for (const key of Object.keys(record[section])) {
    test(`browser accounting rejects changed ${section}/${key}`, () => {
      const changed = structuredClone(record);
      const value = changed[section][key];
      changed[section][key] = typeof value === 'boolean' ? !value
        : typeof value === 'number' ? value + 1 : 'substituted';
      assert.throws(() => browserByteBoundaryAccounting(output(changed)));
    });
  }
  test(`browser accounting rejects extra ${section} producer certification`, () => {
    const changed = structuredClone(record);
    changed[section].passed = true;
    assert.throws(() => browserByteBoundaryAccounting(output(changed)));
  });
}
for (const [name, mutate] of [
  ['missing marker', text => text.split('\n').filter(line => !line.startsWith(marker)).join('\n')],
  ['duplicate marker', text => text.replace(row, `${marker}${JSON.stringify(record)}\n${row}`)],
  ['outside passing block', text => `${marker}${JSON.stringify(record)}\n${text.replace(`${marker}${JSON.stringify(record)}\n`, '')}`],
  ['after passing row', text => text.replace(`${marker}${JSON.stringify(record)}\n`, '').replace(row, `${row}\n${marker}${JSON.stringify(record)}`)],
  ['namespace borrowing', text => text.replace(row, `test other::${selector} ... ok`)],
  ['zero selected execution', text => text.replace('running 1 test', 'running 0 tests').replace('1 passed', '0 passed')],
  ['ignored selected test', text => text.replace(row, `test ${selector} ... ignored`)],
  ['duplicate JSON key', text => text.replace(`${marker}{`, `${marker}{"functions":1,`)],
  ['alternate number syntax', text => text.replace('"componentBytes":1048576', '"componentBytes":1048576.0')],
  ['noncompact JSON', text => text.replace(marker + JSON.stringify(record), marker + JSON.stringify(record, null, 2))],
  ['truncated JSON', text => text.replace(`"childExitCode":0}}`, `"childExitCode":0}`)],
  ['ambiguous marker prefix', text => text.replace(marker, `[unrelated] ${marker}`)],
  ['second Rust execution', text => `${text}\nrunning 1 test\n${row}\n${footer}\n`],
]) {
  test(`browser accounting rejects ${name}`, () => {
    assert.throws(() => browserByteBoundaryAccounting(mutate(output(record))));
  });
}
test('Node-only output cannot stand in for a pinned Chrome proof', () => {
  const changed = structuredClone(record);
  changed.browser = null;
  changed.cleanup.browserConfirmed = false;
  assert.throws(() => browserByteBoundaryAccounting(output(changed)));
});
test('self-reported success without exact public facts is rejected', () => {
  assert.throws(() => browserByteBoundaryAccounting(output({ passed: true, bytes: 1048576 })));
});

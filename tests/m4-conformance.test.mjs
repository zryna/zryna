import assert from 'node:assert/strict';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, mkdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import {
  calculateM4Coverage, expectedRegistrySha256, loadAndValidateM4Conformance,
  registryPath, validateM4Conformance,
} from '../scripts/check-m4-conformance.mjs';
import { inventory, operationalSourcePath, parseCanonical, portablePath, readerOpenFlags, readStableFile } from '../scripts/m4/registry-files.mjs';
import { pinnedBrowserTests } from '../scripts/m4/named-gate-cases.mjs';

const original = () => parseCanonical(readStableFile(registryPath, 512 * 1024));
const mutant = edit => {
  const value = original();
  edit(value);
  return value;
};
function scratch(run) {
  const directory = mkdtempSync(path.join(tmpdir(), 'zryna-m4-registry-'));
  try { run(directory); }
  finally { rmSync(directory, { recursive: true, force: true }); }
}

test('fixed registry is canonical source-bound contract, never an execution certificate', () => {
  const registry = loadAndValidateM4Conformance();
  assert.equal(registry.rows.length, 12);
  assert.equal(registry.scope.witClosure.length, 34);
  assert.match(expectedRegistrySha256, /^[a-f0-9]{64}$/);
  const coverage = calculateM4Coverage(registry);
  assert.equal(coverage.executionAuthenticated, false);
  assert.equal(coverage.eligible, false);
  assert.equal(coverage.contractComplete, true);
  assert.deepEqual(coverage.blockers, []);
  for (const row of registry.rows.filter(entry => entry.mode === 'checkout-browser')) {
    const limit = row.limits.find(entry => entry.id === 'component-bytes');
    for (const id of [limit.mapping.public.exactCase, limit.mapping.public.firstExtraCase]) {
      const entry = registry.cases.find(candidate => candidate.id === id);
      assert.equal(entry.proofScope, 'public-browser-source-authenticated-canonical-component-byte-boundary');
      assert.equal(entry.gateId, 'pinned-browser-three-proof');
      assert.equal(entry.test.selector, 'browser_component_pinned_real_browser_and_node_agree');
    }
  }
  const installed = registry.rows.find(row => row.id === 'installed-command-linux');
  const component = installed.limits.find(limit => limit.id === 'component-bytes');
  assert.equal(registry.cases.find(entry => entry.id === component.mapping.public.exactCase).proofScope,
    'fresh-content-verified-installed-command');
  for (const id of [...component.mapping.bindingCaseIds, component.mapping.public.exactCase,
    component.mapping.public.firstExtraCase]) {
    assert.equal(registry.cases.find(entry => entry.id === id).test.runner, 'installed-invocation');
  }
});

test('unknown nested keys and supplied passed flags cannot become authority', () => {
  for (const edit of [value => { value.passed = true; },
    value => { value.rows[0].host.verified = true; },
    value => { value.cases[0].oracle.passed = true; },
    value => { value.gates[0].optional = true; }]) {
    assert.throws(() => validateM4Conformance(mutant(edit)), /closed schema/);
  }
});

test('operational sources reject generated ignored and containing authority paths', () => {
  for (const forbidden of ['.zryna/out/generated.mjs', '.git/objects/source',
    'node_modules/loader.js', 'target/generated.mjs', 'tests/node_modules/loader.js',
    'scripts/.zryna/generated.mjs', 'crates/target/generated.rs', 'apps/.git/source.rs',
    'tests/Node_Modules/loader.js', 'crates/Target/generated.rs', 'apps/zryna/src/M4_SUPPORT.json',
    'apps/zryna/src/m4_support.json', 'tests/m4-conformance-v1.json',
    'scripts/m4/registry-authority.mjs', 'scripts/m4/limit-contract-authority.mjs',
    'docs/M4_SUPPORT.md', 'docs/generated/m4-support.json']) {
    assert.throws(() => operationalSourcePath(forbidden), /operational source path/);
    assert.throws(() => validateM4Conformance(mutant(value => { value.sources[0].path = forbidden; })),
      /operational source path/);
  }
  for (const valid of ['.github/workflows/ci.yml', 'crates/zryna-driver/src/browser_component/loader.js',
    'tests/browser-component/run.mjs', 'scripts/m4/support.mjs', 'package.json',
    'CONTRIBUTING.md', 'docs/DOCUMENTATION_BUNDLES.md', 'docs/CODE_NAVIGATION.md']) {
    assert.equal(operationalSourcePath(valid), valid);
  }
});

test('missing extra and duplicate row case and gate identities reject', () => {
  for (const edit of [value => { value.rows.pop(); },
    value => { value.rows[0].id = 'invented-linux'; },
    value => { value.cases.push({ ...value.cases[0], kind: 'negative' }); },
    value => { value.gates.push({ ...value.gates[0], accounting: 'exact-installed-invocation-inventory' }); },
    value => { value.rows[0].cases.pop(); }]) {
    assert.throws(() => validateM4Conformance(mutant(edit)), /schema|inventory|duplicate/);
  }
});

test('unregistered fixture unsafe paths and case-colliding sources reject', () => {
  for (const edit of [value => { value.sources[0].path = '../escape'; },
    value => { value.cases[0].fixture.push('tests/not-registered.zry'); },
    value => { value.sources.push({ ...value.sources[0], path: value.sources[0].path.toUpperCase() }); }]) {
    assert.throws(() => validateM4Conformance(mutant(edit)), /unsafe|fixture|case collision/);
  }
});

test('changed fixed oracle and foundation identity reject independently of file digest', () => {
  assert.throws(() => validateM4Conformance(mutant(value => {
    value.cases[0].oracle.assertions = ['process exit0 certifies all rows'];
  })), /frozen cases authority/);
  assert.throws(() => validateM4Conformance(mutant(value => {
    value.foundations[0].registry.sha256 = '0'.repeat(64);
  })), /foundation source descriptor/);
});

test('exact named selector and no skip argv remain mandatory', () => {
  assert.throws(() => validateM4Conformance(mutant(value => {
    value.gates.find(item => item.id === value.cases[0].gateId).argv.push('--skip');
  })), /skipped proof/);
  assert.throws(() => validateM4Conformance(mutant(value => {
    value.cases[0].test.selector = 'unrelated_passing_test';
  })), /selector argv/);
});

test('first-extra value and owning boundary cannot drift', () => {
  for (const edit of [value => {
    value.cases.find(item => item.id === 'component-bytes-first-extra').boundary.value = 1048576;
  }, value => {
    value.cases.find(item => item.id === 'component-bytes-first-extra').boundary.limitId = 'deadline';
  }, value => {
    value.rows[0].limits[0].mapping.guard.firstExtraCase = null;
  }]) assert.throws(() => validateM4Conformance(mutant(edit)), /pair|closed schema|row limit/);
});

test('lowered fixture scope cannot certify a production component ceiling', () => {
  assert.throws(() => validateM4Conformance(mutant(value => {
    value.cases.find(item => item.id === 'component-bytes-exact-limit').proofScope = 'lowered-test-limit';
  })), /owning\/public scope/);
});

test('private guard prerequisite cannot erase partial public component size requirement', () => {
  assert.throws(() => validateM4Conformance(mutant(value => {
    const row = value.rows.find(item => item.id === 'command-linux');
    const mapping = row.limits.find(item => item.id === 'component-bytes').mapping;
    mapping.kind = 'mapped';
    delete mapping.requirements;
    mapping.public = { kind: 'guard-prerequisite', caseIds: mapping.bindingCaseIds };
  })), /private prerequisite substitutes public artifact boundary/);
});

test('canonical reader rejects duplicate fields UTF8 BOM trailing records and alternate trivia', () => {
  for (const bytes of [Buffer.from('{"version":1,"version":1}\n'),
    Buffer.from([0xff, 0xfe]), Buffer.from('\ufeff{\n  "version": 1\n}\n'),
    Buffer.from('{\n  "version": 1\n}\n{}'), Buffer.from('{"version":1}')]) {
    assert.throws(() => parseCanonical(bytes), /strict UTF-8|canonical/);
  }
});

test('same-handle file reader rejects directory symlink and first extra byte', () => {
  scratch(directory => {
    const file = path.join(directory, 'fixture.json');
    writeFileSync(file, '{}\n');
    assert.equal(readStableFile(file, 3).toString(), '{}\n');
    assert.throws(() => readStableFile(file, 2), /byte limit/);
    assert.throws(() => readStableFile(directory, 64), /regular/);
    const link = path.join(directory, 'linked.json');
    // Windows requires its supported link fixture environment; failure is not a skipped pass.
    symlinkSync(file, link, 'file');
    assert.throws(() => readStableFile(link, 64), /regular/);
  });
});

test('fixture inventory detects extra entries and case collisions independently', () => {
  scratch(root => {
    mkdirSync(path.join(root, 'fixtures'));
    writeFileSync(path.join(root, 'fixtures/source.zry'), 'source');
    assert.deepEqual(inventory(root, 'fixtures'), ['fixtures/source.zry']);
    writeFileSync(path.join(root, 'fixtures/extra.zry'), 'extra');
    assert.notDeepEqual(inventory(root, 'fixtures'), ['fixtures/source.zry']);
    // Directory enumeration remains case-sensitive on supported CI test fixtures.
    writeFileSync(path.join(root, 'fixtures/SOURCE.zry'), 'case-collision');
    if (process.platform === 'linux') {
      assert.throws(() => inventory(root, 'fixtures'), /case collision/);
    } else {
      assert.equal(readStableFile(path.join(root, 'fixtures/source.zry'), 64).toString(), 'case-collision');
    }
  });
});


test('Unix reader refuses platforms without nonblocking no-follow open support', () => {
  assert.throws(() => readerOpenFlags('linux', { O_RDONLY: 0, O_NOFOLLOW: 1 }), /O_NONBLOCK/);
  assert.throws(() => readerOpenFlags('linux', { O_RDONLY: 0, O_NONBLOCK: 2 }), /O_NOFOLLOW/);
  assert.equal(readerOpenFlags('linux', { O_RDONLY: 0, O_NOFOLLOW: 1, O_NONBLOCK: 2 }), 3);
});

test('regular-to-FIFO replacement at open rejects without blocking',
  { skip: process.platform !== 'linux' }, () => {
    scratch(directory => {
      const modulePath = fileURLToPath(new URL('../scripts/m4/registry-files.mjs', import.meta.url));
      const program = `
        import assert from 'node:assert/strict';
        import fs from 'node:fs';
        import { syncBuiltinESMExports } from 'node:module';
        import { execFileSync } from 'node:child_process';
        import { pathToFileURL } from 'node:url';
        const [modulePath, directory] = process.argv.slice(1);
        const fixture = directory + '/race.json';
        fs.writeFileSync(fixture, '{}\\n');
        const originalOpen = fs.openSync;
        let replaced = false;
        fs.openSync = (file, flags, ...rest) => {
          if (file === fixture && !replaced) {
            replaced = true;
            fs.unlinkSync(fixture);
            execFileSync('mkfifo', [fixture]);
            assert(flags & fs.constants.O_NONBLOCK);
          }
          return originalOpen(file, flags, ...rest);
        };
        syncBuiltinESMExports();
        const { readStableFile } = await import(pathToFileURL(modulePath));
        assert.throws(() => readStableFile(fixture, 64), /opened identity or size changed/);
        assert(replaced);
      `;
      const result = spawnSync(process.execPath,
        ['--input-type=module', '--eval', program, modulePath, directory],
        { timeout: 3000, encoding: 'utf8', shell: false });
      assert.ifError(result.error);
      assert.equal(result.signal, null);
      assert.equal(result.status, 0, result.stderr);
    });
  });

test('streaming fixture enumeration enforces global entry cap before collecting', () => {
  scratch(root => {
    const directory = path.join(root, 'fixtures');
    mkdirSync(directory);
    for (let index = 0; index < 513; index++) {
      writeFileSync(path.join(directory, `source${index}.zry`), 'source');
    }
    assert.throws(() => inventory(root, 'fixtures'), /entries exceed bound/);
    // Cleanup succeeds after early enumeration failure, including on Windows.
    rmSync(directory, { recursive: true });
    assert.throws(() => inventory(root, 'fixtures'), /ENOENT/);
  });
});

test('fixture enumeration rejects a linked ancestor before opening descendants', () => {
  scratch(root => {
    mkdirSync(path.join(root, 'actual'));
    mkdirSync(path.join(root, 'actual/fixtures'));
    writeFileSync(path.join(root, 'actual/fixtures/source.zry'), 'source');
    symlinkSync(path.join(root, 'actual'), path.join(root, 'ancestor'),
      process.platform === 'win32' ? 'junction' : 'dir');
    assert.throws(() => inventory(root, 'ancestor/fixtures'), /ancestor/);
  });
});

test('both browser rows require the component scalar and full transport pinned proofs', () => {
  const registry = original();
  const runner = registry.gates.find(gate => gate.id === 'pinned-browser-three-proof');
  assert.equal(runner.accounting, 'exact-three-proof-pinned-browser-receipt');
  const identities = registry.cases.filter(entry => entry.gateId === runner.id).map(entry =>
    JSON.stringify([entry.test.package, entry.test.target, entry.test.selector, entry.test.source]));
  assert.deepEqual([...new Set(identities)].sort(), pinnedBrowserTests.map(tuple => JSON.stringify(tuple)).sort());
  for (const row of registry.rows.filter(item => item.profile === 'browser-component-v1')) {
    assert(row.requiredGates.includes(runner.id));
  }
  assert.throws(() => validateM4Conformance(mutant(value => {
    value.cases.find(entry => entry.id === 'browser-full-esm-transport').gateId = value.gates[0].id;
  })), /selector argv|three unique|accounting/);
});


test('workflow dot-directory paths are portable while traversal components reject', () => {
  assert.equal(portablePath('.github/workflows/browser-conformance.yml'),
    '.github/workflows/browser-conformance.yml');
  for (const relative of ['.', '..', '../escape', 'fixtures/../escape', 'fixtures/./source.zry']) {
    assert.throws(() => portablePath(relative), /unsafe portable path/);
  }
});

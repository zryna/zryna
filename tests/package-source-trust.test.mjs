import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import test from 'node:test';

const policyUrl = new URL('../spec/package/SOURCE_TRUST_V0.md', import.meta.url);
const policy = await readFile(policyUrl, 'utf8');

// These independent expectations check policy documents, not package execution.
const operations = [
  ['read-verified-source', 'allow', 'allow', 'allow'],
  ['acquire-source-before-build', 'conditional', 'conditional', 'conditional'],
  ['compiler-tool-process', 'conditional', 'conditional', 'conditional'],
  ['driver-owned-link', 'conditional', 'conditional', 'deny'],
  ['dependency-install-hook', 'deny', 'deny', 'deny'],
  ['native-recipe', 'deny', 'conditional', 'deny'],
  ['load-native-dependency', 'deny', 'conditional', 'deny'],
  ['arbitrary-shell-or-path-tool', 'deny', 'deny', 'deny'],
  ['network-during-build', 'deny', 'deny', 'deny'],
  ['read-ambient-secrets', 'deny', 'deny', 'deny'],
  ['write-source-or-shared-cache', 'deny', 'deny', 'deny'],
  ['write-private-staging', 'allow', 'allow', 'allow'],
  ['execute-built-program', 'deny', 'deny', 'conditional'],
  ['publish-build-artifact', 'conditional', 'conditional', 'deny'],
  ['reuse-build-cache', 'conditional', 'conditional', 'conditional'],
  ['escalate-mode', 'deny', 'deny', 'deny'],
];

const threats = [
  ['unknown-policy', 'Driver / caller policy verifier', 'request', 'TRUST-MODE', 'policy-matrix'],
  ['namespace-confusion', 'Resolver', 'identity', 'TRUST-NAMESPACE', 'source-adversarial'],
  ['source-substitution', 'Resolver / acquisition verifier', 'identity', 'TRUST-SOURCE', 'source-adversarial'],
  ['frozen-lock-drift', 'Resolver', 'identity', 'TRUST-FROZEN', 'clean-offline'],
  ['missing-offline-input', 'Acquisition / cache reader', 'source', 'TRUST-OFFLINE', 'clean-offline'],
  ['checksum-mismatch', 'Acquisition verifier', 'source', 'TRUST-CHECKSUM', 'source-adversarial'],
  ['tampered-cache', 'Cache reader / retained-source verifier', 'source', 'TRUST-CACHE', 'cache-adversarial'],
  ['unsafe-materialization', 'Source materializer / isolation supervisor', 'source', 'TRUST-PATH', 'source-adversarial'],
  ['unauthorized-fetch', 'Acquisition broker', 'source', 'TRUST-NETWORK', 'broker-isolation'],
  ['artifact-cache-integrity', 'Driver / #361 cache verifier', 'plan', 'P361-CACHE', 'cache-adversarial'],
  ['unauthorized-cache-reuse', 'Driver / #362 policy verifier', 'plan', 'TRUST-PLAN', 'cache-adversarial'],
  ['undeclared-tool', 'Driver tool verifier', 'plan', 'TRUST-TOOL', 'native-adversarial'],
  ['unapproved-native-recipe', 'Driver / operator approval verifier', 'plan', 'TRUST-NATIVE', 'native-adversarial'],
  ['forbidden-operation', 'Driver / isolation supervisor', 'plan', 'TRUST-OPERATION', 'policy-matrix'],
  ['missing-isolation', 'Isolation supervisor', 'plan', 'TRUST-ISOLATION', 'process-isolation'],
  ['execution-escape', 'Isolation supervisor', 'execution', 'TRUST-ESCAPE', 'process-isolation'],
  ['execution-budget', 'Isolation supervisor', 'execution', 'TRUST-BUDGET', 'process-isolation'],
  ['incomplete-provenance', 'Driver / provenance consumer', 'publication', 'TRUST-PROVENANCE', 'provenance-binding'],
  ['incomplete-publication', 'Driver / cache writer', 'publication', 'TRUST-PUBLICATION', 'publication-atomicity'],
];

const cases = [
  ['clean-frozen', 'accept', 'source', 'clean-offline'],
  ['offline-missing', 'TRUST-OFFLINE', 'source', 'clean-offline'],
  ['frozen-tool-missing', 'TRUST-OFFLINE', 'source', 'clean-offline'],
  ['frozen-lock-changed', 'TRUST-FROZEN', 'identity', 'clean-offline'],
  ['private-public-confusion', 'TRUST-NAMESPACE', 'identity', 'source-adversarial'],
  ['substituted-origin', 'TRUST-SOURCE', 'identity', 'source-adversarial'],
  ['replaced-digest-pair', 'TRUST-CHECKSUM', 'source', 'source-adversarial'],
  ['tampered-cache-byte', 'TRUST-CACHE', 'source', 'cache-adversarial'],
  ['tampered-cache-metadata', 'TRUST-CACHE', 'source', 'cache-adversarial'],
  ['source-race', 'TRUST-CACHE', 'source', 'cache-adversarial'],
  ['path-escape', 'TRUST-PATH', 'source', 'source-adversarial'],
  ['redirect-substitution', 'TRUST-NETWORK', 'source', 'broker-isolation'],
  ['artifact-cache-miss', 'miss', 'plan', 'cache-adversarial'],
  ['wrong-target-cache', 'P361-CACHE', 'plan', 'cache-adversarial'],
  ['policy-cache-mismatch', 'TRUST-PLAN', 'plan', 'cache-adversarial'],
  ['permissive-cache', 'TRUST-PLAN', 'plan', 'cache-adversarial'],
  ['path-tool-injection', 'TRUST-TOOL', 'plan', 'native-adversarial'],
  ['native-without-opt-in', 'TRUST-NATIVE', 'plan', 'native-adversarial'],
  ['pure-source-hook', 'TRUST-OPERATION', 'plan', 'policy-matrix'],
  ['playground-native', 'TRUST-OPERATION', 'plan', 'policy-matrix'],
  ['sandbox-unavailable', 'TRUST-ISOLATION', 'plan', 'process-isolation'],
  ['network-child', 'TRUST-ESCAPE', 'execution', 'process-isolation'],
  ['budget-extra', 'TRUST-BUDGET', 'execution', 'process-isolation'],
  ['missing-policy-evidence', 'TRUST-PROVENANCE', 'publication', 'provenance-binding'],
  ['interrupted-write', 'TRUST-PUBLICATION', 'publication', 'publication-atomicity'],
  ['unknown-mode', 'TRUST-MODE', 'request', 'policy-matrix'],
  ['earliest-rejection', 'TRUST-MODE', 'request', 'policy-matrix'],
];

function tableAfter(text, heading, header) {
  const sections = text.split(`${heading}\n\n`);
  assert.equal(sections.length, 2, `one ${heading}`);
  const lines = sections[1].split(/\r?\n/);
  const start = lines.findIndex((line) => line.startsWith('|'));
  assert.ok(start >= 0, `table for ${heading}`);
  const block = [];
  for (let index = start; index < lines.length && lines[index].startsWith('|'); index++) {
    const line = lines[index];
    assert.ok(line.endsWith('|'), 'complete table row');
    block.push(line.slice(1, -1).split('|').map((cell) => cell.trim()));
  }
  assert.deepEqual(block[0], header, `columns for ${heading}`);
  assert.ok(block[1].every((cell) => /^:?-+:?$/.test(cell)), 'table separator');
  const rows = block.slice(2);
  assert.ok(rows.every((row) => row.length === header.length && row.every(Boolean)), 'complete cells');
  assert.equal(new Set(rows.map(([id]) => id)).size, rows.length, 'unique rows');
  return rows;
}

function operationRows(text) {
  return tableAfter(text, '### Operation matrix', [
    'Operation', 'pure-source', 'trusted-native', 'untrusted-playground',
  ]);
}

function threatRows(text) {
  return tableAfter(text, '### Threat matrix', [
    'Threat', 'Responsible enforcer', 'Stage', 'Rejection', 'Later gate',
  ]);
}

function caseRows(text) {
  return tableAfter(text, '### Case matrix', [
    'Case', 'Changed input / request', 'Outcome', 'Stage', 'Later gate',
  ]);
}

function checkOperations(text) {
  assert.deepEqual(operationRows(text), operations);
}

function checkThreats(text) {
  assert.deepEqual(threatRows(text), threats);
}

function checkCases(text) {
  const rows = caseRows(text).map(([id, , ...outcome]) => [id, ...outcome]);
  assert.deepEqual(rows, cases);
  const byRejection = new Map(threatRows(text).map(([, , stage, rejection, gate]) => [rejection, [stage, gate]]));
  const covered = new Set();
  for (const [, rejection, stage, gate] of rows) {
    if (rejection === 'accept' || rejection === 'miss') continue;
    assert.deepEqual([stage, gate], byRejection.get(rejection), `owned rejection ${rejection}`);
    covered.add(rejection);
  }
  assert.deepEqual([...covered].sort(), [...byRejection.keys()].sort(), 'every threat has a negative case');
}

function rowText(row) {
  return `| ${row.join(' | ')} |`;
}

function replaceRow(text, original, replacement) {
  const before = rowText(original);
  assert.ok(text.includes(before), 'mutation must affect the document');
  return text.replace(before, replacement === null ? '' : rowText(replacement));
}

test('all three modes have a closed operation matrix', () => {
  checkOperations(policy);
});

test('every denied operation rejects an allow or conditional mutation', () => {
  for (const row of operations) {
    for (let column = 1; column <= 3; column++) {
      if (row[column] !== 'deny') continue;
      for (const decision of ['allow', 'conditional']) {
        const changed = [...row];
        changed[column] = decision;
        assert.throws(() => checkOperations(replaceRow(policy, row, changed)),
          assert.AssertionError, `${row[0]} column ${column}: ${decision}`);
      }
    }
  }
});

test('unknown modes and missing, duplicate or invented operations reject', () => {
  assert.throws(() => checkOperations(policy.replace('trusted-native |', 'unrestricted |')), assert.AssertionError);
  for (const row of operations) {
    assert.throws(() => checkOperations(replaceRow(policy, row, null)), assert.AssertionError);
  }
  const row = rowText(operations[0]);
  assert.throws(() => checkOperations(policy.replace(row, `${row}\n${row}`)), assert.AssertionError);
  assert.throws(() => checkOperations(policy.replace(row,
    `${row}\n| undeclared | allow | allow | allow |`)), assert.AssertionError);
});

test('each threat retains its responsible enforcer, rejecting phase and later gate', () => {
  checkThreats(policy);
  for (const row of threats) {
    for (const column of [1, 2, 3, 4]) {
      const changed = [...row];
      changed[column] = 'unspecified';
      assert.throws(() => checkThreats(replaceRow(policy, row, changed)), assert.AssertionError);
    }
  }
});

test('fixed cases cover every threat, clean frozen input and an absent artifact cache key', () => {
  checkCases(policy);
  assert.equal(cases.filter(([, outcome]) => outcome === 'accept').length, 1);
  assert.equal(cases.filter(([, outcome]) => outcome === 'miss').length, 1);
});

test('negative examples reject acceptance, phase delay, missing gates and omitted cases', () => {
  for (const row of caseRows(policy).filter((row) => !['accept', 'miss'].includes(row[2]))) {
    for (const [column, value] of [[2, 'accept'], [2, 'miss'], [3, 'after-execution'], [4, 'unspecified']]) {
      const changed = [...row];
      changed[column] = value;
      assert.throws(() => checkCases(replaceRow(policy, row, changed)), assert.AssertionError, row[0]);
    }
    assert.throws(() => checkCases(replaceRow(policy, row, null)), assert.AssertionError, row[0]);
  }
});

function checkResolution(text) {
  const rows = tableAfter(text, '### Resolution requests', [
    'Resolution request', 'Network acquisition', 'Lock mutation', 'Missing or altered material',
  ]);
  assert.deepEqual(rows.map(([id]) => id), [
    'Explicit online acquire/update', 'Offline', 'Frozen', 'Offline and frozen',
  ]);
  for (const row of rows.slice(1)) assert.match(row[1], /^Denied(?:,|$)/);
  for (const row of rows.slice(1)) assert.match(row[2], /^Denied(?:;|$)/);
  assert.match(rows[2][3], /Reject missing\/stale lock, missing input or altered bytes; never retry online/);
}

test('offline and frozen have no network fallback or lock mutation', () => {
  checkResolution(policy);
  const rows = tableAfter(policy, '### Resolution requests', [
    'Resolution request', 'Network acquisition', 'Lock mutation', 'Missing or altered material',
  ]);
  for (const row of rows.slice(1)) {
    for (const [column, value] of [[1, 'Allowed on cache miss'], [2, 'Allowed to update or repair lock']]) {
      const changed = [...row];
      changed[column] = value;
      assert.throws(() => checkResolution(replaceRow(policy, row, changed)), assert.AssertionError);
    }
  }
});

test('source trust consumes the exact lock-package pair without competing identity fields', () => {
  const expected = [
    ['Source tuple', '#168 source kind, canonical locator and exact revision'],
    ['Manifest identity', 'Lock-package `id`: domain-separated digest of canonical manifest bytes'],
    ['Source identity', 'Lock-package `sourceSha256`: `source-files` digest of the complete ordered inventory'],
    ['Package instance', 'Exact ordered pair `(id, sourceSha256)` from one validated lock-package record'],
    ['Dependency selection', 'Lock edge `package` selects record `id`; authenticate `sourceSha256` from that record'],
  ];
  const check = (text) => assert.deepEqual(
    tableAfter(text, '### Identity mapping', ['Identity', 'Required binding']), expected,
  );
  check(policy);
  for (const row of expected) {
    assert.throws(() => check(replaceRow(policy, row, null)), assert.AssertionError);
  }
  for (const substitution of ['Package name and alias', 'Manifest id only', '`packageInstance` wire field']) {
    assert.throws(() => check(replaceRow(policy, expected[3],
      ['Package instance', substitution])), assert.AssertionError);
  }
});

test('build-plan cache projection retains policy, environment, tool and source bindings', () => {
  const expected = [
    ['Execution policy', 'Policy id, version and configuration digest'],
    ['Environment', 'Relevant declared environment'],
    ['Host', 'Host triple'],
    ['Compiler and tools', 'Compiler/tool versions and hashes'],
    ['Target execution', 'Target, runtime and profile'],
    ['Input graph', 'Validated #168 lock projection and target/runtime source closure'],
    ['Result inventory', 'Outputs'],
    ['Native extension', 'Optional native inputs'],
  ];
  const check = (text) => assert.deepEqual(
    tableAfter(text, '### Cache projection', ['Binding', 'Required content']), expected,
  );
  check(policy);
  for (const row of expected) {
    assert.throws(() => check(replaceRow(policy, row, null)), assert.AssertionError);
    assert.throws(() => check(replaceRow(policy, row, [row[0], 'Unbound'])), assert.AssertionError);
  }
  assert.throws(() => check(replaceRow(policy, expected[0],
    ['Execution policy', 'Policy id and version'])), assert.AssertionError);
});

test('policy retains ownership boundaries, isolation limits and deferred activation', async () => {
  for (const phrase of [
    'Status: proposed specification',
    'Manifest permissions are not an OS sandbox.',
    'Its handoff is a branded projection from a fully validated #168\ncanonical envelope',
    'exact lock digest, compatibility compiler/profile/target\ncoverage',
    'package-qualified\npath/size/SHA-256 inventory',
    'There is no `packageInstance` or `sourceIdentity` wire field in #168 v1.',
    'The accepted\n#168 lock has no build-edge kind',
    'source-only v0 admits only the\nauthenticated `target/runtime` root and complete lock graph',
    'wire bytes and collection budgets before schema',
    '96 lowercase portable ASCII bytes',
    'material map must contain exactly\nthe declared source keys',
    'preserves #361\'s earlier rejection',
    'The plan-closure `cacheKey` remains\ndistinct from each package\'s `sourceSha256`',
    'opaque `executionPolicy` id, version and configuration digest is\na cache input',
    'closed versioned typed invocation-adapter records',
    'linker output must match an exact target-qualified plan\n  output',
    '`P361-CACHE` rejection',
    'digest rules and provenance\nencoding remain #168-owned',
    'The driver alone validates\ntools, compiles, links, verifies cache outputs',
    'appendix is provisional-pending-364',
    'Do not add fields to #168\'s closed v1 records',
    'Native recipes remain\nunavailable until the optional #361 appendix',
    'no resolver, executor, sandbox, registry or public\nselector',
    'not freshness of remote revocation/yank state',
    'Yank excludes a version from new selection.',
    'These are policy terms, not new CLI flags.',
    'They parse\npolicy text only',
  ]) assert.ok(policy.includes(phrase), phrase);
  for (const issue of [168, 360, 361, 364]) {
    assert.ok(policy.includes(`https://github.com/zryna/zryna/issues/${issue}`));
  }
  assert.ok(policy.includes('https://github.com/zryna/zryna/milestone/6'));
  for (const [, target] of policy.matchAll(/\]\(([^)]+)\)/g)) {
    if (target.startsWith('https://')) continue;
    assert.ok((await stat(new URL(target, policyUrl))).isFile(), target);
  }
  const roadmap = await readFile(new URL('../docs/ROADMAP.md', import.meta.url), 'utf8');
  assert.ok(roadmap.includes('../spec/package/SOURCE_TRUST_V0.md'));
});

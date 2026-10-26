// Pure accounting of explicitly framed untrusted observations, never a proof/receipt issuer.
import assert from 'node:assert/strict';
import { TextDecoder } from 'node:util';
import { canonical, exact, parse, portable, sha256, MAX_LOG } from './input.mjs';
import { CATALOGUE_SHA256 } from './required-owner-authority.mjs';
import { replayTarget } from './required-owner-rust.mjs';
export function parseRequiredDeclarations(bytes) {
  assert.equal(sha256(bytes), CATALOGUE_SHA256, 'reviewed static required inventory');
  const value = parse(bytes); validate(value); return value;
}
function validate(index) {
  assert.equal(sha256(canonical(index)), CATALOGUE_SHA256, 'caller cannot substitute declaration/owner authority');
  assert.equal(index.closureStatus, 'blocked'); assert.equal(index.thresholds, null);
  assert.equal(index.rows.length, 28); assert.equal(new Set(index.rows.map(row => row.id)).size, 28);
}
export function checkRequiredMaterials(index, readMaterial) {
  validate(index); let total = 0;
  for (const pin of index.materialPins) {
    const bytes = readMaterial(pin.path, pin.size);
    assert(Buffer.isBuffer(bytes) && bytes.length === pin.size, 'selected material bytes');
    assert.equal(sha256(bytes), pin.sha256, `changed required declaration/owner: ${pin.path}`);
    total += bytes.length; assert(total <= 16 * 1024 * 1024, 'material aggregate bound');
  }
  return Object.freeze({ status: 'declared-material-bytes-match', sourceAuthenticated: false, proofQualified: false, closureStatus: 'blocked', thresholds: null });
}
function source(value) {
  exact(value, ['role', 'revision', 'tree'], 'untrusted source scope');
  assert(['tested-checkout', 'tested-merge', 'tested-pr-head'].includes(value.role), 'explicit source role');
  assert.match(value.revision, /^[a-f0-9]{40}$/u); assert.match(value.tree, /^[a-f0-9]{40}$/u);
}
function log(descriptor, readLog, paths) {
  exact(descriptor, ['path', 'size', 'sha256'], 'retained log descriptor'); portable(descriptor.path);
  assert(!paths.has(descriptor.path), 'distinct log descriptors'); paths.add(descriptor.path);
  assert(Number.isSafeInteger(descriptor.size) && descriptor.size >= 0 && descriptor.size <= MAX_LOG, 'log byte bound');
  assert.match(descriptor.sha256, /^[a-f0-9]{64}$/u);
  const bytes = readLog(descriptor.path, descriptor.size);
  assert(Buffer.isBuffer(bytes) && bytes.length === descriptor.size, 'retained complete log bytes');
  assert.equal(sha256(bytes), descriptor.sha256, 'retained log hash');
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
}
function framed(section, rows, readLog, paths) {
  exact(section, ['package', 'target', 'stdout', 'stderr'], 'isolated target section');
  const expected = rows.filter(row => row.package === section.package && row.target === section.target);
  assert(expected.length, 'unknown package/target or unrelated suite');
  const stdout = log(section.stdout, readLog, paths), stderr = log(section.stderr, readLog, paths);
  assert(!stderr.includes('\u001b') && !stderr.replaceAll('\r\n', '\n').includes('\r'), 'unambiguous Rust stderr transcript');
  assert(!/^(?:test |test result:|running )/mu.test(stderr), 'Rust target accounting must not be moved across streams');
  const replay = replayTarget(stdout);
  return { package: section.package, target: section.target, ...replay };
}
export function reconcileRequiredOwnerLogs(index, observation, { source: expectedSource, host, readLog, readMaterial }) {
  validate(index); assert.equal(typeof readMaterial, 'function', 'current selected material reader required');
  checkRequiredMaterials(index, readMaterial); source(expectedSource); canonical(observation);
  exact(observation, ['format', 'source', 'host', 'owner', 'argv', 'exitCode', 'signal', 'error', 'defaults', 'targets'], 'owner-log observation');
  assert.equal(observation.format, 'zryna.required-owner-log-observation.v1'); source(observation.source);
  assert.deepEqual(observation.source, expectedSource, 'exact source role/revision/tree, no revision transfer');
  assert(['linux-x86_64', 'windows-x86_64'].includes(host)); assert.equal(observation.host, host);
  const owner = index.owners[observation.owner]; assert(owner, 'unknown or conditional/unselected owner');
  assert(owner.hosts.includes(host) && (!owner.scheduledHosts || owner.scheduledHosts.includes(host)), 'owner not scheduled/selected on this host');
  assert.deepEqual(observation.argv, owner.childArgv, 'exact source-pinned child argv; --list/filter/flag drift forbidden');
  assert.equal(observation.exitCode, 0); assert.equal(observation.signal, null); assert.equal(observation.error, null);
  const required = index.rows.filter(row => row.hosts.includes(host) && (row.primaryOwnerByHost[host] === observation.owner || row.alternatives.includes(observation.owner)));
  assert(required.length, 'no selected required rows');
  const keys = [...new Set(required.map(row => `${row.package}/${row.target}`))].sort();
  const paths = new Set(), consumed = [];
  const selected = (sections, defaults) => {
    assert(Array.isArray(sections) && sections.length === keys.length, 'every selected target section exactly once');
    assert.deepEqual(sections.map(row => `${row.package}/${row.target}`).sort(), keys, 'unique complete selected package/target set');
    return sections.map(section => {
      const applicable = index.rows.filter(row => row.hosts.includes(host));
      const parsed = framed(section, applicable, readLog, paths);
      const known = applicable.filter(row => row.package === parsed.package && row.target === parsed.target);
      const actualIgnored = parsed.rows.filter(row => row.outcome === 'ignored').map(row => row.name).sort();
      if (defaults) {
        assert.deepEqual(actualIgnored, known.map(row => row.fullTestName).sort(), 'missing/unknown default ignored declaration');
      } else {
        assert.equal(parsed.counts.ignored, 0, 'raw ignored rows are not executed proofs');
        assert(parsed.counts.passed > 0, 'zero/listed/filtered-only suite cannot prove coverage');
        for (const row of required.filter(row => row.package === parsed.package && row.target === parsed.target)) {
          assert(parsed.rows.some(actual => actual.name === row.fullTestName && actual.outcome === 'ok'), `missing exact required owner name: ${row.fullTestName}`);
          consumed.push(row.id);
        }
        const knownNames = new Set(applicable.map(row => row.fullTestName));
        const requestedNames = new Set(required.filter(row => row.package === parsed.package && row.target === parsed.target).map(row => row.fullTestName));
        assert(!parsed.rows.some(row => knownNames.has(row.name) && !requestedNames.has(row.name)), 'extra selected required name belongs to another owner');
      }
      return parsed;
    });
  };
  const rawDefaultTargets = selected(observation.defaults, true), ownerTargets = selected(observation.targets, false);
  assert.equal(new Set(consumed).size, required.length, 'required names each consumed once');
  return Object.freeze({ status: 'declared-owner-log-consistent', source: observation.source, host, owner: observation.owner,
    requiredIds: consumed, requiredNames: required.map(row => ({ package: row.package, target: row.target, fullTestName: row.fullTestName })), rawDefaultTargets, ownerTargets,
    coverageCardinality: consumed.length, rawIgnoredSubtracted: 0,
    sourceAuthenticated: false, proofQualified: false, closureStatus: 'blocked', thresholds: null,
    unresolved: ['full-M0 target framing/extraction and source/host/log authentication', 'all other required owner evidence',
      'Linux native-source owner selection', 'conditional installed lifecycle scope/material/supervisor', 'numeric thresholds and other #418 gates'] });
}

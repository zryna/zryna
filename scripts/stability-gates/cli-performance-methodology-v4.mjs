// Pure declaration/argv expansion only. No process launch, collector, baseline or threshold issuer.
import assert from 'node:assert/strict';
import { canonical, parse, portable, sha256 } from './input.mjs';
const SNAPSHOT = 'b73dee72db94b65fb9e0786c43e807e416e8791bf650da2b5babd4c658294b7b';
export function validateMethodology(document) {
  canonical(document);
  assert.equal(document.format, 'zryna.cli-performance-methodology.draft.v4');
  assert.equal(document.closureStatus, 'blocked');
  assert.equal(document.thresholds, null);
  assert.equal(document.baselineAuthority, null);
  assert.equal(document.collection.implemented, false);
  assert.equal(document.collection.hostWallGuardMs, null);
  assert.deepEqual(document.families.map(row => row.profile),
    [null, 'control-flow-v1', 'data-ownership-v1', 'bounded-generics-v1']);
  for (const row of document.families) {
    portable(row.entry);
    for (const pin of [...row.sources, row.oracle.material]) {
      portable(pin.path);
      assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= 2 * 1024 * 1024);
      assert.match(pin.sha256, /^[a-f0-9]{64}$/u);
    }
  }
  // Frozen source-derived draft, never caller-supplied expected hashes or candidate measurements.
  assert.equal(sha256(canonical(document)), SNAPSHOT, 'reviewed current whole-document methodology identity');
  return Object.freeze({ status: 'blocked-current-source-methodology-only', families: 4,
    logicalSelectors: 24, hostCells: 44, thresholds: null, baselineAuthority: null,
    collectorImplemented: false, sourceAuthenticated: false, performanceQualified: false });
}
export function parseMethodology(bytes) {
  const document = parse(bytes);
  validateMethodology(document);
  return document;
}
export function matrixFor(document, host) {
  validateMethodology(document);
  assert(document.hosts.includes(host), 'only declared Linux/Windows x86-64 hosts');
  const rows = [];
  for (const family of document.families) for (const phase of ['build', 'run']) {
    for (const target of document.targets) {
      if (phase === 'run' && target === 'native' && host !== 'linux-x86_64') continue;
      const stem = `m418_${family.id.replaceAll('-', '_')}_${phase}_${target}`;
      const argv = [phase, family.entry, '--target', target, '--provider', 'native',
        '--root', '$SOURCE_ROOT', '--name', stem, '--json'];
      if (family.profile !== null) argv.push('--profile', family.profile);
      if (phase === 'run') {
        argv.push('--export', family.export);
        for (const argument of family.arguments) argv.push(`--arg=${argument.type}:${argument.value}`);
        if (target !== 'native') argv.push('--node', '$NODE');
      }
      rows.push({ id: `${family.id}/${phase}/${target}`, host, phase, target,
        executable: '$PREBUILT_AUTHENTICATED_CLI', argv,
        latencyMeaning: document.phases[phase],
        expectedResponse: { version: 1, ok: true, command: phase,
          manifest: `.zryna/out/${stem}.${phase}/${family.manifest}`,
          results: phase === 'build' ? [] : [{ target,
            outcome: { kind: 'returned', value: family.expectedValue } }], diagnostics: [] },
        performanceQualified: false });
    }
  }
  assert.equal(rows.length, host === 'linux-x86_64' ? 24 : 20);
  return rows;
}
// Byte identity only; no source/host authority is inferred from the supplied reader.
export function checkSelectedBytes(document, readBytes) {
  validateMethodology(document);
  const pins = new Map(document.materials.map(pin => [pin.path, pin]));
  for (const family of document.families) for (const pin of [...family.sources, family.oracle.material]) {
    const prior = pins.get(pin.path);
    if (prior) assert.deepEqual(prior, pin, 'consistent selected pin');
    pins.set(pin.path, pin);
  }
  for (const pin of pins.values()) {
    const bytes = readBytes(pin.path, pin.bytes);
    assert(Buffer.isBuffer(bytes) && bytes.length === pin.bytes, 'selected bytes');
    assert.equal(sha256(bytes), pin.sha256, `selected material: ${pin.path}`);
  }
  return Object.freeze({ files: pins.size, sourceAuthenticated: false, performanceQualified: false });
}

// The graph helper imports only this live function binding; neither module calls it at initialization.
export { checkKnownGraphs } from './cli-performance-source-graph.mjs';

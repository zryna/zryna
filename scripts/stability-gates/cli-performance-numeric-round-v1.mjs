// Proposed normalization seam for EXISTING raw replay owners, not a replacement raw verifier.
import assert from 'node:assert/strict';
import { canonical, exact, parse } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
import { compare, fraction, integer, MAX_WALL_NS, summarize } from './cli-performance-numeric-math-v1.mjs';
import { digest, identity, source, time, unqualified } from './cli-performance-threshold-profile-v1.mjs';
// Unchanged Linux replay's GNU marker is [1-9][0-9]{0,14}, narrower than MAX_SAFE_INTEGER.
const MAX_LINUX_RSS_KIB = 999999999999999n;
function metrics(attempt, index, host) {
  const linux = host === 'linux-x86_64';
  exact(attempt, ['index', 'role', 'outcome', ...(linux ? ['wallNs', 'peakRssKiB'] :
    ['wallTicks', 'qpcFrequency', 'peakJobMemoryUsedBytes'])], 'normalized complete attempt');
  assert.equal(attempt.index, index); assert.equal(attempt.role, index === 0 ? 'warmup' : 'measured');
  assert.equal(attempt.outcome, 'correct-and-clean', 'invalid correctness or cleanup invalidates the round');
  const wall = linux ? fraction(integer(attempt.wallNs)) :
    fraction(integer(attempt.wallTicks) * 1000000000n, integer(attempt.qpcFrequency));
  const memory = integer(linux ? attempt.peakRssKiB : attempt.peakJobMemoryUsedBytes);
  if (linux) assert(memory <= MAX_LINUX_RSS_KIB, 'Linux GNU marker permits at most 15 decimal digits');
  assert(compare(wall, fraction(MAX_WALL_NS)) <= 0, 'unchanged 15-second raw attempt containment');
  return { wall, memory: fraction(memory) };
}
export function checkedRound(value, profile, catalogue, hostPolicy, role) {
  // Snapshot each bounded round separately: no whole-campaign resource-cap extension.
  const round = parse(Buffer.from(canonical(value)));
  exact(round, ['format', 'host', 'role', 'identity', 'source', 'build', 'toolchainSha256', 'collectorSha256',
    'catalogueSha256', 'classId', 'classBefore', 'classAfter', 'wallMethod', 'memoryMethod', 'startedAt', 'endedAt',
    'sourceAuthenticated', 'proofQualified', 'performanceQualified', 'closureStatus', 'thresholdAuthority', 'cells'], 'normalized round');
  assert.equal(round.format, 'zryna.cli-performance-numeric-input.v1'); unqualified(round);
  assert.equal(round.host, hostPolicy.host); assert.equal(round.role, role);
  identity(round.identity); source(round.source);
  const reference = role !== 'held-out-candidate';
  assert.deepEqual(round.source, profile.sources[reference ? 'reference' : 'candidate'], 'same declared source role');
  assert.deepEqual(round.build, hostPolicy[reference ? 'referenceBuild' : 'candidateBuild'], 'same declared release material');
  for (const key of ['toolchainSha256', 'collectorSha256', 'catalogueSha256']) {
    digest(round[key]); assert.equal(round[key], profile[key], key);
  }
  assert.equal(round.classId, hostPolicy.classId);
  assert.deepEqual(round.classBefore, hostPolicy.facts, 'exact before class, no CPU pooling');
  assert.deepEqual(round.classAfter, hostPolicy.facts, 'exact after class, no silent quota or image drift');
  assert.equal(round.wallMethod, hostPolicy.wallMethod); assert.equal(round.memoryMethod, hostPolicy.memoryMethod);
  assert(time(round.startedAt) < time(round.endedAt), 'nonempty round interval');
  const rows = matrixFor(catalogue, round.host);
  assert(Array.isArray(round.cells) && round.cells.length === rows.length, 'complete host matrix');
  const summaries = round.cells.map((cell, cellIndex) => {
    exact(cell, ['selector', 'attempts'], 'complete numeric cell'); assert.equal(cell.selector, rows[cellIndex].id);
    assert(Array.isArray(cell.attempts) && cell.attempts.length === 6, 'one warmup plus ALL five measured attempts');
    const measurements = cell.attempts.map((attempt, index) => metrics(attempt, index, round.host));
    if (round.host === 'windows-x86_64') {
      assert(cell.attempts.every(attempt => attempt.qpcFrequency === cell.attempts[0].qpcFrequency),
        'one native QPC frequency within a cell');
    }
    return { selector: cell.selector, wall: summarize(measurements.slice(1).map(row => row.wall)),
      memory: summarize(measurements.slice(1).map(row => row.memory)) };
  });
  return { round, summaries };
}

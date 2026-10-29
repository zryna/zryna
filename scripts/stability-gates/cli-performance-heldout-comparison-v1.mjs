// Pure equality checks only; the live owner must supply original observations and anchor capability.
import assert from 'node:assert/strict';
import { projectNumericHostFacts } from './cli-performance-campaign-host-facts-v1.mjs';
import { canonical, sha256 } from './input.mjs';
export const HELDOUT_FLAGS = Object.freeze({ sourceAuthenticated: false, proofQualified: false,
  performanceQualified: false, hostAdmitted: false, baselineAdmitted: false, baselineAuthority: null,
  thresholdAuthority: null, thresholds: null, closureStatus: 'blocked' });
function selected(anchor, host, source) {
  for (const name of ['sourceFileBytesVerified', 'tableOriginalBytesVerified', 'tableTemporalOrderVerified'])
    assert.equal(anchor[name], true, `live anchor ${name}`);
  for (const name of ['sourceAuthenticated', 'proofQualified', 'performanceQualified']) assert.equal(anchor[name], false);
  assert.equal(anchor.current.host, host);
  assert.deepEqual({ commit: anchor.current.source.commit, tree: anchor.current.source.tree }, source);
  assert.deepEqual(anchor.baselineSource, source, 'same-source first calibration');
  assert.deepEqual(anchor.profile.sources.candidate, source);
  assert.deepEqual(anchor.profile.sources.reference, source);
  const rows = anchor.profile.hosts.filter(row => row.host === host); assert.equal(rows.length, 1);
  return rows[0];
}
export function compareHeldoutHost(anchor, observation, source) {
  assert.equal(arguments.length, 3);
  const host = observation.host.platform, row = selected(anchor, host, source);
  assert.deepEqual(observation.companion.source, source);
  assert.equal(observation.companion.platform, host);
  assert.deepEqual(observation.companion.kernel, observation.host.kernel);
  if (host === 'linux-x86_64') {
    const text = observation.host.affinity.text;
    for (const field of ['Pid', 'Tgid']) {
      const matches = text.split('\n').filter(line => line.startsWith(`${field}:`)); assert.equal(matches.length, 1);
      assert.equal(matches[0].slice(field.length + 1).trim(), String(observation.companion.collectorNodePid));
    }
  }
  const projected = projectNumericHostFacts(host, observation.host, observation.companion.native?.decoded ?? null,
    { status: 'observed', value: 'github-actions-hosted-vm' });
  assert.equal(projected.classId, row.classId, 'actual premeasurement host class matches immutable table');
  assert.deepEqual(projected.facts, row.facts, 'actual premeasurement host facts match immutable table');
  return { host, classId: projected.classId, factsSha256: sha256(canonical(projected.facts)), ...HELDOUT_FLAGS };
}
export function compareHeldoutTools(anchor, host, source, tools, cliSha256, collectorSha256) {
  assert.equal(arguments.length, 6);
  const row = selected(anchor, host, source), expected = anchor.tools.filter(value => value.host === host);
  assert.equal(expected.length, 1); assert.deepEqual(tools, expected[0], 'actual premeasurement semantic tools match anchored host');
  assert.equal(cliSha256, row.candidateBuild.cliSha256, 'actual measured release CLI matches immutable table');
  assert.equal(collectorSha256, anchor.profile.collectorSha256, 'same fixed collector source bundle');
  return { host, semanticToolsSha256: sha256(canonical(tools)), cliSha256, collectorSha256, ...HELDOUT_FLAGS };
}

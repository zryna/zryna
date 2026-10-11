import assert from 'node:assert/strict';
import path from 'node:path';
import { digest, readStableFile } from './registry-files.mjs';
import { reject } from './evidence-input.mjs';
import { rustAccounting, requireNodeTests } from './evidence-counts.mjs';

const m2Ids = ['registry-and-fixture-contract', 'public-fixed-oracle-corpus', 'internal-boundary-evidence'];
const m2InternalIds = ['portable-contracts', 'adapter-boundaries', 'public-control-flow-profile',
  'driver-pipeline', 'javascript-backend', 'webassembly-backend', 'native-backend', 'native-mir',
  'syntax-boundaries', 'portable-ir', 'control-flow-semantics', 'native-link-run',
  'retained-stage-identity', 'module-closure', 'workspace-source'];
const m3Ids = ['registry-self-tests', 'v4-source-contract', 'candidate-fixed-oracles', 'public-profile-corpus',
  'layout-boundaries', 'runtime-abi-boundaries', 'ownership-ir-boundaries', 'ownership-source-boundaries',
  'targets-and-native-mir', 'candidate-security-runtime-and-transactions'];
const exclusionSuffix = ': unsupported on Windows; Linux evidence required by aggregate';
const m1Common = ['m1_registry_freezes_the_supported_slice',
  'm1_javascript_and_webassembly_match_every_portable_fixture',
  'm1_invalid_source_is_target_independent_and_publishes_nothing',
  'm1_bool_host_normalization_matches_without_enabling_bool_source'];
export function m1Accounting(output, host) {
  if (!['linux', 'windows'].includes(host)) reject('actual M1 host required');
  const accounting = rustAccounting(output);
  const expected = [...m1Common, host === 'linux'
    ? 'm1_linux_three_target_results_and_artifacts_match_every_fixture'
    : 'm1_windows_native_and_all_runs_are_rejected_without_a_bundle'];
  assert.deepEqual([...accounting.names].sort(), expected.sort(), 'exact current-host M1 named passing inventory');
  assert.equal(accounting.ignored, 0, 'M1 required tests must execute nonignored');
  assert.deepEqual(output.match(/^running \d+ tests?$/gm), ['running 5 tests'], 'single complete M1 selected suite');
  return accounting;
}
export const m2RetainedProof = 'native::link_run_tests::retained_stage_identity_rejects_directory_replacement';
export function m2RetainedAccounting(output, host) {
  if (host === 'linux') return { ...rustAccounting(output, m2RetainedProof), excluded: false };
  if (host !== 'windows') reject('actual M2 host required');
  // The unchanged selected test is cfg(Linux x86_64); Windows records zero execution.
  if (!/^\s*running 0 tests\s*test result: ok\. 0 passed; 0 failed; 0 ignored; 0 measured; [1-9]\d* filtered out; finished in [^\n]+\s*$/.test(output)) {
    reject('exact Windows M2 source-cfg exclusion block required');
  }
  return { passed: 0, ignored: 0, names: [], excluded: true };
}
function requireM2RetainedSource(registry, sourceRoot) {
  const sourcePath = 'crates/zryna-driver/src/native.rs';
  const entry = registry.sources.find(item => item.path === sourcePath);
  if (!entry || !sourceRoot) reject('frozen M2 retained-stage source descriptor required');
  const bytes = readStableFile(path.join(sourceRoot, sourcePath), 512 * 1024);
  assert.equal(bytes.length, entry.bytes, 'M2 retained-stage source size');
  assert.equal(digest(bytes), entry.sha256, 'M2 retained-stage source digest');
  assert(bytes.toString('utf8').includes('#[cfg(all(target_os = "linux", target_arch = "x86_64"))]\n    #[test]\n    fn retained_stage_identity_rejects_directory_replacement()'), 'exact M2 retained-stage test source cfg');
}

export function foundationStages(output, { family, expected, completion, exclusions = [] }) {
  const numbered = family !== 'm3';
  const marker = numbered ? new RegExp(`^\\[${family} (\\d+)\\/(\\d+)\\] (.+)$`, 'gm') : /^\[m3\] (.+)$/gm;
  const hits = [...output.matchAll(marker)].map((match, index) => {
    const label = match[numbered ? 3 : 1];
    const excluded = label.endsWith(exclusionSuffix);
    const id = excluded ? label.slice(0, -exclusionSuffix.length) : label;
    if (numbered && (Number(match[1]) !== index + 1 || Number(match[2]) !== expected.length)) reject('foundation stage numbering mismatch');
    if (excluded && !exclusions.includes(id)) reject('unauthorized foundation host exclusion');
    return { id, excluded, index: match.index, marker: match[0] };
  });
  assert.deepEqual(hits.map(hit => hit.id), expected, 'complete fixed ordered unique foundation stage inventory');
  const prefix = completion.slice(0, completion.indexOf(' passed'));
  const endings = [...output.matchAll(new RegExp(`^${prefix} passed[^\\n]*$`, 'gm'))];
  assert.equal(endings.length, 1, 'single foundation completion');
  assert.equal(endings[0][0], completion, 'exact current-host foundation completion');
  const ending = endings[0].index;
  if (!hits.length || ending <= hits.at(-1).index) reject('foundation completion precedes required stages');
  if (/^test .* \.\.\. |^test result:|^\[M0 |^\[m[23]/m.test(output.slice(ending + completion.length))) reject('execution or stages after foundation completion');
  return hits.map((hit, index) => ({ ...hit,
    output: output.slice(hit.index + hit.marker.length, hits[index + 1]?.index ?? ending) }));
}
function frozenFoundation(registry, id, sourceRoot) {
  const entry = registry.foundations.find(item => item.id === id);
  if (!entry || !sourceRoot) reject('validated source foundation inventory required');
  return JSON.parse(readStableFile(path.join(sourceRoot, entry.registry.path), 512 * 1024));
}
export function foundationAccounting(id, output, registry, { sourceRoot, host } = {}) {
  if (!['linux', 'windows'].includes(host)) reject('actual foundation host required');
  const all = rustAccounting(output);
  const exclusionIds = [];
  const executedEvidenceIds = [];
  if (id === 'm0-full') {
    const frozen = frozenFoundation(registry, 'm0', sourceRoot);
    const stages = foundationStages(output, { family: 'M0', expected: frozen.commands.map(command => command.id),
      completion: `M0 conformance passed on ${host}: 18 commands, 13 registered proof suites.` });
    for (const [index, command] of frozen.commands.entries()) {
      if (command.executable === 'cargo' && command.args[0] === 'test') rustAccounting(stages[index].output);
      if (command.executable === 'node' && command.args.includes('--test')) requireNodeTests(stages[index].output);
    }
  } else if (id === 'm1-full') {
    m1Accounting(output, host);
  } else if (id === 'm2-full') {
    const stages = foundationStages(output, { family: 'm2', expected: m2Ids,
      completion: 'M2 conformance passed: 3 ordered gates.' });
    requireNodeTests(stages[0].output);
    rustAccounting(stages[1].output);
    rustAccounting(stages[2].output);
    const internal = foundationStages(stages[2].output, { family: 'm2-quick', expected: m2InternalIds,
      completion: 'M2 quick gate passed: 15 ordered checks.' });
    requireM2RetainedSource(registry, sourceRoot);
    for (const [index, stage] of internal.entries()) {
      if (index < 2) requireNodeTests(stage.output);
      else if (stage.id === 'retained-stage-identity') {
        const accounting = m2RetainedAccounting(stage.output, host);
        (accounting.excluded ? exclusionIds : executedEvidenceIds).push('m2/retained-stage-identity');
      }
      else rustAccounting(stage.output);
    }
  } else if (id === 'm3-full') {
    const frozen = frozenFoundation(registry, 'm3', sourceRoot);
    const linuxOnly = frozen.evidence.filter(entry => entry.host === 'linux-x86_64').map(entry => entry.id);
    assert.equal(linuxOnly.length, 7, 'frozen seven Linux-only M3 evidence identities');
    const stages = foundationStages(output, { family: 'm3', expected: [...m3Ids, ...frozen.evidence.map(entry => entry.id)],
      completion: `M3 full gate passed on ${host === 'windows' ? 'win32' : 'linux'}/x64.`,
      exclusions: host === 'windows' ? linuxOnly : [] });
    for (const [index, stage] of stages.slice(0, 10).entries()) {
      if (index < 2) requireNodeTests(stage.output);
      else rustAccounting(stage.output);
    }
    for (const [index, evidence] of frozen.evidence.entries()) {
      const stage = stages[index + 10];
      if (host === 'windows' && linuxOnly.includes(evidence.id)) {
        if (!stage.excluded || /\S/.test(stage.output)) reject('Windows exclusion missing or borrowing execution');
        exclusionIds.push(evidence.id);
      } else {
        if (stage.excluded) reject('executed M3 proof replaced by exclusion');
        rustAccounting(stage.output, evidence.test);
        executedEvidenceIds.push(evidence.id);
      }
    }
  } else reject('unknown full foundation gate');
  return { ...all, exclusionIds, executedEvidenceIds };
}

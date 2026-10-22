import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { expectedRegistrySha256, loadAndValidateM4Conformance } from '../scripts/check-m4-conformance.mjs';
import { loadReviewedLimitContracts } from '../scripts/m4/limit-contracts.mjs';
import {
  canonicalSupportBytes, checkSupportFiles, extractM4SupportFence, projectM4Support,
  renderM4SupportPage, validateSupportProjection, workspaceRoot,
} from '../scripts/m4/support.mjs';

test('compiled and Markdown canonical bytes independently retain every projected source and contract field', () => {
  const registry = loadAndValidateM4Conformance();
  const plan = loadReviewedLimitContracts(registry, workspaceRoot);
  const expected = {
    schema: 'zryna.m4.support.v1', authority: 'static-contract',
    registrySha256: expectedRegistrySha256, authenticatedEvidence: 'not-evaluated',
    toolchains: registry.toolchains,
    hosts: ['linux', 'win32'].map(os => {
      const { architecture, portabilityClaim } = registry.rows.find(row => row.host.os === os).host;
      return { os, architecture, portabilityClaim };
    }),
    witClosure: registry.scope.witClosure,
    requiredCaseKindsAuthority: 'independent-mode-contract-plan',
    contractPlan: registry.contractPlan, modeRequirements: plan.modeRequirements,
    limitContracts: plan.contracts, limitExclusions: plan.exclusions, sources: registry.sources,
    requiredExternalEvidence: registry.scope.requiredExternalEvidence,
    lifecycleExemptions: registry.scope.lifecycleExemptions,
    rows: registry.rows.map(row => ({
      id: row.id, profile: row.profile, mode: row.mode, host: row.host,
      ownerIssues: row.ownerIssues, contract: row.contract, selector: row.selector,
      identities: row.identities,
      capabilities: row.capabilities, artifacts: row.artifacts, limits: row.limits,
      proofCases: row.cases.map(id => registry.cases.find(entry => entry.id === id)),
      requiredGates: row.requiredGates,
      proofGaps: row.limits.flatMap(limit => {
        if (limit.mapping.kind === 'mapped' || limit.mapping.kind === 'excluded') return [];
        return [{ limit: limit.id, contractId: limit.contractId, mappingKind: limit.mapping.kind,
          requirements: limit.mapping.requirements }];
      }),
      authenticatedExecution: 'not-evaluated', exclusions: row.exclusions,
    })),
    exclusions: registry.exclusions,
  };
  const expectedBytes = Buffer.from(JSON.stringify(expected, null, 2) + '\n');
  const compiled = readFileSync(path.join(workspaceRoot, 'apps/zryna/src/m4_support.json'));
  assert(compiled.equals(expectedBytes), 'complete source-derived bytes, independent of projector');
  const markdown = readFileSync(path.join(workspaceRoot, 'docs/M4_SUPPORT.md'), 'utf8');
  assert(extractM4SupportFence(markdown).equals(expectedBytes));
});

test('partial and unresolved boundary requirements remain visible without execution claims', () => {
  const registry = loadAndValidateM4Conformance();
  const plan = loadReviewedLimitContracts(registry, workspaceRoot);
  const fixture = structuredClone(registry);
  const row = fixture.rows.find(entry => entry.id === 'command-linux');
  row.limits[0].mapping = { kind: 'partial',
    guard: { kind: 'pair', exactCase: 'guard-exact', firstExtraCase: 'guard-extra' },
    bindingCaseIds: [], requirements: ['reachable-public-boundary'] };
  row.limits[1].mapping = { kind: 'unresolved', requirements: ['owning-guard-proof'] };
  const projection = projectM4Support(fixture, expectedRegistrySha256, plan);
  const actual = projection.rows.find(entry => entry.id === row.id);
  assert.deepEqual(actual.proofGaps.slice(0, 2), [
    { limit: row.limits[0].id, contractId: row.limits[0].contractId, mappingKind: 'partial',
      requirements: ['reachable-public-boundary'] },
    { limit: row.limits[1].id, contractId: row.limits[1].contractId, mappingKind: 'unresolved',
      requirements: ['owning-guard-proof'] },
  ]);
  assert.equal(actual.authenticatedExecution, 'not-evaluated');
  assert.equal(projection.authenticatedEvidence, 'not-evaluated');
});

test('CLI contract and Markdown project the complete checked M4 matrix', () => {
  const projection = checkSupportFiles();
  assert.equal(projection.authority, 'static-contract');
  assert.equal(projection.authenticatedEvidence, 'not-evaluated');
  assert.deepEqual(projection.hosts.map(host => [host.os, host.architecture]), [['linux', 'x64'], ['win32', 'x64']]);
  assert.deepEqual(projection.rows.map(row => row.id), [
    'component-linux', 'component-windows', 'browser-linux', 'browser-windows',
    'command-linux', 'command-windows', 'installed-command-linux', 'installed-command-windows',
    'server-status-linux', 'server-status-windows', 'server-clock-linux', 'server-clock-windows',
  ]);
  assert.deepEqual(projection.rows.map(row => [
    row.id, row.host.os, row.host.architecture, row.host.baseline,
  ]), [
    ['component-linux', 'linux', 'x64', 'ubuntu-latest:observed-baseline-required'],
    ['component-windows', 'win32', 'x64', 'windows-latest:observed-baseline-required'],
    ['browser-linux', 'linux', 'x64', 'ubuntu-22.04'],
    ['browser-windows', 'win32', 'x64', 'windows-2022'],
    ['command-linux', 'linux', 'x64', 'ubuntu-latest:observed-baseline-required'],
    ['command-windows', 'win32', 'x64', 'windows-latest:observed-baseline-required'],
    ['installed-command-linux', 'linux', 'x64', 'ubuntu-24.04'],
    ['installed-command-windows', 'win32', 'x64', 'windows-2022'],
    ['server-status-linux', 'linux', 'x64', 'ubuntu-latest:observed-baseline-required'],
    ['server-status-windows', 'win32', 'x64', 'windows-latest:observed-baseline-required'],
    ['server-clock-linux', 'linux', 'x64', 'ubuntu-latest:observed-baseline-required'],
    ['server-clock-windows', 'win32', 'x64', 'windows-latest:observed-baseline-required'],
  ]);
  for (const row of projection.rows) assert.equal(row.authenticatedExecution, 'not-evaluated');
});

test('changed digest, row, host, authority, limits and exclusions fail complete projection equality', () => {
  const expected = projectM4Support(loadAndValidateM4Conformance());
  for (const mutate of [
    p => { p.registrySha256 = 'a'.repeat(64); },
    p => { p.rows.pop(); },
    p => { p.rows.push(structuredClone(p.rows[0])); },
    p => { p.rows[0].host.baseline = 'windows-desktop'; },
    p => { p.authority = 'authenticated-execution'; },
    p => { p.rows[0].limits[0].maximum++; },
    p => { p.rows[0].authenticatedExecution = 'verified'; },
    p => { p.rows[0].capabilities.effective.push('filesystem'); },
    p => { p.rows[0].artifacts.pop(); },
    p => { p.rows[0].identities.world = 'unreviewed:world/v1'; },
    p => { p.rows[0].identities.witClosure.pop(); },
    p => { p.rows[0].profile = 'unreviewed-profile'; },
    p => { p.rows[0].selector.command = 'run'; },
    p => { p.rows[0].proofCases[0].test.source = 'unreviewed/source.rs'; },
    p => { p.rows[0].limits[0].mapping = { kind: 'unresolved', requirements: ['missing-proof'] }; },
    p => { p.sources[0].sha256 = 'b'.repeat(64); },
    p => { p.contractPlan.sha256 = 'c'.repeat(64); },
    p => { p.modeRequirements[0].requiredCaseKinds = []; },
    p => { p.requiredCaseKindsAuthority = 'self-approved'; },
    p => { p.limitContracts[0].sourceBindings = []; },
    p => { p.limitExclusions.pop(); },
    p => { p.exclusions = p.exclusions.filter(e => e !== 'installed-server-status'); },
    p => { p.rows[0].proofGaps = [{ limit: 'invented', requirements: ['invented'] }]; },
  ]) {
    const changed = structuredClone(expected);
    mutate(changed);
    assert.throws(() => validateSupportProjection(canonicalSupportBytes(changed), expected));
  }
  assert.throws(() => validateSupportProjection(Buffer.from(JSON.stringify(expected)), expected));
});

test('support fence is singular and canonical, preserved in the Markdown-only v1 whitelist', () => {
  const expected = projectM4Support(loadAndValidateM4Conformance());
  const page = renderM4SupportPage(expected);
  validateSupportProjection(extractM4SupportFence(page), expected);
  assert.throws(() => extractM4SupportFence(page + page));
  assert.throws(() => extractM4SupportFence(page.replace('```json\n', '```js\n')));
  for (const marker of ['zryna.m4.support.v1:begin', 'zryna.m4.support.v1:end']) {
    assert.throws(() => extractM4SupportFence(page.replace(marker, `<!-- ${marker} -->`)));
    assert.throws(() => extractM4SupportFence(page.replace(marker, `prefix ${marker}`)));
    assert.throws(() => extractM4SupportFence(page.replace(marker, 'missing-marker')));
  }
  assert.throws(() => extractM4SupportFence(page.replace('begin\n\n```json', 'begin\n```json')));
  assert(!page.includes('<!--'), 'support page uses no importer-forbidden HTML markers');
  const reversed = 'zryna.m4.support.v1:end\n\nzryna.m4.support.v1:begin\n\n'
    + '```json\n' + canonicalSupportBytes(expected).toString('utf8') + '```\n\n';
  assert.throws(() => extractM4SupportFence(reversed), /markers must be ordered/u);
  const whitelist = JSON.parse(readFileSync(path.join(workspaceRoot, 'docs/website-bundle-v1.json')));
  assert.equal(whitelist.bundleSchema, 'zryna.docs.bundle.v1');
  for (const source of [
    'docs/M4_SUPPORT.md', 'docs/M4_CONFORMANCE.md', 'docs/WASI_COMMAND_GETTING_STARTED.md',
    'docs/WASI_COMMAND_MANIFEST_V1.md', 'docs/WASI_SERVER_STATUS_V1.md',
    'docs/WASI_SERVER_CLOCK_STATUS_V1.md', 'examples/wasi-command/README.md',
    'examples/wasi-server/README.md', 'examples/wasi-server/CLOCK_EXAMPLE.md',
  ]) {
    assert.equal(whitelist.documents.filter(item => item.source === source).length, 1, source);
  }
  assert(whitelist.documents.every(item => item.path.endsWith('.md')));
  assert(!whitelist.documents.some(item => item.source.endsWith('m4_support.json')));
});

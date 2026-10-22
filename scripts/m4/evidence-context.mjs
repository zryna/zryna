import assert from 'node:assert/strict';
import { array, exact, integer, oid, reject, retained, sha, text } from './evidence-input.mjs';

export function sourceBinding(source, expected) {
  exact(source, ['commit', 'tree', 'ref', 'cleanBefore', 'cleanAfter'], 'source');
  oid(source.commit); oid(source.tree); text(source.ref, 'source ref');
  assert.deepEqual({ commit: source.commit, tree: source.tree, ref: source.ref }, expected, 'external candidate source binding');
  assert.equal(source.cleanBefore, true, 'clean source before execution');
  assert.equal(source.cleanAfter, true, 'clean source after execution');
}
export function hostBinding(host, lane) {
  exact(host, ['os', 'architecture', 'distribution', 'version', 'kernel', 'runnerImage', 'runnerImageVersion'], 'actual host');
  assert(['linux', 'windows'].includes(host.os));
  assert.equal(host.architecture, 'x86_64');
  for (const field of ['distribution', 'version', 'kernel', 'runnerImage', 'runnerImageVersion']) text(host[field], `actual ${field}`);
  if (lane === 'browser') assert.equal(host.version, host.os === 'linux' ? '22.04' : '2022', 'pinned browser OS baseline');
  if (lane === 'installed') assert.equal(host.version, host.os === 'linux' ? '24.04' : '2022', 'pinned installed OS baseline');
  if (host.os === 'linux') assert.equal(host.distribution, 'ubuntu', 'supported Linux distribution');
  if (lane !== 'core' && host.os === 'windows') assert.equal(host.distribution, 'windows-server', 'server receipt is not desktop proof');
}
export function workflowBinding(workflow, source) {
  if (workflow === null) return;
  exact(workflow, ['repository', 'runId', 'attempt', 'jobName', 'jobKey', 'workflowPath',
    'workflowSha256', 'event', 'headCommit', 'executionCommit', 'checkoutCommit'], 'workflow');
  integer(workflow.runId, 1, Number.MAX_SAFE_INTEGER, 'workflow run');
  integer(workflow.attempt, 1, 1000, 'workflow attempt');
  assert.equal(workflow.repository, 'zryna/zryna');
  for (const field of ['jobName', 'jobKey', 'workflowPath', 'event']) text(workflow[field], field);
  sha(workflow.workflowSha256); oid(workflow.headCommit); oid(workflow.executionCommit); oid(workflow.checkoutCommit);
  assert.equal(workflow.checkoutCommit, source.commit, 'actual candidate checkout');
  // Synthetic merge heads require explicit external job/source equivalence authentication.
  // They are never silently rewritten into candidate head identities here.
}
export function observations(receipt, root, expected) {
  array(receipt.tools, 8, 'tools'); array(receipt.materials, 32, 'materials');
  assert.deepEqual(receipt.tools, expected.tools, 'externally bound tool observations');
  assert.deepEqual(receipt.materials, expected.materials, 'externally bound material observations');
  assert.deepEqual(receipt.tools.map(tool => tool.name), ['node', 'rustc', 'cargo', 'pnpm']);
  const pins = { node: '22.22.1', rustc: '1.97.1', cargo: '1.97.1', pnpm: '11.18.0' };
  for (const tool of receipt.tools) {
    exact(tool, ['name', 'version', 'executable', 'versionArgv', 'versionLog'], 'tool');
    assert.equal(tool.version, pins[tool.name], 'pinned tool version');
    exact(tool.executable, ['path', 'bytes', 'sha256'], 'actual executable identity');
    text(tool.executable.path, 'actual executable path', 2048);
    integer(tool.executable.bytes, 1, 512 * 1024 * 1024, 'tool size'); sha(tool.executable.sha256);
    assert.deepEqual(tool.versionArgv, tool.name === 'pnpm'
      ? [receipt.tools[0].executable.path, tool.executable.path, '--version'] : [tool.executable.path, '--version']);
    const output = new TextDecoder('utf-8', { fatal: true }).decode(retained(tool.versionLog, root, 64 * 1024,
      `observations/${tool.name}.version.log`)).trim();
    const pattern = tool.name === 'node' ? /^v22\.22\.1$/ : tool.name === 'pnpm' ? /^11\.18\.0$/
      : new RegExp(`^${tool.name} 1\\.97\\.1(?: |$)`);
    if (!pattern.test(output)) reject('actual version log differs from pinned tool');
  }
  const names = receipt.materials.map(item => item.name);
  if (new Set(names).size !== names.length) reject('duplicate external material');
  for (const material of receipt.materials) {
    exact(material, ['name', 'path', 'bytes', 'sha256'], 'material');
    text(material.name, 'material name', 96); text(material.path, 'material path', 2048);
    integer(material.bytes, 1, 256 * 1024 * 1024, 'material bytes'); sha(material.sha256);
  }
  for (const name of ['typescript-wrapper', 'typescript-implementation', 'cargo-lock', 'runtime-material-binding']) {
    if (!names.includes(name)) reject(`actual material identity ${name} missing`);
  }
}

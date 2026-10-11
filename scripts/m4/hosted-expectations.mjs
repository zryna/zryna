import { isDeepStrictEqual } from 'node:util';

export const HOSTED_LANES = Object.freeze([
  ['core', 'ubuntu-latest', 'm4 (ubuntu-latest)', 'm4-platform'],
  ['core', 'windows-latest', 'm4 (windows-latest)', 'm4-platform'],
  ['browser', 'ubuntu-22.04', 'pinned browser (ubuntu-22.04)', 'conformance'],
  ['browser', 'windows-2022', 'pinned browser (windows-2022)', 'conformance'],
  ['installed', 'ubuntu-24.04', 'fresh installed H1 (ubuntu-24.04)', 'qualify'],
  ['installed', 'windows-2022', 'fresh installed H1 (windows-2022)', 'qualify'],
]);
export function requireEqual(actual, expected, label) {
  if (!isDeepStrictEqual(actual, expected)) throw new Error(`M4 hosted: ${label} differs`);
}
export function exactKeys(value, wanted, label) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`M4 hosted: invalid ${label}`);
  }
  requireEqual(Object.keys(value).sort(), wanted.slice().sort(), `${label} keys`);
}
export function validateExpectations(value) {
  exactKeys(value, ['format', 'repository', 'source', 'registrySha256', 'workflowSources', 'workflowExecution', 'eventSource', 'jobs', 'aggregate'], 'expectations');
  requireEqual(value.format, 'zryna.m4-hosted-expectations.v1', 'expectations format');
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(value.repository)) throw new Error('M4 hosted: invalid repository');
  exactKeys(value.source, ['commit', 'tree', 'ref'], 'source');
  for (const identity of [value.workflowExecution, value.eventSource]) {
    exactKeys(identity, ['commit', 'tree', 'parents'], 'workflow/event source');
    if (![identity.commit, identity.tree].every(sha => /^[a-f0-9]{40}$/.test(sha))
        || !Array.isArray(identity.parents) || identity.parents.length > 2
        || !identity.parents.every(sha => /^[a-f0-9]{40}$/.test(sha))
        || new Set(identity.parents).size !== identity.parents.length) {
      throw new Error('M4 hosted: explicit immutable workflow and event source required');
    }
  }
  if (![value.source.commit, value.source.tree].every(sha => /^[a-f0-9]{40}$/.test(sha))
      || !/^[a-f0-9]{64}$/.test(value.registrySha256)
      || typeof value.source.ref !== 'string' || !/^refs\/(heads|pull)\/[A-Za-z0-9_./-]+$/.test(value.source.ref)) {
    throw new Error('M4 hosted: invalid source identity');
  }
  if (!Array.isArray(value.workflowSources) || value.workflowSources.length !== 3) {
    throw new Error('M4 hosted: complete workflow source inventory required');
  }
  requireEqual(value.workflowSources.map(item => item.path).sort(), [
    '.github/workflows/browser-conformance.yml', '.github/workflows/ci.yml', '.github/workflows/installed-command-h1.yml',
  ], 'workflow paths');
  for (const item of value.workflowSources) {
    exactKeys(item, ['path', 'bytes', 'sha256'], 'workflow source');
    if (!Number.isSafeInteger(item.bytes) || item.bytes < 1 || item.bytes > 1024 * 1024
        || !/^[a-f0-9]{64}$/.test(item.sha256)) throw new Error('M4 hosted: invalid workflow descriptor');
  }
  if (!Array.isArray(value.jobs) || value.jobs.length !== HOSTED_LANES.length) throw new Error('M4 hosted: six host jobs required');
  const seen = new Set();
  for (const job of value.jobs) {
    exactKeys(job, ['lane', 'baseline', 'runId', 'attempt', 'jobId', 'jobName', 'jobKey',
      'runWorkflowPath', 'workflowPath', 'workflowSha256', 'event', 'headCommit', 'checkRunId',
      'checkUrl', 'artifact', 'host', 'tools', 'materials', 'evidenceDirectory'], 'job expectation');
    const lane = HOSTED_LANES.find(item => item[0] === job.lane && item[1] === job.baseline);
    if (!lane || seen.has(`${job.lane}-${job.baseline}`)) throw new Error('M4 hosted: duplicate or unsupported lane');
    seen.add(`${job.lane}-${job.baseline}`);
    // Reusable-job names gain their caller prefix in the Actions Jobs API.
    const expectedName = job.lane === 'core' ? lane[2] : `m4 ${job.lane} / ${lane[2]}`;
    requireEqual(job.jobName, expectedName, 'job name');
    requireEqual(job.jobKey, lane[3], 'job key');
    requireEqual(job.runWorkflowPath, '.github/workflows/ci.yml', 'run workflow path');
    const workflowPath = job.lane === 'core' ? '.github/workflows/ci.yml'
      : job.lane === 'browser' ? '.github/workflows/browser-conformance.yml' : '.github/workflows/installed-command-h1.yml';
    requireEqual(job.workflowPath, workflowPath, 'job workflow path');
    requireEqual(job.workflowSha256, value.workflowSources.find(item => item.path === workflowPath).sha256, 'workflow digest');
    for (const key of ['runId', 'attempt', 'jobId', 'checkRunId']) {
      if (!Number.isSafeInteger(job[key]) || job[key] < 1) throw new Error(`M4 hosted: invalid ${key}`);
    }
    if (!['pull_request', 'workflow_dispatch', 'push'].includes(job.event)
        || !/^[a-f0-9]{40}$/.test(job.headCommit)) throw new Error('M4 hosted: invalid run provenance');
    exactKeys(job.artifact, ['id', 'name', 'sha256', 'bytes', 'expiresAt'], 'artifact');
    if (!Number.isSafeInteger(job.artifact.id) || job.artifact.id < 1
        || !Number.isSafeInteger(job.artifact.bytes) || job.artifact.bytes < 1
        || !/^[a-f0-9]{64}$/.test(job.artifact.sha256)) throw new Error('M4 hosted: invalid artifact identity');
    requireEqual(job.artifact.name, `m4-${job.lane}-${job.baseline}-${value.source.commit}-${job.runId}-${job.attempt}`, 'artifact name');
    if (!Number.isFinite(Date.parse(job.artifact.expiresAt))) throw new Error('M4 hosted: invalid artifact expiry');
    const os = job.baseline.startsWith('ubuntu') ? 'linux' : 'windows';
    if (job.host?.os !== os || job.host?.architecture !== 'x86_64'
        || !['distribution', 'version', 'kernel', 'runnerImage', 'runnerImageVersion'].every(key =>
          typeof job.host[key] === 'string' && job.host[key].length > 0)) throw new Error('M4 hosted: observed host facts required');
    if (!Array.isArray(job.tools) || !job.tools.length || !Array.isArray(job.materials)) throw new Error('M4 hosted: tool/material pins required');
    if (typeof job.evidenceDirectory !== 'string' || job.evidenceDirectory.length > 1024
        || !/^(?:\/|[A-Za-z]:[\\/])/.test(job.evidenceDirectory)) throw new Error('M4 hosted: original evidence directory required');
  }
  const [first] = value.jobs;
  for (const job of value.jobs) for (const key of ['runId', 'attempt', 'event', 'headCommit']) {
    requireEqual(job[key], first[key], `same-run ${key}`);
  }
  if (new Set(value.jobs.map(job => job.jobId)).size !== 6
      || new Set(value.jobs.map(job => job.artifact.id)).size !== 6) throw new Error('M4 hosted: repeated job or artifact id');
  exactKeys(value.aggregate, ['jobId', 'checkRunId', 'checkUrl'], 'M4 aggregate');
  for (const key of ['jobId', 'checkRunId']) {
    if (!Number.isSafeInteger(value.aggregate[key]) || value.aggregate[key] < 1
        || value.jobs.some(job => job[key] === value.aggregate[key])) throw new Error('M4 hosted: invalid aggregate identity');
  }
  if (typeof value.aggregate.checkUrl !== 'string' || !value.aggregate.checkUrl.startsWith(`https://github.com/${value.repository}/`)) {
    throw new Error('M4 hosted: explicit aggregate check URL required');
  }
  return value;
}

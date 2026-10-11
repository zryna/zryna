import { createHash } from 'node:crypto';
import { digest } from './registry-files.mjs';
import { requireEqual } from './hosted-expectations.mjs';

// The execution commit is supplied independently. In a PR it can be a synthetic merge;
// its identity stays distinct from the candidate checkout and authenticated API run head.
export async function authenticateWorkflow(expectations, api, run) {
  const prefix = `/repos/${expectations.repository}`;
  const execution = expectations.workflowExecution;
  const candidate = await api(`${prefix}/git/commits/${expectations.source.commit}`);
  requireEqual(candidate.sha, expectations.source.commit, 'authenticated candidate commit');
  requireEqual(candidate.tree?.sha, expectations.source.tree, 'authenticated candidate tree');
  const commit = execution.commit === expectations.source.commit ? candidate
    : await api(`${prefix}/git/commits/${execution.commit}`);
  requireEqual(commit.sha, execution.commit, 'workflow execution commit');
  requireEqual(commit.tree?.sha, execution.tree, 'workflow execution tree');
  requireEqual(commit.parents?.map(parent => parent.sha), execution.parents, 'workflow execution parents');
  const event = expectations.eventSource;
  const eventCommit = event.commit === execution.commit ? commit
    : event.commit === expectations.source.commit ? candidate : await api(`${prefix}/git/commits/${event.commit}`);
  requireEqual(eventCommit.sha, event.commit, 'event commit');
  requireEqual(eventCommit.tree?.sha, event.tree, 'event tree');
  requireEqual(eventCommit.parents?.map(parent => parent.sha), event.parents, 'event parents');
  const provenance = expectations.jobs[0];
  requireEqual(run.id, provenance.runId, 'workflow run id');
  requireEqual(run.run_attempt, provenance.attempt, 'workflow run attempt');
  requireEqual(run.head_sha, expectations.source.commit, 'workflow run candidate');
  if (!Array.isArray(run.referenced_workflows)) throw new Error('M4 hosted: platform workflow references required');
  for (const workflowPath of ['.github/workflows/browser-conformance.yml', '.github/workflows/installed-command-h1.yml']) {
    const refs = run.referenced_workflows.filter(item => typeof item.path === 'string'
      && item.path.startsWith(`${expectations.repository}/${workflowPath}@`));
    requireEqual(refs.length, 1, 'exact platform-attached reusable workflow');
    if (typeof refs[0].ref !== 'string' || refs[0].ref.length > 1024
        || !/^(?:refs\/(?:heads|pull)\/[A-Za-z0-9_./-]+|[a-f0-9]{40})$/.test(refs[0].ref)) {
      throw new Error('M4 hosted: invalid platform workflow ref');
    }
    requireEqual(refs[0].path, `${expectations.repository}/${workflowPath}@${refs[0].ref}`, 'platform workflow path/ref');
    requireEqual(refs[0].sha, execution.commit, 'platform execution workflow commit');
  }
  if (provenance.event === 'pull_request') {
    requireEqual(event.commit, execution.commit, 'supported PR event/workflow attachment');
    if (event.commit !== provenance.headCommit && !event.parents.includes(provenance.headCommit)) {
      throw new Error('M4 hosted: event commit does not contain authenticated PR head');
    }
  } else {
    requireEqual(event.commit, provenance.headCommit, 'non-PR event commit');
    requireEqual(execution.commit, provenance.headCommit, 'non-PR execution commit');
  }
  for (const descriptor of expectations.workflowSources) {
    const result = await api(`${prefix}/contents/${descriptor.path}?ref=${execution.commit}`);
    requireEqual(result.type, 'file', 'execution workflow file');
    requireEqual(result.path, descriptor.path, 'execution workflow path');
    requireEqual(result.encoding, 'base64', 'execution workflow encoding');
    if (typeof result.content !== 'string' || result.content.length > 2 * 1024 * 1024) {
      throw new Error('M4 hosted: workflow source response exceeds bound');
    }
    const encoded = result.content.replaceAll('\n', '');
    if (!/^[A-Za-z0-9+/]*={0,2}$/.test(encoded)) throw new Error('M4 hosted: invalid source encoding');
    const bytes = Buffer.from(encoded, 'base64');
    requireEqual(bytes.toString('base64'), encoded, 'canonical execution source encoding');
    requireEqual(bytes.length, descriptor.bytes, 'execution workflow byte count');
    requireEqual(result.size, descriptor.bytes, 'execution workflow API byte count');
    requireEqual(digest(bytes), descriptor.sha256, 'execution workflow source hash');
    const blob = createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
    requireEqual(result.sha, blob, 'execution workflow Git blob');
  }
}

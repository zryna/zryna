// Synthetic API only. Exact reviewed workflow bytes are data; no workflow or application runs.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { PERFORMANCE_REPOSITORY, PERFORMANCE_REPOSITORY_ID } from '../scripts/stability-gates/cli-performance-hosted-expectations.mjs';
import { PROSPECTIVE_POLICY_SHA256 } from '../scripts/stability-gates/cli-performance-prospective-policy-v1.mjs';
import { NUMERIC_LABELS, NUMERIC_WORKFLOWS, numericArtifactNames, numericSlots,
  requiredNumericSteps } from '../scripts/stability-gates/cli-performance-campaign-plan-v1.mjs';
export const PREFIX = `/repos/${PERFORMANCE_REPOSITORY}`;
export const NOW = Date.parse('2030-01-02T00:00:00Z');
export function campaignFixture(phase = 'baselines', selectedSource = null) {
  const responses = new Map(), calls = [], repository = { id: PERFORMANCE_REPOSITORY_ID, full_name: PERFORMANCE_REPOSITORY };
  const source = selectedSource ? structuredClone(selectedSource) :
    { commit: 'a'.repeat(40), tree: 'b'.repeat(40), ref: 'refs/heads/feat/performance-regression-gates' };
  const workflows = NUMERIC_WORKFLOWS.map(path => {
    const bytes = readFileSync(new URL(`../${path}`, import.meta.url)), descriptor = { path, bytes: bytes.length, sha256: sha256(bytes) };
    responses.set(`${PREFIX}/contents/${path}?ref=${source.commit}`, { type: 'file', path, size: bytes.length,
      encoding: 'base64', content: bytes.toString('base64'),
      sha: createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex') });
    return descriptor;
  });
  const plan = { format: 'zryna.cli-performance-campaign-plan.v1', phase, runId: 12345, source,
    policySha256: PROSPECTIVE_POLICY_SHA256, workflows };
  const run = { id: plan.runId, run_attempt: 1, status: 'completed', conclusion: 'success', event: 'workflow_dispatch',
    path: `.github/workflows/cli-performance-${phase}-v1.yml`, head_sha: source.commit,
    head_branch: source.ref.slice(11), repository, head_repository: structuredClone(repository), check_suite_id: 76543,
    referenced_workflows: phase === 'anchor' ? [] : [{ sha: source.commit, ref: source.ref,
      path: `${PERFORMANCE_REPOSITORY}/${NUMERIC_WORKFLOWS[3]}@${source.commit}` }] };
  const jobs = [], artifacts = [];
  numericSlots(phase).forEach((slot, index) => {
    const id = 100 + index, minute = index * 10;
    const stamp = m => `2030-01-01T00:${String(m).padStart(2, '0')}:00Z`;
    const job = { id, name: slot.name, run_id: run.id, run_attempt: 1, head_sha: source.commit,
      status: 'completed', conclusion: 'success', labels: [NUMERIC_LABELS[slot.host]], runner_group_id: 0,
      runner_group_name: 'GitHub Actions', runner_id: 1000000001 + index,
      runner_name: `GitHub Actions ${1000000001 + index}`, started_at: stamp(minute), completed_at: stamp(minute + 5),
      check_run_url: `https://api.github.com${PREFIX}/check-runs/${id}`,
      steps: requiredNumericSteps(slot).map(([name, conclusion], step) => ({ number: step + 1, name, status: 'completed', conclusion })) };
    jobs.push(job);
    responses.set(`${PREFIX}/check-runs/${id}`, { id, name: slot.name, head_sha: source.commit,
      status: 'completed', conclusion: 'success', app: { slug: 'github-actions' }, check_suite: { id: run.check_suite_id } });
    for (const name of numericArtifactNames(slot, run.id)) artifacts.push({ id: 200 + artifacts.length, name,
      size_in_bytes: 1000, digest: `sha256:${sha256(Buffer.from(name))}`, expired: false,
      created_at: stamp(minute + 4), expires_at: '2030-02-01T00:00:00Z',
      workflow_run: { id: run.id, repository_id: PERFORMANCE_REPOSITORY_ID, head_repository_id: PERFORMANCE_REPOSITORY_ID,
        head_sha: source.commit, head_branch: run.head_branch } });
  });
  responses.set(`${PREFIX}/actions/runs/${run.id}`, run);
  const history = { total_count: 1, workflow_runs: [run] };
  responses.set(`${PREFIX}/actions/workflows/cli-performance-${phase}-v1.yml/runs?event=workflow_dispatch&head_sha=${source.commit}&per_page=100`, history);
  responses.set(`${PREFIX}/git/commits/${source.commit}`, { sha: source.commit, tree: { sha: source.tree } });
  const jobResponse = { total_count: jobs.length, jobs }, artifactResponse = { total_count: artifacts.length, artifacts };
  responses.set(`${PREFIX}/actions/runs/${run.id}/attempts/1/jobs?per_page=100`, jobResponse);
  responses.set(`${PREFIX}/actions/runs/${run.id}/artifacts?per_page=100`, artifactResponse);
  const api = async path => { calls.push(path); if (!responses.has(path)) throw new Error(`Unexpected fixture endpoint ${path}`);
    return structuredClone(responses.get(path)); };
  return { plan, run, jobs, artifacts, responses, jobResponse, artifactResponse, history, calls, api,
    arguments: () => { const bytes = Buffer.from(canonical(plan)); return [bytes, sha256(bytes), api, NOW]; } };
}

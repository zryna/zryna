import { loadPublicationWorkflowInventory, verifyCompletePublicationJobs } from '../../m4/publication-workflows.mjs';
import { OID, REPO, TAG, bytes, id, requireValue, sha256 } from './common.mjs';
const REQUIRED = ['adapter','m0','m2','m3','rust (ubuntu-latest)','rust (windows-latest)'];
export async function expectedJobs(sourceRoot, api, source) {
  return loadPublicationWorkflowInventory({repository:REPO,source,root:sourceRoot,
    api:path=>api.request(path)});
}
export async function observeMain(api, source, { tagged = false } = {}) {
  const branch=await api.request('/repos/zryna/zryna/branches/main');
  requireValue(branch.name==='main' && branch.protected===true && branch.commit?.sha===source.commit,
    'compiler release current protected main');
  const original=await api.request('/repos/zryna/zryna/releases/tags/v0.2.3');
  requireValue(original.id===388328382 && original.tag_name==='v0.2.3' && original.immutable===true &&
    original.draft===false, 'compiler original immutable published compiler observation');
  const commit=await api.request(`/repos/zryna/zryna/git/commits/${source.commit}`);
  requireValue(commit.sha===source.commit && commit.tree?.sha===source.tree,'compiler release API source tree');
  const checks=branch.protection?.required_status_checks;
  requireValue(checks && bytes([...checks.contexts].sort()).equals(bytes(REQUIRED)) &&
    bytes(checks.checks.map(check=>check.context).sort()).equals(bytes(REQUIRED)), 'compiler release protected contexts');
  if (tagged) {
    const ref=await api.request(`/repos/zryna/zryna/git/ref/tags/${TAG}`);
    requireValue(ref.ref===`refs/tags/${TAG}` && ref.object?.type==='tag' && ref.object.sha===source.tagObject,
      'compiler release remote annotated tag');
    const rule=await api.request('/repos/zryna/zryna/rulesets/22824582');
    requireValue(rule.id===22824582 && rule.name==='immutable-release-tags' && rule.target==='tag' &&
      rule.enforcement==='active' && Array.isArray(rule.conditions?.ref_name?.include) &&
      rule.conditions.ref_name.include.includes('refs/tags/v*') && rule.conditions.ref_name.exclude.length===0 &&
      ['creation','update','deletion'].every(type=>rule.rules.some(entry=>entry.type===type)),
    'compiler release active immutable v tag ruleset');
  }
  return {commit:source.commit,tree:source.tree,requiredContexts:REQUIRED};
}
export async function observeCi(api, source, runId, attempt, jobs) {
  id(runId);id(attempt);
  const run=await api.request(`/repos/zryna/zryna/actions/runs/${runId}/attempts/${attempt}`);
  requireValue(run.id===runId && run.run_attempt===attempt && run.repository?.full_name===REPO &&
    ['.github/workflows/ci.yml','.github/workflows/ci.yml@main','.github/workflows/ci.yml@refs/heads/main'].includes(run.path) &&
    run.event==='workflow_dispatch' && run.head_branch==='main' && run.head_sha===source.commit &&
    run.status==='completed' && run.conclusion==='success','compiler release full same-source CI attempt');
  const actual=[];let total;
  for (let page=1;page<=3;page++) {
    const response=await api.request(`/repos/zryna/zryna/actions/runs/${runId}/attempts/${attempt}/jobs?per_page=100&page=${page}`);
    requireValue(Number.isSafeInteger(response.total_count) && response.total_count<=200 &&
      Array.isArray(response.jobs) && response.jobs.length<=100 && (total===undefined || total===response.total_count),
    'compiler release complete bounded CI jobs');total=response.total_count;actual.push(...response.jobs);
    if (actual.length>=total) break;
    requireValue(response.jobs.length===100,'compiler release truncated CI jobs');
  }
  verifyCompletePublicationJobs({jobs:actual,total_count:total},jobs,
    {runId,attempt,commit:source.commit});
  const seen=new Set();
  for (const job of actual) {
    const check=id(job.check_run_url?.split('/').at(-1));
    requireValue(!seen.has(job.id) && job.run_id===runId && job.head_sha===source.commit &&
      job.status==='completed' && job.conclusion==='success' && OID(job.head_sha), 'compiler release positive CI job');
    seen.add(id(job.id));
    const observed=await api.request(`/repos/zryna/zryna/check-runs/${check}`);
    requireValue(observed.id===check && observed.head_sha===source.commit && observed.status==='completed' &&
      observed.conclusion==='success' && observed.app?.slug==='github-actions', 'compiler release actual CI check');
  }
  return {workflowPath:'.github/workflows/ci.yml',runId,runAttempt:attempt,headSha:source.commit,
    headTree:source.tree,conclusion:'success',jobsSha256:sha256(bytes(actual))};
}

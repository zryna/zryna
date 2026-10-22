import { REF, REPO, WORKFLOW, OID, SHA, command, id, requireValue, sha256 } from './common.mjs';
const fields = ['M6-Full-CI-Run','M6-Full-CI-Attempt','M6-Qualification-Asset',
  'M6-Qualification-SHA256','M6-Qualification-Review-SHA256'];
export function parseTag(data) {
  requireValue(Buffer.isBuffer(data) && data.length > 0 && data.length <= 65536, 'compiler release tag size');
  const text = new TextDecoder('utf-8',{fatal:true}).decode(data);
  const split = text.indexOf('\n\n'); requireValue(split > 0, 'compiler release tag headers');
  const headers = text.slice(0,split).split('\n');
  requireValue(headers.length === 4 && /^object [a-f0-9]{40}$/.test(headers[0]) && headers[1] === 'type commit' &&
    headers[2] === `tag ${REF.slice(10)}` && /^tagger .+ [0-9]+ [+-][0-9]{4}$/.test(headers[3]),
  'compiler release direct annotated tag');
  const message = text.slice(split+2).split(/-----BEGIN (?:PGP|SSH) SIGNATURE-----/)[0].trimEnd();
  const lines = message.split('\n'); const trailers = lines.slice(-5);
  requireValue(lines.length >= 6 && lines.at(-6) === '' &&
    lines.filter(line => line.startsWith('M6-')).length === 5, 'compiler release closed trailers');
  const selected = {};
  for (const line of trailers) {
    const match = /^(M6-[A-Za-z0-9-]+):([^\s]+)$/.exec(line);
    requireValue(match && fields.includes(match[1]) && !Object.hasOwn(selected,match[1]), 'compiler release trailer identity');
    selected[match[1]] = match[2];
  }
  requireValue(fields.every(key => Object.hasOwn(selected,key)) && SHA(selected[fields[3]]) && SHA(selected[fields[4]]),
    'compiler release trailer selection');
  return { commit:headers[0].slice(7), selection:{ ciRun:id(selected[fields[0]]),
    ciAttempt:id(selected[fields[1]]), qualificationAsset:id(selected[fields[2]]),
    qualificationSha256:selected[fields[3]], reviewSha256:selected[fields[4]] } };
}
export function captureSource({ cwd, environment = process.env, spawn } = {}) {
  const run = args => command('git',args,{cwd,spawn,maximum:1024*1024}).stdout;
  const line = args => { const b=run(args).toString('utf8');
    requireValue(/^[^\r\n]+\n$/.test(b),'compiler release Git line'); return b.slice(0,-1); };
  requireValue(environment.GITHUB_REPOSITORY === REPO && OID(environment.GITHUB_SHA) &&
    environment.GITHUB_WORKFLOW_SHA === environment.GITHUB_SHA &&
    environment.GITHUB_WORKFLOW_REF === `${REPO}/${WORKFLOW}@${environment.GITHUB_REF}`,
  'compiler release trusted workflow source');
  const publishing = environment.GITHUB_EVENT_NAME === 'push';
  requireValue(publishing ? environment.GITHUB_REF === REF && environment.GITHUB_REF_TYPE === 'tag' &&
    environment.GITHUB_REF_PROTECTED === 'true' : environment.GITHUB_EVENT_NAME === 'workflow_dispatch' &&
    environment.GITHUB_REF === 'refs/heads/main', 'compiler release workflow event');
  requireValue(run(['status','--porcelain=v1','--untracked-files=all','--ignored=matching']).length === 0,
    'compiler release clean source');
  const commit=line(['rev-parse','HEAD']), tree=line(['rev-parse','HEAD^{tree}']);
  const epoch=line(['show','-s','--format=%ct','HEAD']);
  requireValue(commit === environment.GITHUB_SHA && OID(tree) && /^(0|[1-9][0-9]*)$/.test(epoch) &&
    Number(epoch)<=0xffffffff, 'compiler release source identity');
  const source={repository:`https://github.com/${REPO}`,commit,tree,sourceDateEpoch:Number(epoch)};
  const workflow=run(['show',`${commit}:${WORKFLOW}`]);
  const result={source,workflow:{path:WORKFLOW,bytes:workflow.length,sha256:sha256(workflow)},publishing};
  if (publishing) {
    const tagObject=line(['show-ref','--verify','--hash',REF]);
    requireValue(OID(tagObject) && line(['cat-file','-t',tagObject])==='tag','compiler release annotated reference');
    const raw=run(['cat-file','tag',tagObject]), tag=parseTag(raw);
    requireValue(tag.commit === commit && tagObject!==commit &&
      line(['show-ref','--verify','--hash',REF])===tagObject, 'compiler release stable direct tag');
    result.source={...source,ref:REF,tagObject,tagType:'annotated'};
    result.selection=tag.selection;result.rawTag=raw;
  }
  return result;
}

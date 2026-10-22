import path from 'node:path';
import os from 'node:os';
import { realpathSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { readStableFile } from './registry-files.mjs';
import { oid, reject, text } from './evidence-input.mjs';
import { actualFile, observedTools, observedMaterials } from './observe-materials.mjs';
export { actualFile } from './observe-materials.mjs';

const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export function readObservation(executable, args, cwd = sourceRoot) {
  const result = spawnSync(executable, args, { cwd, shell: false, windowsHide: true,
    encoding: 'utf8', timeout: 30_000, maxBuffer: 1024 * 1024,
    env: { ...process.env, CARGO_TERM_COLOR: 'never' } });
  if (result.error || result.signal || result.status !== 0) reject('read-only observation failed');
  return result.stdout.trim();
}
export function observedSource(expectedCommit, expectedTree, ref) {
  oid(expectedCommit); oid(expectedTree); text(ref, 'source ref');
  const commit = readObservation('git', ['rev-parse', 'HEAD']);
  const tree = readObservation('git', ['rev-parse', 'HEAD^{tree}']);
  if (commit !== expectedCommit || tree !== expectedTree ||
      readObservation('git', ['status', '--porcelain=v1', '--untracked-files=all'])) reject('exact clean candidate required');
  return { commit, tree, ref };
}
function hostObservation() {
  if (process.arch !== 'x64' || !['linux', 'win32'].includes(process.platform)) reject('unsupported actual host');
  let distribution, version;
  if (process.platform === 'linux') {
    const release = readStableFile(realpathSync('/etc/os-release'), 64 * 1024).toString('utf8');
    distribution = /^ID=(?:"([^"\n]+)"|([^\n]+))$/m.exec(release)?.slice(1).find(Boolean);
    version = /^VERSION_ID=(?:"([^"\n]+)"|([^\n]+))$/m.exec(release)?.slice(1).find(Boolean);
  } else {
    const caption = readObservation('powershell.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command',
      '(Get-CimInstance Win32_OperatingSystem).Caption']);
    distribution = caption.includes('Windows Server') ? 'windows-server' : 'windows-desktop';
    version = /\b(2022|2025|2019)\b/.exec(caption)?.[1] ?? caption;
  }
  return { os: process.platform === 'win32' ? 'windows' : 'linux', architecture: 'x86_64',
    distribution, version, kernel: os.release(), runnerImage: process.env.ImageOS ?? 'local-unhosted',
    runnerImageVersion: process.env.ImageVersion ?? 'local-unhosted' };
}
function workflowObservation(source, jobName) {
  if (!process.env.GITHUB_ACTIONS) return null;
  const workflowPath = process.env.M4_WORKFLOW_PATH;
  if (!/^\.github\/workflows\/[a-z0-9-]+\.yml$/.test(workflowPath ?? '')) reject('explicit actual workflow path required');
  const headCommit = process.env.GITHUB_SHA;
  oid(headCommit);
  return { repository: process.env.GITHUB_REPOSITORY, runId: Number(process.env.GITHUB_RUN_ID),
    attempt: Number(process.env.GITHUB_RUN_ATTEMPT), jobName, jobKey: process.env.GITHUB_JOB,
    workflowPath, workflowSha256: actualFile(path.join(sourceRoot, workflowPath)).sha256,
    event: process.env.GITHUB_EVENT_NAME, headCommit,
    executionCommit: process.env.GITHUB_WORKFLOW_SHA ?? headCommit, checkoutCommit: source.commit };
}
export function observeContext({ expectedCommit, expectedTree, sourceRef, jobName }) {
  const source = observedSource(expectedCommit, expectedTree, sourceRef);
  return { format: 'zryna.m4-observation-context.v1', source, host: hostObservation(),
    workflow: workflowObservation(source, jobName), tools: observedTools(), materials: observedMaterials() };
}
function options(argv) {
  const allowed = ['--lane', '--output', '--job-name', '--source-ref', '--expected-commit', '--expected-tree'];
  if (argv.length !== allowed.length * 2) reject('complete observation arguments required');
  const map = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!allowed.includes(argv[index]) || map.has(argv[index])) reject('unknown or repeated observation argument');
    map.set(argv[index], argv[index + 1]);
  }
  if (!['core', 'browser', 'installed'].includes(map.get('--lane')) || !path.isAbsolute(map.get('--output'))) reject('lane or output path invalid');
  return map;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = options(process.argv.slice(2));
    const context = observeContext({ expectedCommit: args.get('--expected-commit'), expectedTree: args.get('--expected-tree'),
      sourceRef: args.get('--source-ref'), jobName: args.get('--job-name') });
    writeFileSync(args.get('--output'), `${JSON.stringify(context, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}

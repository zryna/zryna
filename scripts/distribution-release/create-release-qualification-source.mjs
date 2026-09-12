import { spawnSync } from 'node:child_process';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonicalBounded, sha256 } from './canonical.mjs';
import { validateReleaseQualificationSource } from './validate-release-qualification-source.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPOSITORY = 'zryna/zryna';
const REPOSITORY_URL = 'https://github.com/zryna/zryna';
const REF = 'refs/heads/main';
const WORKFLOW = '.github/workflows/release-qualification.yml';
const WORKFLOW_REF = `${REPOSITORY}/${WORKFLOW}@${REF}`;
const MAX_OUTPUT = 256 * 1024;
const OBJECT_ID = /^[0-9a-f]{40}$/;
const DECIMAL = /^(0|[1-9][0-9]*)$/;

function reject(message) {
  throw new Error(`R406-QUALIFICATION-SOURCE-PRODUCER: ${message}`);
}

function run(spawn, args, cwd, encoding = 'utf8') {
  const result = spawn('git', args, {
    cwd, encoding, maxBuffer: MAX_OUTPUT + 1, shell: false, windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    reject(`git ${args.join(' ')} failed with status ${result.status ?? 'unknown'}`);
  }
  const output = result.stdout;
  const size = Buffer.isBuffer(output) ? output.length : Buffer.byteLength(output, 'utf8');
  if (size > MAX_OUTPUT) reject(`git ${args[0]} output exceeds ${MAX_OUTPUT} bytes`);
  return output;
}

function oneLine(spawn, args, cwd) {
  const output = run(spawn, args, cwd);
  if (!output.endsWith('\n') || output.slice(0, -1).includes('\n') || output.includes('\r')) {
    reject(`git ${args[0]} did not return one canonical line`);
  }
  return output.slice(0, -1);
}

function samePath(left, right) {
  const normalize = (value) => process.platform === 'win32'
    ? resolve(value).toLowerCase() : resolve(value);
  return normalize(left) === normalize(right);
}

function workflowEntry(bytes) {
  const suffix = Buffer.from(`\t${WORKFLOW}\0`);
  if (!bytes.subarray(-suffix.length).equals(suffix) || bytes.indexOf(0) !== bytes.length - 1) {
    reject('qualification workflow tree entry is missing or ambiguous');
  }
  const match = /^(100644) (blob) ([0-9a-f]{40})$/
    .exec(bytes.subarray(0, bytes.length - suffix.length).toString('ascii'));
  if (!match) reject('qualification workflow must be one ordinary non-executable Git blob');
  return match[3];
}

export function createReleaseQualificationSource({
  environment = process.env,
  cwd = environment.ZRYNA_SOURCE_ROOT,
  spawn = spawnSync,
} = {}) {
  if (environment.GITHUB_EVENT_NAME !== 'workflow_dispatch'
    || environment.GITHUB_REPOSITORY !== REPOSITORY
    || environment.GITHUB_REF !== REF
    || environment.GITHUB_REF_TYPE !== 'branch'
    || environment.GITHUB_REF_NAME !== 'main'
    || environment.GITHUB_REF_PROTECTED !== 'true'
    || environment.GITHUB_WORKFLOW_REF !== WORKFLOW_REF
    || !OBJECT_ID.test(environment.GITHUB_SHA ?? '')
    || environment.GITHUB_WORKFLOW_SHA !== environment.GITHUB_SHA) {
    reject('exact protected main qualification workflow context is required');
  }
  if (!isAbsolute(cwd ?? '')) reject('ZRYNA_SOURCE_ROOT must name an absolute isolated checkout');
  const topLevel = oneLine(spawn, ['rev-parse', '--show-toplevel'], cwd);
  if (!samePath(topLevel, cwd)) reject('ZRYNA_SOURCE_ROOT must be the checkout root');
  if (run(spawn, [
    'status', '--porcelain=v1', '--untracked-files=all', '--ignored=matching',
  ], cwd).length !== 0) {
    reject('checkout contains tracked, untracked, or ignored files outside the source tree');
  }
  const commit = oneLine(spawn, ['rev-parse', 'HEAD'], cwd);
  if (commit !== environment.GITHUB_SHA
    || oneLine(spawn, ['cat-file', '-t', commit], cwd) !== 'commit') {
    reject('qualification checkout and workflow commits differ');
  }
  const tree = oneLine(spawn, ['show', '-s', '--format=%T', commit], cwd);
  if (!OBJECT_ID.test(tree)) reject('qualification tree identity is malformed');
  const epochText = oneLine(spawn, ['show', '-s', '--format=%ct', commit], cwd);
  if (!DECIMAL.test(epochText) || Number(epochText) > 4294967295) {
    reject('qualification source epoch is outside its range');
  }
  const entry = run(spawn, ['ls-tree', '-z', '--full-tree', commit, '--', WORKFLOW], cwd, null);
  const workflowObject = workflowEntry(entry);
  const workflowSize = oneLine(spawn, ['cat-file', '-s', workflowObject], cwd);
  if (!DECIMAL.test(workflowSize) || Number(workflowSize) < 1
    || Number(workflowSize) > MAX_OUTPUT) {
    reject('qualification workflow blob exceeds the receipt budget');
  }
  const workflowBytes = run(spawn, ['cat-file', 'blob', workflowObject], cwd, null);
  if (workflowBytes.length !== Number(workflowSize)) {
    reject('qualification workflow size changed while reading');
  }
  if (oneLine(spawn, ['rev-parse', 'HEAD'], cwd) !== commit) {
    reject('qualification checkout changed while producing the receipt');
  }
  return validateReleaseQualificationSource({
    format: 'zryna.release-qualification-source.v1',
    status: 'provisional-candidate',
    productionAdmission: 'forbidden',
    versionCandidate: '0.2.0',
    source: {
      repository: REPOSITORY_URL,
      ref: REF,
      commit,
      tree,
      sourceDateEpoch: Number(epochText),
    },
    workflow: { path: WORKFLOW, size: workflowBytes.length, sha256: sha256(workflowBytes) },
  });
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    process.stdout.write(`${canonicalBounded(createReleaseQualificationSource())}\n`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

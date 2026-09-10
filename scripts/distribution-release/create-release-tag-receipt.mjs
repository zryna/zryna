import { spawnSync } from 'node:child_process';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TextDecoder } from 'node:util';
import { canonicalBounded, sha256 } from './canonical.mjs';
import { validateReleaseTagReceipt } from './validate-release-tag-receipt.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPOSITORY = 'zryna/zryna';
const REPOSITORY_URL = 'https://github.com/zryna/zryna';
const TAG = 'v0.2.0';
const REF = `refs/tags/${TAG}`;
const WORKFLOW = '.github/workflows/release.yml';
const WORKFLOW_REF = `${REPOSITORY}/${WORKFLOW}@${REF}`;
const MAX_OUTPUT = 256 * 1024;
const OBJECT_ID = /^[0-9a-f]{40}$/;
const DECIMAL = /^(0|[1-9][0-9]*)$/;

function reject(message) {
  throw new Error(`R406-TAG-PRODUCER: ${message}`);
}

function run(spawn, args, cwd, encoding = 'utf8') {
  const result = spawn('git', args, {
    cwd, encoding, maxBuffer: MAX_OUTPUT + 1, shell: false, windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    reject(`git ${args.join(' ')} failed with status ${result.status ?? 'unknown'}`);
  }
  const output = result.stdout;
  if ((Buffer.isBuffer(output) ? output.length : Buffer.byteLength(output, 'utf8')) > MAX_OUTPUT) {
    reject(`git ${args[0]} output exceeds ${MAX_OUTPUT} bytes`);
  }
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

function parseDirectTag(bytes, tagObject) {
  const separator = bytes.indexOf(Buffer.from('\n\n'));
  if (separator < 1 || separator > 64 * 1024) reject('annotated tag headers are malformed');
  let header;
  try {
    header = new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(0, separator));
  } catch {
    reject('annotated tag headers are not UTF-8');
  }
  const fields = new Map();
  for (const line of header.split('\n')) {
    const space = line.indexOf(' ');
    if (space < 1 || fields.has(line.slice(0, space))) reject('annotated tag headers are ambiguous');
    fields.set(line.slice(0, space), line.slice(space + 1));
  }
  const commit = fields.get('object');
  if (!OBJECT_ID.test(commit ?? '') || fields.get('type') !== 'commit'
    || fields.get('tag') !== TAG || !fields.has('tagger')) {
    reject('annotated tag does not directly identify the expected release commit and name');
  }
  if (commit === tagObject) reject('annotated tag object and commit must differ');
  return commit;
}

function workflowEntry(bytes) {
  const suffix = Buffer.from(`\t${WORKFLOW}\0`);
  if (!bytes.subarray(-suffix.length).equals(suffix) || bytes.indexOf(0) !== bytes.length - 1) {
    reject('tagged workflow tree entry is missing or ambiguous');
  }
  const prefix = bytes.subarray(0, bytes.length - suffix.length).toString('ascii');
  const match = /^(100644) (blob) ([0-9a-f]{40})$/.exec(prefix);
  if (!match) reject('tagged workflow must be one ordinary non-executable Git blob');
  return match[3];
}

export function createReleaseTagReceipt({
  environment = process.env,
  cwd = environment.ZRYNA_SOURCE_ROOT,
  spawn = spawnSync,
} = {}) {
  if (environment.GITHUB_EVENT_NAME !== 'push'
    || environment.GITHUB_REPOSITORY !== REPOSITORY
    || environment.GITHUB_REF !== REF
    || environment.GITHUB_REF_TYPE !== 'tag'
    || environment.GITHUB_REF_NAME !== TAG
    || environment.GITHUB_REF_PROTECTED !== 'true'
    || environment.GITHUB_WORKFLOW_REF !== WORKFLOW_REF
    || !OBJECT_ID.test(environment.GITHUB_SHA ?? '')
    || environment.GITHUB_WORKFLOW_SHA !== environment.GITHUB_SHA) {
    reject('exact protected tag workflow context is required');
  }
  if (!isAbsolute(cwd ?? '')) reject('ZRYNA_SOURCE_ROOT must name an absolute isolated checkout');
  const topLevel = oneLine(spawn, ['rev-parse', '--show-toplevel'], cwd);
  if (!samePath(topLevel, cwd)) reject('ZRYNA_SOURCE_ROOT must be the checkout root');
  if (run(spawn, [
    'status', '--porcelain=v1', '--untracked-files=all', '--ignored=matching',
  ], cwd).length !== 0) {
    reject('checkout contains tracked, untracked, or ignored files outside the source tree');
  }
  const tagObject = oneLine(spawn, ['show-ref', '--verify', '--hash', REF], cwd);
  if (!OBJECT_ID.test(tagObject)
    || oneLine(spawn, ['cat-file', '-t', tagObject], cwd) !== 'tag') {
    reject('release reference must identify one annotated tag object');
  }
  const tagSize = oneLine(spawn, ['cat-file', '-s', tagObject], cwd);
  if (!DECIMAL.test(tagSize) || Number(tagSize) < 1 || Number(tagSize) > MAX_OUTPUT) {
    reject('annotated tag object exceeds the receipt budget');
  }
  const tagBytes = run(spawn, ['cat-file', 'tag', tagObject], cwd, null);
  if (tagBytes.length !== Number(tagSize)) reject('annotated tag object size changed while reading');
  const commit = parseDirectTag(tagBytes, tagObject);
  if (oneLine(spawn, ['cat-file', '-t', commit], cwd) !== 'commit'
    || oneLine(spawn, ['rev-parse', `${REF}^{commit}`], cwd) !== commit
    || oneLine(spawn, ['rev-parse', 'HEAD'], cwd) !== commit
    || commit !== environment.GITHUB_SHA) {
    reject('tag, checked-out HEAD, and workflow source commits differ');
  }
  const tree = oneLine(spawn, ['show', '-s', '--format=%T', commit], cwd);
  if (!OBJECT_ID.test(tree)) reject('release commit tree identity is malformed');
  const epochText = oneLine(spawn, ['show', '-s', '--format=%ct', commit], cwd);
  if (!DECIMAL.test(epochText) || Number(epochText) > 4294967295) {
    reject('release commit timestamp is outside the source epoch range');
  }
  const entry = run(spawn, ['ls-tree', '-z', '--full-tree', commit, '--', WORKFLOW], cwd, null);
  const workflowObject = workflowEntry(entry);
  const workflowSize = oneLine(spawn, ['cat-file', '-s', workflowObject], cwd);
  if (!DECIMAL.test(workflowSize) || Number(workflowSize) < 1 || Number(workflowSize) > MAX_OUTPUT) {
    reject('tagged workflow blob exceeds the receipt budget');
  }
  const workflowBytes = run(spawn, ['cat-file', 'blob', workflowObject], cwd, null);
  if (workflowBytes.length !== Number(workflowSize)) reject('tagged workflow size changed while reading');
  return validateReleaseTagReceipt({
    format: 'zryna.release-tag-receipt.v1',
    version: TAG.slice(1),
    source: {
      repository: REPOSITORY_URL,
      ref: REF,
      tagType: 'annotated',
      tagObject,
      commit,
      tree,
      sourceDateEpoch: Number(epochText),
    },
    workflow: { path: WORKFLOW, size: workflowBytes.length, sha256: sha256(workflowBytes) },
  });
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    process.stdout.write(`${canonicalBounded(createReleaseTagReceipt())}\n`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

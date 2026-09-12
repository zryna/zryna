import { spawnSync } from 'node:child_process';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { sha256 } from './canonical.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REF = 'refs/tags/v0.2.0';
const MAX_OUTPUT = 256 * 1024;
const OBJECT_ID = /^[0-9a-f]{40}$/;

// This value stays unset until #422's recipe bytes and production integration receive exact review.
export const ACCEPTED_RECIPE_SHA256 = null;

export const REQUIRED_RELEASE_PATHS = Object.freeze([
  '.github/workflows/release.yml',
  'scripts/distribution/release-recipe-v1.json',
  'scripts/distribution-release/compare-release-builds.mjs',
  'scripts/distribution-release/prepare-release-evidence.mjs',
  'scripts/distribution-release/publish-draft-release.mjs',
  'scripts/distribution-release/run-protected-build.mjs',
  'scripts/distribution-release/verify-signed-release.mjs',
]);

function reject(message) {
  throw new Error(`R406-RELEASE-NOT-READY: ${message}`);
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

function entry(bytes, path) {
  const suffix = Buffer.from(`\t${path}\0`);
  if (!bytes.subarray(-suffix.length).equals(suffix) || bytes.indexOf(0) !== bytes.length - 1) {
    return null;
  }
  const match = /^(100644) (blob) ([0-9a-f]{40})$/
    .exec(bytes.subarray(0, bytes.length - suffix.length).toString('ascii'));
  return match?.[3] ?? null;
}

export function checkReleaseReadiness({
  environment = process.env,
  cwd = environment.ZRYNA_SOURCE_ROOT,
  spawn = spawnSync,
} = {}) {
  if (environment.GITHUB_EVENT_NAME !== 'push'
    || environment.GITHUB_REPOSITORY !== 'zryna/zryna'
    || environment.GITHUB_REF !== REF
    || environment.GITHUB_REF_PROTECTED !== 'true'
    || !OBJECT_ID.test(environment.GITHUB_SHA ?? '')
    || environment.GITHUB_WORKFLOW_SHA !== environment.GITHUB_SHA) {
    reject('exact protected release workflow context is required');
  }
  if (!isAbsolute(cwd ?? '')) reject('ZRYNA_SOURCE_ROOT must be an absolute isolated checkout');
  const missing = [];
  const objects = new Map();
  for (const path of REQUIRED_RELEASE_PATHS) {
    const bytes = run(spawn, [
      'ls-tree', '-z', '--full-tree', environment.GITHUB_SHA, '--', path,
    ], cwd, null);
    const object = entry(bytes, path);
    if (object === null) missing.push(path);
    else objects.set(path, object);
  }
  if (missing.length > 0) reject(`missing reviewed prerequisites: ${missing.join(', ')}`);
  if (ACCEPTED_RECIPE_SHA256 === null) {
    reject('no distribution recipe digest has received exact production review');
  }
  const recipeObject = objects.get('scripts/distribution/release-recipe-v1.json');
  const recipe = run(spawn, ['cat-file', 'blob', recipeObject], cwd, null);
  if (sha256(recipe) !== ACCEPTED_RECIPE_SHA256) {
    reject('distribution recipe bytes differ from the accepted production digest');
  }
  return { sourceCommit: environment.GITHUB_SHA, recipeSha256: ACCEPTED_RECIPE_SHA256 };
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    checkReleaseReadiness();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}

import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { INITIAL_COMMIT, POLICY_PATH, fail, physicalLines, reviewChanges, validatePolicy } from './structure/policy.mjs';
import { blobs, commit, git, navigation, readSafe, renames, source, tree, workingPaths } from './structure/repository.mjs';

export function checkRepository({ root, base = process.env.ZRYNA_STRUCTURE_BASE,
  today = new Date().toISOString().slice(0, 10), bootstrap = INITIAL_COMMIT } = {}) {
  if (!root) fail('repository root is required');
  if (!base && process.env.CI) fail('CI must supply ZRYNA_STRUCTURE_BASE as a full trusted commit SHA');
  const comparison = commit(root, base ?? 'HEAD');
  const head = commit(root, 'HEAD');
  git(root, ['merge-base', '--is-ancestor', comparison, head]);
  const policyText = readSafe(root, POLICY_PATH);
  const policy = validatePolicy(JSON.parse(policyText), today);
  let before = tree(root, comparison);
  const trustedPolicyEntry = before.get(POLICY_PATH);
  let trustedPolicy;
  if (trustedPolicyEntry) {
    if (trustedPolicyEntry.mode !== '100644') fail('trusted policy is not a regular file');
    trustedPolicy = validatePolicy(JSON.parse(blobs(root, [trustedPolicyEntry]).get(trustedPolicyEntry.hash)), '0000-00-00');
  }
  if (policy.anchor !== (trustedPolicy?.anchor ?? bootstrap)) fail('anchor differs from trusted policy/bootstrap commit');
  commit(root, policy.anchor);
  git(root, ['merge-base', '--is-ancestor', policy.anchor, head]);
  let ratchetBase = comparison;
  const common = git(root, ['merge-base', policy.anchor, comparison]).trim();
  if (common !== policy.anchor) {
    if (trustedPolicy || common !== comparison) fail('trusted base and initial anchor are incomparable');
    ratchetBase = policy.anchor;
    before = tree(root, ratchetBase);
  }
  const original = tree(root, policy.anchor);
  const historical = blobs(root, [...original.entries(), ...before.entries()]
    .filter(([path, entry]) => source(path) && ['100644', '100755'].includes(entry.mode))
    .map(([, entry]) => entry));
  // Initial production eligibility is frozen with the baseline, not recomputed when a later
  // change removes a test classification. Bootstrap classification requires maintainer review.
  const initialEligibility = new Map(trustedPolicy?.baseline.map(entry => [entry.path, entry.production]));
  const initialExcluded = new Set(policy.classifications.map(entry => entry.path));
  const inventory = [...original.entries()].filter(([path, entry]) => source(path)
    && historical.has(entry.hash) && physicalLines(historical.get(entry.hash)) > 500)
    .map(([path, entry]) => ({ path, lines: physicalLines(historical.get(entry.hash)),
      production: trustedPolicy ? initialEligibility.get(path) : !initialExcluded.has(path) }));
  if (JSON.stringify(policy.baseline) !== JSON.stringify(inventory)) fail('baseline differs from exact anchored source inventory; do not raise or regenerate allowances');
  if (trustedPolicy && JSON.stringify(policy.baseline) !== JSON.stringify(trustedPolicy.baseline)) fail('baseline differs from trusted base policy');
  const messages = [];
  if (!trustedPolicy || policyText !== blobs(root, [trustedPolicyEntry]).get(trustedPolicyEntry.hash)) {
    messages.push('REVIEW policy changed: classifications, baseline and exceptions require explicit maintainer review; this check does not prove approval');
    messages.push(...reviewChanges(trustedPolicy, policy));
  }
  const paths = workingPaths(root);
  const current = new Map();
  for (const path of paths.filter(source)) {
    const text = readSafe(root, path, true);
    if (text !== undefined) current.set(path, physicalLines(text));
  }
  const classifications = new Map(policy.classifications.map(entry => [entry.path, entry]));
  const oldClasses = new Map((trustedPolicy?.classifications ?? policy.classifications).map(entry => [entry.path, entry]));
  const exceptions = new Map(policy.exceptions.map(entry => [entry.path, entry]));
  for (const entry of [...classifications.values(), ...exceptions.values()]) {
    if (!current.has(entry.path)) fail(`stale policy path: ${entry.path}; remove deleted or renamed records explicitly`);
  }
  for (const path of exceptions.keys()) {
    if (classifications.has(path) || current.get(path) <= 500) fail(`stale exception for non-production or ordinary-sized file: ${path}`);
  }
  const baseline = new Map(policy.baseline.filter(entry => entry.production).map(entry => [entry.path, entry.lines]));
  const historicalRenames = renames(root, policy.anchor, ratchetBase);
  const currentRenames = renames(root, ratchetBase);
  const errors = [];
  for (const [path, lines] of current) {
    if (classifications.has(path)) continue;
    const previousPath = currentRenames.get(path) ?? path;
    const originalPath = historicalRenames.get(previousPath) ?? previousPath;
    const previous = before.get(previousPath);
    const previousLines = previous && historical.has(previous.hash)
      ? physicalLines(historical.get(previous.hash)) : undefined;
    const ordinaryCeiling = (
      baseline.has(originalPath) && previousLines > 500 && !oldClasses.has(previousPath)
        ? Math.min(baseline.get(originalPath), previousLines) : 500);
    if (exceptions.has(path) && lines <= ordinaryCeiling) fail(`stale unnecessary exception: ${path}`);
    const ceiling = exceptions.get(path)?.ceiling ?? ordinaryCeiling;
    if (lines > ceiling) errors.push(`ERROR ${path}: ${lines} lines exceeds ${ceiling}; split by cohesive responsibility or obtain an exact reviewed exception`);
    else if (lines >= 350 && lines <= 500) messages.push(`WARN ${path}: ${lines} lines; review cohesion before further growth`);
  }
  navigation(root);
  messages.push(...errors);
  return { ok: errors.length === 0, messages, comparison, files: current.size };
}

const script = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === script) {
  try {
    if (process.argv.length !== 2) fail('no positional options; supply trusted base using ZRYNA_STRUCTURE_BASE');
    const result = checkRepository({ root: resolve(dirname(script), '..') });
    for (const message of result.messages) console.log(message);
    console.log(`Structure checked ${result.files} source files against ${result.comparison}; architecture and compiler gates remain required.`);
    if (!result.ok) process.exitCode = 1;
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}

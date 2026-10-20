// One-shot read-only diagnostic. This never admits, copies, builds or runs an executable.
import {
  closeSync, constants, fstatSync, lstatSync, openSync, readSync,
  readdirSync, realpathSync, writeFileSync,
} from 'node:fs';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const MAX_BINARY = 256 * 1024 * 1024, MAX_ENTRIES = 8192, MAX_ALIASES = 128;
const KEYS = ['dev', 'ino', 'mode', 'nlink', 'size', 'mtimeNs', 'ctimeNs'];
const snapshot = st => Object.fromEntries(KEYS.map(key => [key, String(st[key])]));
const identity = st => ({ ...snapshot(st), regular: st.isFile(), directory: st.isDirectory(),
  symbolicLink: st.isSymbolicLink() });
const equal = (left, right) => KEYS.every(key => String(left[key]) === String(right[key]));
const sameFile = (left, right) => left.dev === right.dev && left.ino === right.ino;
const stat = path => lstatSync(path, { bigint: true, throwIfNoEntry: false });
function reject(message) { throw new Error('M414-ALIAS-OBSERVATION: ' + message); }
function hashBounded(path, expected) {
  if (!expected.isFile() || expected.isSymbolicLink()) return { hashed: false, reason: 'not-direct-regular-file' };
  if (expected.size < 1 || expected.size > MAX_BINARY) return { hashed: false, reason: 'binary-size-budget' };
  const fd = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const opened = fstatSync(fd, { bigint: true }), buffer = Buffer.alloc(1024 * 1024);
    const digest = createHash('sha256'); let count = 0;
    while (count <= Number(expected.size)) {
      const limit = Math.min(buffer.length, Number(expected.size) + 1 - count);
      if (!limit) break;
      const size = readSync(fd, buffer, 0, limit, null);
      if (!size) break;
      digest.update(buffer.subarray(0, size)); count += size;
    }
    const after = fstatSync(fd, { bigint: true }), named = stat(path);
    return { hashed: true, bytes: count, sha256: digest.digest('hex'), opened: snapshot(opened),
      after: snapshot(after), named: named ? identity(named) : null,
      stable: count === Number(expected.size) && equal(expected, opened) && equal(expected, after)
        && named !== undefined && equal(expected, named), noFollowFlagAvailable: Boolean(constants.O_NOFOLLOW) };
  } finally { closeSync(fd); }
}
function observe(source, aliasesPath, kind) {
  const before = stat(source), record = { sourcePath: source, aliasesPath, kind };
  if (!before) return { ...record, status: 'source-absent-no-executable-observation' };
  record.source = identity(before); record.canonicalSourcePath = realpathSync.native(source);
  record.sourceHash = hashBounded(source, before);
  const directory = stat(aliasesPath);
  if (!directory?.isDirectory() || directory.isSymbolicLink()) {
    return { ...record, status: 'unsafe-or-missing-alias-directory', aliasDirectory: directory ? identity(directory) : null };
  }
  record.aliasDirectory = identity(directory);
  const names = readdirSync(aliasesPath).sort();
  if (names.length > MAX_ENTRIES) reject('alias directory entry budget exceeded');
  const entries = [], matches = [];
  for (const name of names) {
    const path = join(aliasesPath, name), entry = stat(path);
    if (!entry) { entries.push({ name, path, disappeared: true }); continue; }
    const matched = sameFile(before, entry);
    const item = { name, path, ...identity(entry), sameSourceIdentity: matched,
      currentCargoNamePolicy: /^zryna-[a-f0-9]{16}(?:\.exe)?$/.test(name) };
    entries.push(item);
    if (!matched || path === source) continue;
    if (matches.length >= MAX_ALIASES) reject('same-file alias observation budget exceeded');
    matches.push({ ...item, hash: hashBounded(path, entry) });
  }
  const sourceAfter = stat(source), directoryAfter = stat(aliasesPath);
  record.entries = entries; record.sameIdentityAliases = matches;
  record.sourceAfter = sourceAfter ? identity(sourceAfter) : null;
  record.aliasDirectoryAfter = directoryAfter ? identity(directoryAfter) : null;
  record.sourceStable = sourceAfter !== undefined && equal(before, sourceAfter);
  record.aliasDirectoryStable = directoryAfter !== undefined && equal(directory, directoryAfter);
  record.explainedLinkCount = 1 + matches.length;
  record.observedNlink = Number(before.nlink);
  record.nlinkClosureObserved = record.explainedLinkCount === record.observedNlink;
  record.sameIdentityHashesMatch = record.sourceHash.hashed && record.sourceHash.stable && matches.every(item => item.hash.hashed
    && item.hash.stable && item.hash.sha256 === record.sourceHash.sha256);
  return { ...record, status: 'observed-only-no-installation-admission' };
}

const { values } = parseArgs({ options: { source: { type: 'string' },
  qualification: { type: 'string' }, output: { type: 'string' } }, strict: true });
if (!values.source || !values.qualification || !values.output) reject('source/qualification/output required');
const sourceRoot = realpathSync.native(resolve(values.source));
const qualification = resolve(values.qualification), output = resolve(values.output);
if (![qualification, output].every(isAbsolute)) reject('absolute paths required');
if (realpathSync.native(dirname(output)) !== dirname(output) || stat(output)) reject('fresh canonical output required');
const git = args => execFileSync('git', args, { cwd: sourceRoot, encoding: 'utf8', maxBuffer: 1024 * 1024 }).trim();
const operator = { commit: git(['rev-parse', 'HEAD']), tree: git(['rev-parse', 'HEAD^{tree}']),
  branch: git(['rev-parse', '--abbrev-ref', 'HEAD']), expectedActionCommit: process.env.GITHUB_SHA ?? null };
if (operator.commit !== operator.expectedActionCommit) reject('actual operator checkout differs from Actions SHA');
const result = { format: 'zryna.native-producer-alias-observation.v1', operator,
  baselineProductionSource: '3cbeac85d23246ed2fc5971536dfdf544d93dee2',
  status: 'diagnostic-observation-only', sourceAcceptanceClaimed: false, productionAdmission: 'forbidden',
  originalProducerFailureRemainsFailure: true, commandsRebuiltByObserver: false, observations: [] };
for (const replica of ['replica-a', 'replica-b']) {
  const debug = join(qualification, replica, 'target', 'x86_64-pc-windows-msvc', 'debug');
  try { result.observations.push(observe(join(debug, 'zryna.exe'), join(debug, 'deps'), 'cargo-cli')); }
  catch (error) { result.observations.push({ kind: 'cargo-cli', replica, status: 'diagnostic-observation-failed',
    error: String(error.message).slice(0, 2000) }); }
}
const cargoBin = join(realpathSync.native(process.env.CARGO_HOME), 'bin');
try { result.observations.push(observe(join(cargoBin, 'rustup.exe'), cargoBin, 'rustup-shim')); }
catch (error) { result.observations.push({ kind: 'rustup-shim', status: 'diagnostic-observation-failed',
  error: String(error.message).slice(0, 2000) }); }
const body = Buffer.from(JSON.stringify(result, null, 2) + '\n');
if (body.length > 8 * 1024 * 1024) reject('observation JSON budget exceeded');
writeFileSync(output, body, { flag: 'wx', mode: 0o600 });
console.log(JSON.stringify({ status: result.status, operator, output,
  sha256: createHash('sha256').update(body).digest('hex'), bytes: body.length,
  sourceAcceptanceClaimed: false }));

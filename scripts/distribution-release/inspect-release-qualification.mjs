import { spawnSync } from 'node:child_process';
import { lstatSync, readFileSync, realpathSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { canonicalBounded, sha256 } from './canonical.mjs';
import { validateReleaseQualificationInputText } from './validate-release-qualification-input.mjs';
import { validateReleaseQualificationInspection } from './validate-release-qualification-inspection.mjs';

const MAX_TOOL = 536870912;
const MAX_OUTPUT = 1024 * 1024;

function reject(message) {
  throw new Error(`R406-QUALIFICATION-BINARY-INSPECTION: ${message}`);
}

function samePath(left, right) {
  const normalize = (value) => process.platform === 'win32'
    ? resolve(value).toLowerCase() : resolve(value);
  return normalize(left) === normalize(right);
}

function verifyTool(path, record, system) {
  if (!isAbsolute(path ?? '') || record?.name !== 'inspector') {
    reject('inspector binding differs');
  }
  const metadata = system.inspect(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size !== record.size
    || metadata.size < 1 || metadata.size > MAX_TOOL
    || !samePath(system.realPath(path), path)) reject('inspector file identity differs');
  const bytes = system.read(path);
  if (bytes.length !== metadata.size || sha256(bytes) !== record.sha256) {
    reject('inspector executable bytes differ');
  }
}

function occurrences(bytes, value) {
  if (typeof value !== 'string' || value.length < 1 || /[\r\n\0]/.test(value)) {
    reject('inspection root differs');
  }
  let count = 0;
  for (const needle of [Buffer.from(value, 'utf8'), Buffer.from(value, 'utf16le')]) {
    for (let offset = 0; (offset = bytes.indexOf(needle, offset)) !== -1; offset += needle.length) count += 1;
  }
  return count;
}

function dependencies(target, output) {
  const values = target === 'x86_64-pc-windows-msvc'
    ? output.split(/\r?\n/).map((line) => line.trim())
      .filter((line) => /^[A-Za-z0-9._+-]+\.dll$/i.test(line))
    : [...output.matchAll(/Shared library: \[([^\]\r\n]{1,160})\]/g)].map((match) => match[1]);
  const unique = [...new Set(values.map((value) => value.toLowerCase()))];
  if (unique.length < 1 || unique.length !== values.length) {
    reject('dynamic dependency output is missing or ambiguous');
  }
  return values.sort((left, right) => left.toLowerCase().localeCompare(right.toLowerCase()));
}

export function inspectReleaseQualification({
  binding, cli, sourceRoot, workRoot, observedTools, spawn = spawnSync,
  system = { inspect: lstatSync, realPath: realpathSync.native, read: readFileSync },
}) {
  if (!Buffer.isBuffer(binding) || !Buffer.isBuffer(cli) || cli.length < 64
    || !isAbsolute(sourceRoot ?? '') || !isAbsolute(workRoot ?? '')) {
    reject('inspection inputs differ');
  }
  const input = validateReleaseQualificationInputText(binding.toString('utf8'));
  const record = input.nativeTools.find(({ name }) => name === 'inspector');
  const path = observedTools?.paths?.inspector;
  const observed = observedTools?.nativeTools?.find(({ name }) => name === 'inspector');
  if (canonicalBounded(record) !== canonicalBounded(observed)) reject('observed inspector differs');
  verifyTool(path, record, system);
  const windows = input.target.triple === 'x86_64-pc-windows-msvc';
  const args = windows ? ['/DEPENDENTS', observedTools.executable] : ['-d', '--wide', observedTools.executable];
  if (!isAbsolute(observedTools?.executable ?? '')) reject('compiled executable path differs');
  const result = spawn(path, args, {
    cwd: workRoot, env: {}, encoding: 'utf8', maxBuffer: MAX_OUTPUT + 1,
    timeout: 30_000, windowsHide: true, shell: false,
  });
  verifyTool(path, record, system);
  if (result.error || result.status !== 0 || result.signal !== null
    || typeof result.stdout !== 'string' || typeof result.stderr !== 'string'
    || Buffer.byteLength(result.stdout) + Buffer.byteLength(result.stderr) > MAX_OUTPUT
    || result.stderr.trim().length !== 0) reject('binary inspector invocation failed');
  const sourceRootOccurrences = occurrences(cli, sourceRoot);
  const workRootOccurrences = occurrences(cli, workRoot);
  if (sourceRootOccurrences !== 0 || workRootOccurrences !== 0) {
    reject('compiled binary contains a controlled root');
  }
  const inspection = validateReleaseQualificationInspection({
    format: 'zryna.release-qualification-inspection.v1', status: 'qualification-only',
    productionAdmission: 'forbidden', target: input.target.triple,
    sourceCommit: input.source.commit, bindingSha256: sha256(binding),
    binary: { format: windows ? 'PE' : 'ELF', architecture: 'x86_64',
      size: cli.length, sha256: sha256(cli) },
    checks: { identityMarker: 'qualification-binding-sha256', sourceRootOccurrences,
      workRootOccurrences, dynamicDependencies: dependencies(input.target.triple, result.stdout) },
  }, input, cli);
  return Buffer.from(`${canonicalBounded(inspection)}\n`);
}

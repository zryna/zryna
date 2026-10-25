import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { isAbsolute, relative, resolve, sep } from 'node:path';
import { sha256 } from './commands.mjs';
function checked(root, executable, args, spawn = spawnSync) {
  const result = spawn(executable, args, { cwd: root, shell: false, windowsHide: true,
    encoding: 'utf8', timeout: 30_000, maxBuffer: 1024 * 1024 });
  if (result.error || result.signal || result.status !== 0 || result.stderr) {
    throw new Error(`M7 observation failed: ${executable} ${args.join(' ')}`);
  }
  return result.stdout.trim();
}
export function sourceIdentity(root, expected, spawn = spawnSync) {
  if (!/^[a-f0-9]{40}$/u.test(expected)) throw new Error('full expected tested source revision required');
  const commit = checked(root, 'git', ['rev-parse', '--verify', 'HEAD'], spawn);
  const tree = checked(root, 'git', ['rev-parse', '--verify', 'HEAD^{tree}'], spawn);
  const state = checked(root, 'git', ['status', '--porcelain=v1', '--untracked-files=normal'], spawn);
  if (commit !== expected || !/^[a-f0-9]{40}$/u.test(tree) || state !== '') {
    throw new Error('M7 source revision/tree must be actual and clean; PR head is a separate role');
  }
  return { testedRevision: commit, testedTree: tree, cleanTrackedAndUntracked: true,
    ignoredBuildOutputsAttested: false };
}
export function tools(root, registry, spawn = spawnSync) {
  const result = [
    { name: 'node', executable: process.execPath, argv: [process.execPath, '--version'], version: process.version },
    ...['rustc', 'cargo'].map(name => ({ name, executable: name, argv: [name, '--version'],
      version: checked(root, name, ['--version'], spawn) })),
  ];
  if (result[0].version !== `v${registry.tools.node}`) throw new Error('M7 requires the already pinned Node version');
  for (const tool of result.slice(1)) {
    if (!tool.version.startsWith(`${tool.name} ${registry.tools[tool.name]} `)) {
      throw new Error(`M7 requires already pinned ${tool.name} ${registry.tools[tool.name]}`);
    }
  }
  return result;
}
export function createEvidence(root, evidence) {
  if (!isAbsolute(evidence)) throw new Error('absolute new evidence directory required');
  const path = relative(root, evidence);
  if (!path || !isAbsolute(path) && path.split(sep)[0] !== '..') throw new Error('evidence must be outside checkout');
  mkdirSync(evidence, { mode: 0o700 });
  mkdirSync(resolve(evidence, 'logs'), { mode: 0o700 });
}
export function retain(root, name, bytes) {
  writeFileSync(resolve(root, name), bytes, { flag: 'wx', mode: 0o600 });
  return { path: name, bytes: bytes.length, sha256: sha256(bytes) };
}
export function writeReceipt(root, receipt) {
  const bytes = Buffer.from(`${JSON.stringify(receipt, null, 2)}\n`);
  if (bytes.length > 256 * 1024) throw new Error('M7 evidence receipt exceeded its existing document bound');
  retain(root, 'receipt.json', bytes);
}

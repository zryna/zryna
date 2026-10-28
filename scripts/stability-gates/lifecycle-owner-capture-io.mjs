// Local preparation observations only; no runtime, material or source admission.
import assert from 'node:assert/strict';
import { mkdirSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname as filesystemDirname, join as filesystemJoin, posix } from 'node:path';
import { exact, MAX_DOCUMENT, MAX_LOG, sha256 } from './input.mjs';
import { portablePath } from '../distribution/canonical.mjs';
import { execute } from './process.mjs';
import { captureSourceCensus } from './source-census-v2.mjs';
import { inspectCargoConfiguration, pinFile } from './release-build-input.mjs';
import { ordinary } from '../m6/compiler-release/common.mjs';
import { validateCompilerTree } from '../m6/compiler-release/material-tree.mjs';
import { selectLifecycleCrates, captureLifecycleCrates } from './lifecycle-owner-materials.mjs';
import { requireSecurityRustWorkspace } from '../distribution/rust-security-materials.mjs';

export const PACKAGE = 'zryna-playground-compiler';
const { basename, dirname, join } = posix;
export const TARGET = 'x86_64-unknown-linux-gnu';
export const BUILD_ARGS = Object.freeze(['test', '--locked', '--offline', '--release', '-p', PACKAGE,
  '--test', 'lifecycle', '--no-run', '--message-format=json', '-vv']);
const contains = (root, path) => path === root || path.startsWith(root + '/');
function absolute(path) {
  assert(typeof path === 'string' && path.length <= 240 && /^\/[A-Za-z0-9._@+/-]+$/u.test(path));
  assert(path !== '/' && path === posix.normalize(path) && !path.endsWith('/') && path.split('/').length <= 17);
}
export function validateCaptureInput(input) {
  exact(input, ['root', 'output', 'source', 'tools', 'environment', 'upstreamLicense'], 'local capture input');
  for (const key of ['root', 'output', 'upstreamLicense']) absolute(input[key]);
  assert(!contains(input.root, input.output) && !contains(input.output, input.root), 'external capture output');
  const targetDirectory = join(input.output, 'target'); absolute(targetDirectory);
  for (const root of ['/materials', '/input', '/output', '/tmp', '/dev', '/proc', '/sys',
    '/lifecycle', '/usr', '/lib', '/lib64', '/resources']) assert(!contains(root, targetDirectory), 'unshadowed embedded compiler path');
  exact(input.source, ['commit', 'tree'], 'independently selected source');
  for (const value of Object.values(input.source)) assert.match(value, /^[a-f0-9]{40}$/u);
  exact(input.tools, ['cargo', 'rustc', 'git'], 'explicit actual local tool paths');
  for (const value of Object.values(input.tools)) absolute(value);
  assert.equal(new Set(Object.values(input.tools)).size, 3, 'distinct tool paths');
  const keys = ['PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'LANG', 'LC_ALL', 'RUSTC', 'RUSTUP_TOOLCHAIN',
    'CARGO_TARGET_DIR', 'CARGO_NET_OFFLINE', 'CARGO_BUILD_JOBS', 'CARGO_INCREMENTAL', 'CARGO_TERM_COLOR'];
  exact(input.environment, keys, 'fixed local thirteen-key environment');
  for (const value of Object.values(input.environment))
    assert(typeof value === 'string' && value.length > 0 && value.length <= 32768 && !value.includes('\0'));
  for (const key of ['HOME', 'CARGO_HOME', 'RUSTUP_HOME']) absolute(input.environment[key]);
  for (const [key, value] of Object.entries({ RUSTC: input.tools.rustc, RUSTUP_TOOLCHAIN: '1.97.1',
    CARGO_TARGET_DIR: targetDirectory, CARGO_NET_OFFLINE: 'true', CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0',
    CARGO_TERM_COLOR: 'never', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8' })) assert.equal(input.environment[key], value);
  assert.equal(input.environment.PATH, dirname(input.tools.rustc) + ':/usr/bin:/bin', 'fixed compiler and native tool search');
  return targetDirectory;
}
export function captureWriter(output) {
  return (name, value) => {
    portablePath(name);
    const raw = Buffer.isBuffer(value), bytes = raw ? value : Buffer.from(JSON.stringify(value) + '\n');
    assert(bytes.length <= (raw ? MAX_LOG : MAX_DOCUMENT), 'bounded retained capture file');
    mkdirSync(filesystemDirname(filesystemJoin(output, name)), { recursive: true, mode: 0o700 });
    writeFileSync(filesystemJoin(output, name), bytes, { flag: 'wx', mode: 0o600 });
    return { path: name, bytes: bytes.length, sha256: sha256(bytes) };
  };
}
export function actualLifecycleMaterials({ root, environment, upstreamLicense }, graph) {
  const input = { metadata: graph.parsed, sourceRoot: root, cargoTree: graph.tree,
    cargoLock: ordinary(join(root, 'Cargo.lock')) };
  const crateArchives = selectLifecycleCrates(input).external.map(({ pin, cacheRoot }) => {
    const index = basename(dirname(cacheRoot)), name = `${pin.name}-${pin.version}`;
    assert.equal(cacheRoot, join(environment.CARGO_HOME, 'registry/src', index, name), 'selected ordinary registry cache');
    return { identity: name, cacheRoot,
      archive: ordinary(join(environment.CARGO_HOME, 'registry/cache', index, name + '.crate'), 64 * 1024 * 1024) };
  });
  return captureLifecycleCrates({ ...input, crateArchives, upstreamLicense: ordinary(upstreamLicense) });
}
export const DEFAULT_OBSERVATIONS = Object.freeze({
  prepare: prepareCapture,
  writer: captureWriter,
  run: execute,
  census: captureSourceCensus,
  pin: pinFile,
  configuration: inspectCargoConfiguration,
  workspace: requireSecurityRustWorkspace,
  materials: actualLifecycleMaterials,
  treeProof: validateCompilerTree,
  host: () => ({ platform: process.platform, arch: process.arch, node: process.version }),
});
export function prepareCapture(input) {
  assert.equal(realpathSync(input.root), input.root, 'canonical actual source root');
  assert.equal(realpathSync(filesystemDirname(input.output)), filesystemDirname(input.output), 'canonical existing output parent');
  mkdirSync(input.output, { mode: 0o700 });
  mkdirSync(filesystemJoin(input.output, 'target'), { mode: 0o700 });
}

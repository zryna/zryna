import path from 'node:path';
import { createRequire } from 'node:module';
import { existsSync, lstatSync, realpathSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { digest, readStableFile } from './registry-files.mjs';
import { reject } from './evidence-input.mjs';

const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
function versionOutput(executable, args) {
  const result = spawnSync(executable, args, { cwd: sourceRoot, shell: false, windowsHide: true,
    encoding: 'utf8', timeout: 30_000, maxBuffer: 1024 * 1024 });
  if (result.error || result.signal || result.status !== 0) reject('tool observation failed');
  return result.stdout.trim();
}
export function actualFile(filename, maximum = 256 * 1024 * 1024) {
  const resolved = realpathSync(filename);
  const bytes = readStableFile(resolved, maximum, 'actual observed material');
  return { path: resolved, bytes: bytes.length, sha256: digest(bytes) };
}
function findExecutable(name) {
  const suffixes = process.platform === 'win32' ? ['.exe', '.cmd', ''] : [''];
  for (const directory of (process.env.PATH ?? '').split(path.delimiter)) {
    if (!directory) continue;
    for (const suffix of suffixes) {
      const candidate = path.join(directory, `${name}${suffix}`);
      if (existsSync(candidate)) return realpathSync(candidate);
    }
  }
  reject(`actual ${name} executable absent from PATH`);
}
export function resolvePnpmScript(launcher = findExecutable('pnpm')) {
  const actual = realpathSync(launcher);
  let target = actual;
  if (!/^pnpm\.(?:mjs|cjs)$/.test(path.basename(actual))) {
    if (!['pnpm', 'pnpm.cmd'].includes(path.basename(actual))) reject('unsupported pnpm launcher metadata');
    // pnpm/action-setup's self-updater emits this metadata in its extensionless shim.
    // Windows selects pnpm.cmd; its same-directory pnpm sidecar carries the same target.
    const files = [actual];
    if (path.basename(actual) === 'pnpm.cmd') files.push(path.join(path.dirname(actual), 'pnpm'));
    const markers = [];
    for (const filename of files) {
      if (!existsSync(filename)) continue;
      const bytes = readStableFile(realpathSync(filename), 64 * 1024, 'pnpm launcher metadata');
      const lines = new TextDecoder('utf-8', { fatal: true }).decode(bytes).split(/\r?\n/);
      for (const line of lines.filter(value => value.includes('cmd-shim-target='))) {
        const match = /^# cmd-shim-target=(.+)$/.exec(line);
        if (!match) reject('malformed direct pnpm target metadata');
        markers.push(match[1]);
      }
    }
    if (markers.length !== 1 || !path.isAbsolute(markers[0]) || markers[0].split(/[\\/]/).some(part => part === '.' || part === '..')) {
      reject('one absolute direct pnpm JS target required; shell launchers are not executed');
    }
    target = markers[0];
  }
  if (!/^pnpm\.(?:mjs|cjs)$/.test(path.basename(target)) || path.basename(path.dirname(target)) !== 'bin') {
    reject('unsupported direct pnpm JS target');
  }
  const targetStat = lstatSync(target);
  if (!targetStat.isFile() || targetStat.isSymbolicLink()) reject('regular direct pnpm target required');
  const selected = path.join(path.dirname(realpathSync(target)), 'pnpm.cjs');
  const entry = path.join(path.dirname(selected), 'pnpm.mjs');
  for (const filename of [selected, entry]) {
    const stat = lstatSync(filename);
    if (!stat.isFile() || stat.isSymbolicLink()) reject('regular direct pnpm JS entries required');
    readStableFile(filename, 64 * 1024, 'direct pnpm JS entry');
  }
  const manifest = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(readStableFile(
    path.join(path.dirname(selected), '../package.json'), 64 * 1024, 'pinned pnpm package')));
  if (manifest.name !== 'pnpm' || manifest.version !== '11.18.0' || manifest.bin?.pnpm !== 'bin/pnpm.mjs') {
    reject('exact pinned pnpm package identity required');
  }
  // Keep the independently pinned compatibility entry; it imports this package's pnpm.mjs.
  return realpathSync(selected);
}
export function observedTools() {
  const pins = { node: '22.22.1', rustc: '1.97.1', cargo: '1.97.1', pnpm: '11.18.0' };
  const rustup = findExecutable('rustup');
  const files = { node: realpathSync(process.execPath),
    rustc: versionOutput(rustup, ['which', '--toolchain', '1.97.1', 'rustc']),
    cargo: versionOutput(rustup, ['which', '--toolchain', '1.97.1', 'cargo']), pnpm: resolvePnpmScript() };
  return Object.entries(pins).map(([name, version]) => {
    const executable = actualFile(files[name], 512 * 1024 * 1024);
    const versionArgv = name === 'pnpm' ? [files.node, executable.path, '--version'] : [executable.path, '--version'];
    const versionText = versionOutput(versionArgv[0], versionArgv.slice(1));
    return { name, version, executable, versionArgv, versionText };
  });
}
export function observedMaterials() {
  const require = createRequire(path.join(sourceRoot, 'adapters/typescript-6/package.json'));
  const wrapper = path.dirname(require.resolve('@typescript/typescript6/package.json'));
  const old = path.dirname(createRequire(path.join(wrapper, 'package.json')).resolve('@typescript/old/package.json'));
  const inputs = [
    ['typescript-wrapper', path.join(wrapper, 'lib/typescript.js')],
    ['typescript-implementation', path.join(old, 'lib/typescript.js')],
    ['cargo-lock', path.join(sourceRoot, 'Cargo.lock')],
    ['runtime-material-binding', path.join(sourceRoot, 'scripts/distribution/command-h1-rust-binding-v1.json')],
  ];
  if (process.env.ZRYNA_QUALIFICATION_NODE_ROOT) inputs.push(['installed-node', path.join(
    process.env.ZRYNA_QUALIFICATION_NODE_ROOT, process.platform === 'win32' ? 'node.exe' : 'bin/node')]);
  return inputs.map(([name, filename]) => ({ name, ...actualFile(filename) }));
}

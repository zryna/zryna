import path from 'node:path';
import { createRequire } from 'node:module';
import { existsSync, realpathSync } from 'node:fs';
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
function pnpmScript() {
  const launcher = findExecutable('pnpm');
  const candidates = [launcher.endsWith('.cjs') ? launcher : null,
    path.join(path.dirname(launcher), 'node_modules/pnpm/bin/pnpm.cjs'),
    path.join(path.dirname(launcher), '../lib/node_modules/pnpm/bin/pnpm.cjs'),
    path.join(path.dirname(launcher), '../pnpm/bin/pnpm.cjs'),
    path.join(path.dirname(launcher), 'pnpm.cjs')].filter(Boolean);
  const selected = candidates.find(candidate => existsSync(candidate));
  if (!selected) reject('direct pinned pnpm JS entry required; shell launchers are not executed');
  return realpathSync(selected);
}
export function observedTools() {
  const pins = { node: '22.22.1', rustc: '1.97.1', cargo: '1.97.1', pnpm: '11.18.0' };
  const rustup = findExecutable('rustup');
  const files = { node: realpathSync(process.execPath),
    rustc: versionOutput(rustup, ['which', '--toolchain', '1.97.1', 'rustc']),
    cargo: versionOutput(rustup, ['which', '--toolchain', '1.97.1', 'cargo']), pnpm: pnpmScript() };
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

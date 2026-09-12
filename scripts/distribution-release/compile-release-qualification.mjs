import { spawnSync } from 'node:child_process';
import {
  lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync,
} from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { canonical, sha256 } from './canonical.mjs';
import { validateReleaseQualificationInputText } from './validate-release-qualification-input.mjs';

const MAX_BINARY = 268435456;
const MAX_OUTPUT = 16 * 1024 * 1024;

function reject(message) {
  throw new Error(`R406-QUALIFICATION-COMPILE: ${message}`);
}

function samePath(left, right) {
  const normalize = (value) => process.platform === 'win32'
    ? resolve(value).toLowerCase() : resolve(value);
  return normalize(left) === normalize(right);
}

function directory(path, system) {
  const metadata = system.inspect(path);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()
    || !samePath(system.realPath(path), path)) reject('controlled directory identity differs');
}

function tool(path, record, system) {
  const metadata = system.inspect(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size !== record.size
    || !samePath(system.realPath(path), path)) reject(`${record.name} executable identity differs`);
  const bytes = system.read(path);
  if (bytes.length !== record.size || sha256(bytes) !== record.sha256) {
    reject(`${record.name} executable bytes differ`);
  }
}

function replace(value, replacements) {
  let result = value;
  for (const [token, replacement] of Object.entries(replacements)) {
    result = result.replaceAll(token, replacement);
  }
  if (/@[A-Za-z0-9_-]+@/.test(result)) reject('compile token is unresolved');
  return result;
}

export function compileReleaseQualification({
  binding, sourceRoot, workRoot, cargoHome, observedTools,
  spawn = spawnSync,
  system = { inspect: lstatSync, realPath: realpathSync.native, read: readFileSync,
    list: readdirSync, make: mkdirSync },
}) {
  if (!Buffer.isBuffer(binding) || ![sourceRoot, workRoot, cargoHome].every(isAbsolute)
    || samePath(sourceRoot, workRoot) || samePath(sourceRoot, cargoHome)
    || !samePath(dirname(cargoHome), workRoot)) reject('exact isolated compile roots are required');
  const input = validateReleaseQualificationInputText(binding.toString('utf8'));
  if (canonical(input.toolchains) !== canonical(observedTools?.toolchains)
    || canonical(input.nativeTools) !== canonical(observedTools?.nativeTools)) {
    reject('observed tools differ from the qualification binding');
  }
  directory(sourceRoot, system);
  directory(workRoot, system);
  directory(cargoHome, system);
  if (canonical(system.list(workRoot).sort()) !== canonical(['cargo-home'])) {
    reject('compile work root must contain only its preprovisioned Cargo home');
  }
  const targetRoot = join(workRoot, 'target');
  const tempRoot = join(workRoot, 'temp');
  system.make(targetRoot, { recursive: false, mode: 0o700 });
  system.make(tempRoot, { recursive: false, mode: 0o700 });
  const paths = observedTools.paths;
  const records = new Map([...input.toolchains, ...input.nativeTools]
    .map((record) => [record.name, record]));
  for (const name of ['cargo', 'rustc', 'linker']) {
    if (!records.has(name) || !isAbsolute(paths?.[name] ?? '')) {
      reject(`${name} observed path is missing`);
    }
    tool(paths[name], records.get(name), system);
  }
  const replacements = {
    '@cargo-home@': cargoHome,
    '@linker@': paths.linker,
    '@qualification-binding-sha256@': sha256(binding),
    '@rustc@': paths.rustc,
    '@source-date-epoch@': String(input.source.sourceDateEpoch),
    '@source-root@': sourceRoot,
    '@target-root@': targetRoot,
    '@temp-root@': tempRoot,
    '@work-root@': workRoot,
  };
  const environment = Object.fromEntries(input.compile.environment
    .map(({ name, value }) => [name, replace(value, replacements)]));
  const rustFlags = input.compile.encodedRustFlags
    .map((value) => replace(value, replacements));
  environment.CARGO_ENCODED_RUSTFLAGS = rustFlags.join('\x1f');
  const result = spawn(paths.cargo, input.compile.argv.slice(1), {
    cwd: sourceRoot, env: environment, encoding: null, maxBuffer: MAX_OUTPUT + 1,
    timeout: 45 * 60_000, windowsHide: true, shell: false,
  });
  for (const name of ['cargo', 'rustc', 'linker']) tool(paths[name], records.get(name), system);
  if (result.error || result.status !== 0 || result.signal !== null
    || !Buffer.isBuffer(result.stdout) || !Buffer.isBuffer(result.stderr)
    || result.stdout.length + result.stderr.length > MAX_OUTPUT) {
    reject(`Cargo qualification compile failed with status ${result.status ?? 'unknown'}`);
  }
  const executable = join(targetRoot, input.target.triple, 'release',
    input.target.triple === 'x86_64-pc-windows-msvc' ? 'zryna.exe' : 'zryna');
  const metadata = system.inspect(executable);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size < 64
    || metadata.size > MAX_BINARY || !samePath(system.realPath(executable), executable)) {
    reject('compiled CLI file identity differs');
  }
  const cli = system.read(executable);
  if (cli.length !== metadata.size) reject('compiled CLI bytes differ');
  return { cli, environment, encodedRustFlags: rustFlags, executable };
}

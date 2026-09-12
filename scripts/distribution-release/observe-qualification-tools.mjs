import { spawnSync } from 'node:child_process';
import { lstatSync, readFileSync, realpathSync } from 'node:fs';
import { dirname, isAbsolute, resolve } from 'node:path';
import { NODE_TARGETS, NODE_UPSTREAM } from '../distribution/materials.mjs';
import { canonical, parseCanonical, sha256 } from './canonical.mjs';
import { validateReleaseQualificationArchitecture } from './validate-release-qualification-architecture.mjs';

const MAX_TOOL = 536870912;
const TARGETS = Object.freeze({
  'x86_64-unknown-linux-gnu': {
    platform: 'linux',
    tools: [
      ['inspector', '/usr/bin/x86_64-linux-gnu-readelf', ['--version'],
        'ubuntu-24.04:/usr/bin/x86_64-linux-gnu-readelf'],
      ['linker', '/usr/bin/x86_64-linux-gnu-gcc-13', ['--version'],
        'ubuntu-24.04:/usr/bin/x86_64-linux-gnu-gcc-13'],
      ['xz', '/usr/bin/xz', ['--version'], 'ubuntu-24.04:/usr/bin/xz'],
    ],
  },
  'x86_64-pc-windows-msvc': { platform: 'win32', tools: null },
});

function reject(message) {
  throw new Error(`R406-QUALIFICATION-TOOLS: ${message}`);
}

function samePath(left, right, platform) {
  const normalize = (value) => platform === 'win32'
    ? resolve(value).toLowerCase() : resolve(value);
  return normalize(left) === normalize(right);
}

function inspectTool(path, expectedDigest, platform, system) {
  if (!isAbsolute(path ?? '') || !/^[0-9a-f]{64}$/.test(expectedDigest ?? '')) {
    reject('tool path or digest differs');
  }
  const metadata = system.inspectFile(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size < 1
    || metadata.size > MAX_TOOL || !samePath(system.realPath(path), path, platform)) {
    reject('tool must be one direct regular executable');
  }
  const bytes = system.readFile(path);
  if (bytes.length !== metadata.size || sha256(bytes) !== expectedDigest) {
    reject('tool executable bytes differ');
  }
  return { size: bytes.length, sha256: expectedDigest };
}

function versionLine(path, args, cwd, spawn) {
  const result = spawn(path, args, {
    cwd, env: {}, encoding: 'utf8', maxBuffer: 65537, timeout: 30_000,
    windowsHide: true, shell: false,
  });
  if (result.error || result.signal !== null || !Number.isInteger(result.status)
    || typeof result.stdout !== 'string' || typeof result.stderr !== 'string'
    || Buffer.byteLength(result.stdout) + Buffer.byteLength(result.stderr) > 65536) {
    reject('tool version command failed');
  }
  const lines = `${result.stdout}\n${result.stderr}`.split(/\r?\n/)
    .map((line) => line.trim()).filter(Boolean);
  if (lines.length < 1 || !/^[ -~]{1,512}$/.test(lines[0])) {
    reject('tool version output differs');
  }
  return lines[0];
}

function observationEvidence(target, tool) {
  return sha256(Buffer.from(canonical({ target, ...tool }), 'utf8'));
}

function windowsTools(environment) {
  const linker = environment.ZRYNA_LINKER_PATH;
  const inspector = environment.ZRYNA_INSPECTOR_PATH;
  const pattern = /^[A-Za-z]:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\[0-9.]+\\bin\\Hostx64\\x64\\(link|dumpbin)\.exe$/i;
  if (!pattern.test(linker ?? '') || !pattern.test(inspector ?? '')
    || dirname(linker).toLowerCase() !== dirname(inspector).toLowerCase()
    || !/\\link\.exe$/i.test(linker) || !/\\dumpbin\.exe$/i.test(inspector)) {
    reject('Windows native tool roots differ');
  }
  return [
    ['inspector', inspector, ['/?'], `windows-2022:${inspector}`],
    ['linker', linker, ['/?'], `windows-2022:${linker}`],
  ];
}

function hostEnvironment(target, environment) {
  if (target !== 'x86_64-pc-windows-msvc') return {};
  const result = {};
  for (const [name, variable] of [
    ['INCLUDE', 'INCLUDE'], ['LIB', 'LIB'], ['LIBPATH', 'LIBPATH'],
    ['PATH', 'ZRYNA_QUALIFICATION_PATH'], ['SystemRoot', 'SystemRoot'],
  ]) {
    const value = environment[variable];
    if (typeof value !== 'string' || !/^[ -~]{1,512}$/.test(value)) {
      reject(`Windows ${variable} environment differs`);
    }
    result[name] = value;
  }
  return result;
}

export function observeQualificationTools({
  target, architectureBytes, workingRoot, environment = process.env, spawn = spawnSync,
  nodePath = process.execPath,
  system = { platform: process.platform, inspectFile: lstatSync,
    realPath: realpathSync.native, readFile: readFileSync },
}) {
  const targetConfig = TARGETS[target];
  if (!targetConfig || system.platform !== targetConfig.platform || !isAbsolute(workingRoot ?? '')) {
    reject('tool observation target or working root differs');
  }
  const architecture = validateReleaseQualificationArchitecture(
    parseCanonical(architectureBytes.toString('utf8')),
  );
  const evidence = sha256(architectureBytes);
  const primary = [
    ['cargo', environment.ZRYNA_CARGO_PATH, architecture.toolchain.cargoSha256,
      'repository:rust-toolchain.toml'],
    ['node', nodePath, NODE_TARGETS[target]?.executable[1],
      'https://nodejs.org/dist/v22.22.1/'],
    ['rustc', environment.ZRYNA_RUSTC_PATH, architecture.toolchain.rustcSha256,
      'repository:rust-toolchain.toml'],
  ].map(([name, path, digest, origin]) => {
    const identity = inspectTool(path, digest, system.platform, system);
    const version = name === 'node' ? '22.22.1' : '1.97.1';
    return { record: { name, version, origin, ...identity,
      observationEvidenceSha256: name === 'node' ? NODE_UPSTREAM.signatureSha256 : evidence }, path };
  });
  const specifications = targetConfig.tools ?? windowsTools(environment);
  const native = specifications.map(([name, path, args, origin]) => {
    const first = inspectTool(path, sha256(system.readFile(path)), system.platform, system);
    const version = versionLine(path, args, workingRoot, spawn);
    const second = inspectTool(path, first.sha256, system.platform, system);
    if (canonical(first) !== canonical(second)) reject('native tool changed during observation');
    const record = { name, version, origin, ...first };
    return { record: { ...record, observationEvidenceSha256: observationEvidence(target, record) },
      path };
  });
  return {
    toolchains: primary.map(({ record }) => record),
    nativeTools: native.map(({ record }) => record),
    paths: Object.fromEntries([...primary, ...native].map(({ record, path }) => [record.name, path])),
    hostEnvironment: hostEnvironment(target, environment),
  };
}

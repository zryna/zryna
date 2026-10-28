// Shared synthetic retained release fixture; no compiler or image execution.
import assert from 'node:assert/strict';
import { posix, win32 } from 'node:path';
import { sha256 } from '../scripts/stability-gates/input.mjs';
export function releaseImageFixture(windows = false) {
  const host = windows ? 'windows-x86_64' : 'linux-x86_64', p = windows ? win32 : posix;
  const root = windows ? 'C:\\source' : '/source', output = windows ? 'D:\\evidence\\capture' : '/evidence/capture';
  const directory = p.join(output, 'release-build'), targetDirectory = p.join(directory, 'cargo-target');
  const target = windows ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu', name = windows ? 'zryna.exe' : 'zryna';
  const toolRoot = windows ? 'C:\\tools' : '/tools', home = windows ? 'C:\\home' : '/home';
  const cargoHome = windows ? 'C:\\cache\\cargo' : '/cache/cargo', rustup = windows ? 'C:\\cache\\rustup' : '/cache/rustup';
  const keys = ['cargo', 'rustc', 'cc', 'linker', 'node', 'git'];
  const tools = Object.fromEntries(keys.map(key => [key, p.join(toolRoot, key + (windows ? '.exe' : ''))]));
  const env = { PATH: toolRoot, HOME: home, CARGO_HOME: cargoHome, RUSTUP_HOME: rustup, LANG: 'C', LC_ALL: 'C',
    GITHUB_ACTIONS: 'true', GITHUB_REPOSITORY: 'zryna/zryna', GITHUB_RUN_ID: '123', GITHUB_RUN_ATTEMPT: '1',
    RUNNER_OS: windows ? 'Windows' : 'Linux', RUNNER_ARCH: 'X64', ImageOS: windows ? 'win22' : 'ubuntu24', ImageVersion: 'fixture' };
  if (windows) Object.assign(env, { SystemRoot: 'C:\\Windows', WINDIR: 'C:\\Windows', TEMP: 'C:\\temp', TMP: 'C:\\temp',
    USERPROFILE: home, INCLUDE: 'fixture', LIB: 'fixture', LIBPATH: 'fixture', WindowsSdkDir: 'C:\\sdk\\',
    WindowsSDKVersion: '10.0.26100.0\\', UniversalCRTSdkDir: 'C:\\sdk\\', UCRTVersion: '10.0.26100.0',
    VCToolsInstallDir: 'C:\\vc\\', VCToolsVersion: '14.44.35207' });
  else env.TMPDIR = '/tmp';
  const expectedSource = { commit: 'a'.repeat(40), tree: 'b'.repeat(40) };
  const operator = { calibrationDirectory: windows ? 'C:\\calibration' : null, environment: env,
    expectedHead: expectedSource.commit, expectedTree: expectedSource.tree, root, round: 'baseline-round-1', tools };
  const effective = { ...env, CARGO_TARGET_DIR: targetDirectory, CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0',
    ...(windows ? { RUSTFLAGS: '-Clink-arg=/Brepro' } : {}), CARGO_NET_OFFLINE: 'true', RUSTUP_TOOLCHAIN: '1.97.1',
    RUSTC: tools.rustc, CC: tools.cc, [windows ? 'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER' : 'CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER']: tools.linker };
  const pin = path => ({ path, bytes: 128, sha256: 'f'.repeat(64), mode: windows ? '666' : '755', prefixHex: '00'.repeat(128) });
  const source = { ...expectedSource, manifestSha256: 'c'.repeat(64), inventorySha256: 'd'.repeat(64) };
  const sdk = windows ? [pin('C:\\sdk\\Include\\10.0.26100.0\\shared\\sdkddkver.h'), pin('C:\\sdk\\Include\\10.0.26100.0\\ucrt\\corecrt.h')] : null;
  const config = [cargoHome, p.join(root, '.cargo'), p.join(p.parse(root).root, '.cargo')]
    .flatMap(path => ['config', 'config.toml'].map(name => ({ path: p.join(path, name), present: false })));
  const executable = p.join(targetDirectory, target, 'release', name), measured = p.join(output, 'retained-release-cli', name);
  const build = { protocol: 's418-release-build-capture-v1', kind: 'observed-release-profile-build', target,
    hostPlatform: windows ? 'win32' : 'linux', runnerFamily: windows ? 'windows-2022' : 'ubuntu-24.04',
    source: { before: structuredClone(source), after: structuredClone(source) }, environment: effective,
    tools: { before: Object.fromEntries(keys.map(key => [key, pin(tools[key])])), after: Object.fromEntries(keys.map(key => [key, pin(tools[key])])) },
    sdk: { before: structuredClone(sdk), after: structuredClone(sdk) }, cargoConfiguration: { before: structuredClone(config), after: structuredClone(config) },
    cli: { before: { expectedPath: executable, exists: false, sha256: null }, expectedPath: executable },
    profile: { requestedByActualCommand: 'release', observed: 'release', verifiedFrom: ['actual-command', 'actual-build-output', 'fresh-target-release-path'] },
    authenticated: false, sourceAuthenticated: false, productionReleaseAdmission: false, baselineEligible: false,
    thresholdsEnabled: false, baseline: null, thresholds: null };
  const retained = { format: 'zryna.cli-performance-retained-release-cli.v2', links: { builtBefore: 2, builtAfter: 2, copy: 1 },
    targetDirectory, measuredPath: measured, sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
  const collectorKeys = windows ? ['PATH', 'SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'CARGO_HOME', 'RUSTUP_HOME', 'LIB', 'INCLUDE']
    : ['PATH', 'HOME', 'TMPDIR', 'CARGO_HOME', 'RUSTUP_HOME'];
  const capture = { sourceCommit: source.commit, sourceTree: source.tree, round: operator.round, cliBuildClass: 'release',
    host: { platform: host }, tools: { node: { path: tools.node, bytes: 128, sha256: 'f'.repeat(64) } },
    environment: { ...Object.fromEntries(collectorKeys.map(key => [key, env[key]])), RUSTUP_TOOLCHAIN: '1.97.1', RUSTC: tools.rustc },
    sourceAuthenticated: false, performanceQualified: false, productionReleaseAdmission: false, baselineAuthority: null, thresholds: null };
  const files = new Map(), commands = {}, results = {}, streams = {};
  const specs = { cargo: ['--version'], rustc: ['-vV'], cc: windows ? [] : ['--version'], linker: windows ? ['/?'] : ['--version'],
    node: ['--version'], git: ['--version'], 'release-build': ['build', '--locked', '--offline', '--release', '--target', target, '-p', 'zryna', '--bin', 'zryna'] };
  for (const key of Object.keys(specs)) {
    commands[key] = { executable: tools[key === 'release-build' ? 'cargo' : key], args: specs[key], cwd: root,
      environment: effective, timeoutMs: key === 'release-build' ? 1200000 : 30000, shell: false };
    let stdout = key + ' synthetic version\n', stderr = '';
    if (key === 'cargo') stdout = 'cargo 1.97.1 (c980f4866 2026-06-30)\n';
    if (key === 'rustc') stdout = `rustc 1.97.1 (8bab26f4f 2026-07-14)\nhost: ${target}\n`;
    if (key === 'node') stdout = 'v22.22.1\n';
    if (key === 'release-build') { stdout = ''; stderr = '    Finished `release` profile [optimized] target(s)\n'; }
    if (windows && key === 'linker') stdout = 'Microsoft (R) Incremental Linker Version 14.44.35229.0\r\n usage: LINK [options] [files] [@commandfile]\r\n';
    streams[key] = { stdout: Buffer.from(stdout), stderr: Buffer.from(stderr) };
    results[key] = { command: commands[key], termination: { exitCode: windows && key === 'linker' ? 1100 : 0,
      signal: null, error: null, elapsedMs: 10 }, logs: {} };
  }
  function rebindImage(image, mode = windows ? '666' : '755') {
    const built = { path: executable, bytes: image.length, sha256: sha256(image), mode, prefixHex: image.subarray(0, 128).toString('hex') };
    build.cli.after = built; retained.sourceBefore = structuredClone(built); retained.sourceAfter = structuredClone(built);
    retained.copy = { ...built, path: measured }; capture.tools.cli = { path: measured, bytes: image.length, sha256: sha256(image) };
    files.set(`retained-release-cli/${name}`, image);
  }
  const image = Buffer.alloc(256);
  if (windows) { image.write('MZ'); image.writeUInt32LE(128, 60); image.write('PE\0\0', 128); image.writeUInt16LE(0x8664, 132); }
  else { Buffer.from([127, 69, 76, 70, 2, 1]).copy(image); image.writeUInt16LE(62, 18); }
  rebindImage(image);
  function sync() {
    const put = (name, value) => files.set(name, Buffer.from(JSON.stringify(value) + '\n'));
    for (const key of Object.keys(specs)) {
      for (const name of ['stdout', 'stderr']) {
        const bytes = streams[key][name], relative = `release-build/commands/${key}.${name}.raw`; files.set(relative, bytes);
        results[key].logs[name] = { path: p.join(directory, 'commands', `${key}.${name}.raw`), bytes: bytes.length, sha256: sha256(bytes) };
      }
      put(`release-build/commands/${key}.command.json`, commands[key]); put(`release-build/commands/${key}.result.json`, results[key]);
    }
    Object.assign(build, { command: commands['release-build'], build: results['release-build'].termination, logs: results['release-build'].logs });
    put('operator-input.json', operator); put('release-build/capture.json', build);
    put('release-build/observed.json', { ...build, profile: { requestedByActualCommand: 'release', observed: null } });
    put('retained-release-cli.json', retained); put('round/capture.json', capture);
    put('release-build/cargo-configuration-before.json', build.cargoConfiguration.before);
  }
  sync();
  return { host, expectedSource, files, operator, build, retained, capture, commands, results, streams, sync, rebindImage,
    read: (name, maximum) => { assert(files.has(name), name); assert(files.get(name).length <= maximum); return files.get(name); } };
}

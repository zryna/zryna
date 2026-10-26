import { existsSync, mkdirSync, readSync, closeSync, openSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { execute } from './process.mjs';
import { captureSourceCensus, sourceUnchanged } from './source-census-v2.mjs';
import { buildArguments, controlledEnvironment, digest, inspectCargoConfiguration, outside, pinFile, requireValue, sdkFiles,
  TARGETS, validateBinary, validateVersions } from './release-build-input.mjs';

function document(path, value) {
  const bytes = Buffer.from(`${JSON.stringify(value)}\n`);
  requireValue(bytes.length <= 256 * 1024, 'capture document bound');
  writeFileSync(path, bytes, { flag: 'wx', mode: 0o600 });
  return { path, bytes: bytes.length, sha256: digest(bytes) };
}
// Raw output and command metadata are durable before semantic interpretation.
async function observe(directory, name, executable, args, root, env, timeoutMs) {
  const command = { executable, args, cwd: root, environment: env, timeoutMs, shell: false };
  document(join(directory, `${name}.command.json`), command);
  const result = await execute(executable, args, { root, env, timeoutMs });
  const logs = {};
  for (const stream of ['stdout', 'stderr']) {
    const path = join(directory, `${name}.${stream}.raw`);
    writeFileSync(path, result[stream], { flag: 'wx', mode: 0o600 });
    logs[stream] = { path, bytes: result[stream].length, sha256: digest(result[stream]) };
  }
  const { stdout, stderr, ...termination } = result;
  const receipt = { command, termination, logs };
  document(join(directory, `${name}.result.json`), receipt);
  return { ...receipt, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') };
}
function successful(result, allowed = [0]) {
  requireValue(!result.termination.error && !result.termination.signal &&
    allowed.includes(result.termination.exitCode), 'observed command completed');
}
function validatePE(path) {
  const fd = openSync(path, 'r');
  try {
    const dos = Buffer.alloc(64); requireValue(readSync(fd, dos, 0, 64, 0) === 64, 'DOS header');
    const offset = dos.readUInt32LE(60);
    requireValue(offset >= 64 && offset <= 2 * 1024 * 1024 - 8, 'bounded PE header offset');
    const pe = Buffer.alloc(8); requireValue(readSync(fd, pe, 0, 8, offset) === 8 &&
      pe.subarray(0, 4).equals(Buffer.from([80, 69, 0, 0])) && pe.readUInt16LE(4) === 0x8664,
    'PE x64 signature and machine');
  } finally { closeSync(fd); }
}
export async function captureReleaseBuild({ root, output, target, tools, environment }) {
  requireValue(Object.hasOwn(TARGETS, target) && process.platform === TARGETS[target][0] &&
    process.arch === 'x64', 'native supported host target');
  root = realpathSync(root); output = resolve(output);
  requireValue(outside(root, realpathSync(dirname(output))), 'fresh output parent outside source');
  const keys = ['cargo', 'rustc', 'cc', 'linker', 'node', 'git'];
  requireValue(Object.keys(tools).length === keys.length && keys.every(key => Object.hasOwn(tools, key)),
    'six explicit tool paths');
  const toolPinsBefore = Object.fromEntries(keys.map(key => [key, pinFile(tools[key])]));
  requireValue(realpathSync(tools.node) === realpathSync(process.execPath), 'actual capture Node identity');
  mkdirSync(output, { mode: 0o700 });
  const directory = join(output, 'commands'); mkdirSync(directory, { mode: 0o700 });
  const targetDirectory = join(output, 'cargo-target'); mkdirSync(targetDirectory, { mode: 0o700 });
  const executable = join(targetDirectory, target, 'release', winName(target));
  const cliBefore = { expectedPath: executable, exists: existsSync(executable), sha256: null };
  requireValue(!cliBefore.exists, 'release CLI absent in fresh external target');
  const env = controlledEnvironment(environment, { target, targetDirectory, tools });
  const configurationBefore = inspectCargoConfiguration(root, env.CARGO_HOME);
  document(join(output, 'cargo-configuration-before.json'), configurationBefore);
  requireValue(configurationBefore.every(item => !item.present), 'forbid all Cargo configuration overrides');
  const win = process.platform === 'win32';
  const sdkBefore = win ? sdkFiles(env).map(path => pinFile(path, 2 * 1024 * 1024)) : null;
  const specs = { cargo: ['--version'], rustc: ['-vV'], cc: win ? [] : ['--version'],
    linker: win ? ['/?'] : ['--version'], node: ['--version'], git: ['--version'] };
  const observations = {};
  for (const key of keys) {
    observations[key] = await observe(directory, key, tools[key], specs[key], root, env, 30_000);
    successful(observations[key], win && key === 'cc' ? [0, 2] : [0]);
  }
  validateVersions(observations, target);
  const before = captureSourceCensus({ root, directory: join(output, 'source-before'), executable: tools.git, env });
  const args = buildArguments(target);
  const build = await observe(directory, 'release-build', tools.cargo, args, root, env, 20 * 60_000);
  // Capture after-state even for a failed build, before applying any success/profile checks.
  let after;
  try { after = captureSourceCensus({ root, directory: join(output, 'source-after'), executable: tools.git, env }); }
  catch (error) { document(join(output, 'source-after-error.json'), { message: String(error.message) }); throw error; }
  let binary = null;
  try { binary = pinFile(executable); } catch (error) {
    document(join(output, 'binary-error.json'), { expectedPath: executable, message: String(error.message) });
  }
  const toolPinsAfter = Object.fromEntries(keys.map(key => [key, pinFile(tools[key])]));
  const sdkAfter = win ? sdkFiles(env).map(path => pinFile(path, 2 * 1024 * 1024)) : null;
  const configurationAfter = inspectCargoConfiguration(root, env.CARGO_HOME);
  const receipt = { protocol: 's418-release-build-capture-v1', kind: 'observed-release-profile-build',
    target, hostPlatform: process.platform, runnerFamily: TARGETS[target][1],
    source: { before, after }, environment: env, tools: { before: toolPinsBefore, after: toolPinsAfter },
    sdk: { before: sdkBefore, after: sdkAfter }, command: build.command, build: build.termination,
    cargoConfiguration: { before: configurationBefore, after: configurationAfter },
    logs: build.logs, cli: { before: cliBefore, after: binary, expectedPath: executable },
    profile: { requestedByActualCommand: 'release', observed: null }, authenticated: false,
    sourceAuthenticated: false, productionReleaseAdmission: false, baselineEligible: false,
    thresholdsEnabled: false, baseline: null, thresholds: null };
  document(join(output, 'observed.json'), receipt);
  successful(build); sourceUnchanged(before, after);
  requireValue(configurationAfter.every(item => !item.present), 'Cargo configuration remained absent');
  requireValue(JSON.stringify(toolPinsBefore) === JSON.stringify(toolPinsAfter) &&
    JSON.stringify(sdkBefore) === JSON.stringify(sdkAfter), 'stable selected tool and SDK identities');
  requireValue(binary !== null, 'expected release CLI exists'); validateBinary(binary, target);
  if (win) { validatePE(executable); requireValue(pinFile(executable).sha256 === binary.sha256, 'stable PE inspection'); }
  requireValue(/^\s*Finished `release` profile \[optimized\].*$/m.test(build.stderr) &&
    !/^\s*Finished `release` profile \[unoptimized/m.test(build.stderr),
    'actual build output confirms optimized release profile');
  receipt.profile.observed = 'release';
  receipt.profile.verifiedFrom = ['actual-command', 'actual-build-output', 'fresh-target-release-path'];
  document(join(output, 'capture.json'), receipt);
  return receipt;
}
function winName(target) { return target === 'x86_64-pc-windows-msvc' ? 'zryna.exe' : 'zryna'; }

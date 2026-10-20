// Executes the ordinary installed compiler, never the private activation harness.
import assert from 'node:assert/strict';
import {
  existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { bytes, sha256 } from '../../scripts/distribution/canonical.mjs';
import { cases } from './fixtures.mjs';
import { runInstalledPureCommandH1 } from './command-h1.mjs';
import { executeRecorded, ordinaryBytes, saveEvidence, successful } from './commands.mjs';

function inventory(root, prefix = '') {
  const result = [];
  for (const name of readdirSync(join(root, prefix)).sort()) {
    const path = prefix ? prefix + '/' + name : name, st = lstatSync(join(root, path));
    assert(!st.isSymbolicLink());
    if (st.isDirectory()) result.push(...inventory(root, path));
    else { assert(st.isFile() && st.nlink === 1); result.push(path); }
  }
  return result;
}

function readManifest(project, result, definition, target) {
  const response = JSON.parse(successful(result)); assert.equal(response.ok, true);
  assert.equal(typeof response.manifest, 'string');
  assert(response.manifest.startsWith('.zryna/out/') && !response.manifest.includes('..')
    && !isAbsolute(response.manifest));
  const path = join(project, response.manifest), data = ordinaryBytes(path, 32 * 1024 * 1024);
  const manifest = JSON.parse(data);
  assert.equal(manifest.version, definition.version); assert.equal(manifest.profile, definition.manifestProfile);
  assert.equal(manifest.command, 'build'); assert.deepEqual(manifest.results, []);
  assert(manifest.targets.includes(target)); assert(manifest.artifacts.length > 0);
  const artifacts = manifest.artifacts.map(item => {
    assert.equal(item.target, target);
    const logicalPath = item.path ?? `${item.target}/${item.filename}`;
    assert(typeof logicalPath === 'string'
      && /^(javascript|webassembly|native)\/[A-Za-z_][A-Za-z0-9_-]*\.(mjs|wasm|o)$/.test(logicalPath));
    const full = join(dirname(path), ...logicalPath.split('/')), body = ordinaryBytes(full);
    assert.equal(body.length, item.bytes); assert.equal(sha256(body), item.sha256);
    return { ...item, full, body };
  });
  return { manifest, artifacts, path, data };
}

async function observeArtifact(artifacts, target) {
  if (target === 'javascript') {
    const entry = artifacts.find(item => (item.path ?? item.filename).endsWith('.mjs'));
    assert(entry); const module = await import(pathToFileURL(entry.full).href);
    assert.equal(typeof module.main, 'function'); assert.equal(module.main(), 42);
    return { target, actualValue: 42, observer: 'trusted coordinator Node, outside installed CLI environment' };
  }
  if (target === 'webassembly') {
    const entry = artifacts.find(item => (item.path ?? item.filename).endsWith('.wasm'));
    assert(entry); const { instance } = await WebAssembly.instantiate(entry.body, {});
    assert.equal(typeof instance.exports.main, 'function'); assert.equal(instance.exports.main(), 42);
    return { target, actualValue: 42, observer: 'trusted coordinator WebAssembly host, outside installed CLI environment' };
  }
  for (const item of artifacts) {
    assert(item.body.subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46])));
    assert.equal(item.body[4], 2); assert.equal(item.body[5], 1);
    assert.equal(item.body.readUInt16LE(16), 1); assert.equal(item.body.readUInt16LE(18), 62);
  }
  return { target, objectBytesAndElfMachineVerified: true, executionClaimed: false };
}

export async function runNativeInstalledSmoke({ source, output, installation, target, verifiedFiles }) {
  const logs = join(output, 'smoke-commands'); mkdirSync(logs, { mode: 0o700 });
  const relocated = join(output, 'relocated-native-install'); renameSync(installation, relocated);
  const cli = join(relocated, 'bin', process.platform === 'win32' ? 'zryna.exe' : 'zryna');
  const projects = join(output, 'ordinary-projects'), cwd = join(output, 'unrelated-cwd');
  const emptyPath = join(output, 'empty-path');
  for (const path of [projects, cwd, emptyPath]) mkdirSync(path, { mode: 0o700 });
  const actualInventory = inventory(relocated);
  assert.deepEqual(actualInventory, verifiedFiles.map(file => file.path).sort());
  assert(actualInventory.every(path => !path.startsWith('runtime/')
    && !path.startsWith('lib/zryna/bootstrap/') && !/(^|\/)(node(?:\.exe)?|pnpm(?:\.cmd)?)$/.test(path)));
  const environment = { PATH: emptyPath };
  for (const key of ['SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'TMPDIR']) {
    if (process.env[key]) environment[key] = process.env[key];
  }
  let serial = 0;
  const invoke = (id, args) => executeRecorded(logs, String(++serial).padStart(3, '0') + '-' + id,
    cli, args, cwd, environment);
  assert.equal(successful(invoke('ordinary-version', ['--version'])).toString('utf8').trim(), 'zryna 0.2.3');
  const observations = [];
  for (const definition of cases) {
    const name = 'native-' + definition.id, project = join(projects, name);
    successful(invoke(name + '-new', ['new', project, '--json']));
    const manifestPath = join(project, 'zryna.package.json');
    const packageManifest = JSON.parse(readFileSync(manifestPath));
    packageManifest.compatibility.profile = definition.profile;
    packageManifest.files = [];
    for (const [path, text] of Object.entries(definition.files)) {
      const data = Buffer.from(text); writeFileSync(join(project, path), data);
      packageManifest.files.push({ path, size: data.length, sha256: sha256(data) });
    }
    packageManifest.files.sort((left, right) => left.path < right.path ? -1 : 1);
    writeFileSync(manifestPath, bytes(packageManifest));
    successful(invoke(name + '-resolve', ['package', 'resolve', name, '--source-root', projects,
      '--mode', 'update', '--json']));
    const profile = definition.cliProfile ? ['--profile', definition.cliProfile] : [];
    const selected = ['javascript', 'webassembly', 'native'];
    for (const selectedTarget of selected) {
      const args = ['build', 'src/main.zry', '--project-root', project, '--provider', 'native',
        '--target', selectedTarget, '--name', definition.id + '-' + selectedTarget, '--json', ...profile];
      const built = readManifest(project, invoke(definition.id + '-' + selectedTarget, args), definition, selectedTarget);
      const observed = await observeArtifact(built.artifacts, selectedTarget);
      observations.push({ id: definition.id + '-' + selectedTarget, ...observed,
        manifestSha256: sha256(built.data), artifacts: built.manifest.artifacts });
    }
    const defaultBuilt = readManifest(project, invoke(definition.id + '-native-default',
      ['build', 'src/main.zry', '--project-root', project, '--target', 'javascript',
        '--name', definition.id + '-native-default', '--json', ...profile]), definition, 'javascript');
    observations.push({ id: definition.id + '-native-default',
      ...await observeArtifact(defaultBuilt.artifacts, 'javascript'),
      manifestSha256: sha256(defaultBuilt.data), providerArgumentOmitted: true });
    for (const provider of ['bootstrap', 'compare']) {
      const stem = definition.id + '-absent-' + provider;
      const result = invoke(stem, ['build', 'src/main.zry', '--project-root', project,
        '--target', 'javascript', '--name', stem, '--provider', provider, '--json', ...profile]);
      assert.equal(result.exitCode, 2); const response = JSON.parse(result.stdoutBytes);
      assert.equal(response.ok, false); assert.equal(response.manifest, null);
      assert(!existsSync(join(project, '.zryna/out', stem + '.build')));
      observations.push({ id: stem, exitCode: result.exitCode, artifactPublicationAbsent: true });
    }
    if (process.platform === 'win32') {
      const stem = definition.id + '-unsupported-native';
      const result = invoke(stem, ['run', 'src/main.zry', '--project-root', project,
        '--export', 'main', '--target', 'native', '--name', stem, '--provider', 'native', '--json', ...profile]);
      assert.notEqual(result.exitCode, 0); const response = JSON.parse(result.stdoutBytes);
      assert.equal(response.ok, false); assert.equal(response.manifest, null);
      assert(response.diagnostics.some(item => item.code === 'ZRYNA-N4002'));
      assert(!existsSync(join(project, '.zryna/out', stem + '.run')));
      observations.push({ id: stem, exitCode: result.exitCode,
        actualHostRejection: 'ZRYNA-N4002', positiveNativeExecutionClaimed: false });
    }
    if (definition.id === 'm1' && process.platform === 'linux') {
      const result = invoke('m1-native-sdk-run', ['run', 'src/main.zry', '--project-root', project,
        '--target', 'native', '--name', 'm1-native-sdk-run', '--provider', 'native', '--export', 'main', '--json']);
      const response = JSON.parse(successful(result));
      assert.equal(response.ok, true); assert.deepEqual(response.results,
        [{ target: 'native', outcome: { kind: 'returned', value: { type: 'i32', value: 42 } } }]);
      const manifest = JSON.parse(ordinaryBytes(join(project, response.manifest), 32 * 1024 * 1024));
      assert.deepEqual(manifest.results, [{ target: 'native', outcome: { kind: 'returned', type: 'i32', value: 42 } }]);
      observations.push({ id: 'm1-native-sdk-run', actualValue: 42,
        sdk: 'unchanged driver authenticated Linux system toolchain', nodeRuntimeSelected: false });
    }
  }
  const commandH1 = runInstalledPureCommandH1({ projects, output, invoke });
  assert.deepEqual(readdirSync(emptyPath), []);
  assert.deepEqual(inventory(relocated), actualInventory);
  const result = { format: 'zryna.native-public-installation-smoke.v1',
    status: 'actual-ordinary-installed-native-cli-smoke-passed', target, source,
    installation: relocated, cli, cwd, path: emptyPath, environmentNames: Object.keys(environment).sort(),
    noNodeOrBootstrapPayload: true, actualInventory, clearedEnvironment: true,
    runtimeObservationsBySeparateCoordinator: true, observations, commandH1, failed: 0,
    privateActivationHarnessUsed: false, independentCleanMachineClaimed: false };
  saveEvidence(output, 'installed-smoke.json', result);
  return result;
}

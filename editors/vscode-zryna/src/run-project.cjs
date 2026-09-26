'use strict';

const fs = require('node:fs/promises');
const path = require('node:path');
const { createHash, randomBytes } = require('node:crypto');
const { compilerProcess, cancelled } = require('./run-process.cjs');
const { sourceText, exportsIn, argumentError } = require('./run-input.cjs');

const digest = bytes => createHash('sha256').update(bytes).digest('hex');

async function regularFile(file, limit) {
  const stat = await fs.lstat(file);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > limit) throw new Error('Unexpected run file or size.');
  const bytes = await fs.readFile(file);
  if (bytes.length > limit) throw new Error('Run file exceeds limit.');
  return bytes;
}

async function verifyOutput(project, selection, hash) {
  const source = await regularFile(path.join(project, 'src', 'main.zry'), 2 * 1024 * 1024);
  const packageManifest = JSON.parse(await regularFile(path.join(project, 'zryna.package.json'), 65536));
  const lock = JSON.parse(await regularFile(path.join(project, 'zryna.lock.json'), 1024 * 1024));
  const compatibility = packageManifest.compatibility;
  if (digest(source) !== hash || packageManifest.files?.length !== 1
    || packageManifest.files[0].path !== 'src/main.zry'
    || packageManifest.files[0].sha256 !== hash || packageManifest.files[0].size !== source.length
    || compatibility?.profile !== selection.profile || compatibility.compiler !== '0.2.3'
    || lock.format !== 'zryna.lock.v1'
    || JSON.stringify(lock.compatibility) !== JSON.stringify(compatibility)
    || !/^[a-f0-9]{64}$/.test(lock.root) || !Array.isArray(lock.packages) || !lock.packages.length) {
    throw new Error('Run project no longer matches the authenticated saved source and profile.');
  }
  const folder = path.join(project, '.zryna', 'out', 'result.run');
  for (const relative of ['.zryna', '.zryna/out', '.zryna/out/result.run', `.zryna/out/result.run/${selection.target}`]) {
    const stat = await fs.lstat(path.join(project, relative));
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('Unexpected output directory.');
  }
  const controlFlow = selection.profile === 'control-flow-v1';
  const manifest = JSON.parse(await regularFile(path.join(folder,
    controlFlow ? 'zryna-manifest-v2.json' : 'zryna-manifest-v1.json'), 1024 * 1024));
  const suffix = selection.target === 'javascript' ? 'mjs' : 'wasm';
  const relative = `${selection.target}/result.${suffix}`;
  const artifact = manifest.artifacts?.[0];
  const sourceMatches = controlFlow
    ? /^[a-f0-9]{64}$/.test(manifest.graph_sha256)
      && JSON.stringify(manifest.sources) === JSON.stringify([{ id: 0, path: 'src/main.zry', sha256: hash }])
      && JSON.stringify(manifest.edges) === '[]'
    : manifest.source_sha256 === hash;
  const argumentsMatch = JSON.stringify(manifest.invocation?.arguments) === JSON.stringify(selection.args.map(argument => ({
    type: argument.type, value: argument.type === 'bool' ? argument.value === 'true' : Number(argument.value),
  })));
  if (manifest.version !== (controlFlow ? 2 : 1) || manifest.command !== 'run'
    || manifest.profile !== (controlFlow ? 'zryna-control-flow-v1' : 'zryna-m1-cli-v1')
    || !sourceMatches || manifest.stem !== 'result'
    || manifest.entrypoint !== 'src/main.zry' || JSON.stringify(manifest.targets) !== JSON.stringify([selection.target])
    || manifest.invocation?.export !== selection.name || !argumentsMatch
    || manifest.artifacts?.length !== 1 || artifact.path !== relative || artifact.target !== selection.target
    || artifact.kind !== (selection.target === 'javascript' ? 'ecmascript-module' : 'core-webassembly-module')
    || manifest.results?.length !== 1 || manifest.results[0]?.target !== selection.target
    || (manifest.results[0]?.outcome?.kind === 'returned'
      && (manifest.results[0].outcome.type !== selection.resultType
        || (selection.resultType === 'bool' && typeof manifest.results[0].outcome.value !== 'boolean')
        || (selection.resultType === 'i32' && (!Number.isInteger(manifest.results[0].outcome.value)
          || manifest.results[0].outcome.value < -2147483648
          || manifest.results[0].outcome.value > 2147483647))))
    || !['returned', 'trapped'].includes(manifest.results[0]?.outcome?.kind)) {
    throw new Error('Compiler output does not match the selected saved-source invocation.');
  }
  const file = path.join(folder, relative);
  const bytes = await regularFile(file, 16 * 1024 * 1024);
  if (bytes.length !== artifact.bytes || digest(bytes) !== artifact.sha256) throw new Error('Compiler artifact hash mismatch.');
  return { project, selection, folder, file, target: selection.target, results: manifest.results, hash };
}

async function runProject({ compiler, storage, bytes, selection, token, report = () => {} }, call = compilerProcess) {
  if (!selection || !['i32-v1', 'control-flow-v1'].includes(selection.profile)
    || !['javascript', 'webassembly'].includes(selection.target)) throw new Error('Invalid Run selection.');
  const candidate = exportsIn(sourceText(bytes, selection.profile), selection.profile)
    .find(item => item.name === selection.name);
  if (!candidate || candidate.resultType !== selection.resultType
    || candidate.parameters.length !== selection.args?.length
    || selection.args.some((argument, index) => !argument || argument.type !== candidate.parameters[index].type
      || argumentError(argument.value, argument.type))) throw new Error('Invalid Run selection.');
  cancelled(token);
  if (!path.isAbsolute(storage)) throw new Error('Run storage must be absolute.');
  await fs.mkdir(storage, { recursive: true });
  const stat = await fs.lstat(storage);
  if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('Run storage must be a real directory.');
  const name = `run-${randomBytes(12).toString('hex')}`;
  const project = path.join(storage, name);
  const hash = digest(bytes);
  report(`Saved source SHA-256: ${hash}\nRun project: ${project}`);
  cancelled(token);
  await call(compiler, ['new', project], storage, token);
  cancelled(token);
  const manifestPath = path.join(project, 'zryna.package.json');
  const manifest = JSON.parse(await regularFile(manifestPath, 65536));
  if (manifest.format !== 'zryna.package.v1' || manifest.name !== name || manifest.files?.length !== 1
    || manifest.files[0].path !== 'src/main.zry' || manifest.dependencies?.length !== 0
    || manifest.compatibility?.compiler !== '0.2.3' || manifest.compatibility.profile !== 'i32-v1') {
    throw new Error('Expected an installed v0.2.3 scalar project scaffold.');
  }
  await regularFile(path.join(project, 'src/main.zry'), 1024);
  cancelled(token);
  await fs.writeFile(path.join(project, 'src/main.zry'), bytes);
  manifest.files[0].sha256 = hash;
  manifest.files[0].size = bytes.length;
  if (selection.profile === 'control-flow-v1') {
    manifest.compatibility.profile = 'control-flow-v1';
    manifest.compatibility.targets = ['javascript', 'webassembly'];
  }
  await fs.writeFile(manifestPath, JSON.stringify(manifest) + '\n');
  cancelled(token);
  await call(compiler, ['package', 'resolve', name, '--source-root', storage, '--mode', 'update'], storage, token);
  cancelled(token);
  const result = await call(compiler, ['run', 'src/main.zry', '--project-root', project, '--target', selection.target,
    '--export', selection.name, ...selection.args.map(argument => `--arg=${argument.type}:${argument.value}`),
    '--name', 'result', ...(selection.profile === 'control-flow-v1' ? ['--profile', 'control-flow-v1'] : [])], storage, token);
  cancelled(token);
  report(result.stdout);
  if (result.stderr) report(result.stderr);
  const verified = await verifyOutput(project, selection, hash);
  cancelled(token);
  return verified;
}

module.exports = { runProject, verifyOutput, regularFile };

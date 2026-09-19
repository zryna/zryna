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
  const folder = path.join(project, '.zryna', 'out', 'result.run');
  for (const relative of ['.zryna', '.zryna/out', '.zryna/out/result.run', `.zryna/out/result.run/${selection.target}`]) {
    const stat = await fs.lstat(path.join(project, relative));
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('Unexpected output directory.');
  }
  const manifest = JSON.parse(await regularFile(path.join(folder, 'zryna-manifest-v1.json'), 1024 * 1024));
  const suffix = selection.target === 'javascript' ? 'mjs' : 'wasm';
  const relative = `${selection.target}/result.${suffix}`;
  const artifact = manifest.artifacts?.[0];
  if (manifest.version !== 1 || manifest.command !== 'run' || manifest.profile !== 'zryna-m1-cli-v1'
    || manifest.source_sha256 !== hash || manifest.stem !== 'result'
    || manifest.entrypoint !== 'src/main.zry' || JSON.stringify(manifest.targets) !== JSON.stringify([selection.target])
    || manifest.invocation?.export !== selection.name
    || JSON.stringify(manifest.invocation.arguments) !== JSON.stringify(selection.args.map(value => ({ type: 'i32', value: Number(value) })))
    || manifest.artifacts?.length !== 1 || artifact.path !== relative || artifact.target !== selection.target) {
    throw new Error('Compiler output does not match the selected saved-source invocation.');
  }
  const file = path.join(folder, relative);
  const bytes = await regularFile(file, 16 * 1024 * 1024);
  if (bytes.length !== artifact.bytes || digest(bytes) !== artifact.sha256) throw new Error('Compiler artifact hash mismatch.');
  return { project, selection, folder, file, target: selection.target, results: manifest.results, hash };
}

async function runProject({ compiler, storage, bytes, selection, token, report = () => {} }, call = compilerProcess) {
  const candidate = exportsIn(sourceText(bytes)).find(item => item.name === selection.name);
  if (!candidate || candidate.parameters.length !== selection.args.length || selection.args.some(argumentError)
    || !['javascript', 'webassembly'].includes(selection.target)) throw new Error('Invalid Run selection.');
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
  await fs.writeFile(manifestPath, JSON.stringify(manifest) + '\n');
  cancelled(token);
  await call(compiler, ['package', 'resolve', name, '--source-root', storage, '--mode', 'update'], storage, token);
  cancelled(token);
  const result = await call(compiler, ['run', 'src/main.zry', '--project-root', project, '--target', selection.target,
    '--export', selection.name, ...selection.args.map(value => `--arg=i32:${value}`), '--name', 'result'], storage, token);
  cancelled(token);
  report(result.stdout);
  if (result.stderr) report(result.stderr);
  const verified = await verifyOutput(project, selection, hash);
  cancelled(token);
  return verified;
}

module.exports = { runProject, verifyOutput, regularFile };

import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdirSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { loadAndValidateM4Conformance, expectedRegistrySha256 } from './check-m4-conformance.mjs';
import { parseCanonical, readStableFile } from './m4/registry-files.mjs';
import { MAX_LOG, MAX_RECEIPT, exact, reject } from './m4/evidence-input.mjs';
import { selectM4Gates, resolveM4Argv } from './m4/evidence.mjs';
import { actualFile, observedSource, readObservation } from './m4/observe-context.mjs';
import { sourceCommands } from './m4/evidence-source.mjs';
import { retain, browserAttachments, installedAttachments, stageM4Evidence } from './m4/collect-output.mjs';

const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
function options(argv) {
  const allowed = ['--lane', '--evidence', '--expected-commit', '--expected-tree', '--source-ref', '--context'];
  if (argv.length !== 2 * allowed.length) reject('complete fixed runner arguments required');
  const map = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!allowed.includes(argv[index]) || map.has(argv[index])) reject('unknown or repeated runner option');
    map.set(argv[index], argv[index + 1]);
  }
  return Object.fromEntries([...map].map(([key, value]) => [key.slice(2), value]));
}
function observationReceipts(context, root) {
  return context.tools.map(tool => {
    exact(tool, ['name', 'version', 'executable', 'versionArgv', 'versionText'], 'observed tool');
    assert.deepEqual(actualFile(tool.executable.path, 512 * 1024 * 1024), tool.executable, 'actual tool did not change');
    const output = readObservation(tool.versionArgv[0], tool.versionArgv.slice(1));
    assert.equal(output, tool.versionText, 'actual version replay');
    const { versionText: ignored, ...receipt } = tool;
    return { ...receipt, versionLog: retain(root, `observations/${tool.name}.version.log`, Buffer.from(`${output}\n`)) };
  });
}
function sourceLogs(root, phase) {
  return Object.fromEntries(Object.entries(sourceCommands).map(([field, argv]) => {
    const result = spawnSync(argv[0], argv.slice(1), { cwd: sourceRoot, shell: false,
      windowsHide: true, timeout: 30_000, maxBuffer: 1024 * 1024 });
    const prefix = `source/${phase}-${field}`;
    return [field, { argv, exitCode: result.status, signal: result.signal, error: result.error?.code ?? null,
      stdout: retain(root, `${prefix}.stdout.log`, result.stdout ?? Buffer.alloc(0)),
      stderr: retain(root, `${prefix}.stderr.log`, result.stderr ?? Buffer.alloc(0)) }];
  }));
}
export async function runM4(options) {
  const root = options.evidence;
  if (!path.isAbsolute(root ?? '') || !path.isAbsolute(options.context ?? '')) reject('absolute external paths required');
  const relative = path.relative(sourceRoot, root);
  if (!relative || (!path.isAbsolute(relative) && relative.split(path.sep)[0] !== '..')) reject('evidence directory must be outside checkout');
  const context = parseCanonical(readStableFile(options.context, MAX_RECEIPT));
  exact(context, ['format', 'source', 'host', 'workflow', 'tools', 'materials'], 'observation context');
  assert.equal(context.format, 'zryna.m4-observation-context.v1');
  const source = observedSource(options['expected-commit'], options['expected-tree'], options['source-ref']);
  assert.deepEqual(context.source, source, 'observation exact candidate');
  const registry = loadAndValidateM4Conformance();
  const gates = selectM4Gates(registry, options.lane, context.host.os);
  mkdirSync(root, { mode: 0o700 });
  const tools = observationReceipts(context, root);
  const receipt = { format: 'zryna.m4-evidence.v1', registrySha256: expectedRegistrySha256,
    evidenceDirectory: root, source: { ...source, cleanBefore: true, cleanAfter: false },
    sourceState: { before: sourceLogs(root, 'before'), after: null },
    host: context.host, lane: options.lane, tools, materials: context.materials,
    workflow: context.workflow, results: [] };
  let failure;
  for (const gate of gates) {
    const argv = resolveM4Argv(gate, root, context.host.os);
    const tool = tools.find(item => item.name === argv[0]);
    if (!tool) reject('fixed gate actual executable missing');
    const launchArgv = argv[0] === 'pnpm' ? [tools[0].executable.path, tool.executable.path, ...argv.slice(1)]
      : [tool.executable.path, ...argv.slice(1)];
    console.log(`[m4 ${options.lane}] ${gate.id}`);
    const started = performance.now();
    const result = spawnSync(launchArgv[0], launchArgv.slice(1), { cwd: sourceRoot, shell: false,
      windowsHide: true, timeout: 6 * 60 * 60 * 1000, maxBuffer: MAX_LOG,
      env: { ...process.env, CARGO_TERM_COLOR: 'never', RUSTUP_TOOLCHAIN: '1.97.1',
        PATH: [path.dirname(tools[0].executable.path), path.dirname(tools[2].executable.path), process.env.PATH].join(path.delimiter) } });
    const entry = { id: gate.id, argv, launchArgv, exitCode: result.status, signal: result.signal,
      error: result.error?.code ?? null, elapsedMs: Math.max(1, Math.round(performance.now() - started)),
      stdout: retain(root, `logs/${gate.id}.stdout.log`, result.stdout ?? Buffer.alloc(0)),
      stderr: retain(root, `logs/${gate.id}.stderr.log`, result.stderr ?? Buffer.alloc(0)), attachments: [] };
    receipt.results.push(entry);
    if (result.error || result.signal || result.status !== 0) { failure = new Error(`${gate.id}: actual command failed`); break; }
    try {
      if (gate.id === 'pinned-browser-three-proof') entry.attachments = browserAttachments(root);
      if (gate.id === 'installed-qualification') entry.attachments = installedAttachments(root);
    } catch (error) { failure = error; break; }
  }
  try {
    receipt.sourceState.after = sourceLogs(root, 'after');
    assert.deepEqual(observedSource(source.commit, source.tree, source.ref), source);
    for (const tool of context.tools) assert.deepEqual(actualFile(tool.executable.path, 512 * 1024 * 1024), tool.executable);
    for (const material of context.materials) assert.deepEqual({ name: material.name, ...actualFile(material.path) }, material);
    receipt.source.cleanAfter = true;
  } catch (error) { failure ??= error; }
  writeFileSync(path.join(root, 'receipt.json'), `${JSON.stringify(receipt, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  if (failure) throw failure;
  const expected = { source, registrySha256: expectedRegistrySha256, evidenceDirectory: root,
    host: receipt.host, lane: receipt.lane, workflow: receipt.workflow, tools, materials: receipt.materials };
  await stageM4Evidence(receipt, { root, sourceRoot, registry, expected });
  console.log('M4 lane execution collected; hosted authentication, other lanes/hosts, source gaps and live publication remain separate.');
  return receipt;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runM4(options(process.argv.slice(2))).catch(error => { console.error(error.message); process.exitCode = 1; });
}

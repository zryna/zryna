import assert from 'node:assert/strict';
import { spawnSync, spawn } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync, renameSync, copyFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { verifyInstallation, hash } = require('../../editors/vscode-zryna/src/installation.cjs');
const { Connection } = require('../../editors/vscode-zryna/src/connection.cjs');
const { runProject } = require('../../editors/vscode-zryna/src/run-project.cjs');

const args = process.argv.slice(2);
if (args.length !== 6 || args[0] !== '--installation' || args[2] !== '--digest' || args[4] !== '--work') {
  throw new Error('Expected --installation <extracted candidate> --digest <reviewed manifest SHA256> --work <new directory>.');
}
const original = resolve(args[1]);
const digest = args[3];
const work = resolve(args[5]);
mkdirSync(work, { recursive: false });
verifyInstallation(original, digest);
const relocated = join(work, 'relocated setup');
renameSync(original, relocated);
const config = verifyInstallation(relocated, digest);
mkdirSync(join(work, 'empty-path'));
mkdirSync(join(work, 'temporary'));
const env = { PATH: join(work, 'empty-path'), HOME: work, USERPROFILE: work,
  TMPDIR: join(work, 'temporary'), TEMP: join(work, 'temporary'), TMP: join(work, 'temporary'),
  LANG: 'C', LC_ALL: 'C', ...(process.platform === 'win32' ? { SystemRoot: process.env.SystemRoot,
    WINDIR: process.env.SystemRoot, PATHEXT: '.EXE', COMSPEC: join(process.env.SystemRoot, 'System32/cmd.exe') } : {}) };
const command = (exe, argv, cwd = work) => {
  const result = spawnSync(exe, argv, { cwd, env, encoding: 'utf8', shell: false, windowsHide: true,
    timeout: 120000, maxBuffer: 1024 * 1024 });
  assert.equal(result.status, 0, result.stderr || result.stdout || String(result.error));
  return result.stdout.trim();
};
for (const tool of ['node', 'cargo', 'rustc', 'pnpm']) {
  const result = spawnSync(tool, ['--version'], { cwd: work, env, shell: false, windowsHide: true });
  assert.ok(result.error || result.status !== 0, `${tool} must not be found in acceptance PATH`);
}
assert.equal(command(config.compilerPath, ['--version']), 'zryna 0.2.3');
assert.equal(command(config.serverPath, ['--version']),
  `zryna-language-server 0.3.0 portable-setup-v1 ${config.manifest.sourceCommit}`);
command(config.compilerPath, ['new', 'hello']);
for (const target of ['javascript', 'webassembly']) {
  assert.equal(command(config.compilerPath, ['run', 'src/main.zry', '--project-root', 'hello',
    '--target', target, '--export', 'main', '--name', target]), `${target}: i32 42`);
}
const project = join(work, 'practice');
mkdirSync(project);
copyFileSync(join(relocated, 'examples/main.zry'), join(project, 'main.zry'));
const bytes = readFileSync(join(project, 'main.zry'));
const sourceHash = hash(bytes);
const reports = [];
const invocation = async (executable, argv, cwd) => ({ stdout: command(executable, argv, cwd), stderr: '' });
for (const [target, name, values, expected] of [['javascript', 'add', ['13', '-4'], 9],
  ['webassembly', 'double', ['13'], 26]]) {
  const result = await runProject({ compiler: config.compilerPath, storage: join(work, 'run-storage'),
    bytes, selection: { target, name, args: values }, report: line => reports.push(line) }, invocation);
  assert.ok(existsSync(result.file));
  assert.ok(reports.some(line => line.includes(`${target}: i32 ${expected}`)));
}
const notices = [];
const connection = new Connection(config, (method, params) => notices.push({ method, params }), () => {},
  (executable, argv, options) => spawn(executable, argv, { ...options, env }));
try {
  const initialize = await connection.request('initialize', {
    rootUri: pathToFileURL(project).href, capabilities: { general: { positionEncodings: ['utf-16'] } },
  });
  assert.equal(initialize.serverInfo.version, '0.3.0');
  assert.equal(initialize.capabilities.experimental.zrynaSourceCommit, config.manifest.sourceCommit);
  const uri = pathToFileURL(join(project, 'main.zry')).href;
  connection.notify('initialized', {});
  connection.notify('textDocument/didOpen', { textDocument: { uri, languageId: 'zryna', version: 1, text: bytes.toString() } });
  const edits = await connection.request('textDocument/formatting', {
    textDocument: { uri }, options: { tabSize: 2, insertSpaces: true },
  });
  assert.ok(edits.length > 0);
  const definition = await connection.request('textDocument/definition', {
    textDocument: { uri }, position: { line: 0, character: bytes.toString().indexOf('x+y') },
  });
  assert.equal(definition.uri, uri);
  connection.notify('textDocument/didChange', { textDocument: { uri, version: 2 },
    contentChanges: [{ text: 'export function bad(x:i32):i32{return missing;}\n' }] });
  try {
    const invalid = await connection.request('textDocument/formatting', {
      textDocument: { uri }, options: { tabSize: 2, insertSpaces: true },
    });
    assert.equal(invalid.length, 0);
  } catch (error) { assert.match(error.message, /ZRYNA|format|revision|rejected/i); }
  assert.ok(notices.some(value => value.method === 'textDocument/publishDiagnostics'
    && value.params.version === 2 && value.params.diagnostics.length > 0));
} finally { await connection.stop(); }
assert.equal(hash(readFileSync(join(project, 'main.zry'))), sourceHash);
assert.throws(() => verifyInstallation(relocated, '0'.repeat(64)));
const worker = join(relocated, 'compiler/lib/zryna/bootstrap/worker.mjs');
const savedWorker = readFileSync(worker);
try {
  writeFileSync(worker, 'throw new Error("substituted worker");\n');
  assert.throws(() => verifyInstallation(relocated, digest));
  const rejected = spawnSync(config.serverPath, ['--installed-root', config.compilerRoot], {
    cwd: work, env, shell: false, windowsHide: true, encoding: 'utf8', timeout: 10000,
  });
  assert.notEqual(rejected.status, 0);
  assert.match(rejected.stderr, /installed worker differs/);
} finally { writeFileSync(worker, savedWorker); }
verifyInstallation(relocated, digest);
const receipt = { format: 'zryna.portable-acceptance.v1', sourceCommit: config.manifest.sourceCommit,
  manifestSha256: digest, target: config.manifest.target, runtimePathIsolated: true,
  compilerStarter: { javascript: 42, webassembly: 42 }, editorRun: { javascript: 9, webassembly: 26 },
  formatting: true, diagnostics: true, definition: true, sourcePreserved: sourceHash, relocated: true,
  mismatchRejected: true, substitutedWorkerRejected: true,
  environment: { platform: process.platform, hostRelease: require('node:os').release(),
    independentCleanMachine: false }, installation: relocated };
writeFileSync(join(work, 'acceptance.json'), `${JSON.stringify(receipt, null, 2)}\n`, { flag: 'wx' });
console.log(JSON.stringify(receipt, null, 2));

import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { downloadAndUnzipVSCode } from '@vscode/test-electron';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const extension = join(root, 'editors', 'vscode-zryna');
const runner = join(extension, 'test', 'host-runner');
const [setupArg, digest, ...rest] = process.argv.slice(2);
const prepareTrust = rest.includes('--prepare-trust');
const executables = rest.filter(item => item !== '--prepare-trust');
if (!setupArg || !/^[a-f0-9]{64}$/.test(digest ?? '') || executables.length > 1) {
  console.error('Usage: node scripts/run-editor-host-acceptance.mjs <verified-setup-directory> <setup.json-sha256> [VS-Code-executable] [--prepare-trust]');
  process.exit(2);
}

const setup = resolve(setupArg);
const state = join(tmpdir(), `zryna-vscode-host-state-${createHash('sha256').update(root).digest('hex').slice(0, 12)}`);
const temporary = await mkdtemp(join(tmpdir(), 'zryna-editor-host-'));
const workspace = join(state, 'workspace');
const userData = join(state, 'user-data');
const extensions = join(state, 'extensions');
const resultPath = join(temporary, 'result.json');
const configPath = join(temporary, 'config.json');
const started = Date.now();
let timer;
function removeWithin(parent, target, options) {
  if (!resolve(target).startsWith(`${resolve(parent)}${sep}`)) throw new Error('Cleanup path escaped its isolated directory.');
  return rm(target, options);
}
try {
  await mkdir(workspace, { recursive: true });
  await removeWithin(state, join(workspace, 'main.zry'), { force: true });
  const actual = createHash('sha256').update(await readFile(join(setup, 'setup.json'))).digest('hex');
  if (actual !== digest) throw new Error('Setup digest differs from the supplied reviewed digest.');
  const code = executables.length ? resolve(executables[0]) : await downloadAndUnzipVSCode({
    version: '1.138.0', cachePath: join(tmpdir(), 'zryna-vscode-test-cache'),
  });
  const downloadMs = Date.now() - started;
  await writeFile(configPath, JSON.stringify({ setup, digest, resultPath, prepareTrust }));
  console.log(`Isolated test workspace: ${workspace}`);
  const args = [
    '--new-window', '--skip-welcome', '--skip-release-notes', '--disable-updates',
    `--user-data-dir=${userData}`,
    `--extensions-dir=${extensions}`,
    `--extensionDevelopmentPath=${extension}`,
    `--extensionDevelopmentPath=${runner}`,
    workspace,
  ];
  const child = spawn(code, args, {
    env: { ...process.env, ZRYNA_EDITOR_HOST_CONFIG: configPath },
    stdio: ['ignore', 'pipe', 'pipe'], windowsHide: false,
  });
  let output = '';
  for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => {
    output += bytes.toString();
    if (output.length > 100000) output = output.slice(-100000);
  });
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => { child.kill(); reject(new Error('VS Code extension-host acceptance timed out after 6 minutes.')); }, 360000);
  });
  const exit = new Promise((fulfill, reject) => {
    child.once('error', reject);
    child.once('exit', (status, signal) => fulfill({ status, signal }));
  });
  const ended = await Promise.race([exit, timeout]);
  const result = await readFile(resultPath, 'utf8').then(JSON.parse, () => null);
  console.log(JSON.stringify({ durationMs: Date.now() - started, setupAndDownloadMs: downloadMs,
    editorExit: ended, result, ...(ended.status !== 0 || !result?.passed ? { output: output.slice(-12000) } : {}) }, null, 2));
  if (ended.status !== 0 || !result?.passed) process.exitCode = 1;
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  clearTimeout(timer);
  await removeWithin(tmpdir(), temporary, { recursive: true, force: true });
  await removeWithin(state, join(workspace, 'main.zry'), { force: true });
  await removeWithin(state, join(userData, 'User', 'globalStorage', 'zryna.zryna', 'runs'),
    { recursive: true, force: true });
}
